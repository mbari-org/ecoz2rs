extern crate serde;

use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};

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

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LpcaInput {
    pub x: Vec<f64>,
    pub p: usize,
}

impl LpcaInput {
    fn save(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        let f = File::create(filename)?;
        let bw = BufWriter::new(f);
        serde_cbor::to_writer(bw, &self)?;
        Ok(())
    }
}

pub fn lpca_save_input(x: &Vec<f64>, p: usize, filename: &str) -> Result<(), Box<dyn Error>> {
    let mut input = LpcaInput { x: x.to_vec(), p };
    input.save(filename)
}

pub fn lpca_load_input(filename: &str) -> Result<LpcaInput, Box<dyn Error>> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);
    let inputs = serde_cbor::from_reader(br)?;
    Ok(inputs)
}
