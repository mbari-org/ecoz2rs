extern crate clap;

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use clap::StructOpt;

use crate::ecoz2_lib::lpc_signals;
use crate::prd::Predictor;
use crate::sgn;
use crate::utl;

mod libpar;
mod lpc_rs;
pub mod lpca_cepstrum_rs;
pub mod lpca_r_rs;
mod lpca_rs;

#[derive(StructOpt, Debug)]
pub struct LpcOpts {
    /// Prediction order
    #[structopt(short = 'P', long, default_value = "36")]
    prediction_order: usize,

    /// Analysis window length in milliseconds
    #[structopt(short = 'W', long, default_value = "45")]
    window_length_ms: usize,

    /// Window offset length in milliseconds
    #[structopt(short = 'O', long, default_value = "15")]
    offset_length_ms: usize,

    /// Only process a class if it has at least this number of signals
    #[structopt(short = 'm', long, default_value = "0")]
    minpc: usize,

    /// Put the generated predictors into two different training
    /// and test subsets (with the given approx ratio).
    /// DEPRECATED.
    #[structopt(short = 's', long, default_value = "0")]
    split: f32,

    /// Signal files to process. If directories are included, then
    /// all `.wav` under them will be used.
    /// If a `.csv` is given, then it's assumed to contain columns
    /// `tt,class,selection` to process desired signals according to
    /// `--signals-dir-template`, `--tt`, `--class`.
    /// TODO --class
    #[structopt(long, required = true, min_values = 1, parse(from_os_str))]
    signals: Vec<PathBuf>,

    #[structopt(long, default_value = "data/signals")]
    signals_dir_template: String,

    /// TRAIN or TEST
    #[structopt(long)]
    tt: Option<String>,

    #[structopt(long)]
    class: Option<String>,

    /// Min time in secs to report processing time per signal
    #[structopt(short = 'X', default_value = "5")]
    mintrpt: f32,

    /// Use Rust "parallel" implementation
    #[structopt(long)]
    zrsp: bool,

    /// Use Rust implementation
    #[structopt(long)]
    zrs: bool,

    #[structopt(long)]
    verbose: bool,
}

pub fn main(opts: LpcOpts) {
    let res = main_lpc(opts);

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn main_lpc(opts: LpcOpts) -> Result<(), Box<dyn Error>> {
    let LpcOpts {
        prediction_order,
        window_length_ms,
        offset_length_ms,
        minpc,
        split,
        signals,
        signals_dir_template,
        tt,
        class,
        mintrpt,
        zrsp,
        zrs,
        verbose,
    } = opts;

    let tt = tt.unwrap_or_default();

    let sgn_filenames = utl::resolve_files3(
        &signals,
        tt.as_str(),
        &class,
        "".to_string(),
        signals_dir_template,
        ".wav",
    )?;

    // println!("sgn_filenames = {:?}", sgn_filenames);
    // return Ok(()).into();

    if zrs || zrsp {
        if split > 0. {
            return Err("--split is deprecated and unsupported by the Rust implementation".into());
        }
        lpc_signals_rs(
            sgn_filenames,
            prediction_order,
            window_length_ms,
            offset_length_ms,
            minpc,
            mintrpt,
            zrsp,
            verbose,
        )?;
    } else {
        lpc_signals(
            prediction_order,
            window_length_ms,
            offset_length_ms,
            minpc,
            split,
            sgn_filenames,
            mintrpt,
            verbose,
        );
    }

    Ok(())
}

/// The Rust implementation of `lpc_signals` (`ecoz2/src/lpc/lpc_signals.c`).
///
/// Groups the signals by class, drops classes below `minpc`, and writes each
/// predictor to `data/predictors/<class>/<stem>.prd` in the traditional format,
/// so the output is interchangeable with the C's.
///
/// The C shuffles each class's file list before processing. That only affects
/// the TRAIN/TEST assignment under the deprecated `--split`, which this path
/// rejects, so the shuffle is not reproduced and the outputs are identical.
#[allow(clippy::too_many_arguments)]
fn lpc_signals_rs(
    sgn_filenames: Vec<PathBuf>,
    prediction_order: usize,
    window_length_ms: usize,
    offset_length_ms: usize,
    minpc: usize,
    mintrpt: f32,
    parallel: bool,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    // Grouped in first-seen order, as the C's list is.
    let mut class_order: Vec<String> = Vec::new();
    let mut by_class: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for f in sgn_filenames {
        let class_name = utl::class_name_of(&f);
        if !by_class.contains_key(&class_name) {
            class_order.push(class_name.clone());
            by_class.insert(class_name.clone(), Vec::new());
        }
        by_class.get_mut(&class_name).unwrap().push(f);
    }

    println!("lpc_signals: number of classes: {}", class_order.len());

    for class_name in &class_order {
        let files = &by_class[class_name];
        if minpc > 0 && files.len() < minpc {
            println!(
                "class '{}': insufficient #signals={}",
                class_name,
                files.len()
            );
            continue;
        }
        println!("class '{}': {}", class_name, files.len());

        for sgn_filename in files {
            if verbose {
                println!("  {}", sgn_filename.display());
            }
            let s = sgn::load(sgn_filename.to_str().unwrap());

            let before = Instant::now();
            let vectors = if parallel {
                libpar::lpa_on_signal(prediction_order, window_length_ms, offset_length_ms, &s)
            } else {
                lpc_rs::lpa_on_signal(prediction_order, window_length_ms, offset_length_ms, &s)
            };
            let elapsed = before.elapsed();
            if verbose && elapsed.as_secs_f32() >= mintrpt {
                println!("processing took {:.2?}", elapsed);
            }

            let vectors = match vectors {
                Some(v) => v,
                None => {
                    eprintln!("{}: cannot create lpc predictor", sgn_filename.display());
                    continue;
                }
            };

            let predictor = Predictor {
                class_name: class_name.clone(),
                prediction_order,
                vectors,
            };
            let out = utl::output_filename(sgn_filename, "predictors", ".prd");
            predictor.save(&out)?;
            if verbose {
                println!("{}: '{}': predictor saved", out.display(), class_name);
            }
        }
    }
    Ok(())
}
