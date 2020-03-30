extern crate structopt;

use std::error::Error;
use std::path::PathBuf;

use structopt::StructOpt;

use ecoz2_lib::ecoz2_vq_classify;
use ecoz2_lib::ecoz2_vq_learn;
use ecoz2_lib::ecoz2_vq_quantize;
use ecoz2_lib::ecoz2_vq_show;
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
    /// Prediction order
    #[structopt(short = "P", long, default_value = "36")]
    prediction_order: usize,

    /// Epsilon parameter for convergence.
    #[structopt(short = "e", long = "epsilon", default_value = "0.05")]
    epsilon: f64,

    /// Class name ID to associate to codebook.
    #[structopt(short = "w", long = "class-name")]
    class_name: Option<String>,

    /// Predictor files for training. If a directory is given, then
    /// all `.prd` under it will be used.
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
    /// If a directory is given, then all `.cb` under it will be used.
    #[structopt(
        short,
        long = "codebooks",
        required = true,
        min_values = 1,
        parse(from_os_str)
    )]
    cb_filenames: Vec<PathBuf>,

    /// Predictor files to classify.
    /// If a directory is given, then all `.prd` under it will be used.
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
        prediction_order,
        epsilon,
        class_name,
        predictor_filenames,
    } = opts;

    let codebook_class_name = match class_name {
        Some(name) => name,
        None => "_".to_string(),
    };

    let actual_predictor_filenames = utl::get_actual_filenames(predictor_filenames, ".prd")?;

    ecoz2_vq_learn(
        prediction_order,
        epsilon,
        codebook_class_name,
        actual_predictor_filenames,
    );

    Ok(())
}

pub fn main_vq_quantize(opts: VqQuantizeOpts) -> Result<(), Box<dyn Error>> {
    let VqQuantizeOpts {
        codebook,
        predictor_filenames,
    } = opts;

    let actual_predictor_filenames = utl::get_actual_filenames(predictor_filenames, ".prd")?;

    println!(
        "number of predictor files: {}",
        actual_predictor_filenames.len()
    );

    ecoz2_vq_quantize(codebook, actual_predictor_filenames);

    Ok(())
}

pub fn main_vq_classify(opts: VqClassifyOpts) -> Result<(), Box<dyn Error>> {
    let VqClassifyOpts {
        show_ranked,
        cb_filenames,
        prd_filenames,
    } = opts;

    let actual_cb_filenames = utl::get_actual_filenames(cb_filenames, ".cb")?;

    let actual_prd_filenames = utl::get_actual_filenames(prd_filenames, ".prd")?;

    println!(
        "number of codebooks: {}  number of predictors: {}",
        actual_cb_filenames.len(),
        actual_prd_filenames.len()
    );
    println!("show_ranked = {}", show_ranked);

    ecoz2_vq_classify(actual_cb_filenames, actual_prd_filenames, show_ranked);

    Ok(())
}

pub fn main_vq_show(opts: VqShowOpts) -> Result<(), Box<dyn Error>> {
    let VqShowOpts { from, to, codebook } = opts;

    ecoz2_vq_show(codebook, from, to);

    Ok(())
}
