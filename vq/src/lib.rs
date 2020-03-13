extern crate ecoz2_lib;
extern crate structopt;
extern crate utl;

use std::error::Error;
use std::path::PathBuf;

use ecoz2_lib::ecoz2_vq_learn;
use ecoz2_lib::ecoz2_vq_quantize;
use structopt::StructOpt;
use EcozVqCommand::{Learn, Quantize};

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

pub fn main(opts: VqMainOpts) {
    let res = match opts.cmd {
        Learn(opts) => main_vq_learn(opts),

        Quantize(opts) => main_vq_quantize(opts),
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

    println!(
        "num_actual_predictors: {}",
        actual_predictor_filenames.len()
    );

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
        "num_actual_predictors: {}",
        actual_predictor_filenames.len()
    );

    ecoz2_vq_quantize(codebook, actual_predictor_filenames);

    Ok(())
}
