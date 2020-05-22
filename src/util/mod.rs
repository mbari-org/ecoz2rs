extern crate structopt;

use std::error::Error;
use std::path::PathBuf;

use rand::seq::SliceRandom;
use rand::thread_rng;
use structopt::StructOpt;

use crate::utl;

use self::EcozUtilCommand::Split;

#[derive(StructOpt, Debug)]
pub struct UtilMainOpts {
    #[structopt(subcommand)]
    cmd: EcozUtilCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "util", about = "Utilities")]
enum EcozUtilCommand {
    #[structopt(about = "Generate train/test instance list")]
    Split(UtilSplitOpts),
}

#[derive(StructOpt, Debug)]
pub struct UtilSplitOpts {
    /// Source files
    #[structopt(
        long,
        name = "files",
        required = true,
        min_values = 1,
        parse(from_os_str)
    )]
    files: Vec<PathBuf>,

    /// File name extension in case of any directories in source files
    #[structopt(long, name = "ext", required = true)]
    file_ext: String,

    /// Fraction for training
    #[structopt(long, name = "fraction", required = true)]
    train_fraction: f32,
}

pub fn main(opts: UtilMainOpts) {
    let res = match opts.cmd {
        Split(opts) => split(opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

fn split(opts: UtilSplitOpts) -> Result<(), Box<dyn Error>> {
    let UtilSplitOpts {
        files,
        file_ext,
        train_fraction,
    } = opts;

    if train_fraction < 0f32 || train_fraction > 1f32 {
        return Err("Invalid train_fraction".into());
    }

    let filenames =
        utl::resolve_filenames(files, &file_ext, format!("{} files", file_ext).as_str()).unwrap();

    let num_train = (train_fraction * filenames.len() as f32) as usize;
    let num_test = filenames.len() - num_train;
    //eprintln!("num_train={}  num_test={}", num_train, num_test);

    let mut markers = {
        let mut trains = vec!["TRAIN".to_string(); num_train];
        let mut tests = vec!["TEST".to_string(); num_test];
        trains.append(&mut tests);
        trains
    };
    markers.shuffle(&mut thread_rng());
    //println!("shuffled markers = {:?}", markers);

    //println!("{},{}", "what", "filename");
    for (marker, filename) in markers.iter().zip(filenames) {
        println!("{},{}", marker, filename.to_str().unwrap());
    }

    Ok(())
}
