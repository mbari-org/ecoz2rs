//! Rust implementation of the HMM probability kernels.
//!
//! Mirrors `ecoz2/src/hmm/hmm_prob.c`, which follows the scaling scheme in
//! Dawei Shen, "Some Mathematics for HMM" (2008).
//!
//! Everything here stays in strict IEEE, per the rule in `notes.md`: N is small
//! (3-10 states), so the inner sums have no reduction headroom worth chasing,
//! and the scaling factors are precisely what keeps the forward recursion from
//! underflowing. Parallelism belongs at the level of whole sequences.

use crate::utl::cfmt::HmmData;

/// A model prepared for repeated probability evaluations.
pub struct Hmm {
    pub class_name: String,
    pub n: usize,
    pub m: usize,
    pi: Vec<f64>,
    /// transitions, flat N*N, row-major by source state
    a: Vec<f64>,
    /// emissions, flat N*M, row-major by state
    b: Vec<f64>,
}

impl Hmm {
    pub fn from_data(d: HmmData) -> Hmm {
        let n = d.num_states();
        let m = d.num_symbols();
        Hmm {
            class_name: d.class_name,
            n,
            m,
            pi: d.pi,
            a: d.a.concat(),
            b: d.b.concat(),
        }
    }

    #[inline]
    fn b_at(&self, state: usize, symbol: u16) -> f64 {
        self.b[state * self.m + symbol as usize]
    }

    /// log P(O | λ), by the scaled forward recursion.
    ///
    /// The scale factors `c[t]` are accumulated as `-Σ ln c[t]`, which is the
    /// log-likelihood; the C uses `logl` there, which is `long double` and so
    /// carries extra precision on x86 but not on aarch64, where it is just
    /// `double`. This uses `f64::ln` consistently on every target.
    ///
    /// Returns `None` for an empty observation sequence.
    pub fn log_prob(&self, o: &[u16], work: &mut Work) -> Option<f64> {
        let t_len = o.len();
        if t_len == 0 {
            return None;
        }
        let n = self.n;
        work.ensure(n);
        let (alpha_h, alpha2) = (&mut work.alpha_h, &mut work.alpha2);

        let mut sum = 0f64;
        for (i, a2) in alpha2.iter_mut().enumerate().take(n) {
            let val = self.pi[i] * self.b_at(i, o[0]);
            *a2 = val;
            sum += val;
        }
        let mut c = 1.0f64 / sum;
        let mut log_prob = -c.ln();
        for i in 0..n {
            alpha_h[i] = c * alpha2[i];
        }

        for &symbol in &o[1..] {
            let mut sum_t = 0f64;
            for (i, a2) in alpha2.iter_mut().enumerate().take(n) {
                let mut s = 0f64;
                for (j, &ah) in alpha_h.iter().enumerate().take(n) {
                    s += ah * self.a[j * n + i];
                }
                s *= self.b[i * self.m + symbol as usize];
                *a2 = s;
                sum_t += s;
            }
            c = 1.0f64 / sum_t;
            log_prob -= c.ln();
            for i in 0..n {
                alpha_h[i] = c * alpha2[i];
            }
        }
        Some(log_prob)
    }
}

/// Scratch buffers reused across sequences.
///
/// The C keeps full `T x N` matrices per model; only the previous column is
/// ever read, so two vectors of length N suffice and the working set stays in
/// cache regardless of sequence length.
#[derive(Default)]
pub struct Work {
    alpha_h: Vec<f64>,
    alpha2: Vec<f64>,
}

impl Work {
    pub fn new() -> Work {
        Work::default()
    }
    fn ensure(&mut self, n: usize) {
        if self.alpha_h.len() < n {
            self.alpha_h = vec![0f64; n];
            self.alpha2 = vec![0f64; n];
        }
    }
}

/// Ranks models by log-probability, best (highest) first.
///
/// Matches `_sort_probs` followed by the reversed walk in `hmm_classify.c`,
/// which reads the ascending order back to front.
pub fn rank_descending(probs: &[f64]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..probs.len()).collect();
    idx.sort_by(|&a, &b| probs[a].partial_cmp(&probs[b]).unwrap());
    idx.reverse();
    idx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utl::cfmt::HmmData;

    /// A two-state model that always emits symbol 0 gives probability 1, hence
    /// log-probability 0, for any sequence of zeros.
    fn certain_model() -> Hmm {
        Hmm::from_data(HmmData {
            class_name: "X".to_string(),
            pi: vec![1.0, 0.0],
            a: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            b: vec![vec![1.0, 0.0], vec![1.0, 0.0]],
        })
    }

    #[test]
    fn a_certain_model_has_log_prob_zero() {
        let hmm = certain_model();
        let mut w = Work::new();
        let lp = hmm.log_prob(&[0, 0, 0, 0], &mut w).unwrap();
        assert!(lp.abs() < 1e-12, "expected 0, got {}", lp);
    }

    /// Halving the emission probability at each of T steps costs T*ln(2).
    #[test]
    fn log_prob_accumulates_per_observation() {
        let hmm = Hmm::from_data(HmmData {
            class_name: "X".to_string(),
            pi: vec![1.0],
            a: vec![vec![1.0]],
            b: vec![vec![0.5, 0.5]],
        });
        let mut w = Work::new();
        let lp = hmm.log_prob(&[0, 1, 0], &mut w).unwrap();
        assert!((lp - 3.0 * 0.5f64.ln()).abs() < 1e-12, "got {}", lp);
    }

    #[test]
    fn an_empty_sequence_has_no_probability() {
        let hmm = certain_model();
        let mut w = Work::new();
        assert!(hmm.log_prob(&[], &mut w).is_none());
    }

    /// The work buffers must not leak state between sequences of different length.
    #[test]
    fn reusing_the_work_buffer_gives_the_same_answer() {
        let hmm = certain_model();
        let mut shared = Work::new();
        for o in [vec![0u16; 3], vec![0u16; 17], vec![0u16; 2]] {
            let mut fresh = Work::new();
            assert_eq!(
                hmm.log_prob(&o, &mut shared).unwrap(),
                hmm.log_prob(&o, &mut fresh).unwrap()
            );
        }
    }

    #[test]
    fn ranking_puts_the_highest_log_prob_first() {
        assert_eq!(rank_descending(&[-3.0, -1.0, -2.0]), vec![1, 2, 0]);
    }
}
