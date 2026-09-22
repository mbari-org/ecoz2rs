extern crate clap;

use std::error::Error;
use std::path::PathBuf;

use clap::StructOpt;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{rng, RngExt, SeedableRng};
use regex::Regex;

use crate::utl;

mod cmp;

use self::EcozUtilCommand::{Cmp, Split};

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

    #[structopt(about = "Compare ECOZ2 artifacts (files or directories)")]
    Cmp(cmp::UtilCmpOpts),
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

    /// Seed for the shuffle, so a split can be regenerated.
    /// Negative (the default) draws a random one, which is reported on stderr.
    #[structopt(short = 's', long, default_value = "-1")]
    seed: i64,
}

pub fn main(opts: UtilMainOpts) {
    let res = match opts.cmd {
        Split(opts) => split(opts),
        Cmp(opts) => cmp::main(opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

/// TRAIN/TEST markers for `total` instances, in the requested proportion,
/// shuffled with the given seed. Separated out so the reproducibility guarantee
/// can be tested without touching the filesystem.
fn shuffled_markers(total: usize, train_fraction: f32, seed: u64) -> Vec<String> {
    let num_train = (train_fraction * total as f32) as usize;
    let num_test = total - num_train;
    let mut markers = vec!["TRAIN".to_string(); num_train];
    markers.extend(vec!["TEST".to_string(); num_test]);
    markers.shuffle(&mut StdRng::seed_from_u64(seed));
    markers
}

fn split(opts: UtilSplitOpts) -> Result<(), Box<dyn Error>> {
    let UtilSplitOpts {
        files,
        file_ext,
        train_fraction,
        seed,
    } = opts;

    if !(0f32..=1f32).contains(&train_fraction) {
        return Err("Invalid train_fraction".into());
    }

    let filenames = {
        let mut f =
            utl::resolve_filenames(files, &file_ext, format!("{} files", file_ext).as_str())?;
        // Sorted so the markers below land on the same files every time: the
        // listing walks the filesystem, whose order is not guaranteed, and the
        // shuffled markers are zipped against it positionally.
        f.sort();
        f
    };

    // extract class name and selection number:
    let split_re: Regex = Regex::new(r".*/([^/]+)/(\d+)\.[^/]+$").unwrap();
    // Eg.: from "data/predictors/B/00123.prd" -> 1="B", 2="00123"
    let class_and_selections: Vec<(String, String)> = filenames
        .iter()
        .filter_map(|f| {
            let s = f.to_str().unwrap().to_string();
            split_re.captures(&s).map(|caps| {
                let class: String = caps.get(1).unwrap().as_str().to_string();
                let selection: String = caps.get(2).unwrap().as_str().to_string();
                (class, selection)
            })
        })
        .collect();

    // Seeded explicitly so a documented experiment's partition can be
    // regenerated from the command line, rather than only from a checked-in
    // tt-list.csv. An unseeded run reports the seed it drew, on stderr so that
    // stdout stays a clean CSV for `>>` redirection.
    let seed_used: u64 = if seed < 0 {
        rng().random()
    } else {
        seed as u64
    };
    eprintln!("util split: seed={}", seed_used);

    let tt_markers = shuffled_markers(class_and_selections.len(), train_fraction, seed_used);

    // generate output:
    for (tt, (class, selection)) in tt_markers.iter().zip(class_and_selections) {
        println!("{},{},{}", tt, class, selection);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_keep_the_requested_proportion() {
        let m = shuffled_markers(127, 0.8, 42);
        assert_eq!(m.len(), 127);
        assert_eq!(m.iter().filter(|s| *s == "TRAIN").count(), 101);
        assert_eq!(m.iter().filter(|s| *s == "TEST").count(), 26);
    }

    /// The point of `--seed`: the same seed must give the same partition, and a
    /// different seed a different one.
    #[test]
    fn the_same_seed_reproduces_the_same_split() {
        assert_eq!(
            shuffled_markers(127, 0.8, 42),
            shuffled_markers(127, 0.8, 42)
        );
        assert_ne!(
            shuffled_markers(127, 0.8, 42),
            shuffled_markers(127, 0.8, 7)
        );
    }

    #[test]
    fn degenerate_fractions_are_still_consistent() {
        assert!(shuffled_markers(10, 0.0, 1).iter().all(|s| s == "TEST"));
        assert!(shuffled_markers(10, 1.0, 1).iter().all(|s| s == "TRAIN"));
        assert!(shuffled_markers(0, 0.8, 1).is_empty());
    }
}
