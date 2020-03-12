extern crate ecoz2_lib;
extern crate libc;
extern crate structopt;
extern crate utl;

use std::error::Error;
use std::path::PathBuf;

use ecoz2_lib::ecoz2_hmm_classify;
use ecoz2_lib::ecoz2_hmm_learn;
use structopt::StructOpt;
use EcozHmmCommand::{Classify, Learn};

#[derive(StructOpt, Debug)]
pub struct HmmMainOpts {
    #[structopt(subcommand)]
    cmd: EcozHmmCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "hmm", about = "HMM operations")]
enum EcozHmmCommand {
    #[structopt(about = "Codebook training")]
    Learn(HmmLearnOpts),

    #[structopt(about = "HMM based classification")]
    Classify(HmmClassifyOpts),
}

#[derive(StructOpt, Debug)]
pub struct HmmLearnOpts {
    /// Number of states
    #[structopt(short = "N", long, default_value = "5")]
    num_states: usize,

    /// Type of model to generate:
    ///    0: random values for pi, A, and B
    ///    1: uniform distributions
    ///    2: cascade-2; random B
    ///    3: cascade-3; random B
    #[structopt(short = "t", default_value = "3")]
    type_: usize,

    /// Maximum number of iterations
    #[structopt(short = "I", long)]
    max_iterations: usize,

    /// epsilon restriction on B.
    /// 0 means do not apply this restriction
    #[structopt(short = "e", default_value = "1e-05")]
    epsilon: f64,

    /// val_auto.
    #[structopt(short = "a", default_value = "0.3")]
    val_auto: f64,

    /// Training sequences.
    /// If a directory is given, then all `.seq` under it will be used.
    #[structopt(parse(from_os_str))]
    sequence_filenames: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct HmmClassifyOpts {
    /// Show ranked models for incorrect classifications
    #[structopt(short = "r", long)]
    show_ranked: bool,

    /// HMM models.
    /// If a directory is given, then all `.hmm` under it will be used.
    #[structopt(
        short,
        long = "models",
        required = true,
        min_values = 1,
        parse(from_os_str)
    )]
    model_filenames: Vec<PathBuf>,

    /// Sequences to classify.
    /// If a directory is given, then all `.seq` under it will be used.
    #[structopt(
        short,
        long = "sequences",
        required = true,
        min_values = 1,
        parse(from_os_str)
    )]
    sequence_filenames: Vec<PathBuf>,
}

pub fn main(opts: HmmMainOpts) {
    let res = match opts.cmd {
        Learn(opts) => main_hmm_learn(opts),
        Classify(opts) => main_hmm_classify(opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn main_hmm_learn(opts: HmmLearnOpts) -> Result<(), Box<dyn Error>> {
    let HmmLearnOpts {
        num_states,
        type_,
        max_iterations,
        epsilon,
        val_auto,
        sequence_filenames,
    } = opts;

    let actual_sequence_filenames = utl::get_actual_filenames(sequence_filenames, ".seq")?;

    let num_actual_sequences = actual_sequence_filenames.len();
    println!("num_actual_sequences: {}", num_actual_sequences);
    println!("val_auto = {}", val_auto);

    ecoz2_hmm_learn(
        num_states,
        type_,
        actual_sequence_filenames,
        num_actual_sequences,
        epsilon,
        val_auto,
        max_iterations,
    );

    Ok(())
}

pub fn main_hmm_classify(opts: HmmClassifyOpts) -> Result<(), Box<dyn Error>> {
    let HmmClassifyOpts {
        show_ranked,
        model_filenames,
        sequence_filenames,
    } = opts;

    let actual_model_filenames = utl::get_actual_filenames(model_filenames, ".hmm")?;

    let actual_sequence_filenames = utl::get_actual_filenames(sequence_filenames, ".seq")?;

    println!(
        "num_actual_models: {}  num_actual_sequences: {}",
        actual_model_filenames.len(),
        actual_sequence_filenames.len()
    );
    println!("show_ranked = {}", show_ranked);

    ecoz2_hmm_classify(
        actual_model_filenames,
        actual_sequence_filenames,
        show_ranked,
    );

    Ok(())
}
