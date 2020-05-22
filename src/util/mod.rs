extern crate structopt;

use std::error::Error;
use std::path::PathBuf;

use rand::seq::SliceRandom;
use rand::thread_rng;
use regex::Regex;
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
    /// Source files only to extract selection number and class
    #[structopt(
        long,
        name = "files",
        required = true,
        min_values = 1,
        parse(from_os_str)
    )]
    files: Vec<PathBuf>,

    /// File name extension used for the extraction and also
    /// in case of any directories in source files
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

    // extract class name and selection number:
    let split_re: Regex = Regex::new(r".*/([^/]+)/(\d+)\.[^/]+$").unwrap();
    // Eg.: from "data/predictors/B/00123.prd" -> 1="B", 2="00123"
    let class_and_selections: Vec<(String, String)> = filenames
        .iter()
        .map(|f| {
            let s = f.to_str().unwrap().to_string();
            split_re.captures(&s).map(|caps| {
                let class: String = caps.get(1).unwrap().as_str().to_string();
                let selection: String = caps.get(2).unwrap().as_str().to_string();
                (class, selection)
            })
        })
        .flatten()
        .collect();

    let num_train = (train_fraction * class_and_selections.len() as f32) as usize;
    let num_test = class_and_selections.len() - num_train;
    //eprintln!("num_train={}  num_test={}", num_train, num_test);

    // get TRAIN and TEST markers in the given proportion:
    let mut tt_markers = {
        let mut trains = vec!["TRAIN".to_string(); num_train];
        let mut tests = vec!["TEST".to_string(); num_test];
        trains.append(&mut tests);
        trains
    };
    // shuffle the markers:
    tt_markers.shuffle(&mut thread_rng());

    // generate output:
    for (tt, (class, selection)) in tt_markers.iter().zip(class_and_selections) {
        println!("{},{},{}", tt, class, selection);
    }

    Ok(())
}
