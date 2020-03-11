extern crate serde;
extern crate structopt;

use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::process;

use serde::Deserialize;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct CsvShowOpts {
    /// File to read
    #[structopt(short, long, parse(from_os_str))]
    file: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Instance {
    pub selection: i32,

    #[serde(rename = "Begin Time (s)")]
    pub begin_time: f32,

    #[serde(rename = "End Time (s)")]
    pub end_time: f32,

    #[serde(rename = "Type")]
    pub type_: String,
}

pub fn main_csv_show(opts: CsvShowOpts) {
    let CsvShowOpts { file } = opts;

    let filename: &str = file.to_str().unwrap();

    match load_instances(&filename) {
        Ok(instances) => {
            for instance in instances {
                println!("{:?}", instance);
            }
        }
        Err(err) => {
            println!("{}", err);
            process::exit(1);
        }
    }
}

pub fn load_instances(filename: &str) -> Result<Vec<Instance>, Box<dyn Error>> {
    let file = File::open(filename)?;
    let mut rdr = csv::ReaderBuilder::new().delimiter(b'\t').from_reader(file);

    let instances: Vec<Instance> = rdr
        .deserialize()
        .map(|result| {
            let instance: Instance = result.unwrap();
            instance
        })
        .collect::<Vec<_>>();

    Ok(instances)
}
