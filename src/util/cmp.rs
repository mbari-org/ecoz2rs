//! `ecoz2 util cmp` -- compare two ECOZ2 artifacts, or two trees of them.
//!
//! Intended for validating one implementation of a pipeline stage against
//! another (C vs. Rust, or a re-run against stored reference output).  Each
//! artifact kind gets the comparison that suits it: float-valued models are
//! compared by relative difference, quantized sequences by exact symbol
//! agreement, since a symbol either flips at a cell boundary or it does not.

use std::error::Error;
use std::path::{Path, PathBuf};

use clap::StructOpt;
use colored::*;
use walkdir::WalkDir;

use crate::utl::cfmt;
use crate::utl::cfmt::{Artifact, Section};

#[derive(StructOpt, Debug)]
pub struct UtilCmpOpts {
    /// Reference file or directory
    #[structopt(parse(from_os_str))]
    a: PathBuf,

    /// File or directory to compare against the reference
    #[structopt(parse(from_os_str))]
    b: PathBuf,

    /// Maximum relative difference accepted on float-valued artifacts
    #[structopt(short, long, default_value = "1e-9")]
    tolerance: f64,

    /// Minimum symbol agreement accepted on sequences, as a fraction
    #[structopt(long, default_value = "1.0")]
    min_agreement: f64,

    /// Accept a codebook whose codewords are the same but reordered.
    ///
    /// Near-tied cells can swap positions under a tiny numeric change. The
    /// codebook is then equivalent as a model, but the symbol indices it emits
    /// are relabeled, so this is off by default.
    #[structopt(long)]
    allow_permutation: bool,

    /// Emit the report as JSON
    #[structopt(long)]
    json: bool,

    /// In directory mode, report every file rather than only the summary
    #[structopt(short, long)]
    verbose: bool,
}

/// Outcome of comparing a single pair of files.
#[derive(Debug, Default)]
struct FileDiff {
    kind: String,
    /// Largest absolute difference over all float sections
    max_abs: f64,
    /// Largest relative difference over all float sections
    max_rel: f64,
    /// Section and flat index where `max_rel` occurred
    worst_at: String,
    n_floats: usize,
    n_symbols: usize,
    n_symbols_diff: usize,
    /// Rows (codewords / predictor vectors) that differ beyond tolerance
    n_rows_diff: usize,
    /// ...of which are present in the other file at a different index
    n_rows_permuted: usize,
    error: Option<String>,
}

impl FileDiff {
    fn agreement(&self) -> f64 {
        if self.n_symbols == 0 {
            1.0
        } else {
            (self.n_symbols - self.n_symbols_diff) as f64 / self.n_symbols as f64
        }
    }

    /// True when every differing row is simply somewhere else in the other file.
    fn only_permuted(&self) -> bool {
        self.n_rows_diff > 0 && self.n_rows_diff == self.n_rows_permuted
    }

    fn ok(&self, tolerance: f64, min_agreement: f64, allow_permutation: bool) -> bool {
        if self.error.is_some() || self.agreement() < min_agreement {
            return false;
        }
        if self.max_rel <= tolerance {
            return true;
        }
        allow_permutation && self.only_permuted()
    }
}

/// Relative difference of `b` with respect to reference `a`, falling back to
/// the absolute difference where `a` is essentially zero.  Matches the
/// convention used by the `lpca3` agreement test.
fn rel_diff(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return if a.is_nan() && b.is_nan() {
            0.0
        } else {
            f64::INFINITY
        };
    }
    let d = (a - b).abs();
    if a.abs() > 1e-300 {
        d / a.abs()
    } else {
        d
    }
}

/// Of the rows that differ beyond `tolerance`, how many appear in the other
/// artifact at a different index? Used to tell a relabeled codebook apart from
/// a numerically different one.
fn count_permuted_rows(va: &[f64], vb: &[f64], w: usize, tolerance: f64) -> (usize, usize) {
    if w == 0 || !va.len().is_multiple_of(w) {
        return (0, 0);
    }
    let rows = va.len() / w;
    let row = |v: &[f64], i: usize| -> Vec<f64> { v[i * w..(i + 1) * w].to_vec() };
    let close = |x: &[f64], y: &[f64]| x.iter().zip(y).all(|(&p, &q)| rel_diff(p, q) <= tolerance);

    let differing: Vec<usize> = (0..rows)
        .filter(|&i| !close(&row(va, i), &row(vb, i)))
        .collect();

    // Match each differing row of A to an unused differing row of B. The sets
    // are small (tens out of thousands), so the quadratic scan is fine.
    let mut taken = vec![false; differing.len()];
    let mut permuted = 0;
    for &i in &differing {
        let ai = row(va, i);
        for (k, &j) in differing.iter().enumerate() {
            if taken[k] || j == i {
                continue;
            }
            if close(&ai, &row(vb, j)) {
                taken[k] = true;
                permuted += 1;
                break;
            }
        }
    }
    (differing.len(), permuted)
}

fn compare_artifacts(a: &Artifact, b: &Artifact) -> FileDiff {
    let mut diff = FileDiff {
        kind: a.kind.to_string(),
        ..Default::default()
    };

    if a.kind != b.kind {
        diff.error = Some(format!("kind differs: {} vs {}", a.kind, b.kind));
        return diff;
    }
    if a.dims != b.dims {
        diff.error = Some(format!("shape differs: {} vs {}", a.dims, b.dims));
        return diff;
    }

    for ((name, sa), (_, sb)) in a.sections.iter().zip(&b.sections) {
        if sa.len() != sb.len() {
            diff.error = Some(format!(
                "section '{}' length differs: {} vs {}",
                name,
                sa.len(),
                sb.len()
            ));
            return diff;
        }
        match (sa, sb) {
            (Section::Floats(va), Section::Floats(vb)) => {
                diff.n_floats += va.len();
                for (i, (&x, &y)) in va.iter().zip(vb).enumerate() {
                    let r = rel_diff(x, y);
                    if r > diff.max_rel {
                        diff.max_rel = r;
                        diff.worst_at = format!("{}[{}]", name, i);
                    }
                    let abs = (x - y).abs();
                    if abs > diff.max_abs {
                        diff.max_abs = abs;
                    }
                }
            }
            (Section::Symbols(va), Section::Symbols(vb)) => {
                diff.n_symbols += va.len();
                diff.n_symbols_diff += va.iter().zip(vb).filter(|(x, y)| x != y).count();
            }
            _ => {
                diff.error = Some(format!("section '{}' type differs", name));
                return diff;
            }
        }
    }

    // Row-structured artifacts: distinguish "the codewords moved" from "the
    // codewords changed". PERMUTATION_TOL is deliberately loose relative to the
    // 1e-9 pass tolerance -- a swapped pair still carries the ordinary noise.
    const PERMUTATION_TOL: f64 = 1e-6;
    if let Some(w) = a.row_len {
        if let (Some((_, Section::Floats(va))), Some((_, Section::Floats(vb)))) =
            (a.sections.first(), b.sections.first())
        {
            let (nd, np) = count_permuted_rows(va, vb, w, PERMUTATION_TOL);
            diff.n_rows_diff = nd;
            diff.n_rows_permuted = np;
        }
    }

    diff
}

fn compare_files(a: &Path, b: &Path) -> FileDiff {
    let load_both =
        || -> Result<(Artifact, Artifact), Box<dyn Error>> { Ok((cfmt::load(a)?, cfmt::load(b)?)) };
    match load_both() {
        Ok((aa, bb)) => compare_artifacts(&aa, &bb),
        Err(e) => FileDiff {
            error: Some(e.to_string()),
            ..Default::default()
        },
    }
}

/// Aggregate over a directory pair.
#[derive(Default)]
struct Summary {
    compared: usize,
    missing: Vec<String>,
    failures: Vec<(String, String)>,
    max_abs: f64,
    max_rel: f64,
    worst_file: String,
    n_symbols: usize,
    n_symbols_diff: usize,
    n_rows_diff: usize,
    n_rows_permuted: usize,
    kinds: Vec<String>,
}

impl Summary {
    fn absorb(
        &mut self,
        label: &str,
        d: &FileDiff,
        tolerance: f64,
        min_agreement: f64,
        allow_permutation: bool,
    ) {
        self.compared += 1;
        if !self.kinds.contains(&d.kind) && !d.kind.is_empty() {
            self.kinds.push(d.kind.clone());
        }
        if d.max_rel > self.max_rel {
            self.max_rel = d.max_rel;
            self.worst_file = format!("{} {}", label, d.worst_at);
        }
        if d.max_abs > self.max_abs {
            self.max_abs = d.max_abs;
        }
        self.n_symbols += d.n_symbols;
        self.n_symbols_diff += d.n_symbols_diff;
        self.n_rows_diff += d.n_rows_diff;
        self.n_rows_permuted += d.n_rows_permuted;
        if !d.ok(tolerance, min_agreement, allow_permutation) {
            let why = match &d.error {
                Some(e) => e.clone(),
                None if d.n_symbols_diff > 0 => {
                    format!("{} of {} symbols differ", d.n_symbols_diff, d.n_symbols)
                }
                None if d.only_permuted() => format!(
                    "{} rows reordered (same values); pass --allow-permutation to accept",
                    d.n_rows_diff
                ),
                None => format!("max rel diff {:.3e}", d.max_rel),
            };
            self.failures.push((label.to_string(), why));
        }
    }

    fn agreement(&self) -> f64 {
        if self.n_symbols == 0 {
            1.0
        } else {
            (self.n_symbols - self.n_symbols_diff) as f64 / self.n_symbols as f64
        }
    }

    fn ok(&self) -> bool {
        self.failures.is_empty() && self.missing.is_empty()
    }
}

fn has_known_extension(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| cfmt::EXTENSIONS.contains(&e))
        .unwrap_or(false)
}

pub fn main(opts: UtilCmpOpts) -> Result<(), Box<dyn Error>> {
    let UtilCmpOpts {
        a,
        b,
        tolerance,
        min_agreement,
        allow_permutation,
        json,
        verbose,
    } = opts;

    if !a.exists() {
        return Err(format!("no such file or directory: {}", a.display()).into());
    }
    if !b.exists() {
        return Err(format!("no such file or directory: {}", b.display()).into());
    }

    let mut summary = Summary::default();

    if a.is_file() {
        let d = compare_files(&a, &b);
        summary.absorb(
            a.file_name().unwrap().to_string_lossy().as_ref(),
            &d,
            tolerance,
            min_agreement,
            allow_permutation,
        );
        if !json {
            report_one(&a, &b, &d, tolerance, min_agreement, allow_permutation);
        }
    } else {
        for entry in WalkDir::new(&a).into_iter().filter_map(|e| e.ok()) {
            let pa = entry.path();
            if !pa.is_file() || !has_known_extension(pa) {
                continue;
            }
            let rel = pa.strip_prefix(&a)?;
            let pb = b.join(rel);
            let label = rel.to_string_lossy().to_string();
            if !pb.is_file() {
                summary.missing.push(label);
                continue;
            }
            let d = compare_files(pa, &pb);
            if verbose && !json {
                println!(
                    "  {:<48} {}",
                    label,
                    one_line(&d, tolerance, min_agreement, allow_permutation)
                );
            }
            summary.absorb(&label, &d, tolerance, min_agreement, allow_permutation);
        }
        if !json {
            report_summary(&a, &b, &summary, tolerance, min_agreement);
        }
    }

    if json {
        print_json(&a, &b, &summary, tolerance, min_agreement);
    }

    if !summary.ok() {
        std::process::exit(1);
    }
    Ok(())
}

fn verdict(ok: bool) -> ColoredString {
    if ok {
        "OK".green()
    } else {
        "FAILED".red()
    }
}

fn one_line(d: &FileDiff, tolerance: f64, min_agreement: f64, allow_perm: bool) -> String {
    if let Some(e) = &d.error {
        return format!("{}: {}", "ERROR".red(), e);
    }
    let detail = if d.n_symbols > 0 {
        format!(
            "agreement {:.6}% ({} of {} differ)",
            100.0 * d.agreement(),
            d.n_symbols_diff,
            d.n_symbols
        )
    } else {
        format!("max rel {:.3e}", d.max_rel)
    };
    format!(
        "{}  {}",
        detail,
        verdict(d.ok(tolerance, min_agreement, allow_perm))
    )
}

fn report_one(
    a: &Path,
    b: &Path,
    d: &FileDiff,
    tolerance: f64,
    min_agreement: f64,
    allow_perm: bool,
) {
    println!("A: {}", a.display());
    println!("B: {}", b.display());
    if let Some(e) = &d.error {
        println!("  {}: {}", "ERROR".red(), e);
        return;
    }
    println!("<{}>", d.kind);
    if d.n_floats > 0 {
        println!(
            "  floats   n={:<10} max abs {:.3e}   max rel {:.3e}   (worst at {})",
            d.n_floats, d.max_abs, d.max_rel, d.worst_at
        );
    }
    if d.n_symbols > 0 {
        println!(
            "  symbols  n={:<10} differing {}   agreement {:.6}%",
            d.n_symbols,
            d.n_symbols_diff,
            100.0 * d.agreement()
        );
    }
    if d.n_rows_diff > 0 {
        println!(
            "  rows     {} differ, {} of them reordered rather than changed",
            d.n_rows_diff, d.n_rows_permuted
        );
    }
    println!(
        "  => {}  (rel tol {:.0e}, min agreement {:.6}%)",
        verdict(d.ok(tolerance, min_agreement, allow_perm)),
        tolerance,
        100.0 * min_agreement
    );
}

fn report_summary(a: &Path, b: &Path, s: &Summary, tolerance: f64, min_agreement: f64) {
    println!("A: {}", a.display());
    println!("B: {}", b.display());
    println!("  kinds        {}", s.kinds.join(", "));
    println!("  compared     {} file(s)", s.compared);
    if !s.missing.is_empty() {
        println!("  {}   {} file(s)", "missing in B".red(), s.missing.len());
        for m in s.missing.iter().take(10) {
            println!("      {}", m);
        }
        if s.missing.len() > 10 {
            println!("      ... and {} more", s.missing.len() - 10);
        }
    }
    if s.n_symbols > 0 {
        println!(
            "  symbols      agreement {:.6}%  ({} of {} differ)",
            100.0 * s.agreement(),
            s.n_symbols_diff,
            s.n_symbols
        );
    }
    if s.max_rel > 0.0 || s.n_symbols == 0 {
        println!(
            "  floats       max abs {:.3e}   max rel {:.3e}   (worst: {})",
            s.max_abs, s.max_rel, s.worst_file
        );
    }
    if s.n_rows_diff > 0 {
        println!(
            "  rows         {} differ, {} of them reordered rather than changed",
            s.n_rows_diff, s.n_rows_permuted
        );
    }
    if !s.failures.is_empty() {
        println!(
            "  {}     {} file(s)",
            "over tolerance".red(),
            s.failures.len()
        );
        for (f, why) in s.failures.iter().take(10) {
            println!("      {}: {}", f, why);
        }
        if s.failures.len() > 10 {
            println!("      ... and {} more", s.failures.len() - 10);
        }
    }
    println!(
        "  => {}  (rel tol {:.0e}, min agreement {:.6}%)",
        verdict(s.ok()),
        tolerance,
        100.0 * min_agreement
    );
}

fn print_json(a: &Path, b: &Path, s: &Summary, tolerance: f64, min_agreement: f64) {
    let report = serde_json::json!({
        "a": a.display().to_string(),
        "b": b.display().to_string(),
        "kinds": s.kinds,
        "compared": s.compared,
        "missing": s.missing,
        "failures": s.failures.iter()
            .map(|(f, w)| serde_json::json!({"file": f, "why": w}))
            .collect::<Vec<_>>(),
        "max_abs_diff": s.max_abs,
        "max_rel_diff": s.max_rel,
        "worst_at": s.worst_file,
        "symbols_total": s.n_symbols,
        "symbols_differing": s.n_symbols_diff,
        "symbol_agreement": s.agreement(),
        "rows_differing": s.n_rows_diff,
        "rows_reordered": s.n_rows_permuted,
        "tolerance": tolerance,
        "min_agreement": min_agreement,
        "ok": s.ok(),
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
