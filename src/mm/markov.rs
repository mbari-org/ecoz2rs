extern crate itertools;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

use colored::*;
use ndarray::prelude::*;

use crate::c12n;
use crate::sequence;
use crate::serde;

const EQ_EPSILON: f64 = 1e-10;

/// A trained Markov model.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MM {
    pub class_name: String,
    pub pi: Array1<f64>,
    pub a: Array2<f64>,
}

impl MM {
    pub fn save(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        let f = File::create(filename)?;
        let bw = BufWriter::new(f);
        serde_cbor::to_writer(bw, &self)?;
        Ok(())
    }

    pub fn show(&mut self) {
        let codebook_size = self.pi.len();
        println!(
            "class_name='{}', codebook_size={}",
            self.class_name, codebook_size,
        );
        println!("pi = {}", self.pi);
        println!(" A = {}", self.a);
    }

    /// log probability of generating the symbol sequence
    pub fn log_prob_sequence(&self, seq: &sequence::Sequence) -> f64 {
        let mut p = self.pi[seq.symbols[0] as usize].log10();
        for t in 0..seq.symbols.len() - 1 {
            p += self.a[[seq.symbols[t] as usize, seq.symbols[t + 1] as usize]].log10();
        }
        p
    }
}

pub fn load(filename: &str) -> Result<MM, Box<dyn Error>> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);
    let mm = serde_cbor::from_reader(br)?;
    Ok(mm)
}

pub fn learn(seq_filenames: &Vec<PathBuf>) -> Result<MM, Box<dyn Error>> {
    // get relevant dimensions from first given sequence;
    let seq = sequence::load(seq_filenames[0].to_str().unwrap())?;

    let class_name = seq.class_name;
    let codebook_size = seq.codebook_size as usize;

    println!(
        "MM learn: num sequences={} class='{}' codebook_size={}",
        seq_filenames.len(),
        class_name,
        codebook_size
    );

    // init counters:
    let mut pi = Array1::from_elem(codebook_size, 1_f64);
    let mut n_js = Array1::from_elem(codebook_size, 0);
    let mut a = Array2::from_elem((codebook_size, codebook_size), 1_f64);
    // note: pi and are initially just counters.

    // capture counts:  (for simplicity, let this reload that 1st sequence again)
    for seq_filename in seq_filenames {
        let filename = seq_filename.to_str().unwrap();
        let seq = sequence::load(filename)?;

        //println!(" {}: '{}'", filename, seq.class_name);
        print!("{}", ".".magenta());
        std::io::stdout().flush().unwrap();

        // conformity checks:
        if codebook_size != seq.codebook_size as usize {
            return Err(format!(
                "conformity error: codebook size: {} != {}",
                codebook_size, seq.codebook_size
            )
            .into());
        }
        if class_name != seq.class_name {
            return Err(format!(
                "conformity error: class_name: {} != {}",
                class_name, seq.class_name
            )
            .into());
        }

        // update counts:
        pi[seq.symbols[0] as usize] += 1_f64;
        for jk in seq.symbols.windows(2) {
            let j = jk[0] as usize;
            let k = jk[1] as usize;
            n_js[j] += 1; // one more transition from symbol j
            a[[j, k]] += 1_f64; // one more j->k transition
        }
    }
    println!();

    let num_seqs = seq_filenames.len() as f64;

    // normalize pi:
    pi /= num_seqs + codebook_size as f64;
    assert_approx_eq!(pi.sum(), 1.0, EQ_EPSILON);

    // normalize rows in a:
    for (j, mut a_row) in a.axis_iter_mut(Axis(0)).enumerate() {
        a_row /= n_js[j] as f64 + codebook_size as f64;
        assert_approx_eq!(a_row.sum(), 1.0, EQ_EPSILON);
    }

    Ok(MM { class_name, pi, a })
}

pub fn classify(
    mm_filenames: Vec<PathBuf>,
    seq_filenames: Vec<PathBuf>,
    show_ranked: bool,
) -> Result<(), Box<dyn Error>> {
    println!("Loading MM models");
    let models: Vec<MM> = mm_filenames
        .iter()
        .map(|n| load(n.to_str().unwrap()).unwrap())
        .collect();

    let model_class_names = models.iter().map(|m| m.class_name.clone()).collect();
    let mut c12n = c12n::C12nResults::new(model_class_names);

    println!("Classifying sequences");
    for filename in seq_filenames {
        let filename = filename.to_str().unwrap();
        let seq = sequence::load(filename)?;

        let class_id_opt = &models.iter().position(|m| m.class_name == seq.class_name);
        if let Some(class_id) = *class_id_opt {
            let probs: Vec<f64> = models.iter().map(|m| m.log_prob_sequence(&seq)).collect();
            c12n.add_case(class_id, probs, show_ranked, || {
                format!("\n{}: '{}'", filename, seq.class_name)
            });
        }
    }

    println!();

    let class_names: Vec<&String> = models.iter().map(|m| &m.class_name).collect();
    c12n.report_results(class_names, "mm-classification.json".to_string());

    Ok(())
}
