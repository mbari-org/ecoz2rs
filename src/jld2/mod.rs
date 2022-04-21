extern crate serde;
extern crate structopt;

use std::error::Error;
use std::path::PathBuf;
use std::process;

use hdf5::{File, Result};

#[derive(structopt::StructOpt, Debug)]
pub struct Jld2ShowOpts {
    /// File to read
    #[structopt(short, long, parse(from_os_str))]
    file: PathBuf,
}

// TODO
#[derive(Debug, serde::Deserialize)]
pub struct InstanceInfo {
    pub begin_time: f32,
    pub end_time: f32,
    // ...
}

pub fn main_jld2_show(opts: Jld2ShowOpts) {
    let Jld2ShowOpts { file } = opts;

    let filename: &str = file.to_str().unwrap();

    match load_jld2_file(&filename) {
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

pub fn load_jld2_file(filename: &str) -> Result<Vec<InstanceInfo>, Box<dyn Error>> {
    let file = File::open(filename)?;
    let ds = file.dataset("clicks")?;

    println!("Read dataset: {:?}", ds);

    let pl = ds.as_plist()?;
    println!("pl = {:?}", pl); // <HDF5 property list: unknown class>

    let reader = ds.as_reader();
    println!("reader = {:?}", reader);

    println!("ndim          = {}", ds.ndim());
    println!("dtype         = {:?}", ds.dtype()?);
    println!("layout        = {:?}", ds.layout());
    println!("is_chunked    = {:?}", ds.is_chunked());
    println!("is_resizable  = {:?}", ds.is_resizable());
    println!("is_scalar     = {:?}", ds.is_scalar());
    println!("is_valid      = {:?}", ds.is_valid());
    // println!("read_scalar   = {}", ds.read_scalar::<f32>()?);

    let out_vec = ds.read_raw::<f32>()?;
    for value in out_vec {
        println!("{}", value);
    }

    // TODO
    let instances: Vec<InstanceInfo> = vec![];
    Ok(instances)
}
