extern crate structopt;

use std::error::Error;
use std::path::PathBuf;

use structopt::StructOpt;

use ecoz2_lib::vq_classify;
use ecoz2_lib::vq_learn;
use ecoz2_lib::vq_quantize;
use ecoz2_lib::vq_show;
use utl;

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
    #[structopt(short = "B", long)]
    base_codebook: Option<String>,

    /// Prediction order (required if -B not given).
    #[structopt(short = "P", long)]
    prediction_order: Option<usize>,

    /// Epsilon parameter for convergence.
    #[structopt(short = "e", long = "epsilon", default_value = "0.05")]
    epsilon: f64,

    /// Class name ID to associate to codebook (ignored if -B given).
    #[structopt(short = "w", long = "class-name")]
    class_name: Option<String>,

    /// Predictor files for training. If directories are included, then
    /// all `.prd` under them will be used.
    #[structopt(parse(from_os_str))]
    predictor_filenames: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct VqQuantizeOpts {
    /// Reference codebook for quantization.
    #[structopt(long = "codebook", parse(from_os_str))]
    codebook: PathBuf,

    /// LPC vector sequences to be quantized.
    #[structopt(parse(from_os_str))]
    predictor_filenames: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct VqClassifyOpts {
    /// Show ranked models for incorrect classifications
    #[structopt(short = "r", long)]
    show_ranked: bool,

    /// Codebook models.
    /// If directories are included, then all `.cb` under them will be used.
    #[structopt(
        short,
        long = "codebooks",
        required = true,
        min_values = 1,
        parse(from_os_str)
    )]
    cb_filenames: Vec<PathBuf>,

    /// Predictor files to classify.
    /// If directories are included, then all `.prd` under them will be used.
    #[structopt(
        short,
        long = "predictors",
        required = true,
        min_values = 1,
        parse(from_os_str)
    )]
    prd_filenames: Vec<PathBuf>,
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
        predictor_filenames,
    } = opts;

    if let (Some(_), Some(_)) = (&base_codebook, prediction_order) {
        return Err("Only one of base codebook or prediction order expected").unwrap();
    }

    let codebook_class_name = match class_name {
        Some(name) => name,
        None => "_".to_string(),
    };

    let actual_prd_filenames =
        utl::get_actual_filenames(predictor_filenames, ".prd", "predictors")?;

    fn callback(m: i32, avg_distortion: f64, sigma: f64, inertia: f64) {
        println!(
            "rust callback: M={} avg_distortion={} sigma={} inertia={}",
            m, avg_distortion, sigma, inertia
        );
    }

    vq_learn(
        base_codebook,
        prediction_order,
        epsilon,
        codebook_class_name,
        actual_prd_filenames,
        callback,
    );

    Ok(())
}

pub fn main_vq_quantize(opts: VqQuantizeOpts) -> Result<(), Box<dyn Error>> {
    let VqQuantizeOpts {
        codebook,
        predictor_filenames,
    } = opts;

    let actual_prd_filenames =
        utl::get_actual_filenames(predictor_filenames, ".prd", "predictors")?;

    println!("number of predictor files: {}", actual_prd_filenames.len());

    vq_quantize(codebook, actual_prd_filenames);

    Ok(())
}

pub fn main_vq_classify(opts: VqClassifyOpts) -> Result<(), Box<dyn Error>> {
    let VqClassifyOpts {
        show_ranked,
        cb_filenames,
        prd_filenames,
    } = opts;

    let actual_cb_filenames = utl::get_actual_filenames(cb_filenames, ".cb", "codebooks")?;

    let actual_prd_filenames = utl::get_actual_filenames(prd_filenames, ".prd", "predictors")?;

    println!(
        "number of codebooks: {}  number of predictors: {}",
        actual_cb_filenames.len(),
        actual_prd_filenames.len()
    );
    println!("show_ranked = {}", show_ranked);

    vq_classify(actual_cb_filenames, actual_prd_filenames, show_ranked);

    Ok(())
}

pub fn main_vq_show(opts: VqShowOpts) -> Result<(), Box<dyn Error>> {
    let VqShowOpts { from, to, codebook } = opts;

    vq_show(codebook, from, to);

    Ok(())
}
