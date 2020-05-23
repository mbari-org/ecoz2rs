use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use structopt::StructOpt;

use crate::utl;

use self::EcozNBayesCommand::{Classify, Learn, Show};

mod nbayes;

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
    /// Number of symbols (codebook size)
    #[structopt(short = "M", long, name = "M", required = true)]
    codebook_size: usize,

    /// Class name for the trained model
    #[structopt(long, name = "class")]
    class_name: Option<String>,

    /// Training sequences.
    /// If a single `.csv` file is given, then the "TRAIN" files indicated there will be used.
    /// Otherwise, if directories are included, then all `.seq` under them will be used.
    #[structopt(parse(from_os_str))]
    sequences: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct NBayesClassifyOpts {
    /// Number of symbols (codebook size)
    #[structopt(short = "M", long, required = true)]
    codebook_size: usize,

    /// Show ranked models for incorrect classifications
    #[structopt(short = "r", long)]
    show_ranked: bool,

    /// TRAIN or TEST
    #[structopt(long, required = true)]
    tt: String,

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
    let NBayesLearnOpts {
        codebook_size,
        class_name,
        sequences,
    } = opts;

    let seq_filenames = utl::resolve_files(
        sequences,
        "TRAIN",
        class_name,
        format!("sequences/M{}", codebook_size),
        ".seq",
    )?;

    let mut model = nbayes::learn(codebook_size, seq_filenames)?;

    let nb_dir_str = format!("data/nbs/M{}", codebook_size);
    let nb_dir = Path::new(&nb_dir_str);
    std::fs::create_dir_all(nb_dir)?;
    let filename = format!("{}/{}.nb", nb_dir.to_str().unwrap(), model.class_name);
    println!("NB model trained");
    model.save(&filename.as_str())?;
    println!("NB model saved: {}\n\n", filename);
    Ok(())
}

pub fn main_nbayes_classify(opts: NBayesClassifyOpts) -> Result<(), Box<dyn Error>> {
    let NBayesClassifyOpts {
        codebook_size,
        show_ranked,
        tt,
        models,
        sequences,
    } = opts;

    let nb_filenames = utl::resolve_filenames(models, ".nb", "models")?;

    let seq_filenames = utl::resolve_files(
        sequences,
        tt.as_str(),
        None,
        format!("sequences/M{}", codebook_size),
        ".seq",
    )?;

    println!(
        "number of NBayes models: {}  number of sequences: {}",
        nb_filenames.len(),
        seq_filenames.len()
    );
    println!("show_ranked = {}", show_ranked);

    nbayes::classify(nb_filenames, seq_filenames, show_ranked)
}

pub fn main_nbayes_show(opts: NBayesShowOpts) -> Result<(), Box<dyn Error>> {
    let NBayesShowOpts { model } = opts;

    let mut model = nbayes::load(model.to_str().unwrap())?;
    &model.show();

    Ok(())
}
