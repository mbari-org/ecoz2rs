use std::error::Error;
use std::f64::consts::PI;
use std::path::PathBuf;
use std::time::Instant;

use prd::Predictor;
use sgn;

pub fn lpc_rs(
    file: PathBuf,
    output: Option<PathBuf>,
    prediction_order: usize,
    window_length_ms: usize,
    offset_length_ms: usize,
) {
    let filename: &str = file.to_str().unwrap();

    let out_filename: &str = match output {
        Some(ref fname) => &fname.to_str().unwrap(),
        None => "predictor.prd",
    };

    println!("Loading: {}", filename);
    let s = sgn::load(&filename);
    &s.show();
    //sgn::save(&s, "output.wav");

    let before = Instant::now();
    let vectors = lpa_on_signal(prediction_order, window_length_ms, offset_length_ms, &s).unwrap();
    let elapsed = before.elapsed();
    if elapsed.as_secs() > 5 {
        println!("processing took: {:.2?}", elapsed);
    }

    let class_name = "_".to_string();
    let mut predictor = Predictor {
        class_name,
        prediction_order,
        vectors,
    };

    &predictor.save(out_filename).unwrap();
    println!(
        "{} saved.  Class: '{}':  {} vectors",
        out_filename,
        &predictor.class_name,
        &predictor.vectors.len()
    );
}

struct LPAnalyzerSer {
    pub prediction_order: usize,
    pub win_size: usize,

    hamming: Vec<f64>,
    reflex: Vec<f64>,
    pred: Vec<f64>,
    frame: Vec<f64>,
}

pub fn create_hamming(win_size: usize) -> Vec<f64> {
    (0..win_size)
        .map(|n| 0.54 - 0.46 * (((n * 2) as f64 * PI) / (win_size - 1) as f64).cos())
        .collect::<Vec<_>>()
}

impl LPAnalyzerSer {
    fn new(prediction_order: usize, win_size: usize) -> LPAnalyzerSer {
        let hamming = create_hamming(win_size);

        let reflex = vec![0f64; prediction_order + 1]; // reflection coefficients
        let pred = vec![0f64; prediction_order + 1]; // prediction coefficients

        // perform linear prediction to each frame:
        let frame = vec![0f64; win_size];

        LPAnalyzerSer {
            prediction_order,
            win_size,
            hamming,
            reflex,
            pred,
            frame,
        }
    }

    #[inline]
    fn process_frame(
        &mut self,
        samples: &[i32],
        mut vector: &mut [f64],
    ) -> Result<(), Box<dyn Error>> {
        self.fill_frame(&samples);
        self.remove_mean();
        self.preemphasis();
        self.apply_hamming();

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
            ))?
        }
    }

    #[inline]
    fn fill_frame(&mut self, from: &[i32]) {
        for (n, elem) in self.frame.iter_mut().enumerate() {
            *elem = f64::from(from[n]);
        }
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
    fn apply_hamming(&mut self) {
        for (elem, h) in self.frame.iter_mut().zip(&self.hamming) {
            *elem *= *h
        }
    }
}

fn lpa_on_signal(
    p: usize,
    window_length_ms: usize,
    offset_length_ms: usize,
    s: &sgn::Sgn,
) -> Option<Vec<Vec<f64>>> {
    let signal = &s.samples;
    let num_samples: usize = s.num_samples;
    let sample_rate: usize = s.sample_rate;

    // number of samples corresponding to window_length_ms:
    let win_size = (window_length_ms * sample_rate) / 1000;

    // number of samples corresponding to offset_length_ms:
    let offset = (offset_length_ms * sample_rate) / 1000;

    if win_size > num_samples {
        eprintln!("ERROR: lpa_on_signal: signal too short\n");
        return None;
    }

    let mut lpa = LPAnalyzerSer::new(p, win_size);

    // total number of frames:
    let mut num_frames = (num_samples - (win_size - offset)) / offset;
    // discard last section if incomplete:
    if (num_frames - 1) * offset + win_size > num_samples {
        num_frames -= 1;
    }

    println!(
        "lpa_on_signal: p={} numSamples={} sampleRate={} winSize={} offset={} num_frames={}",
        p, num_samples, sample_rate, win_size, offset, num_frames
    );

    let mut vectors = vec![vec![0f64; p + 1]; num_frames];

    // perform linear prediction to each frame:
    let mut frames_processed = 0;

    for (t, mut vector) in vectors.iter_mut().enumerate() {
        let signal_from = t * offset;
        let signal_to = signal_from + win_size;
        let samples = &signal[signal_from..signal_to];

        lpa.process_frame(&samples, &mut vector).unwrap();
        frames_processed += 1;
        if frames_processed % 15000 == 0 {
            println!("  {} frames processed", frames_processed);
        }
    }
    println!("  {} total frames processed", frames_processed);

    println!("  SER lpa_on_signal complete: {} vectors", vectors.len());

    Some(vectors)
}

#[inline]
pub fn lpca(x: &[f64], p: usize, r: &mut [f64], rc: &mut [f64], a: &mut [f64]) -> (i32, f64) {
    let n = x.len();

    let mut pe: f64 = 0.;

    let mut i = 0;
    while i <= p {
        let mut sum = 0.0f64;
        let mut k = 0;
        while k < n - i {
            sum += x[k] * x[k + i];
            k += 1
        }
        r[i] = sum;
        i += 1
    }
    let r0 = r[0];
    if 0.0f64 == r0 {
        return (1, pe);
    }

    pe = r0;
    a[0] = 1.0f64;
    let mut k = 1;
    while k <= p {
        let mut sum = 0.0f64;
        i = 1;
        while i <= k {
            sum -= a[k - i] * r[i];
            i += 1
        }
        let akk = sum / pe;
        rc[k] = akk;

        a[k] = akk;
        i = 1;
        while i <= k >> 1 {
            let ai = a[i];
            let aj = a[k - i];
            a[i] = ai + akk * aj;
            a[k - i] = aj + akk * ai;
            i += 1
        }

        pe *= 1.0f64 - akk * akk;
        if pe <= 0.0f64 {
            return (2, pe);
        }
        k += 1
    }

    (0, pe)
}
