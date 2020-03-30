extern crate serde;
use serde::{Deserialize, Serialize};

extern crate structopt;
use std::path::PathBuf;
use structopt::StructOpt;

use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(StructOpt, Debug)]
pub struct PrdShowOpts {
    /// File to read
    #[structopt(short, long, parse(from_os_str))]
    file: PathBuf,
}

pub fn main_prd_show(opts: PrdShowOpts) {
    let PrdShowOpts { file } = opts;

    let filename: &str = file.to_str().unwrap();

    let mut predictor = load(&filename).unwrap();
    println!("Predictor loaded: {}", filename);
    &predictor.show();
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Predictor {
    pub class_name: String,
    pub prediction_order: usize,
    pub vectors: Vec<Vec<f64>>,
}

impl Predictor {
    pub fn save(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        let f = File::create(filename)?;
        let bw = BufWriter::new(f);
        serde_cbor::to_writer(bw, &self)?;
        Ok(())
    }

    pub fn show(&mut self) {
        println!(
            " class_name = '{}' prediction_order: {} sequences: {}",
            self.class_name,
            self.prediction_order,
            self.vectors.len(),
        );
    }
}

pub fn load(filename: &str) -> Result<Predictor, Box<dyn Error>> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);
    let predictor = serde_cbor::from_reader(br)?;
    Ok(predictor)
}
