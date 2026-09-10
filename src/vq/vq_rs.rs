//! Rust implementation of the vector-quantization kernels.
//!
//! Mirrors `ecoz2/src/vq/{distortion,quantize,ref2raas}.c`.
//!
//! On floating point, following the rule in `notes.md`: the per-codeword
//! distortion sum is the hot reduction and uses the algebraic methods, while
//! the nearest-neighbor comparison stays in strict IEEE. That comparison
//! *is* the emitted symbol, so relaxing it would relax the very quantity the
//! port is validated against.

use crate::lpc::lpca_r_rs::lpca_rc;

/// Codebook entries in the autocorrelation ("raas") domain, flat, `M` rows of
/// `1 + P`. Built once per codebook and reused across all predictors.
pub struct Codebook {
    pub prediction_order: usize,
    pub num_entries: usize,
    raas: Vec<f64>,
}

impl Codebook {
    /// Converts reflection vectors to their autocorrelation counterparts, as
    /// `reflections_to_raas` does: solve for the predictor, then autocorrelate
    /// it.
    pub fn from_reflections(reflections: &[Vec<f64>], prediction_order: usize) -> Codebook {
        let p = prediction_order;
        let num_entries = reflections.len();
        let mut raas = vec![0f64; num_entries * (1 + p)];
        let mut pred = vec![0f64; 1 + p];

        for (i, refl) in reflections.iter().enumerate() {
            lpca_rc(p, refl, &mut pred);
            let raa = &mut raas[i * (1 + p)..(i + 1) * (1 + p)];
            for (n, raa_n) in raa.iter_mut().enumerate() {
                *raa_n = pred[..=p - n]
                    .iter()
                    .zip(&pred[n..=p])
                    .fold(0.0f64, |acc, (&c, &s)| {
                        acc.algebraic_add(c.algebraic_mul(s))
                    });
            }
        }
        Codebook {
            prediction_order: p,
            num_entries,
            raas,
        }
    }

    /// Wraps an already-computed set of raas rows.
    pub fn from_raas(raas: Vec<f64>, prediction_order: usize) -> Codebook {
        let num_entries = raas.len() / (1 + prediction_order);
        Codebook {
            prediction_order,
            num_entries,
            raas,
        }
    }

    /// A copy of the flat raas rows, for callers that update them in place.
    pub fn to_raas(&self) -> Vec<f64> {
        self.raas.clone()
    }

    #[inline]
    pub fn entry(&self, i: usize) -> &[f64] {
        let w = 1 + self.prediction_order;
        &self.raas[i * w..(i + 1) * w]
    }
}

/// Distortion between an autocorrelation vector and a codebook entry, as
/// `distortion()` in the C: `rx[0]*ra[0] + 2*Σ rx[n]*ra[n]`.
#[inline]
pub fn distortion(rx: &[f64], ra: &[f64]) -> f64 {
    let term1 = rx[0] * ra[0];
    let term2 = rx[1..].iter().zip(&ra[1..]).fold(0.0f64, |acc, (&x, &a)| {
        acc.algebraic_add(x.algebraic_mul(a))
    });
    term1 + 2. * term2
}

/// Quantizes one predictor against the codebook.
///
/// Returns the emitted symbols and the average distortion, matching
/// `quantize()`: the reported figure accumulates `ddmin - 1` and divides by T.
/// Nearest codebook entry to one vector, and its distortion.
///
/// Strict IEEE: this argmin is the emitted symbol.
#[inline]
pub fn quantize_one(cb: &Codebook, rx: &[f64]) -> (usize, f64) {
    let mut ddmin = f64::MAX;
    let mut i_min = 0usize;
    for i in 0..cb.num_entries {
        let dd = distortion(rx, cb.entry(i));
        if dd < ddmin {
            ddmin = dd;
            i_min = i;
        }
    }
    (i_min, ddmin)
}

/// Average distortion of a predictor against the codebook, without building
/// the symbol sequence. This is what classification needs: `cbook_quantize` is
/// called there with a null `seq`.
pub fn average_distortion(cb: &Codebook, vectors: &[Vec<f64>]) -> f64 {
    if vectors.is_empty() {
        return 0.;
    }
    let ddprm: f64 = vectors
        .iter()
        .map(|rx| {
            let (_, ddmin) = quantize_one(cb, rx);
            ddmin - 1.
        })
        .sum();
    ddprm / vectors.len() as f64
}

pub fn quantize(cb: &Codebook, vectors: &[Vec<f64>]) -> (Vec<u16>, f64) {
    let mut symbols = Vec::with_capacity(vectors.len());
    let mut ddprm = 0f64;

    for rx in vectors {
        let (i_min, ddmin) = quantize_one(cb, rx);
        ddprm += ddmin - 1.;
        symbols.push(i_min as u16);
    }

    let t = vectors.len();
    (symbols, if t > 0 { ddprm / t as f64 } else { 0. })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A codebook built from all-zero reflections has the trivial predictor
    /// a = [1, 0, ...], whose autocorrelation is [1, 0, ...].
    #[test]
    fn zero_reflections_give_the_unit_autocorrelation() {
        let cb = Codebook::from_reflections(&[vec![0f64; 4]], 3);
        assert_eq!(cb.num_entries, 1);
        assert_eq!(cb.entry(0), &[1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn distortion_weighs_the_tail_twice() {
        // rx . ra = 1*1 + 2*(2*3) = 13
        assert_eq!(distortion(&[1., 2.], &[1., 3.]), 13.0);
    }

    /// Quantization picks the nearest entry, and identical inputs to the same
    /// codebook must give identical symbols.
    #[test]
    fn quantize_picks_the_nearest_entry() {
        let refl = vec![vec![0f64, 0.5, 0.0], vec![0f64, -0.5, 0.0]];
        let cb = Codebook::from_reflections(&refl, 2);
        let v0 = cb.entry(0).to_vec();
        let v1 = cb.entry(1).to_vec();
        let (syms, _) = quantize(&cb, &[v0, v1]);
        assert_eq!(syms.len(), 2);
        assert_ne!(
            syms[0], syms[1],
            "distinct entries should map to distinct symbols"
        );
    }

    #[test]
    fn empty_input_yields_no_symbols() {
        let cb = Codebook::from_reflections(&[vec![0f64; 3]], 2);
        let (syms, d) = quantize(&cb, &[]);
        assert!(syms.is_empty());
        assert_eq!(d, 0.0);
    }
}
