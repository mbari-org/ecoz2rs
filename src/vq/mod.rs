extern crate structopt;

use std::error::Error;
use std::path::PathBuf;

use structopt::StructOpt;

use crate::ecoz2_lib::vq_classify;
use crate::ecoz2_lib::vq_learn;
use crate::ecoz2_lib::vq_quantize;
use crate::ecoz2_lib::vq_show;
use crate::utl;

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
    #[structopt(short = "B", long, name = "codebook")]
    base_codebook: Option<String>,

    /// Prediction order (required if -B not given).
    #[structopt(short = "P", long, name = "P")]
    prediction_order: Option<usize>,

    /// Epsilon parameter for convergence.
    #[structopt(short = "e", long = "epsilon", default_value = "0.05", name = "ε")]
    epsilon: f64,

    /// Class name to associate to generated codebook (ignored if -B given).
    #[structopt(long, name = "class")]
    class_name: Option<String>,

    /// Predictor files for training.
    /// If a single `.csv` file is given, then the "TRAIN" files indicated there will be used.
    /// Otherwise, if directories are included, then all `.prd` under them will be used.
    #[structopt(long = "predictors", parse(from_os_str), name = "files")]
    predictor_filenames: Vec<PathBuf>,

    /// Experiment key to log to comet.
    /// Only has effect if the COMET_API_KEY env var is defined.
    #[structopt(long)]
    exp_key: Option<String>,
}

#[derive(StructOpt, Debug)]
pub struct VqQuantizeOpts {
    /// Reference codebook for quantization.
    #[structopt(long = "codebook", parse(from_os_str))]
    codebook: PathBuf,

    /// LPC vector sequences to be quantized.
    #[structopt(parse(from_os_str))]
    predictor_filenames: Vec<PathBuf>,

    /// Show file names are they are processed.
    #[structopt(short, long)]
    show_filenames: bool,
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
        exp_key,
    } = opts;

    if let (Some(_), Some(_)) = (&base_codebook, prediction_order) {
        return Err("Only one of base codebook or prediction order expected").unwrap();
    }

    let codebook_class_name = match class_name {
        Some(name) => name.clone(),
        None => "_".to_string(),
    };

    let is_tt_list = predictor_filenames.len() == 1
        && predictor_filenames[0].to_str().unwrap().ends_with(".csv");

    let prd_filenames = if is_tt_list {
        let class = codebook_class_name.clone();
        utl::get_files_from_csv(
            &predictor_filenames[0],
            "TRAIN",
            Some(class),
            "predictors",
            ".prd",
        )?
    } else {
        //println!("resolving {:?}", predictor_filenames);
        utl::resolve_filenames(predictor_filenames, ".prd", "predictors")?
    };

    //    for prd_filename in prd_filenames {
    //        println!("  {:?}", prd_filename);
    //    }

    vq_learn(
        base_codebook,
        prediction_order,
        epsilon,
        codebook_class_name,
        prd_filenames,
        exp_key,
    );

    Ok(())
}

pub fn main_vq_quantize(opts: VqQuantizeOpts) -> Result<(), Box<dyn Error>> {
    let VqQuantizeOpts {
        codebook,
        predictor_filenames,
        show_filenames,
    } = opts;

    let prd_filenames = utl::resolve_filenames(predictor_filenames, ".prd", "predictors")?;

    println!("number of predictor files: {}", prd_filenames.len());

    vq_quantize(codebook, prd_filenames, show_filenames);

    Ok(())
}

pub fn main_vq_classify(opts: VqClassifyOpts) -> Result<(), Box<dyn Error>> {
    let VqClassifyOpts {
        show_ranked,
        cb_filenames,
        prd_filenames,
    } = opts;

    let cb_filenames = utl::resolve_filenames(cb_filenames, ".cb", "codebooks")?;

    let prd_filenames = utl::resolve_filenames(prd_filenames, ".prd", "predictors")?;

    println!(
        "number of codebooks: {}  number of predictors: {}",
        cb_filenames.len(),
        prd_filenames.len()
    );
    println!("show_ranked = {}", show_ranked);

    vq_classify(cb_filenames, prd_filenames, show_ranked);

    Ok(())
}

pub fn main_vq_show(opts: VqShowOpts) -> Result<(), Box<dyn Error>> {
    let VqShowOpts { from, to, codebook } = opts;

    vq_show(codebook, from, to);

    Ok(())
}
