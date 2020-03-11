extern crate libc;
extern crate structopt;
extern crate utl;

use std::ffi::CString;
use std::path::Path;
use std::path::PathBuf;

use libc::c_char;
use libc::c_double;
use libc::c_int;
use structopt::StructOpt;

extern "C" {
    fn vq_learn(
        prediction_order: c_int,
        epsilon: c_double,
        codebook_class_name: *const c_char,
        predictor_filenames: *const *const c_char,
        num_predictors: c_int,
    );
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

pub fn main_vq_learn(opts: VqLearnOpts) {
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

    let actual_predictor_filenames: Vec<PathBuf> = if predictor_filenames.len() == 1 {
        let path = Path::new(&predictor_filenames[0]);
        if path.is_dir() {
            match utl::list_files(path, "prd") {
                Ok(list) => list,
                Err(err) => panic!("cannot list dir {}: {}", path.display(), err),
            }
        } else {
            predictor_filenames
        }
    } else {
        predictor_filenames
    };

    let num_actual_predictors = actual_predictor_filenames.len() as c_int;
    println!("num_actual_predictors: {}", num_actual_predictors);

    let c_strings: Vec<CString> = actual_predictor_filenames
        .into_iter()
        .map(|predictor_filename| {
            let str = predictor_filename.to_str().unwrap();
            let c_string = CString::new(str).unwrap();
            //println!("c_string = {:?}", c_string);
            c_string
        })
        .collect();

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
}
