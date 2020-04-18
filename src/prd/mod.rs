extern crate serde;
extern crate structopt;

use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use structopt::StructOpt;

use ecoz2_lib::prd_show_file;

use self::EcozPrdCommand::Show;

#[derive(StructOpt, Debug)]
pub struct PrdMainOpts {
    #[structopt(subcommand)]
    cmd: EcozPrdCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "prd", about = "Predictor file operations")]
enum EcozPrdCommand {
    #[structopt(about = "Predictor file info")]
    Show(PrdShowOpts),
}

#[derive(StructOpt, Debug)]
pub struct PrdShowOpts {
    /// Show reflection coefficients
    #[structopt(short = "k", long = "reflection")]
    show_reflection: bool,

    /// Start for coefficient range selection
    #[structopt(short = "f", long, default_value = "1")]
    from: usize,

    /// End for coefficient range selection
    #[structopt(short = "t", long, default_value = "0")]
    to: usize,

    /// File to read
    #[structopt(parse(from_os_str))]
    file: PathBuf,

    /// Use Rust implementation
    #[structopt(long)]
    zrs: bool,
}

pub fn main(opts: PrdMainOpts) {
    let res = match opts.cmd {
        Show(opts) => prd_show(opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn prd_show(opts: PrdShowOpts) -> Result<(), Box<dyn Error>> {
    let PrdShowOpts {
        show_reflection,
        from,
        to,
        file,
        zrs,
    } = opts;

    if zrs {
        prd_show_rs(file, show_reflection, from, to);
    } else {
        prd_show_file(file, show_reflection, from, to);
    }

    Ok(())
}

// NOTE: for Rust implementation (preliminary)

fn prd_show_rs(prd_filename: PathBuf, show_reflections: bool, from: usize, to: usize) {
    let filename = prd_filename.to_str().unwrap();
    let mut prd = load(filename).unwrap();
    println!("# {}", filename);
    prd.show(show_reflections, from, to);
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Predictor {
    pub class_name: String,
    pub prediction_order: usize,
    pub vectors: Vec<Vec<f64>>,
}

impl Predictor {
    pub fn save(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        let f = File::create(filename)?;
        let bw = BufWriter::new(f);
        serde_cbor::to_writer(bw, &self)?;
        Ok(())
    }

    pub fn show(&mut self, show_reflections: bool, from: usize, to: usize) {
        let p = self.prediction_order;
        let to_ = if to == 0 || to > p { p } else { to };

        if show_reflections {
            eprintln!("WARN: show_reflections UNIMPLEMENTED")
        }

        println!(
            "# class_name='{}', T={} P={}",
            self.class_name,
            self.vectors.len(),
            self.prediction_order,
        );
        let mut comma = "";
        for i in from..=to_ {
            print!("{}r{}", comma, i);
            comma = ",";
        }
        println!();
        for v in &self.vectors {
            let mut comma = "";
            for i in from..=to_ {
                print!("{}{:.5}", comma, v[i]);
                comma = ",";
            }
            println!();
        }
    }
}

pub fn load(filename: &str) -> Result<Predictor, Box<dyn Error>> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);
    let predictor = serde_cbor::from_reader(br)?;
    Ok(predictor)
}
