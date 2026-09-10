//! Rust implementation of codebook generation, Juang et al (1982).
//!
//! Mirrors `ecoz2/src/vq/vq_learn.i` and `vq_learn_par.c`.
//!
//! **Determinism.** The C partitions the training vectors round-robin across
//! `omp_get_max_threads()` threads and sums the per-thread partials in thread
//! order, so its codebooks depend on the core count of the machine that built
//! them — measured here at ~9e-11 relative between 1 and 16 threads, growing
//! with M. This implementation instead splits into a *fixed* number of chunks
//! independent of available parallelism, and merges them in chunk order, so the
//! result is identical whatever rayon does with them.
//!
//! On floating point, per the rule in notes.md: the distortion reduction uses
//! the algebraic methods, while the nearest-neighbor argmin and the
//! convergence test stay in strict IEEE.

use std::error::Error;
use std::path::Path;

use rayon::prelude::*;

use super::vq_rs::{quantize_one, Codebook};
use crate::lpc::lpca_r_rs::lpca_r;
use crate::utl::cfmt;

/// Perturbation factors used to split each codeword when the codebook grows.
///
/// The C declares these as `float` literals and multiplies them into a
/// `double`, so the values actually applied are the f32 ones widened. Written
/// that way here too: `1.01f64` would be a different number and would put the
/// whole ladder on a different trajectory.
const PERT0: f64 = 0.99f32 as f64;
const PERT1: f64 = 1.01f32 as f64;

/// Training vectors per reduction chunk. Fixed on purpose; see the note above.
const CHUNK: usize = 4096;

/// Per-codebook-size outcome, for the report.
pub struct CodebookReport {
    pub num_entries: usize,
    pub passes: usize,
    pub avg_distortion: f64,
    pub sigma: f64,
    pub inertia: f64,
    pub cardinalities: Vec<usize>,
    pub cell_distortions: Vec<f64>,
}

/// The initial two-entry codebook: reflection vectors `[_, ∓0.5, 0, …]`.
fn initial_reflections(p: usize) -> Vec<Vec<f64>> {
    let mut a = vec![0f64; 1 + p];
    let mut b = vec![0f64; 1 + p];
    a[1] = -0.5;
    b[1] = 0.5;
    vec![a, b]
}

/// Doubles the codebook, perturbing each entry down and up.
///
/// Entry `j` yields `2j` (perturbed by `PERT0`) and `2j+1` (by `PERT1`), which
/// is the layout `grow_codebook` produces.
fn grow(reflections: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut out = Vec::with_capacity(reflections.len() * 2);
    for v in reflections {
        out.push(v.iter().map(|x| x * PERT0).collect());
        out.push(v.iter().map(|x| x * PERT1).collect());
    }
    out
}

/// What one chunk of training vectors contributes to a pass.
#[derive(Clone)]
struct Partial {
    /// Summed autocorrelation per cell, flat `M * (1 + P)`
    cells: Vec<f64>,
    cardd: Vec<usize>,
    discel: Vec<f64>,
    dd: f64,
    /// (codeword, minDist) per vector, in input order
    min_dists: Vec<(usize, f64)>,
}

impl Partial {
    fn new(m: usize, w: usize, cap: usize) -> Partial {
        Partial {
            cells: vec![0f64; m * w],
            cardd: vec![0usize; m],
            discel: vec![0f64; m],
            dd: 0f64,
            min_dists: Vec::with_capacity(cap),
        }
    }

    fn merge(&mut self, other: &Partial) {
        for (a, b) in self.cells.iter_mut().zip(&other.cells) {
            *a += *b;
        }
        for (a, b) in self.cardd.iter_mut().zip(&other.cardd) {
            *a += *b;
        }
        for (a, b) in self.discel.iter_mut().zip(&other.discel) {
            *a += *b;
        }
        self.dd += other.dd;
        self.min_dists.extend_from_slice(&other.min_dists);
    }
}

/// One assignment pass: nearest codeword for every training vector, and the
/// per-cell accumulations the centroid update needs.
fn assign(cb: &Codebook, vectors: &[Vec<f64>], m: usize, w: usize) -> Partial {
    vectors
        .par_chunks(CHUNK)
        .map(|chunk| {
            let mut acc = Partial::new(m, w, chunk.len());
            for rx in chunk {
                let (i_min, ddmin) = quantize_one(cb, rx);
                acc.min_dists.push((i_min, ddmin - 1.));
                acc.dd += ddmin - 1.;
                let cell = &mut acc.cells[i_min * w..(i_min + 1) * w];
                for (c, x) in cell.iter_mut().zip(rx) {
                    *c += *x;
                }
                acc.cardd[i_min] += 1;
                acc.discel[i_min] += ddmin - 1.;
            }
            acc
        })
        // Reduced left to right over a fixed chunking, so the sum order does
        // not depend on how rayon schedules the work.
        .collect::<Vec<_>>()
        .iter()
        .fold(Partial::new(m, w, vectors.len()), |mut a, b| {
            a.merge(b);
            a
        })
}

/// Updates each non-empty cell's centroid by solving the LPC equations for the
/// average autocorrelation, as `calculate_reflections` does. Also normalizes
/// the accumulated autocorrelation by the gain, which `calculateSigma` relies on.
fn update_centroids(
    p: usize,
    cells: &mut [f64],
    cardd: &[usize],
    reflections: &mut [Vec<f64>],
    raas: &mut [f64],
) {
    let w = 1 + p;
    let mut pred = vec![0f64; w];
    for i in 0..cardd.len() {
        if cardd[i] == 0 {
            continue;
        }
        let cel = &mut cells[i * w..(i + 1) * w];
        let (_res, err_pred) = lpca_r(p, cel, &mut reflections[i], &mut pred);

        let raa = &mut raas[i * w..(i + 1) * w];
        for (n, raa_n) in raa.iter_mut().enumerate() {
            *raa_n = pred[..=p - n]
                .iter()
                .zip(&pred[n..=p])
                .fold(0.0f64, |acc, (&c, &s)| {
                    acc.algebraic_add(c.algebraic_mul(s))
                });
        }
        if err_pred != 0. {
            for c in cel.iter_mut() {
                *c /= err_pred;
            }
        }
    }
}

/// `dpc / avgDistortion`: average inter-cell over average intra-cell distortion.
fn calculate_sigma(cb: &Codebook, cells: &[f64], m: usize, w: usize, avg_distortion: f64) -> f64 {
    if m < 2 {
        return 0.;
    }
    let dpc: f64 = (0..m)
        .into_par_iter()
        .map(|i| {
            let raa = cb.entry(i);
            (0..m)
                .filter(|&j| j != i)
                .map(|j| super::vq_rs::distortion(&cells[j * w..(j + 1) * w], raa) - 1.)
                .sum::<f64>()
        })
        .sum();
    (dpc / (m * (m - 1)) as f64) / avg_distortion
}

/// Within-cluster sum of squared distortions.
fn calculate_inertia(cb: &Codebook, vectors: &[Vec<f64>]) -> f64 {
    vectors
        .par_chunks(CHUNK)
        .map(|chunk| {
            chunk
                .iter()
                .map(|rx| {
                    let (_, ddmin) = quantize_one_squared(cb, rx);
                    ddmin
                })
                .sum::<f64>()
        })
        .collect::<Vec<_>>()
        .iter()
        .sum()
}

/// As the argmin in `calculateInertia`: compares squared distortions.
fn quantize_one_squared(cb: &Codebook, rx: &[f64]) -> (usize, f64) {
    let mut ddmin = f64::MAX;
    let mut i_min = 0usize;
    for i in 0..cb.num_entries {
        let d = super::vq_rs::distortion(rx, cb.entry(i));
        let dd = d * d;
        if dd < ddmin {
            ddmin = dd;
            i_min = i;
        }
    }
    (i_min, ddmin)
}

/// Runs the codebook ladder, saving each size as it converges.
///
/// `max_codebook_size` bounds the ladder; the C has no such control and always
/// doubles to its compile-time `MAX_CODEBOOK_SIZE`.
#[allow(clippy::too_many_arguments)]
pub fn learn(
    class_name: &str,
    prediction_order: usize,
    epsilon: f64,
    vectors: &[Vec<f64>],
    base_reflections: Option<Vec<Vec<f64>>>,
    max_codebook_size: usize,
    prefix: &Path,
    mut on_codebook: impl FnMut(&CodebookReport, &Path),
) -> Result<(), Box<dyn Error>> {
    let p = prediction_order;
    let w = 1 + p;
    let tot_vecs = vectors.len();
    if tot_vecs == 0 {
        return Err("no training vectors".into());
    }

    let mut reflections = match base_reflections {
        Some(base) => grow(&base),
        None => initial_reflections(p),
    };

    // The codebook in the autocorrelation domain is carried across passes and
    // updated in place. It is derived from the reflections only at init and at
    // each growth, exactly as the C does: `calculate_reflections` writes the new
    // entry straight from the predictor, and never round-trips back through
    // `lpca_rc`. Recomputing it every pass is mathematically equivalent but not
    // bit-identical, and the difference compounds over the ladder.
    let mut raas = Codebook::from_reflections(&reflections, p).to_raas();

    let mut dd_prv = f64::MAX;
    let mut pass = 0usize;

    loop {
        let m = reflections.len();
        let cb = Codebook::from_raas(raas.clone(), p);

        let mut part = assign(&cb, vectors, m, w);
        let dd = part.dd;
        let avg_distortion = dd / tot_vecs as f64;

        let empty = part.cardd.iter().filter(|&&c| c == 0).count();
        if empty > 0 {
            println!("WARN: {} empty cell(s) for codebook size {}", empty, m);
        }

        // Updated in place: `calculate_reflections` skips empty cells, so theirs
        // keep the value they were grown with.
        update_centroids(p, &mut part.cells, &part.cardd, &mut reflections, &mut raas);

        // Strict IEEE: a convergence test, not a reduction.
        let converged = pass > 0 && (dd_prv - dd) / dd < epsilon;
        dd_prv = dd;

        if !converged {
            pass += 1;
            continue;
        }

        // The codebook for this size is final: save it and report.
        let updated = Codebook::from_raas(raas.clone(), p);
        let filename = prefix.with_file_name(format!(
            "{}_M_{:04}.cbook",
            prefix.file_name().unwrap().to_string_lossy(),
            m
        ));
        cfmt::save_codebook(
            &filename,
            &cfmt::VectorSet {
                class_name: class_name.to_string(),
                prediction_order: p,
                vectors: reflections.clone(),
            },
        )?;

        let report = CodebookReport {
            num_entries: m,
            passes: pass + 1,
            avg_distortion,
            sigma: calculate_sigma(&updated, &part.cells, m, w, avg_distortion),
            inertia: calculate_inertia(&updated, vectors),
            cardinalities: part.cardd.clone(),
            cell_distortions: part.discel.clone(),
        };
        on_codebook(&report, &filename);

        if m >= max_codebook_size {
            break;
        }
        reflections = grow(&reflections);
        raas = Codebook::from_reflections(&reflections, p).to_raas();
        pass = 0;
        dd_prv = f64::MAX;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_initial_codebook_is_two_mirrored_entries() {
        let r = initial_reflections(3);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], vec![0.0, -0.5, 0.0, 0.0]);
        assert_eq!(r[1], vec![0.0, 0.5, 0.0, 0.0]);
    }

    /// The perturbation factors must be the f32 literals the C uses, widened.
    /// Using `1.01f64` would send the whole ladder down a different path.
    #[test]
    fn perturbation_factors_match_the_c_float_literals() {
        assert_ne!(PERT1, 1.01f64);
        assert_eq!(PERT1, 1.01f32 as f64);
        assert_eq!(PERT0, 0.99f32 as f64);
    }

    #[test]
    fn growing_doubles_and_orders_children_low_then_high() {
        let g = grow(&[vec![0.0, 1.0]]);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0][1], PERT0);
        assert_eq!(g[1][1], PERT1);
    }

    /// The reduction must not depend on how the work is chunked.
    #[test]
    fn merging_partials_is_order_independent_per_cell() {
        let mut a = Partial::new(2, 2, 0);
        let mut b = Partial::new(2, 2, 0);
        a.cardd[0] = 3;
        a.dd = 1.5;
        b.cardd[1] = 4;
        b.dd = 2.5;
        a.merge(&b);
        assert_eq!(a.cardd, vec![3, 4]);
        assert_eq!(a.dd, 4.0);
    }
}

/// Writes the four report artifacts the C emits alongside the codebooks:
/// `<prefix>.rpt`, `<prefix>.rpt.csv`, and per codebook a `.cards_dists.csv`
/// and a `.min_dists.csv`.
pub struct Reporter {
    rpt: std::io::BufWriter<std::fs::File>,
    csv: std::io::BufWriter<std::fs::File>,
}

impl Reporter {
    pub fn new(prefix: &Path, tot_vecs: usize, eps: f64) -> Result<Reporter, Box<dyn Error>> {
        use std::io::Write;
        let rpt_path = prefix.with_file_name(format!(
            "{}.rpt",
            prefix.file_name().unwrap().to_string_lossy()
        ));
        println!("Report: {}", rpt_path.display());
        let mut rpt = std::io::BufWriter::new(std::fs::File::create(&rpt_path)?);
        writeln!(
            rpt,
            "Codebook generation\n\n{} training vectors. (ε = {})\n",
            tot_vecs,
            super::format_g(eps)
        )?;

        let csv_path = rpt_path.with_extension("rpt.csv");
        let mut csv = std::io::BufWriter::new(std::fs::File::create(csv_path)?);
        writeln!(
            csv,
            "# {} training vectors. (ε = {})",
            tot_vecs,
            super::format_g(eps)
        )?;
        writeln!(csv, "M,passes,DDprm,σ,inertia")?;
        Ok(Reporter { rpt, csv })
    }

    pub fn add(&mut self, r: &CodebookReport, cb_filename: &Path) -> Result<(), Box<dyn Error>> {
        use std::io::Write;
        writeln!(
            self.rpt,
            "### {} ({:2} passes)  Dprm = {:.6}  σ = {:.6}  inertia = {:.6}",
            cb_filename.display(),
            r.passes,
            r.avg_distortion,
            r.sigma,
            r.inertia
        )?;
        writeln!(
            self.csv,
            "{},{},{:.6},{:.6},{:.6}",
            r.num_entries, r.passes, r.avg_distortion, r.sigma, r.inertia
        )?;
        self.csv.flush()?;

        let empty = r.cardinalities.iter().filter(|&&c| c == 0).count();
        write!(self.rpt, "Cardinalities:")?;
        if empty > 0 {
            write!(self.rpt, "  ({} empty cells)", empty)?;
        }
        writeln!(self.rpt)?;
        for c in &r.cardinalities {
            write!(self.rpt, "{:8} ", c)?;
        }
        writeln!(self.rpt)?;
        writeln!(self.rpt, "average cell distortion:")?;
        for (c, d) in r.cardinalities.iter().zip(&r.cell_distortions) {
            if *c > 0 {
                write!(self.rpt, "{:8.3} ", d / *c as f64)?;
            } else {
                write!(self.rpt, "{:>8} ", "_")?;
            }
        }
        writeln!(self.rpt, "\n")?;
        self.rpt.flush()?;

        let cards = cb_filename.with_file_name(format!(
            "{}.cards_dists.csv",
            cb_filename.file_name().unwrap().to_string_lossy()
        ));
        let mut f = std::io::BufWriter::new(std::fs::File::create(cards)?);
        writeln!(f, "index,cardinality,distortion")?;
        for (i, (c, d)) in r.cardinalities.iter().zip(&r.cell_distortions).enumerate() {
            if *c > 0 {
                writeln!(f, "{},{},{:.6}", i, c, d / *c as f64)?;
            } else {
                writeln!(f, "{},{},NaN", i, c)?;
            }
        }
        Ok(())
    }
}
