use std::error::Error;
use std::path::PathBuf;

use structopt::StructOpt;

use utl;

use self::EcozNBayesCommand::{Classify, Learn, Show};

mod nb;

#[derive(StructOpt, Debug)]
pub struct NBayesMainOpts {
    #[structopt(subcommand)]
    cmd: EcozNBayesCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "nbayes", about = "NBayes operations")]
enum EcozNBayesCommand {
    #[structopt(about = "NBayes training")]
    Learn(NBayesLearnOpts),

    #[structopt(about = "NBayes based classification")]
    Classify(NBayesClassifyOpts),

    #[structopt(about = "Show NBayes model")]
    Show(NBayesShowOpts),
}

#[derive(StructOpt, Debug)]
pub struct NBayesLearnOpts {
    /// Training sequences.
    /// If directories are included, then all `.seq` under them will be used.
    #[structopt(parse(from_os_str))]
    sequence_filenames: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct NBayesClassifyOpts {
    /// Show ranked models for incorrect classifications
    #[structopt(short = "r", long)]
    show_ranked: bool,

    /// NBayes models.
    /// If directories are included, then all `.nbayes` under them will be used.
    #[structopt(
        short,
        long = "models",
        required = true,
        min_values = 1,
        parse(from_os_str)
    )]
    model_filenames: Vec<PathBuf>,

    /// Sequences to classify.
    /// If directories are included, then all `.seq` under them will be used.
    #[structopt(
        short,
        long = "sequences",
        required = true,
        min_values = 1,
        parse(from_os_str)
    )]
    sequence_filenames: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct NBayesShowOpts {
    /// NBayes model.
    #[structopt(short, long, parse(from_os_str))]
    model: PathBuf,
}

pub fn main(opts: NBayesMainOpts) {
    let res = match opts.cmd {
        Learn(opts) => main_nbayes_learn(opts),

        Classify(opts) => main_nbayes_classify(opts),

        Show(opts) => main_nbayes_show(opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn main_nbayes_learn(opts: NBayesLearnOpts) -> Result<(), Box<dyn Error>> {
    let NBayesLearnOpts { sequence_filenames } = opts;

    let actual_sequence_filenames =
        utl::get_actual_filenames(sequence_filenames, ".seq", "sequences")?;

    println!("num_actual_sequences: {}", actual_sequence_filenames.len());

    let mut model = nb::learn(actual_sequence_filenames)?;
    let filename = format!("./{}.nb", model.class_name);
    println!("NBayes model trained: {}", filename);
    model.save(&filename.as_str())
}

pub fn main_nbayes_classify(opts: NBayesClassifyOpts) -> Result<(), Box<dyn Error>> {
    let NBayesClassifyOpts {
        show_ranked,
        model_filenames,
        sequence_filenames,
    } = opts;

    let actual_model_filenames = utl::get_actual_filenames(model_filenames, ".nbayes", "models")?;

    let actual_sequence_filenames =
        utl::get_actual_filenames(sequence_filenames, ".seq", "sequences")?;

    println!(
        "number of NBayes models: {}  number of sequences: {}",
        actual_model_filenames.len(),
        actual_sequence_filenames.len()
    );
    println!("show_ranked = {}", show_ranked);

    //    nbayes_classify(
    //        actual_model_filenames,
    //        actual_sequence_filenames,
    //        show_ranked,
    //    );

    Ok(())
}

pub fn main_nbayes_show(opts: NBayesShowOpts) -> Result<(), Box<dyn Error>> {
    let NBayesShowOpts { model } = opts;

    let mut model = nb::load(model.to_str().unwrap())?;
    &model.show();

    Ok(())
}
