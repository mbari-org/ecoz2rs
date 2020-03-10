extern crate structopt;
use structopt::StructOpt;

extern crate serde;
use serde::Deserialize;

use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

#[derive(StructOpt, Debug)]
pub struct CsvShowOpts {
    /// File to read
    #[structopt(short, long, parse(from_os_str))]
    file: PathBuf,
}

pub fn main_csv_show(opts: CsvShowOpts) {
    let CsvShowOpts { file } = opts;

    let filename: &str = file.to_str().unwrap();

    let s = read_selections(&filename);
    println!("Selections loaded: {}, s={:?}", filename, s);
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    selection: i32,

    view: String,

    channel: i32,

    #[serde(rename = "Begin Time (s)")]
    begin_time: f32,

    #[serde(rename = "End Time (s)")]
    end_time: f32,

    #[serde(rename = "Low Freq (Hz)")]
    low_freq: f32,

    #[serde(rename = "High Freq (Hz)")]
    high_freq: f32,
}

pub fn read_selections(filename: &str) -> Result<(), Box<dyn Error>> {
    let file = File::open(filename)?;
    let mut rdr = csv::ReaderBuilder::new().delimiter(b'\t').from_reader(file);

    for result in rdr.deserialize() {
        let record: Record = result?;
        println!("{:?}", record);
    }
    Ok(())
}
