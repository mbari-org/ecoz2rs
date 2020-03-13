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

    fn hmm_classify(
        model_filenames: *const *const c_char,
        num_models: c_int,
        sequence_filenames: *const *const c_char,
        num_sequences: c_int,
        show_ranked: c_int,
    );
}

pub fn ecoz2_vq_learn(
    prediction_order: usize,
    epsilon: f64,
    codebook_class_name: String,
    predictor_filenames: Vec<PathBuf>,
) {
    let vpc_predictors: Vec<*const c_char> = to_vec_of_ptr_const_c_char(predictor_filenames);

    unsafe {
        let class_name = CString::new(codebook_class_name).unwrap().as_ptr() as *const i8;

        vq_learn(
            prediction_order as c_int,
            epsilon as c_double,
            class_name,
            vpc_predictors.as_ptr(),
            vpc_predictors.len() as c_int,
        )
    }
}

pub fn ecoz2_vq_quantize(nom_raas: PathBuf, predictor_filenames: Vec<PathBuf>) {
    println!("nom_raas = {}", nom_raas.display());

    let codebook_c_string = CString::new(nom_raas.to_str().unwrap()).unwrap();

    let vpc_predictors: Vec<*const c_char> = to_vec_of_ptr_const_c_char(predictor_filenames);

    unsafe {
        let raas_name = codebook_c_string.as_ptr() as *const i8;

        vq_quantize(
            raas_name,
            vpc_predictors.as_ptr(),
            vpc_predictors.len() as c_int,
        )
    }
}

pub fn ecoz2_hmm_learn(
    n: usize,
    model_type: usize,
    sequence_filenames: Vec<PathBuf>,
    hmm_epsilon: f64,
    val_auto: f64,
    max_iterations: usize,
) {
    let vpc_sequences: Vec<*const c_char> = to_vec_of_ptr_const_c_char(sequence_filenames);

    unsafe {
        hmm_learn(
            n as c_int,
            model_type as c_int,
            vpc_sequences.as_ptr(),
            vpc_sequences.len() as c_int,
            hmm_epsilon as c_double,
            val_auto as c_double,
            max_iterations as c_int,
        );
    }
}

pub fn ecoz2_hmm_classify(
    model_filenames: Vec<PathBuf>,
    sequence_filenames: Vec<PathBuf>,
    show_ranked: bool,
) {
    let vpc_models: Vec<*const c_char> = to_vec_of_ptr_const_c_char(model_filenames);

    let vpc_sequences: Vec<*const c_char> = to_vec_of_ptr_const_c_char(sequence_filenames);

    unsafe {
        hmm_classify(
            vpc_models.as_ptr(),
            vpc_models.len() as c_int,
            vpc_sequences.as_ptr(),
            vpc_sequences.len() as c_int,
            show_ranked as c_int,
        );
    }
}

fn to_vec_of_ptr_const_c_char(paths: Vec<PathBuf>) -> Vec<*const c_char> {
    let vec_of_cstring: Vec<CString> = paths
        .into_iter()
        .map(|predictor_filename| {
            let str = predictor_filename.to_str().unwrap();
            let c_string = CString::new(str).unwrap();
            //println!("c_string = {:?}", c_string);
            c_string
        })
        .collect();

    vec_of_cstring
        .into_iter()
        .map(|c_string| {
            let ptr = c_string.as_ptr();
            std::mem::forget(c_string);
            ptr
        })
        .collect()
}
