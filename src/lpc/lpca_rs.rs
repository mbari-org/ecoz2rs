#![allow(clippy::many_single_char_names)]
extern crate serde;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;

#[allow(dead_code)]
pub fn lpca_load_input(filename: &str) -> Result<LpcaInput, Box<dyn Error>> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);
    let input = serde_cbor::from_reader(br)?;
    Ok(input)
}

/// a Rust version of lpca (equivalent to the previous commit), just for reference.
/// (the actual performance "problem" is related with lack of fast-math):
///
#[allow(dead_code)]
#[inline]
pub fn lpca(x: &[f64], p: usize, r: &mut [f64], rc: &mut [f64], a: &mut [f64]) -> (i32, f64) {
    lpca1(x, p, r, rc, a)
}

#[allow(dead_code)]
#[inline]
pub fn lpca1(x: &[f64], p: usize, r: &mut [f64], rc: &mut [f64], a: &mut [f64]) -> (i32, f64) {
    let n = x.len();

    // this is the expensive part:
    for i in 0..=p {
        let mut sum = 0.0f64;
        for k in 0..n - i {
            sum += x[k] * x[k + i];
        }
        r[i] = sum;
    }

    let mut pe: f64 = 0.;
    let r0 = r[0];
    if 0.0f64 == r0 {
        return (1, pe);
    }

    pe = r0;
    a[0] = 1.0f64;
    for k in 1..=p {
        let mut sum = 0.0f64;
        for i in 1..=k {
            sum -= a[k - i] * r[i];
        }

        let akk = sum / pe;

        rc[k] = akk;
        a[k] = akk;

        let k2 = k >> 1;

        for i in 1..=k2 {
            let ai = a[i];
            let aj = a[k - i];
            a[i] = ai + akk * aj;
            a[k - i] = aj + akk * ai;
        }

        pe *= 1.0f64 - akk * akk;
        if pe <= 0.0f64 {
            return (2, pe);
        }
    }

    (0, pe)
}

/// Like lpca1 but with use of iterators; similar performance.
#[allow(dead_code)]
#[inline]
pub fn lpca2(x: &[f64], p: usize, r: &mut [f64], rc: &mut [f64], a: &mut [f64]) -> (i32, f64) {
    let n = x.len();

    // this is the expensive part:
    for (i, r_i) in r.iter_mut().enumerate() {
        *r_i = x[0..n - i]
            .iter()
            .zip(&x[i..n])
            .map(|(&c, &s)| c * s)
            .sum::<f64>();
    }

    let mut pe: f64 = 0.;
    let r0 = r[0];
    if 0.0f64 == r0 {
        return (1, pe);
    }

    pe = r0;
    a[0] = 1.0f64;
    for k in 1..=p {
        let sum = -a[0..k]
            .iter()
            .rev()
            .zip(&r[1..=k])
            .map(|(&c, &s)| c * s)
            .sum::<f64>();

        let akk = sum / pe;

        rc[k] = akk;
        a[k] = akk;

        let k2 = k >> 1;

        // note: when k is even, we handle the "middle" element after this:
        let (a_left, a_right) = a[1..k].split_at_mut(k2);
        a_left
            .iter_mut()
            .zip(a_right.iter_mut().rev())
            .for_each(|(ai, aj)| {
                let tmp = *ai;
                *ai += akk * *aj;
                *aj += akk * tmp;
            });
        if k & 1 == 0 {
            // handle pending "overlapping" element in the middle:
            a[k2] += akk * a[k2];
        }

        pe *= 1.0f64 - akk * akk;
        if pe <= 0.0f64 {
            return (2, pe);
        }
    }

    (0, pe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lpca() {
        let input = lpca_load_input("signal_frame.inputs").unwrap();

        let prediction_order = 36;

        println!(
            "input length={}, prediction_order={}",
            input.x.len(),
            prediction_order
        );

        let mut vector1 = vec![0f64; prediction_order + 1];
        let mut reflex1 = vec![0f64; prediction_order + 1];
        let mut pred1 = vec![0f64; prediction_order + 1];
        lpca1(
            &input.x[..],
            prediction_order,
            &mut vector1,
            &mut reflex1,
            &mut pred1,
        );

        let mut vector2 = vec![0f64; prediction_order + 1];
        let mut reflex2 = vec![0f64; prediction_order + 1];
        let mut pred2 = vec![0f64; prediction_order + 1];
        lpca2(
            &input.x[..],
            prediction_order,
            &mut vector2,
            &mut reflex2,
            &mut pred2,
        );

        assert_eq!(vector1, vector2);
        assert_eq!(reflex1, reflex2);
        assert_eq!(pred1, pred2);
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LpcaInput {
    pub x: Vec<f64>,
    pub p: usize,
}

impl LpcaInput {
    fn save(&mut self, filename: &str) {
        let f = File::create(filename).unwrap();
        let bw = BufWriter::new(f);
        serde_cbor::to_writer(bw, &self).unwrap();
    }
}

pub fn lpca_save_input(x: &[f64], p: usize, filename: &str) {
    let mut input = LpcaInput { x: x.to_vec(), p };
    input.save(filename)
}
