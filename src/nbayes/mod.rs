use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use structopt::StructOpt;

use crate::utl;

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
    sequences: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct NBayesClassifyOpts {
    /// Show ranked models for incorrect classifications
    #[structopt(short = "r", long)]
    show_ranked: bool,

    /// NBayes models.
    /// If directories are included, then all `.nb` under them will be used.
    #[structopt(long, required = true, min_values = 1, parse(from_os_str))]
    models: Vec<PathBuf>,

    /// Sequences to classify.
    /// If directories are included, then all `.seq` under them will be used.
    #[structopt(long, required = true, min_values = 1, parse(from_os_str))]
    sequences: Vec<PathBuf>,
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
    let NBayesLearnOpts { sequences } = opts;

    let seq_filenames = utl::get_actual_filenames(sequences, ".seq", "sequences")?;

    println!("nbayes learn: num sequences={}", seq_filenames.len());

    let mut model = nb::learn(seq_filenames)?;

    let codebook_size = model.frequencies.len();
    let nb_dir_str = format!("data/nbs/M{}", codebook_size);
    let nb_dir = Path::new(&nb_dir_str);
    std::fs::create_dir_all(nb_dir)?;
    let filename = format!("{}/{}.nb", nb_dir.to_str().unwrap(), model.class_name);
    println!("NBayes model trained: {}", filename);
    model.save(&filename.as_str())
}

pub fn main_nbayes_classify(opts: NBayesClassifyOpts) -> Result<(), Box<dyn Error>> {
    let NBayesClassifyOpts {
        show_ranked,
        models,
        sequences,
    } = opts;

    let nb_filenames = utl::get_actual_filenames(models, ".nb", "models")?;

    let seq_filenames = utl::get_actual_filenames(sequences, ".seq", "sequences")?;

    println!(
        "number of NBayes models: {}  number of sequences: {}",
        nb_filenames.len(),
        seq_filenames.len()
    );
    println!("show_ranked = {}", show_ranked);

    nb::classify(nb_filenames, seq_filenames, show_ranked)
}

pub fn main_nbayes_show(opts: NBayesShowOpts) -> Result<(), Box<dyn Error>> {
    let NBayesShowOpts { model } = opts;

    let mut model = nb::load(model.to_str().unwrap())?;
    &model.show();

    Ok(())
}
