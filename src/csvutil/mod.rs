extern crate serde;
extern crate structopt;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process;

#[derive(structopt::StructOpt, Debug)]
pub struct CsvShowOpts {
    /// File to read
    #[structopt(short, long, parse(from_os_str))]
    file: PathBuf,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InstanceInfo {
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

    match load_instance_info(&filename) {
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

pub fn load_instance_info(filename: &str) -> Result<Vec<InstanceInfo>, Box<dyn Error>> {
    let file = File::open(filename)?;
    let br = BufReader::new(file);
    let mut rdr = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .delimiter(b'\t')
        .from_reader(br);

    let instances: Vec<InstanceInfo> = rdr
        .deserialize()
        .map(|result| result.unwrap())
        .collect::<Vec<_>>();

    Ok(instances)
}
