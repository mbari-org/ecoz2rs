use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use structopt::StructOpt;

use crate::utl;

use self::EcozMMCommand::{Classify, Learn, Show};

mod markov;

#[derive(StructOpt, Debug)]
pub struct MMMainOpts {
    #[structopt(subcommand)]
    cmd: EcozMMCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "mm", about = "MM operations")]
enum EcozMMCommand {
    #[structopt(about = "MM training")]
    Learn(MMLearnOpts),

    #[structopt(about = "MM based classification")]
    Classify(MMClassifyOpts),

    #[structopt(about = "Show MM model")]
    Show(MMShowOpts),
}

#[derive(StructOpt, Debug)]
pub struct MMLearnOpts {
    /// Training sequences.
    /// If directories are included, then all `.seq` under them will be used.
    #[structopt(parse(from_os_str))]
    sequences: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct MMClassifyOpts {
    /// Show ranked models for incorrect classifications
    #[structopt(short = "r", long)]
    show_ranked: bool,

    /// MM models.
    /// If directories are included, then all `.mm` under them will be used.
    #[structopt(long, required = true, min_values = 1, parse(from_os_str))]
    models: Vec<PathBuf>,

    /// Sequences to classify.
    /// If directories are included, then all `.seq` under them will be used.
    #[structopt(long, required = true, min_values = 1, parse(from_os_str))]
    sequences: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct MMShowOpts {
    /// MM model.
    #[structopt(short, long, parse(from_os_str))]
    model: PathBuf,
}

pub fn main(opts: MMMainOpts) {
    let res = match opts.cmd {
        Learn(opts) => main_mm_learn(opts),

        Classify(opts) => main_mm_classify(opts),

        Show(opts) => main_mm_show(opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn main_mm_learn(opts: MMLearnOpts) -> Result<(), Box<dyn Error>> {
    let MMLearnOpts { sequences } = opts;

    let seq_filenames = utl::get_actual_filenames(sequences, ".seq", "sequences")?;

    println!("mm learn: num sequences={}", seq_filenames.len());

    let mut model = markov::learn(&seq_filenames)?;

    let codebook_size = model.pi.len();
    let mm_dir_str = format!("data/mms/M{}", codebook_size);
    let mm_dir = Path::new(&mm_dir_str);
    std::fs::create_dir_all(mm_dir)?;
    let filename = format!("{}/{}.mm", mm_dir.to_str().unwrap(), model.class_name);
    println!("MM model trained: {}", filename);
    model.save(&filename.as_str())
}

pub fn main_mm_classify(opts: MMClassifyOpts) -> Result<(), Box<dyn Error>> {
    let MMClassifyOpts {
        show_ranked,
        models,
        sequences,
    } = opts;

    let mm_filenames = utl::get_actual_filenames(models, ".mm", "models")?;

    let seq_filenames = utl::get_actual_filenames(sequences, ".seq", "sequences")?;

    println!(
        "number of MM models: {}  number of sequences: {}",
        mm_filenames.len(),
        seq_filenames.len()
    );
    println!("show_ranked = {}", show_ranked);

    markov::classify(mm_filenames, seq_filenames, show_ranked)
}

pub fn main_mm_show(opts: MMShowOpts) -> Result<(), Box<dyn Error>> {
    let MMShowOpts { model } = opts;

    let mut model = markov::load(model.to_str().unwrap())?;
    &model.show();

    Ok(())
}
