use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

// perf note: instead of using Rust impl of lpca:
//use super::lpca_rs::lpca;
// use the C impl:
use crate::ecoz2_lib::lpca_c::lpca;
use crate::prd::Predictor;
use crate::sgn;
use crate::utl;

use super::lpc_rs::create_hamming;

pub fn lpc_par(
    file: PathBuf,
    output: Option<PathBuf>,
    prediction_order: usize,
    window_length_ms: usize,
    offset_length_ms: usize,
) {
    let filename: &str = file.to_str().unwrap();
    let out_filename: &str = match output {
        Some(ref fname) => &fname.to_str().unwrap(),
        None => "predictor_par.prd",
    };

    println!("Loading: {}", filename);
    let s = sgn::load(&filename);
    s.show();
    //sgn::save(&s, "output.wav");

    let before = Instant::now();
    let vectors = lpa_on_signal(prediction_order, window_length_ms, offset_length_ms, &s).unwrap();
    let elapsed = before.elapsed();
    if elapsed.as_secs() > 5 {
        println!("processing took: {:.2?}", elapsed);
    }

    let class_name = "_".to_string();
    let predictor = Predictor {
        class_name,
        prediction_order,
        vectors,
    };

    utl::save_ser(&predictor, &out_filename).unwrap();
    println!(
        "{} saved.  Class: '{}':  {} vectors",
        out_filename,
        &predictor.class_name,
        &predictor.vectors.len()
    );
}

struct LPAnalyzerPar {
    pub prediction_order: usize,
    pub win_size: usize,

    reflex: Vec<f64>,
    pred: Vec<f64>,
    frame: Vec<f64>,
}

impl LPAnalyzerPar {
    fn new(prediction_order: usize, win_size: usize) -> LPAnalyzerPar {
        let reflex = vec![0f64; prediction_order + 1]; // reflection coefficients
        let pred = vec![0f64; prediction_order + 1]; // prediction coefficients

        // perform linear prediction to each frame:
        let frame = vec![0f64; win_size];

        LPAnalyzerPar {
            prediction_order,
            win_size,
            reflex,
            pred,
            frame,
        }
    }

    #[inline]
    pub fn process_frame(
        &mut self,
        samples: &[f64],
        hamming: &[f64],
        mut vector: &mut [f64],
    ) -> Result<(), Box<dyn Error>> {
        self.fill_frame(&samples);
        self.remove_mean();
        self.preemphasis();
        self.apply_hamming(hamming);

        let (res_lpca, err_pred) = lpca(
            &self.frame,
            self.prediction_order,
            &mut vector,
            &mut self.reflex,
            &mut self.pred,
        );
        if res_lpca == 0 {
            // normalize autocorrelation sequence by gain:
            if err_pred != 0. {
                for elem in vector.iter_mut() {
                    *elem /= err_pred;
                }
            }
            Ok(())
        } else {
            Err(format!(
                "ERROR: lpa_on_signal: res_lpca = {},  err_pred = {}",
                res_lpca, err_pred
            )
            .into())
        }
    }

    #[inline]
    fn fill_frame(&mut self, from: &[f64]) {
        self.frame.copy_from_slice(from);
    }

    #[inline]
    fn remove_mean(&mut self) {
        let sum: f64 = self.frame.iter().sum();
        let mean = sum / self.frame.len() as f64;
        for elem in self.frame.iter_mut() {
            *elem -= mean;
        }
    }

    #[inline]
    fn preemphasis(&mut self) {
        for n in (1..self.frame.len()).rev() {
            self.frame[n] -= 0.95 * self.frame[n - 1];
        }
    }

    #[inline]
    fn apply_hamming(&mut self, hamming: &[f64]) {
        for (elem, h) in self.frame.iter_mut().zip(hamming) {
            *elem *= *h
        }
    }
}

pub fn lpa_on_signal(
    p: usize,
    window_length_ms: usize,
    offset_length_ms: usize,
    s: &sgn::Sgn,
) -> Option<Vec<Vec<f64>>> {
    let num_samples: usize = s.num_samples;
    let sample_rate: usize = s.sample_rate;

    let signal = Arc::new({
        let mut x = vec![0f64; num_samples];
        x.clone_from_slice(&s.samples[..num_samples]);
        x
    });

    // number of samples corresponding to window_length_ms:
    let win_size = (window_length_ms * sample_rate) / 1000;

    // number of samples corresponding to offset_length_ms:
    let offset = (offset_length_ms * sample_rate) / 1000;

    if win_size > num_samples {
        eprintln!("ERROR: lpa_on_signal: signal too short\n");
        return None;
    }

    // total number of frames:
    let mut num_frames = (num_samples - (win_size - offset)) / offset;
    // discard last section if incomplete:
    if (num_frames - 1) * offset + win_size > num_samples {
        num_frames -= 1;
    }

    let cores: usize = num_cpus::get();

    println!(
        "lpa_on_signal: p={} numSamples={} sampleRate={} winSize={} offset={} num_frames={} (cores={})",
        p, num_samples, sample_rate, win_size, offset, num_frames, cores
    );

    let hamming: Arc<Vec<_>> = Arc::new(create_hamming(win_size));

    let (tx, rx) = mpsc::channel();

    let mut children = vec![];

    let frames_per_thread = num_frames / cores;
    let extra_frames_last_thread = num_frames % cores;

    for th in 0..cores {
        let c_hamming = hamming.clone();
        let c_tx = tx.clone();

        let mut lpa = LPAnalyzerPar::new(p, win_size);

        let signal = signal.clone();

        let handle = thread::spawn(move || {
            let frame_low = th * frames_per_thread;
            let frame_upp = frame_low + frames_per_thread + {
                if th == cores - 1 {
                    extra_frames_last_thread
                } else {
                    0
                }
            };

            for f in frame_low..frame_upp {
                let signal_from = f * offset;
                let signal_to = signal_from + win_size;
                let samples = &signal[signal_from..signal_to];

                let mut vector = vec![0f64; p + 1];
                lpa.process_frame(&samples, &c_hamming, &mut vector)
                    .unwrap();
                c_tx.send((f, vector)).unwrap();
            }
        });
        children.push(handle);
    }

    drop(tx);

    for child in children {
        child.join().unwrap();
    }

    let mut vectors = vec![vec![0f64; 0]; num_frames];
    for (f, vector) in &rx {
        vectors[f] = vector;
    }

    println!("  PAR lpa_on_signal complete: {} vectors", vectors.len());

    Some(vectors)
}
