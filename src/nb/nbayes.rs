use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::path::PathBuf;

use colored::*;

use crate::c12n;
use crate::sequence;

/// A trained NBayes model.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct NBayes {
    pub class_name: String,
    pub total_symbols: usize,
    pub frequencies: Vec<usize>,
}

impl NBayes {
    pub fn show(&mut self) {
        let codebook_size = self.frequencies.len();
        println!(
            "# class_name='{}', M={} total_symbols={}",
            self.class_name, codebook_size, self.total_symbols,
        );

        println!("{:4}, {:4}, prob", "m", "frequency");

        for (s, f) in self.frequencies.iter().enumerate() {
            let prob = self.prob_symbol(s);
            println!("{:4}, {:4}, {:.7}", s, f, prob);
        }
    }

    /// probability of generating the symbol, using an m-estimate
    pub fn prob_symbol(&self, symbol: usize) -> f64 {
        let codebook_size = self.frequencies.len();
        let f = self.frequencies[symbol] as f64;
        (f + 1.0) / (self.total_symbols + codebook_size) as f64
    }

    /// log probability of generating the symbol
    pub fn log_prob_symbol(&self, symbol: usize) -> f64 {
        self.prob_symbol(symbol).log10()
    }

    /// log probability of generating the symbol sequence
    pub fn log_prob_sequence(&self, seq: &sequence::Sequence) -> f64 {
        seq.symbols.iter().fold(0_f64, |acc, s| {
            acc + self.log_prob_symbol(*s as usize) as f64
        })
    }
}

pub fn load(filename: &str) -> Result<NBayes, Box<dyn Error>> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);
    let nbayes = serde_cbor::from_reader(br)?;
    Ok(nbayes)
}

pub fn learn(codebook_size: usize, seq_filenames: Vec<PathBuf>) -> Result<NBayes, Box<dyn Error>> {
    // get class name from first sequence
    let seq = sequence::load(seq_filenames[0].to_str().unwrap())?;
    let class_name = seq.class_name;

    println!(
        "NB learn: num sequences={} class='{}' codebook_size={}",
        seq_filenames.len(),
        class_name,
        codebook_size
    );

    let mut total_symbols: usize = 0;
    let mut frequencies = vec![0_usize; seq.codebook_size as usize];

    // for simplicity, let this reload that 1st sequence again
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

        total_symbols += seq.symbols.len();
        for s in seq.symbols {
            frequencies[s as usize] += 1;
        }
    }
    println!();

    Ok(NBayes {
        class_name,
        total_symbols,
        frequencies,
    })
}

pub fn classify(
    nb_filenames: Vec<PathBuf>,
    seq_filenames: Vec<PathBuf>,
    show_ranked: bool,
    codebook_size: usize,
) -> Result<(), Box<dyn Error>> {
    println!("Loading NBayes models");
    let models: Vec<NBayes> = nb_filenames
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
            c12n.add_case(class_id, &seq.class_name, probs, show_ranked, || {
                format!("\n{}: '{}'\n", filename, seq.class_name)
            });
        }
    }

    println!();

    let class_names: Vec<&String> = models.iter().map(|m| &m.class_name).collect();
    let out_base_name = format!("nb_{}", codebook_size);
    c12n.report_results(class_names, out_base_name);

    Ok(())
}
