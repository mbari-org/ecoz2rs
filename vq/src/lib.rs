extern crate libc;
extern crate structopt;
extern crate utl;

use std::error::Error;
use std::ffi::CString;
use std::path::Path;
use std::path::PathBuf;

use libc::c_char;
use libc::c_double;
use libc::c_int;
use structopt::StructOpt;
use EcozVqCommand::{Learn, Quantize};

extern "C" {
    fn vq_learn(
        prediction_order: c_int,
        epsilon: c_double,
        codebook_class_name: *const c_char,
        predictor_filenames: *const *const c_char,
        num_predictors: c_int,
    );

    fn vq_quantize(
        nom_raas: *const c_char,
        predictor_filenames: *const *const c_char,
        num_predictors: c_int,
    );
}

#[derive(StructOpt, Debug)]
pub struct VqMainOpts {
    #[structopt(subcommand)]
    cmd: EcozVqCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "vq", about = "VQ operations")]
enum EcozVqCommand {
    #[structopt(about = "Codebook training")]
    Learn(VqLearnOpts),

    #[structopt(about = "Vector quantization")]
    Quantize(VqQuantizeOpts),
}

#[derive(StructOpt, Debug)]
pub struct VqLearnOpts {
    /// Prediction order
    #[structopt(short = "P", long, default_value = "36")]
    prediction_order: usize,

    /// Epsilon parameter for convergence.
    #[structopt(short = "e", long = "epsilon", default_value = "0.05")]
    epsilon: f64,

    /// Class name ID to associate to codebook.
    #[structopt(short = "w", long = "class-name")]
    class_name: Option<String>,

    /// Predictor files for training. If a directory is given, then
    /// all `.prd` under it will be used.
    #[structopt(parse(from_os_str))]
    predictor_filenames: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct VqQuantizeOpts {
    /// Reference codebook for quantization.
    #[structopt(long = "codebook", parse(from_os_str))]
    codebook: PathBuf,

    /// LPC vector sequences to be quantized.
    #[structopt(parse(from_os_str))]
    predictor_filenames: Vec<PathBuf>,
}

pub fn main(opts: VqMainOpts) {
    let res = match opts.cmd {
        Learn(opts) => main_vq_learn(opts),

        Quantize(opts) => main_vq_quantize(opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn main_vq_learn(opts: VqLearnOpts) -> Result<(), Box<dyn Error>> {
    let VqLearnOpts {
        prediction_order,
        epsilon,
        class_name,
        predictor_filenames,
    } = opts;

    let codebook_class_name = CString::new(match class_name {
        Some(name) => name,
        None => "_".to_string(),
    })
    .unwrap();

    let actual_predictor_filenames = get_actual_predictor_filenames(predictor_filenames)?;

    let num_actual_predictors = actual_predictor_filenames.len() as c_int;
    println!("num_actual_predictors: {}", num_actual_predictors);

    let c_strings: Vec<CString> = to_cstrings(actual_predictor_filenames);

    unsafe {
        let c_predictor_filenames: Vec<*const c_char> = c_strings
            .into_iter()
            .map(|c_string| {
                let ptr = c_string.as_ptr();
                std::mem::forget(c_string);
                ptr
            })
            .collect();

        vq_learn(
            prediction_order as c_int,
            epsilon as c_double,
            codebook_class_name.as_ptr() as *const i8,
            c_predictor_filenames.as_ptr(),
            num_actual_predictors,
        );
    }

    Ok(())
}

pub fn main_vq_quantize(opts: VqQuantizeOpts) -> Result<(), Box<dyn Error>> {
    let VqQuantizeOpts {
        codebook,
        predictor_filenames,
    } = opts;

    let codebook_c_string = CString::new(codebook.to_str().unwrap()).unwrap();

    let actual_predictor_filenames = get_actual_predictor_filenames(predictor_filenames)?;

    let num_actual_predictors = actual_predictor_filenames.len() as c_int;
    println!("num_actual_predictors: {}", num_actual_predictors);

    let c_strings: Vec<CString> = to_cstrings(actual_predictor_filenames);

    unsafe {
        let c_predictor_filenames: Vec<*const c_char> = c_strings
            .into_iter()
            .map(|c_string| {
                let ptr = c_string.as_ptr();
                std::mem::forget(c_string);
                ptr
            })
            .collect();

        vq_quantize(
            codebook_c_string.as_ptr() as *const i8,
            c_predictor_filenames.as_ptr(),
            num_actual_predictors,
        );
    }

    Ok(())
}

fn get_actual_predictor_filenames(
    predictor_filenames: Vec<PathBuf>,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let list = if predictor_filenames.len() == 1 {
        let path = Path::new(&predictor_filenames[0]);
        if path.is_dir() {
            utl::list_files(path, "prd")?
        } else {
            predictor_filenames
        }
    } else {
        predictor_filenames
    };
    Ok(list)
}

fn to_cstrings(paths: Vec<PathBuf>) -> Vec<CString> {
    paths
        .into_iter()
        .map(|predictor_filename| {
            let str = predictor_filename.to_str().unwrap();
            let c_string = CString::new(str).unwrap();
            //println!("c_string = {:?}", c_string);
            c_string
        })
        .collect()
}
