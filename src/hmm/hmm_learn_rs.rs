//! Rust implementation of HMM training.
//!
//! Mirrors `ecoz2/src/hmm/{hmm_learn.c, hmm_refinement.c, hmm_estimateB.c,
//! hmm_adjustb.c, hmm_genQopt.c, distr.c}` — Baum-Welch re-estimation over
//! multiple sequences, with the scaling of Rabiner / Stamp / Shen.
//!
//! Strict IEEE throughout, per the rule in `notes.md`, and measured rather than
//! assumed: switching these loops to the algebraic methods gives no speedup at
//! all, and relaxing every one of them is ~4% *slower* at N=20. The alpha, beta
//! and gamma passes are dependent recursions, not reductions — splitting a
//! chain that short into partial sums costs more setup than it saves — and the
//! scaling factors are what keep them from underflowing.
//!
//! **On reproducing the C exactly.** Model type 1 (uniform) uses no randomness
//! at all, so that path is directly comparable with the C. The other types seed
//! `pi`, `A` and the fallback rows from `rand()`, which is libc-specific; this
//! uses a specified PRNG instead, so those runs agree with the C in behavior
//! rather than bit for bit. That is the deliberate trade recorded under
//! "Determinism" in notes.md — the C's own seeded runs are already not portable.

use std::error::Error;
use std::path::Path;

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use crate::utl::cfmt;

/// Model initialization modes, as `enum hmm_modes_init`. Mode 0 (fully random)
/// is the `_` arm of the matches below rather than a named constant.
pub const UNIFORM: usize = 1;
pub const CASCADE2: usize = 2;
pub const CASCADE3: usize = 3;

// --- distributions (distr.c) ------------------------------------------------

fn dis_set_uniform(dis: &mut [f64]) {
    let len = dis.len();
    let val = 1.0 / len as f64;
    let mut cumulative = 0.0;
    for d in dis.iter_mut().take(len - 1) {
        *d = val;
        cumulative += val;
    }
    dis[len - 1] = 1.0 - cumulative;
}

/// Random distribution with no zero values: draws in [1, 2), then normalizes.
fn dis_set_random(dis: &mut [f64], rng: &mut StdRng) {
    let mut cumulative = 0.0;
    for d in dis.iter_mut() {
        *d = rng.random::<f64>() + 1.0;
        cumulative += *d;
    }
    for d in dis.iter_mut() {
        *d /= cumulative;
    }
}

/// Left-to-right distribution: zeros everywhere except a random block of at
/// most `delta` values starting at `from`.
fn dis_set_random_delta(dis: &mut [f64], from: usize, delta: usize, rng: &mut StdRng) {
    for d in dis.iter_mut() {
        *d = 0.0;
    }
    let numval = (dis.len() - from).min(delta);
    if numval > 0 {
        dis_set_random(&mut dis[from..from + numval], rng);
    }
}

// --- the model --------------------------------------------------------------

pub struct HmmModel {
    pub class_name: String,
    pub n: usize,
    pub m: usize,
    pub pi: Vec<f64>,
    pub a: Vec<Vec<f64>>,
    pub b: Vec<Vec<f64>>,
}

impl HmmModel {
    pub fn new(class_name: &str, n: usize, m: usize) -> HmmModel {
        HmmModel {
            class_name: class_name.to_string(),
            n,
            m,
            pi: vec![0.0; n],
            a: vec![vec![0.0; n]; n],
            b: vec![vec![0.0; m]; n],
        }
    }

    pub fn init(&mut self, mode: usize, rng: &mut StdRng) {
        match mode {
            UNIFORM => {
                dis_set_uniform(&mut self.pi);
                for i in 0..self.n {
                    dis_set_uniform(&mut self.a[i]);
                    dis_set_uniform(&mut self.b[i]);
                }
            }
            CASCADE2 | CASCADE3 => {
                dis_set_random_delta(&mut self.pi, 0, 1, rng);
                for i in 0..self.n {
                    dis_set_random_delta(&mut self.a[i], i, mode, rng);
                    dis_set_random(&mut self.b[i], rng);
                }
            }
            // RANDOM, and anything else, as the C's default arm
            _ => {
                dis_set_random(&mut self.pi, rng);
                for i in 0..self.n {
                    dis_set_random(&mut self.a[i], rng);
                    dis_set_random(&mut self.b[i], rng);
                }
            }
        }
    }

    fn init_a_row(&mut self, i: usize, mode: usize, rng: &mut StdRng) {
        match mode {
            UNIFORM => dis_set_uniform(&mut self.a[i]),
            CASCADE2 | CASCADE3 => dis_set_random_delta(&mut self.a[i], i, mode, rng),
            _ => dis_set_random(&mut self.a[i], rng),
        }
    }

    /// Raises every emission below `epsilon` to it, taking the difference
    /// proportionally from the entries above, so each row still sums to 1.
    pub fn adjust_b_epsilon(&mut self, epsilon: f64) {
        if epsilon <= 0.0 {
            return;
        }
        for row in self.b.iter_mut() {
            let mut total_added = 0.0;
            let mut total_extra = 0.0;
            for v in row.iter_mut() {
                if *v < epsilon {
                    total_added += epsilon - *v;
                    *v = epsilon;
                } else if *v > epsilon {
                    total_extra += *v - epsilon;
                }
            }
            if total_added != 0.0 {
                for v in row.iter_mut() {
                    if *v > epsilon {
                        let fraction = (*v - epsilon) / total_extra;
                        *v -= total_added * fraction;
                    }
                }
            }
        }
    }

    pub fn to_data(&self) -> cfmt::HmmData {
        cfmt::HmmData {
            class_name: self.class_name.clone(),
            pi: self.pi.clone(),
            a: self.a.clone(),
            b: self.b.clone(),
        }
    }
}

// --- Viterbi (hmm_genQopt.c) ------------------------------------------------

/// Most likely state sequence, in the log domain. Returns its log-probability
/// and fills `q_opt` when given.
#[allow(clippy::too_many_arguments)]
fn gen_q_opt(
    model: &HmmModel,
    o: &[u16],
    q_opt: Option<&mut [usize]>,
    log_a: &[Vec<f64>],
    log_b: &[Vec<f64>],
    phi: &mut [Vec<f64>],
    psi: &mut [Vec<usize>],
) -> f64 {
    let n = model.n;
    let t_len = o.len();

    for i in 0..n {
        phi[0][i] = model.pi[i].ln() + log_b[i][o[0] as usize];
        psi[0][i] = 0;
    }

    for t in 1..t_len {
        for j in 0..n {
            let mut max_tj = 0.0f64;
            let mut arg_max = 0usize;
            for i in 0..n {
                let val = phi[t - 1][i] + log_a[i][j];
                if i == 0 || max_tj < val {
                    max_tj = val;
                    arg_max = i;
                }
            }
            phi[t][j] = max_tj + log_b[j][o[t] as usize];
            psi[t][j] = arg_max;
        }
    }

    let mut log_prob = 0.0f64;
    let mut arg_max = 0usize;
    for (i, &val) in phi[t_len - 1].iter().enumerate() {
        if i == 0 || log_prob < val {
            log_prob = val;
            arg_max = i;
        }
    }

    if let Some(q) = q_opt {
        q[t_len - 1] = arg_max;
        for t in (0..t_len - 1).rev() {
            q[t] = psi[t + 1][q[t + 1]];
        }
    }
    log_prob
}

/// Initial B from symbol frequencies along each sequence's Viterbi path.
fn estimate_b(model: &mut HmmModel, seqs: &[Vec<u16>], epsilon: f64) {
    let (n, m) = (model.n, model.m);
    let max_t = seqs.iter().map(|s| s.len()).max().unwrap_or(0);
    if max_t == 0 {
        return;
    }

    let log_a: Vec<Vec<f64>> = model
        .a
        .iter()
        .map(|row| row.iter().map(|v| v.ln()).collect())
        .collect();
    let mut log_b = vec![vec![0f64; m]; n];
    let mut phi = vec![vec![0f64; n]; max_t];
    let mut psi = vec![vec![0usize; n]; max_t];
    let mut q_opt = vec![0usize; max_t];

    let mut num = vec![vec![0usize; m]; n];
    let mut den = vec![0usize; n];

    for o in seqs {
        // Only the symbols actually present are needed, as the C fills.
        for (lb, b_row) in log_b.iter_mut().zip(&model.b) {
            for &k in o {
                lb[k as usize] = b_row[k as usize].ln();
            }
        }
        gen_q_opt(
            model,
            o,
            Some(&mut q_opt[..o.len()]),
            &log_a,
            &log_b,
            &mut phi,
            &mut psi,
        );
        for (t, &sym) in o.iter().enumerate() {
            let state = q_opt[t];
            num[state][sym as usize] += 1;
            den[state] += 1;
        }
    }

    let mut num_not_emitting = 0;
    for j in 0..n {
        if den[j] == 0 {
            dis_set_uniform(&mut model.b[j]);
            num_not_emitting += 1;
        } else {
            for (b, n) in model.b[j].iter_mut().zip(&num[j]) {
                *b = *n as f64 / den[j] as f64;
            }
        }
    }
    println!("num_not_emitting_states={}", num_not_emitting);
    model.adjust_b_epsilon(epsilon);
}

// --- Baum-Welch (hmm_refinement.c) ------------------------------------------

struct Refiner {
    alpha: Vec<Vec<f64>>,
    beta: Vec<Vec<f64>>,
    c: Vec<f64>,
    gamma1: Vec<Vec<f64>>,
    gamma2: Vec<Vec<Vec<f64>>>,
    num_a: Vec<Vec<f64>>,
    den_a: Vec<f64>,
    num_b: Vec<Vec<f64>>,
    den_b: Vec<f64>,
}

impl Refiner {
    fn new(n: usize, m: usize, max_t: usize) -> Refiner {
        Refiner {
            alpha: vec![vec![0f64; n]; max_t],
            beta: vec![vec![0f64; n]; max_t],
            c: vec![0f64; max_t],
            gamma1: vec![vec![0f64; n]; max_t],
            gamma2: vec![vec![vec![0f64; n]; n]; max_t],
            num_a: vec![vec![0f64; n]; n],
            den_a: vec![0f64; n],
            num_b: vec![vec![0f64; m]; n],
            den_b: vec![0f64; n],
        }
    }

    fn init_counters(&mut self) {
        for row in self.num_a.iter_mut() {
            row.fill(0.0);
        }
        self.den_a.fill(0.0);
        for row in self.num_b.iter_mut() {
            row.fill(0.0);
        }
        self.den_b.fill(0.0);
    }

    /// Scaled alpha and beta passes, then the gammas.
    fn generate_gammas(&mut self, model: &HmmModel, o: &[u16]) {
        let t_len = o.len();
        let (pi, a, b) = (&model.pi, &model.a, &model.b);
        let Refiner {
            alpha,
            beta,
            c,
            gamma1,
            gamma2,
            ..
        } = self;

        // alpha[0]
        let mut c0 = 0.0;
        for ((a0, &pi_i), b_row) in alpha[0].iter_mut().zip(pi).zip(b) {
            let val = pi_i * b_row[o[0] as usize];
            *a0 = val;
            c0 += val;
        }
        c[0] = 1.0 / c0;
        for a0 in alpha[0].iter_mut() {
            *a0 *= c[0];
        }

        // alpha[t], from alpha[t-1]: split so the previous row can be read
        // while the current one is written.
        for t in 1..t_len {
            let (before, from_t) = alpha.split_at_mut(t);
            let prev = &before[t - 1];
            let cur = &mut from_t[0];
            let sym = o[t] as usize;

            let mut ct = 0.0;
            for (i, (a_ti, b_row)) in cur.iter_mut().zip(b).enumerate() {
                // a[j][i] is a column of A: the transition *into* state i.
                let mut acc = 0.0;
                for (&prev_j, a_row) in prev.iter().zip(a) {
                    acc += prev_j * a_row[i];
                }
                acc *= b_row[sym];
                *a_ti = acc;
                ct += acc;
            }
            c[t] = 1.0 / ct;
            for a_ti in cur.iter_mut() {
                *a_ti *= c[t];
            }
        }

        // beta, scaled by the same factors as alpha
        let c_last = c[t_len - 1];
        for bt in beta[t_len - 1].iter_mut() {
            *bt = c_last;
        }
        for t in (0..t_len - 1).rev() {
            let (upto_t, from_next) = beta.split_at_mut(t + 1);
            let cur = &mut upto_t[t];
            let next = &from_next[0];
            let (sym, ct) = (o[t + 1] as usize, c[t]);

            for (bt, a_row) in cur.iter_mut().zip(a) {
                let mut acc = 0.0;
                for ((&a_ij, b_row), &beta_next) in a_row.iter().zip(b).zip(next) {
                    acc += a_ij * b_row[sym] * beta_next;
                }
                *bt = acc * ct;
            }
        }

        // gammas need no normalization, alpha and beta being scaled already
        for (t, g2_t) in gamma2.iter_mut().enumerate().take(t_len - 1) {
            let sym = o[t + 1] as usize;
            let (alpha_t, beta_next, g1_t) = (&alpha[t], &beta[t + 1], &mut gamma1[t]);

            for (i, g2_ti) in g2_t.iter_mut().enumerate() {
                let mut g1 = 0.0;
                for (j, g2) in g2_ti.iter_mut().enumerate() {
                    let v = alpha_t[i] * a[i][j] * b[j][sym] * beta_next[j];
                    *g2 = v;
                    g1 += v;
                }
                g1_t[i] = g1;
            }
        }
        gamma1[t_len - 1].copy_from_slice(&alpha[t_len - 1]);
    }

    /// Folds one sequence's gammas into the numerators and denominators.
    ///
    /// Unlike the passes above these *are* long reductions over independent
    /// terms — T is in the hundreds — so by the rule in `notes.md` they would be
    /// candidates for the algebraic methods. Measured, they gain nothing, and
    /// the likely reason is the layout: `gamma1[t][i]` and `gamma2[t][i][j]`
    /// stride across separate `Vec` allocations, so the reduction chases
    /// pointers and cannot vectorize. Flattening those (or moving to `ndarray`)
    /// is the prerequisite to re-testing this.
    fn accumulate(&mut self, n: usize, m: usize, o: &[u16]) {
        let t_len = o.len();
        for i in 0..n {
            let mut denom = 0.0;
            for t in 0..t_len - 1 {
                denom += self.gamma1[t][i];
            }
            self.den_a[i] += denom;
            for j in 0..n {
                let mut numer = 0.0;
                for t in 0..t_len - 1 {
                    numer += self.gamma2[t][i][j];
                }
                self.num_a[i][j] += numer;
            }
        }
        for i in 0..n {
            let mut denom = 0.0;
            for t in 0..t_len {
                denom += self.gamma1[t][i];
            }
            self.den_b[i] += denom;
            // The C scans all M symbols per state; only those present can
            // contribute, so accumulate by observation instead.
            for (t, &sym) in o.iter().enumerate() {
                self.num_b[i][sym as usize] += self.gamma1[t][i];
            }
            let _ = m;
        }
    }

    fn step(
        &mut self,
        model: &mut HmmModel,
        seqs: &[Vec<u16>],
        mode: usize,
        eps: f64,
        rng: &mut StdRng,
    ) {
        self.init_counters();
        for o in seqs {
            self.generate_gammas(model, o);
            self.accumulate(model.n, model.m, o);
        }

        let (n, m) = (model.n, model.m);
        let mut zero_dens = 0;
        for i in 0..n {
            if self.den_a[i] != 0.0 {
                for j in 0..n {
                    model.a[i][j] = self.num_a[i][j] / self.den_a[i];
                }
            } else {
                zero_dens += 1;
                model.init_a_row(i, mode, rng);
            }
        }
        if zero_dens > 0 {
            print!("!({})", zero_dens);
        }

        let mut zero_dens = 0;
        for i in 0..n {
            if self.den_b[i] != 0.0 {
                for k in 0..m {
                    model.b[i][k] = self.num_b[i][k] / self.den_b[i];
                }
            } else {
                zero_dens += 1;
                dis_set_random(&mut model.b[i], rng);
            }
        }
        if zero_dens > 0 {
            print!("¡({})", zero_dens);
        }
        model.adjust_b_epsilon(eps);
    }
}

// --- driver -----------------------------------------------------------------

/// Outcome of a training run, for the report.
pub struct LearnOutcome {
    pub num_refinements: usize,
    pub sum_log_prob: f64,
    pub num_seqs: usize,
    pub max_t: usize,
    /// (iteration, Σ log P) at each step, for the `.csv`
    pub trace: Vec<(usize, f64)>,
}

/// Trains one model, as `hmm_learn`.
#[allow(clippy::too_many_arguments)]
pub fn learn(
    class_name: &str,
    n: usize,
    model_type: usize,
    seqs: &[Vec<u16>],
    m: usize,
    hmm_epsilon: f64,
    val_auto: f64,
    max_iterations: i32,
    seed: u64,
    model_path: &Path,
) -> Result<LearnOutcome, Box<dyn Error>> {
    if seqs.is_empty() {
        return Err("no training sequences".into());
    }
    let max_t = seqs.iter().map(|s| s.len()).max().unwrap_or(0);
    let mut rng = StdRng::seed_from_u64(seed);

    let mut model = HmmModel::new(class_name, n, m);
    model.init(model_type, &mut rng);
    estimate_b(&mut model, seqs, hmm_epsilon);

    let sum_log_prob = |model: &HmmModel| -> f64 {
        let mut work = super::hmm_rs::Work::new();
        let scorer = super::hmm_rs::Hmm::from_data(model.to_data());
        seqs.iter()
            .map(|o| scorer.log_prob(o, &mut work).unwrap_or(0.0))
            .sum()
    };

    let mut slp = sum_log_prob(&model);
    let mut trace = vec![(0usize, slp)];

    let abs_log_val_auto = if val_auto > 0.0 {
        val_auto.ln().abs()
    } else {
        0.0
    };

    let mut refiner = Refiner::new(n, m, max_t);
    let mut num_refinements = 0usize;
    let mut prev = slp;

    while max_iterations < 0 || (num_refinements as i32) < max_iterations {
        refiner.step(&mut model, seqs, model_type, hmm_epsilon, &mut rng);
        num_refinements += 1;

        slp = sum_log_prob(&model);
        trace.push((num_refinements, slp));

        let change = slp - prev;
        println!(
            " {:3}: Δ = {:+10.6}  sum_log_prob = {:+10.6}  prev = {:+10.6}  '{}'",
            num_refinements, change, slp, prev, class_name
        );

        if slp >= 0.0 {
            eprintln!("\nWARNING: sum_log_prob non negative");
            break;
        }
        if val_auto > 0.0 && change.abs() <= abs_log_val_auto {
            break;
        }
        prev = slp;
    }

    cfmt::save_hmm(model_path, &model.to_data())?;

    Ok(LearnOutcome {
        num_refinements,
        sum_log_prob: slp,
        num_seqs: seqs.len(),
        max_t,
        trace,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(1)
    }

    #[test]
    fn uniform_rows_sum_to_one_and_use_no_randomness() {
        let mut d = vec![0f64; 7];
        dis_set_uniform(&mut d);
        assert!((d.iter().sum::<f64>() - 1.0).abs() < 1e-15);
        assert!(d.iter().all(|v| (*v - 1.0 / 7.0).abs() < 1e-12));
    }

    #[test]
    fn random_rows_sum_to_one_with_no_zeros() {
        let mut d = vec![0f64; 5];
        dis_set_random(&mut d, &mut rng());
        assert!((d.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert!(d.iter().all(|v| *v > 0.0));
    }

    /// Left-to-right: zeros before `from`, and at most `delta` non-zeros.
    #[test]
    fn random_delta_is_a_left_to_right_band() {
        let mut d = vec![0f64; 6];
        dis_set_random_delta(&mut d, 2, 3, &mut rng());
        assert_eq!(&d[..2], &[0.0, 0.0]);
        assert!(d[2] > 0.0 && d[3] > 0.0 && d[4] > 0.0);
        assert_eq!(d[5], 0.0);
        assert!((d.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    /// The band is clipped by the end of the row, as `min(len - from, delta)`.
    #[test]
    fn random_delta_clips_at_the_end() {
        let mut d = vec![0f64; 4];
        dis_set_random_delta(&mut d, 3, 3, &mut rng());
        assert_eq!(&d[..3], &[0.0, 0.0, 0.0]);
        assert!((d[3] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn b_epsilon_lifts_the_floor_and_keeps_rows_normalized() {
        let mut model = HmmModel::new("X", 1, 4);
        model.b[0] = vec![0.0, 0.0, 0.5, 0.5];
        model.adjust_b_epsilon(1e-5);
        assert!(model.b[0].iter().all(|v| *v >= 1e-5 - 1e-18));
        assert!((model.b[0].iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn b_epsilon_of_zero_is_a_no_op() {
        let mut model = HmmModel::new("X", 1, 3);
        model.b[0] = vec![0.0, 0.25, 0.75];
        model.adjust_b_epsilon(0.0);
        assert_eq!(model.b[0], vec![0.0, 0.25, 0.75]);
    }

    /// Uniform init must not touch the RNG, which is what makes `-t 1`
    /// comparable with the C bit for bit.
    #[test]
    fn uniform_init_consumes_no_randomness() {
        let mut r1 = StdRng::seed_from_u64(7);
        let mut model = HmmModel::new("X", 3, 5);
        model.init(UNIFORM, &mut r1);
        let mut r2 = StdRng::seed_from_u64(7);
        assert_eq!(r1.random::<u64>(), r2.random::<u64>());
        assert!((model.pi.iter().sum::<f64>() - 1.0).abs() < 1e-15);
    }
}
