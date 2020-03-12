extern crate libc;

use std::ffi::CString;
use std::path::PathBuf;

use libc::c_char;
use libc::c_double;
use libc::c_int;

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

    fn hmm_learn(
        N: c_int,
        model_type: c_int,
        sequence_filenames: *const *const c_char,
        num_sequences: c_int,
        hmm_epsilon: c_double,
        val_auto: c_double,
        max_iterations: c_int,
    );
}

pub fn ecoz2_vq_learn(
    prediction_order: usize,
    epsilon: f64,
    codebook_class_name: String,
    predictor_filenames: Vec<PathBuf>,
    num_predictors: usize,
) {
    let c_strings: Vec<CString> = to_cstrings(predictor_filenames);
    let c_chars: Vec<*const c_char> = to_c_chars(c_strings);

    unsafe {
        let class_name = CString::new(codebook_class_name).unwrap().as_ptr() as *const i8;

        vq_learn(
            prediction_order as c_int,
            epsilon as c_double,
            class_name,
            c_chars.as_ptr(),
            num_predictors as c_int,
        )
    }
}

pub fn ecoz2_vq_quantize(
    nom_raas: PathBuf,
    predictor_filenames: Vec<PathBuf>,
    num_predictors: usize,
) {
    println!("nom_raas = {}", nom_raas.display());

    let codebook_c_string = CString::new(nom_raas.to_str().unwrap()).unwrap();

    let c_strings: Vec<CString> = to_cstrings(predictor_filenames);
    let c_chars: Vec<*const c_char> = to_c_chars(c_strings);

    unsafe {
        let raas_name = codebook_c_string.as_ptr() as *const i8;

        vq_quantize(raas_name, c_chars.as_ptr(), num_predictors as c_int)
    }
}

pub fn ecoz2_hmm_learn(
    n: usize,
    model_type: usize,
    sequence_filenames: Vec<PathBuf>,
    num_sequences: usize,
    hmm_epsilon: f64,
    val_auto: f64,
    max_iterations: usize,
) {
    let c_strings: Vec<CString> = to_cstrings(sequence_filenames);
    let c_chars: Vec<*const c_char> = to_c_chars(c_strings);

    unsafe {
        hmm_learn(
            n as c_int,
            model_type as c_int,
            c_chars.as_ptr(),
            num_sequences as c_int,
            hmm_epsilon as c_double,
            val_auto as c_double,
            max_iterations as c_int,
        );
    }
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

fn to_c_chars(c_strings: Vec<CString>) -> Vec<*const c_char> {
    c_strings
        .into_iter()
        .map(|c_string| {
            let ptr = c_string.as_ptr();
            std::mem::forget(c_string);
            ptr
        })
        .collect()
}
