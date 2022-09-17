extern crate clap;
extern crate libc;

use std::error::Error;
use std::path::PathBuf;

use clap::StructOpt;
use colored::*;

use crate::ecoz2_lib::hmm_classify_predictors;
use crate::ecoz2_lib::hmm_classify_sequences;
use crate::ecoz2_lib::hmm_learn;
use crate::ecoz2_lib::hmm_show;
use crate::ecoz2_lib::set_random_seed;
use crate::ecoz2_lib::version;
use crate::utl;

use self::EcozHmmCommand::{Classify, Learn, Show};

#[derive(StructOpt, Debug)]
pub struct HmmMainOpts {
    #[structopt(subcommand)]
    cmd: EcozHmmCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "hmm", about = "HMM operations")]
enum EcozHmmCommand {
    #[structopt(about = "HMM training")]
    Learn(HmmLearnOpts),

    #[structopt(about = "HMM based classification")]
    Classify(HmmClassifyOpts),

    #[structopt(about = "Show HMM model")]
    Show(HmmShowOpts),
}

#[derive(StructOpt, Debug)]
pub struct HmmLearnOpts {
    /// Number of states
    #[structopt(short = 'N', long, name = "N", default_value = "5")]
    num_states: usize,

    /// Number of symbols (codebook size)
    #[structopt(short = 'M', long, name = "M", required = true)]
    codebook_size: usize,

    /// Type of model to generate:
    ///    0: random values for pi, A, and B
    ///    1: uniform distributions
    ///    2: cascade-2; random B
    ///    3: cascade-3; random B
    #[structopt(short = 't', default_value = "3")]
    type_: usize,

    /// Maximum number of iterations. Default (-1) means no limit.
    #[structopt(short = 'I', long, name = "I", default_value = "-1")]
    max_iterations: i32,

    /// epsilon restriction on B.
    /// 0 means do not apply this restriction
    #[structopt(short = 'e', default_value = "1e-05")]
    epsilon: f64,

    /// val_auto.
    #[structopt(short = 'a', default_value = "0.3")]
    val_auto: f64,

    /// Seed for random numbers. Negative means random seed.
    /// Otherwise, the given seed is used, which will allow for reproducibility.
    #[structopt(short = 's', long, default_value = "-1")]
    seed: i64,

    /// Use serialized implementation
    #[structopt(long)]
    ser: bool,

    /// Training sequences.
    /// If a single `.csv` file is given, then the "TRAIN" files indicated there will be used,
    /// and only the ones corresponding to a class name if `--class-name` is given.
    /// Otherwise, if directories are included, then all `.seq` under them will be used.
    #[structopt(long, parse(from_os_str), name = "files")]
    sequences: Vec<PathBuf>,

    /// If training sequences are given via a `.csv` file,
    /// only select the ones with this class name.
    #[structopt(long, name = "class")]
    class_name: Option<String>,
}

#[derive(StructOpt, Debug)]
pub struct HmmClassifyOpts {
    /// Show ranked models for incorrect classifications
    #[structopt(short = 'r', long)]
    show_ranked: bool,

    /// File to report classification results for each sequence.
    #[structopt(short, long = "c12n", parse(from_os_str))]
    classification_filename: Option<PathBuf>,

    /// HMM models.
    /// If directories are included, then all `.hmm` under them will be used.
    #[structopt(short, long, required = true, min_values = 1, parse(from_os_str))]
    models: Vec<PathBuf>,

    /// TRAIN or TEST
    #[structopt(long, required = true)]
    tt: String,

    /// If sequences or predictors are given via a `.csv` file,
    /// only select the ones with this class name.
    #[structopt(long, name = "class")]
    class_name: Option<String>,

    /// Sequences to classify.
    /// If directories are included, then all `.seq` under them will be used.
    #[structopt(
        short,
        long,
        required_unless("predictors"),
        min_values = 1,
        parse(from_os_str)
    )]
    sequences: Vec<PathBuf>,

    /// Number of symbols (codebook size) when `--sequences` with
    /// a `.csv` file is given. Helps determine the path to the sequences.
    #[structopt(short = 'M', long, required = true)]
    codebook_size: usize,

    /// Predictor files to classify.
    /// If a single `.csv` file is given, then only the ones indicated with `--tt` will be used.
    /// Otherwise, if directories are included, then all `.prd` under them will be used.
    #[structopt(long, min_values = 1, parse(from_os_str))]
    predictors: Vec<PathBuf>,

    #[structopt(long, default_value = "data/predictors")]
    predictors_dir_template: String,

    /// Codebook models when `--predictors` is given.
    /// If directories are included, then all `.cb` under them will be used.
    #[structopt(long, required_unless("sequences"), min_values = 1, parse(from_os_str))]
    codebooks: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct HmmShowOpts {
    /// HMM model.
    #[structopt(short, long, parse(from_os_str))]
    hmm: PathBuf,

    /// HMM model.
    #[structopt(short, long, default_value = "%Lg ")]
    format: String,
}

pub fn main(opts: HmmMainOpts) {
    let res = match opts.cmd {
        Learn(opts) => main_hmm_learn(opts),

        Classify(opts) => main_hmm_classify(opts),

        Show(opts) => main_hmm_show(opts),
    };

    if let Err(err) = res {
        println!("{}", err.to_string().red());
    }
}

pub fn main_hmm_learn(opts: HmmLearnOpts) -> Result<(), Box<dyn Error>> {
    let HmmLearnOpts {
        num_states,
        codebook_size,
        type_,
        max_iterations,
        epsilon,
        val_auto,
        seed,
        ser,
        sequences,
        class_name,
    } = opts;

    let seq_filenames = utl::resolve_files(
        sequences,
        "TRAIN",
        class_name,
        format!("sequences/M{}", codebook_size),
        ".seq",
    )?;

    println!("ECOZ2 C version: {}", version()?);

    println!("sequences: {}", seq_filenames.len());
    println!("val_auto = {}", val_auto);

    set_random_seed(seed);

    fn callback(_var: &str, _val: f64) {
        //println!("rust callback called var={} val={}", var, val);
    }

    hmm_learn(
        num_states,
        type_,
        seq_filenames,
        epsilon,
        val_auto,
        max_iterations,
        !ser,
        callback,
    );

    Ok(())
}

pub fn main_hmm_classify(opts: HmmClassifyOpts) -> Result<(), Box<dyn Error>> {
    let HmmClassifyOpts {
        show_ranked,
        classification_filename,
        models,
        tt,
        class_name,
        sequences,
        codebook_size,
        predictors,
        predictors_dir_template,
        codebooks,
    } = opts;

    assert_ne!(predictors.is_empty(), sequences.is_empty());

    let hmm_filenames = utl::resolve_filenames(models, ".hmm", "models")?;

    if !sequences.is_empty() {
        let seq_filenames = utl::resolve_files(
            sequences,
            tt.as_str(),
            class_name,
            format!("sequences/M{}", codebook_size),
            ".seq",
        )?;

        println!("ECOZ2 C version: {}", version()?);

        println!(
            "number of HMM models: {}  number of sequences: {}",
            hmm_filenames.len(),
            seq_filenames.len()
        );
        println!("show_ranked = {}", show_ranked);

        hmm_classify_sequences(
            hmm_filenames,
            seq_filenames,
            show_ranked,
            classification_filename,
        );
    } else {
        let cb_filenames = utl::resolve_filenames(codebooks, ".cbook", "codebooks")?;

        let prd_filenames = utl::resolve_files3(
            &predictors,
            tt.as_str(),
            &class_name,
            "".to_string(),
            predictors_dir_template,
            ".prd",
        )?;

        hmm_classify_predictors(
            hmm_filenames,
            cb_filenames,
            prd_filenames,
            show_ranked,
            classification_filename,
        );
    }

    Ok(())
}

pub fn main_hmm_show(opts: HmmShowOpts) -> Result<(), Box<dyn Error>> {
    let HmmShowOpts { hmm, format } = opts;

    hmm_show(hmm, format);

    Ok(())
}
