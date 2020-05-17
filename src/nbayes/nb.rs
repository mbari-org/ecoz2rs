use sequence;
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct NBayes {
    pub class_name: String,
    pub codebook_size: u32,
    pub total: usize,
    pub frequencies: Vec<usize>,
}

impl NBayes {
    pub fn save(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        let f = File::create(filename)?;
        serde_cbor::to_writer(f, &self)?;
        Ok(())
    }

    pub fn show(&mut self) {
        println!(
            "# class_name='{}', M={} total={}, frequencies={}",
            self.class_name,
            self.codebook_size,
            self.total,
            self.frequencies.len(),
        );

        print!("# frequencies: ({}) = ", self.frequencies.len());
        let mut comma = "";
        for i in 0..=self.codebook_size {
            print!("{}{}", comma, i);
            comma = ", ";
        }
        println!();

        print!("# probabilities: ({}) = ", self.frequencies.len());
        let mut comma = "";
        for i in 0..=self.codebook_size {
            let prob = self.prob_symbol(i.try_into().unwrap());
            print!("{}{}", comma, prob);
            comma = ", ";
        }
        println!();
    }

    pub fn prob_symbol(&mut self, symbol: usize) -> f64 {
        let f = self.frequencies[symbol];
        let p = (f + 1) as f64 / (self.total + self.codebook_size as usize) as f64;
        return p;
    }
}

pub fn load(filename: &str) -> Result<NBayes, Box<dyn Error>> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);
    let nbayes = serde_cbor::from_reader(br)?;
    Ok(nbayes)
}

pub fn learn(sequence_filenames: Vec<PathBuf>) -> Result<NBayes, Box<dyn Error>> {
    // these two got from first given sequence:
    let mut class_name: Option<String> = None;
    let mut codebook_size: u32 = 0;

    let mut total: usize = 0;
    let mut frequencies = vec![];

    for seq_filename in sequence_filenames {
        let filename = seq_filename.to_str().unwrap();
        let seq = sequence::load(filename)?;
        println!(" {}: '{:?}'", filename, seq.class_name);
        match &class_name {
            Some(cn) => {
                if *cn != seq.class_name {
                    return Err(format!(
                        "conformity error: Not same class: {} != {}",
                        *cn, seq.class_name
                    )
                    .into());
                }
                if codebook_size != seq.codebook_size {
                    return Err(format!(
                        "conformity error: Not same codebook size: {} != {}",
                        codebook_size, seq.codebook_size
                    )
                    .into());
                }
            }

            None => {
                class_name = Some(seq.class_name);
                codebook_size = seq.codebook_size;
                frequencies = vec![0_usize; codebook_size as usize];
            }
        }

        total += seq.symbols.len();
        for s in seq.symbols {
            frequencies[s as usize] += 1;
        }
    }

    Ok(NBayes {
        class_name: class_name.unwrap(),
        codebook_size,
        total,
        frequencies,
    })
}
