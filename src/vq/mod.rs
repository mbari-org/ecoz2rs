extern crate clap;

use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use clap::StructOpt;

use crate::c12n;
use crate::utl;
use crate::utl::cfmt;
use crate::utl::pf;

mod vq_learn_rs;
pub mod vq_rs;

use self::EcozVqCommand::{Classify, Learn, Quantize, Show};

#[derive(StructOpt, Debug)]
pub struct VqMainOpts {
    #[structopt(subcommand)]
    cmd: EcozVqCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "vq", about = "VQ operations")]
enum EcozVqCommand {
    #[structopt(about = "Codebook training")]
    Learn(VqLearnOpts),

    #[structopt(about = "Vector quantization")]
    Quantize(VqQuantizeOpts),

    #[structopt(about = "VQ based classification")]
    Classify(VqClassifyOpts),

    #[structopt(about = "Show codebook")]
    Show(VqShowOpts),
}

#[derive(StructOpt, Debug)]
pub struct VqLearnOpts {
    /// Start training from this base codebook.
    #[structopt(short = 'B', long, name = "codebook")]
    base_codebook: Option<String>,

    /// Prediction order (required if -B not given).
    #[structopt(short = 'P', long, name = "P")]
    prediction_order: Option<usize>,

    /// Epsilon parameter for convergence.
    #[structopt(short = 'e', long = "epsilon", default_value = "0.05", name = "ε")]
    epsilon: f64,

    /// Class name to associate to generated codebook (ignored if -B given)
    /// and also, if a `.csv` file is given with the `--predictors` option,
    /// to only consider instances of such given class.
    #[structopt(long, name = "class")]
    class_name: Option<String>,

    /// Predictor files for training.
    /// If a single `.csv` file is given, then the "TRAIN" files indicated there will be used
    /// (and only, if `--class-name` is given, the ones for the corresponding class).
    /// Otherwise, if directories are included, then all `.prd` under them will be used.
    #[structopt(long, parse(from_os_str), name = "files")]
    predictors: Vec<PathBuf>,

    /// Stop the codebook ladder at this size.
    #[structopt(long, default_value = "4096")]
    max_codebook_size: usize,
}

#[derive(StructOpt, Debug)]
pub struct VqQuantizeOpts {
    /// Reference codebook for quantization.
    #[structopt(long = "codebook", parse(from_os_str))]
    codebook: PathBuf,

    /// Predictor files to be quantized.
    #[structopt(long, required = true, parse(from_os_str), name = "files")]
    predictors: Vec<PathBuf>,

    #[structopt(long, default_value = "data/predictors")]
    predictors_dir_template: String,

    /// Optional selection of TRAIN or TEST instances
    /// when `.csv` is given to `--predictors`.
    #[structopt(long)]
    tt: Option<String>,

    /// Only this class when `.csv` is given to `--predictors`.
    #[structopt(long, name = "class")]
    class_name: Option<String>,

    /// Show file names as they are processed.
    #[structopt(short, long)]
    show_filenames: bool,
}

#[derive(StructOpt, Debug)]
pub struct VqClassifyOpts {
    /// Show ranked models for incorrect classifications
    #[structopt(short = 'r', long)]
    show_ranked: bool,

    /// Codebook models.
    /// If directories are included, then all `.cb` under them will be used.
    #[structopt(long, required = true, min_values = 1, parse(from_os_str))]
    codebooks: Vec<PathBuf>,

    /// TRAIN or TEST
    #[structopt(long, required = true)]
    tt: String,

    /// Predictor files to classify.
    /// If a single `.csv` file is given, then only the ones indicated with `--tt` will be used.
    /// Otherwise, if directories are included, then all `.prd` under them will be used.
    #[structopt(long, required = true, min_values = 1, parse(from_os_str))]
    predictors: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct VqShowOpts {
    /// Start index for coefficient range selection
    #[structopt(short, long, default_value = "-1")]
    from: i32,

    /// Limit index for coefficient range selection
    #[structopt(short, long, default_value = "-1")]
    to: i32,

    /// Codebook.
    #[structopt(parse(from_os_str))]
    codebook: PathBuf,
}

pub fn main(opts: VqMainOpts) {
    let res = match opts.cmd {
        Learn(opts) => main_vq_learn(opts),

        Quantize(opts) => main_vq_quantize(opts),

        Classify(opts) => main_vq_classify(opts),

        Show(opts) => main_vq_show(opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn main_vq_learn(opts: VqLearnOpts) -> Result<(), Box<dyn Error>> {
    let VqLearnOpts {
        base_codebook,
        prediction_order,
        epsilon,
        class_name,
        predictors,
        max_codebook_size,
    } = opts;

    if let (Some(_), Some(_)) = (&base_codebook, prediction_order) {
        return Err("Only one of base codebook or prediction order expected".into());
    }

    let codebook_class_name = match &class_name {
        Some(name) => name.clone(),
        None => "_".to_string(),
    };

    let prd_filenames = utl::resolve_files(
        predictors,
        "TRAIN",
        class_name,
        "predictors".to_string(),
        ".prd",
    )?;

    vq_learn_rs_driver(
        base_codebook,
        prediction_order,
        epsilon,
        &codebook_class_name,
        &prd_filenames,
        max_codebook_size,
    )
}

/// The Rust implementation of `vq_learn` (`ecoz2/src/vq/vq_learn.i`).
fn vq_learn_rs_driver(
    base_codebook: Option<String>,
    prediction_order: Option<usize>,
    epsilon: f64,
    class_name: &str,
    prd_filenames: &[PathBuf],
    max_codebook_size: usize,
) -> Result<(), Box<dyn Error>> {
    // With a base codebook, P and the class come from it, as in the C.
    let (p, class_name, base_reflections) = match &base_codebook {
        Some(bc) => {
            let cb = cfmt::load_codebook(Path::new(bc))?;
            println!(
                "\nCodebook generation:\n\nbase_codebook: {}  num_vecs={} prediction_order={} class='{}'  epsilon={}\n",
                bc,
                cb.vectors.len(),
                cb.prediction_order,
                cb.class_name,
                pf::g(epsilon)
            );
            (cb.prediction_order, cb.class_name.clone(), Some(cb.vectors))
        }
        None => {
            let p = prediction_order.ok_or("-P is required when -B is not given")?;
            println!(
                "\nCodebook generation:\n\nprediction_order={} class='{}'  epsilon={}\n",
                p,
                class_name,
                pf::g(epsilon)
            );
            (p, class_name.to_string(), None)
        }
    };

    // Load all training vectors up front, as the C does.
    let mut vectors: Vec<Vec<f64>> = Vec::new();
    for f in prd_filenames {
        let prd = cfmt::load_predictor(f)?;
        if prd.prediction_order != p {
            return Err(format!(
                "{}: prediction order {} does not match {}",
                f.display(),
                prd.prediction_order,
                p
            )
            .into());
        }
        vectors.extend(prd.vectors);
    }
    println!("{} training vectors (ε={})", vectors.len(), pf::g(epsilon));

    let class_dir = PathBuf::from(format!("data/codebooks/{}", class_name));
    std::fs::create_dir_all(&class_dir)?;
    let prefix = class_dir.join(format!("eps_{}", pf::g(epsilon)));

    let mut rpt = vq_learn_rs::Reporter::new(&prefix, vectors.len(), epsilon)?;
    vq_learn_rs::learn(
        &class_name,
        p,
        epsilon,
        &vectors,
        base_reflections,
        max_codebook_size,
        &prefix,
        |report, filename| {
            println!(
                "{}  ({} passes)  DP={} σ={} inertia={}",
                filename.display(),
                report.passes,
                report.avg_distortion,
                report.sigma,
                report.inertia
            );
            if let Err(e) = rpt.add(report, filename) {
                eprintln!("error writing report: {}", e);
            }
        },
    )?;
    Ok(())
}

pub fn main_vq_quantize(opts: VqQuantizeOpts) -> Result<(), Box<dyn Error>> {
    let VqQuantizeOpts {
        codebook,
        predictors,
        predictors_dir_template,
        tt,
        class_name,
        show_filenames,
    } = opts;

    let tt = tt.unwrap_or_default();

    let prd_filenames = utl::resolve_files3(
        &predictors,
        tt.as_str(),
        &class_name,
        "".to_string(),
        predictors_dir_template,
        ".prd",
    )?;

    println!("number of predictor files: {}", prd_filenames.len());

    vq_quantize_rs(&codebook, &prd_filenames, show_filenames)
}

/// The Rust implementation of `vq_quantize` (`ecoz2/src/vq/vq_quantize.c`).
///
/// Writes `data/sequences/M<M>/<class>/<stem>.seq` in the traditional format,
/// so the output is interchangeable with the C's.
fn vq_quantize_rs(
    codebook: &Path,
    prd_filenames: &[PathBuf],
    show_filenames: bool,
) -> Result<(), Box<dyn Error>> {
    let cb = cfmt::load_codebook(codebook)?;
    let m = cb.vectors.len();
    let quantizer = vq_rs::Codebook::from_reflections(&cb.vectors, cb.prediction_order);
    println!("{}: {} symbols", codebook.display(), m);

    let mut ddprm_total = 0f64;
    let mut num_seqs = 0usize;

    for prd_filename in prd_filenames {
        let prd = cfmt::load_predictor(prd_filename)?;
        if prd.prediction_order != cb.prediction_order {
            return Err(format!(
                "{}: prediction order {} does not match the codebook's {}",
                prd_filename.display(),
                prd.prediction_order,
                cb.prediction_order
            )
            .into());
        }

        let (symbols, ddprm) = vq_rs::quantize(&quantizer, &prd.vectors);
        ddprm_total += ddprm;
        num_seqs += 1;

        // The class recorded in the sequence comes from the predictor's path,
        // as `get_class_name` does; the C does not take it from the predictor.
        let class_name = utl::class_name_of(prd_filename);
        if class_name.is_empty() {
            println!("WARN {}: no className", prd_filename.display());
        }
        let out = utl::output_filename(prd_filename, &format!("sequences/M{}", m), ".seq");
        cfmt::save_sequence(&out, &class_name, m, &symbols)?;

        if show_filenames {
            println!(
                "{} className='{}' ({:1.4})",
                out.display(),
                class_name,
                ddprm
            );
        }
    }

    if num_seqs > 0 {
        println!(
            "\ntotal: {} sequences; total  average distortion = {}",
            num_seqs,
            ddprm_total / num_seqs as f64
        );
    }
    Ok(())
}

pub fn main_vq_classify(opts: VqClassifyOpts) -> Result<(), Box<dyn Error>> {
    let VqClassifyOpts {
        show_ranked,
        codebooks,
        tt,
        predictors,
    } = opts;

    let cb_filenames = utl::resolve_filenames(codebooks, ".cbook", "codebooks")?;

    let prd_filenames = utl::resolve_files(
        predictors,
        tt.as_str(),
        None,
        "predictors".to_string(),
        ".prd",
    )?;

    println!(
        "number of codebooks: {}  number of predictors: {}",
        cb_filenames.len(),
        prd_filenames.len()
    );
    println!("show_ranked = {}", show_ranked);

    vq_classify_rs(&cb_filenames, &prd_filenames, show_ranked)
}

/// The Rust implementation of `vq_classify` (`ecoz2/src/vq/vq_classify.c`):
/// quantize each predictor against every class codebook and rank the classes by
/// resulting average distortion.
fn vq_classify_rs(
    cb_filenames: &[PathBuf],
    prd_filenames: &[PathBuf],
    show_ranked: bool,
) -> Result<(), Box<dyn Error>> {
    println!("\nLoading models:");
    let mut class_names: Vec<String> = Vec::new();
    let mut models: Vec<vq_rs::Codebook> = Vec::new();
    let mut num_vecs: Option<usize> = None;

    for (i, f) in cb_filenames.iter().enumerate() {
        println!("{:2}: {}", i, f.display());
        let cb = cfmt::load_codebook(f)?;
        match num_vecs {
            None => num_vecs = Some(cb.vectors.len()),
            Some(n) if n != cb.vectors.len() => {
                return Err(format!(
                    "{}: conformity error: {} entries, expected {}",
                    f.display(),
                    cb.vectors.len(),
                    n
                )
                .into())
            }
            _ => {}
        }
        models.push(vq_rs::Codebook::from_reflections(
            &cb.vectors,
            cb.prediction_order,
        ));
        class_names.push(cb.class_name);
    }

    let mut c12n = c12n::C12nResults::new(class_names.clone());
    println!();

    for filename in prd_filenames {
        let prd = cfmt::load_predictor(filename)?;
        let class_id = match class_names.iter().position(|n| *n == prd.class_name) {
            Some(i) => i,
            None => {
                eprintln!(
                    "\n{}: no model loaded for className='{}'",
                    filename.display(),
                    prd.class_name
                );
                continue;
            }
        };

        // `add_case` ranks ascending and takes the last as the winner, so the
        // distortions are negated: for VQ the *smallest* distortion wins.
        let scores: Vec<f64> = models
            .iter()
            .map(|m| -vq_rs::average_distortion(m, &prd.vectors))
            .collect();

        c12n.add_case(class_id, &prd.class_name, scores, show_ranked, || {
            format!("\n{}: '{}'\n", filename.display(), prd.class_name)
        });
    }
    println!();

    let names: Vec<&String> = class_names.iter().collect();
    c12n.report_results(names, format!("vq_{}", num_vecs.unwrap_or(0)));
    Ok(())
}

pub fn main_vq_show(opts: VqShowOpts) -> Result<(), Box<dyn Error>> {
    let VqShowOpts { from, to, codebook } = opts;

    vq_show_rs(&codebook, from, to)
}

/// The Rust implementation of `vq_show` (`ecoz2/src/vq/vq_show.c`): the
/// reflection coefficients of each codeword, as CSV.
fn vq_show_rs(codebook: &Path, from: i32, to: i32) -> Result<(), Box<dyn Error>> {
    let cb = cfmt::load_codebook(codebook)?;
    let p = cb.prediction_order;
    let from = if from < 0 { 1 } else { from as usize };
    let to = if to < 0 || to as usize > p {
        p
    } else {
        to as usize
    };

    println!("# {}:", codebook.display());
    println!(
        "# P={}  size={}  className='{}'",
        p,
        cb.vectors.len(),
        cb.class_name
    );
    println!(
        "{}",
        (from..=to)
            .map(|k| format!("k{}", k))
            .collect::<Vec<_>>()
            .join(",")
    );
    for refl in &cb.vectors {
        println!(
            "{}",
            (from..=to)
                .map(|k| pf::g(refl[k]))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    println!();
    Ok(())
}
