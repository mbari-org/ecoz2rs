extern crate libc;

pub mod lpca_c;

use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::{c_float, c_long, c_ulong};
use std::path::PathBuf;
use std::str::Utf8Error;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;

use crate::comet_client::CometClient;

use self::libc::{c_char, c_double, c_int, c_uint};

use std::collections::HashMap;

// https://users.rust-lang.org/t/ownership-issue-with-a-static-hashmap/27239

lazy_static! {
    static ref COMET_CLIENTS: RwLock<HashMap<c_int, Arc<CometClient>>> =
        RwLock::new(HashMap::new());
}
static mut NEXT_REF_ID: c_int = 0;

pub fn register_cc(comet_client: CometClient) -> c_int {
    let ref_id: c_int;
    unsafe {
        ref_id = NEXT_REF_ID;
        NEXT_REF_ID += 1;
    };
    COMET_CLIENTS
        .write()
        .unwrap()
        .insert(ref_id, Arc::new(comet_client));

    ref_id
}

pub fn unregister_cc(ref_id: c_int) {
    COMET_CLIENTS.write().unwrap().remove(&ref_id);
}

pub fn find_cc(ref_id: &c_int) -> Option<Arc<CometClient>> {
    COMET_CLIENTS
        .read()
        .unwrap()
        .get(ref_id)
        .map(|v| Arc::clone(v))
}

#[repr(C)]
pub struct Ecoz2ObserverRef {
    ref_id: c_int,
}

impl Ecoz2ObserverRef {
    fn new(ref_id: c_int) -> Self {
        Ecoz2ObserverRef { ref_id }
    }

    fn step(&self, m: i32, avg_distortion: f64, sigma: f64, inertia: f64) {
        println!(
            "   Ecoz2ObserverRef.step: M={} avg_distortion={} sigma={} inertia={}",
            m, avg_distortion, sigma, inertia
        );
        if let Some(a) = find_cc(&self.ref_id) {
            a.log_vq_learn(m, avg_distortion, sigma, inertia);
        }
    }
}

extern "C" {
    fn ecoz2_version() -> *const c_char;

    fn ecoz2_set_random_seed(seed: c_long) -> c_ulong;

    fn ecoz2_lpc_signals(
        prediction_order: c_int,
        window_length_ms: c_int,
        offset_length_ms: c_int,
        minpc: c_int,
        split: c_float,
        sgn_filenames: *const *const c_char,
        num_signals: c_int,
        mintrpt: c_float, // min time in secs to report processing time per signal
    );

    fn ecoz2_prd_show_file(
        prd_filename: *const c_char,
        show_reflections: c_int,
        from: c_int,
        to: c_int,
    );

    fn ecoz2_vq_learn(
        prediction_order: c_int,
        epsilon: c_double,
        codebook_class_name: *const c_char,
        predictor_filenames: *const *const c_char,
        num_predictors: c_int,

        target: *mut Ecoz2ObserverRef,
        callback: extern "C" fn(*mut Ecoz2ObserverRef, c_int, c_double, c_double, c_double),
    );

    fn ecoz2_vq_learn_using_base_codebook(
        base_codebook: *const c_char,
        epsilon: c_double,
        predictor_filenames: *const *const c_char,
        num_predictors: c_int,

        target: *mut Ecoz2ObserverRef,
        callback: extern "C" fn(*mut Ecoz2ObserverRef, c_int, c_double, c_double, c_double),
    );

    fn ecoz2_vq_quantize(
        nom_raas: *const c_char,
        predictor_filenames: *const *const c_char,
        num_predictors: c_int,
        show_filenames: c_int,
    );

    fn ecoz2_vq_classify(
        cb_filenames: *const *const c_char,
        num_codebooks: c_int,
        prd_filenames: *const *const c_char,
        num_predictors: c_int,
        show_ranked: c_int,
    );

    fn ecoz2_vq_show(codebook_filename: *const c_char, from: c_int, to: c_int);

    fn ecoz2_hmm_learn(
        N: c_int,
        model_type: c_int,
        sequence_filenames: *const *const c_char,
        num_sequences: c_uint,
        hmm_epsilon: c_double,
        val_auto: c_double,
        max_iterations: c_int,
        use_par: c_int,

        callback: extern "C" fn(*mut c_char, c_double),
    );

    fn ecoz2_hmm_classify(
        model_filenames: *const *const c_char,
        num_models: c_uint,
        sequence_filenames: *const *const c_char,
        num_sequences: c_uint,
        show_ranked: c_int,
        classification_filename: *const c_char,
    );

    fn ecoz2_hmm_show(hmm_filename: *const c_char, format: *const c_char);

// TODO to be removed
//    fn ecoz2_seq_show_files(
//        with_prob: c_int,
//        gen_q_opt: c_int,
//        show_sequence: c_int,
//        hmm_filename: *const c_char,
//        sequence_filenames: *const *const c_char,
//        num_sequences: c_int,
//    );
}

pub fn version() -> Result<&'static str, Utf8Error> {
    unsafe {
        let char_ptr = ecoz2_version();
        let c_str = CStr::from_ptr(char_ptr);
        c_str.to_str()
    }
}

pub fn set_random_seed(seed: i64) -> u64 {
    unsafe { ecoz2_set_random_seed(seed as c_long) as u64 }
}

pub fn lpc_signals(
    prediction_order: usize,
    window_length_ms: usize,
    offset_length_ms: usize,
    minpc: usize,
    split: f32,
    sgn_filenames: Vec<PathBuf>,
    mintrpt: f32,
) {
    let vpc_signals: Vec<*const c_char> = to_vec_of_ptr_const_c_char(sgn_filenames);

    unsafe {
        ecoz2_lpc_signals(
            prediction_order as c_int,
            window_length_ms as c_int,
            offset_length_ms as c_int,
            minpc as c_int,
            split as c_float,
            vpc_signals.as_ptr(),
            vpc_signals.len() as c_int,
            mintrpt as c_float,
        )
    }
}

pub fn prd_show_file(prd_filename: PathBuf, show_reflections: bool, from: usize, to: usize) {
    let prd_filename_c_string = CString::new(prd_filename.to_str().unwrap()).unwrap();

    unsafe {
        let filename = prd_filename_c_string.as_ptr() as *const i8;

        ecoz2_prd_show_file(
            filename,
            show_reflections as c_int,
            from as c_int,
            to as c_int,
        )
    }
}

#[no_mangle]
extern "C" fn c_vq_learn_callback(
    target: *mut Ecoz2ObserverRef,
    m: c_int,
    avg_distortion: c_double,
    sigma: c_double,
    inertia: c_double,
) {
    unsafe {
        (*target).step(
            m as i32,
            avg_distortion as f64,
            sigma as f64,
            inertia as f64,
        )
    }
}

pub fn vq_learn(
    base_codebook_opt: Option<String>,
    prediction_order_opt: Option<usize>,
    epsilon: f64,
    codebook_class_name: String,
    predictor_filenames: Vec<PathBuf>,
    exp_key: Option<String>,
) {
    assert_ne!(base_codebook_opt.is_some(), prediction_order_opt.is_some());

    println!(
        "vq_learn: base_codebook_opt={:?} prediction_order={:?}, epsilon={} codebook_class_name={} predictor_filenames: {}",
        base_codebook_opt,
        prediction_order_opt,
        epsilon,
        &codebook_class_name,
        predictor_filenames.len()
    );

    // thread needed to increment stack size, currently required
    // for the C implementation
    let child = thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let class_name = CString::new(codebook_class_name).unwrap();
            let vpc_predictors: Vec<*const c_char> =
                to_vec_of_ptr_const_c_char(predictor_filenames);

            let comet_client = CometClient::new(exp_key);

            if let Some(p) = prediction_order_opt {
                comet_client.log_parameter(&"P".to_string(), &format!("{}", p));
            }
            comet_client.log_parameter(&"epsilon".to_string(), &format!("{}", epsilon));

            let ref_id = register_cc(comet_client);

            let observer = Ecoz2ObserverRef::new(ref_id);
            let mut obs_ref = Box::new(observer);

            unsafe {
                match base_codebook_opt {
                    Some(base_codebook) => ecoz2_vq_learn_using_base_codebook(
                        base_codebook.as_ptr() as *const i8,
                        epsilon as c_double,
                        vpc_predictors.as_ptr(),
                        vpc_predictors.len() as c_int,
                        &mut *obs_ref,
                        c_vq_learn_callback,
                    ),

                    None => {
                        let prediction_order = prediction_order_opt.unwrap();
                        ecoz2_vq_learn(
                            prediction_order as c_int,
                            epsilon as c_double,
                            class_name.as_ptr() as *const i8,
                            vpc_predictors.as_ptr(),
                            vpc_predictors.len() as c_int,
                            &mut *obs_ref,
                            c_vq_learn_callback,
                        )
                    }
                }
            }

            unregister_cc(ref_id);
        })
        .unwrap();

    child.join().unwrap();
}

pub fn vq_quantize(nom_raas: PathBuf, predictor_filenames: Vec<PathBuf>, show_filenames: bool) {
    println!("nom_raas = {}", nom_raas.display());

    let codebook_c_string = CString::new(nom_raas.to_str().unwrap()).unwrap();

    let vpc_predictors: Vec<*const c_char> = to_vec_of_ptr_const_c_char(predictor_filenames);

    unsafe {
        let raas_name = codebook_c_string.as_ptr() as *const i8;

        ecoz2_vq_quantize(
            raas_name,
            vpc_predictors.as_ptr(),
            vpc_predictors.len() as c_int,
            (if show_filenames { 1 } else { 0 }) as c_int,
        )
    }
}

pub fn vq_classify(cb_filenames: Vec<PathBuf>, prd_filenames: Vec<PathBuf>, show_ranked: bool) {
    let vpc_codebooks: Vec<*const c_char> = to_vec_of_ptr_const_c_char(cb_filenames);

    let vpc_predictors: Vec<*const c_char> = to_vec_of_ptr_const_c_char(prd_filenames);

    unsafe {
        ecoz2_vq_classify(
            vpc_codebooks.as_ptr(),
            vpc_codebooks.len() as c_int,
            vpc_predictors.as_ptr(),
            vpc_predictors.len() as c_int,
            show_ranked as c_int,
        );
    }
}

pub fn vq_show(codebook_filename: PathBuf, from: i32, to: i32) {
    println!("codebook_filename = {}", codebook_filename.display());

    let codebook_c_string = CString::new(codebook_filename.to_str().unwrap()).unwrap();

    unsafe {
        ecoz2_vq_show(
            codebook_c_string.as_ptr() as *const i8,
            from as c_int,
            to as c_int,
        )
    }
}

// HMM_LEARN_CALLBACK: to control that only one ongoing ecoz2_hmm_learn call is running.
// Need to do this along with the c_hmm_learn_callback below because using a closure
// within hmm_learn may not be possible, or be more complicated.
static mut HMM_LEARN_CALLBACK: Option<fn(&str, f64)> = None;

#[no_mangle]
extern "C" fn c_hmm_learn_callback(variable: *mut c_char, value: c_double) {
    unsafe {
        if let Some(cb) = HMM_LEARN_CALLBACK {
            let c_string = CStr::from_ptr(variable);
            //println!("   c_hmm_learn_callback: var={:?} val={}", c_string, value);
            let var = c_string.to_str().unwrap();
            let val = value as f64;
            cb(var, val)
        }
    }
}

pub fn hmm_learn(
    n: usize,
    model_type: usize,
    sequence_filenames: Vec<PathBuf>,
    hmm_epsilon: f64,
    val_auto: f64,
    max_iterations: i32,
    use_par: bool,
    callback: fn(&str, f64),
) {
    unsafe {
        match HMM_LEARN_CALLBACK {
            Some(_) => panic!("Ongoing ecoz2_hmm_learn call"),
            None => HMM_LEARN_CALLBACK = Some(callback),
        }
    }

    let vpc_sequences: Vec<*const c_char> = to_vec_of_ptr_const_c_char(sequence_filenames);

    unsafe {
        ecoz2_hmm_learn(
            n as c_int,
            model_type as c_int,
            vpc_sequences.as_ptr(),
            vpc_sequences.len() as c_uint,
            hmm_epsilon as c_double,
            val_auto as c_double,
            max_iterations as c_int,
            if use_par { 1 } else { 0 } as c_int,
            c_hmm_learn_callback,
        );

        HMM_LEARN_CALLBACK = None;
    }
}

pub fn hmm_classify(
    model_filenames: Vec<PathBuf>,
    sequence_filenames: Vec<PathBuf>,
    show_ranked: bool,
    classification_filename: Option<PathBuf>,
) {
    let vpc_models: Vec<*const c_char> = to_vec_of_ptr_const_c_char(model_filenames);

    let vpc_sequences: Vec<*const c_char> = to_vec_of_ptr_const_c_char(sequence_filenames);

    let classification_filename: *const c_char = match classification_filename {
        Some(filename) => to_ptr_const_c_char(filename),
        None => std::ptr::null(),
    };

    unsafe {
        ecoz2_hmm_classify(
            vpc_models.as_ptr(),
            vpc_models.len() as c_uint,
            vpc_sequences.as_ptr(),
            vpc_sequences.len() as c_uint,
            show_ranked as c_int,
            classification_filename,
        );
    }
}

pub fn hmm_show(hmm_filename: PathBuf, format: String) {
    println!(
        "hmm_show: hmm_filename={} format={}",
        hmm_filename.display(),
        format
    );

    let hmm_c_string = CString::new(hmm_filename.to_str().unwrap()).unwrap();
    let format_c_string = CString::new(format).unwrap();

    unsafe {
        ecoz2_hmm_show(
            hmm_c_string.as_ptr() as *const i8,
            format_c_string.as_ptr() as *const i8,
        );
    }
}

// TODO to be removed once rust impl is completed
/*
pub fn seq_show_files(
    with_prob: bool,
    gen_q_opt: bool,
    no_sequence: bool,
    hmm_filename_opt: Option<PathBuf>,
    seq_filenames: Vec<PathBuf>,
) {
    println!(
        "\nseq_show_files: with_prob={} gen_q_opt={} no_sequence={} hmm_filename_opt={:?} seq_filenames={}\n",
        with_prob, gen_q_opt, no_sequence, hmm_filename_opt, seq_filenames.len()
    );

    let hmm_c_string = match hmm_filename_opt {
        Some(hmm_filename) => CString::new(hmm_filename.to_str().unwrap()).unwrap(),
        None => CString::new("").unwrap(),
    };

    let vpc_sequences: Vec<*const c_char> = to_vec_of_ptr_const_c_char(seq_filenames);

    unsafe {
        ecoz2_seq_show_files(
            with_prob as c_int,
            gen_q_opt as c_int,
            no_sequence as c_int,
            hmm_c_string.as_ptr() as *const i8,
            vpc_sequences.as_ptr(),
            vpc_sequences.len() as c_int,
        );
    }
}
*/

fn to_vec_of_ptr_const_c_char(paths: Vec<PathBuf>) -> Vec<*const c_char> {
    let vec_of_cstring: Vec<CString> = paths
        .into_iter()
        .map(|path| {
            let str = path.to_str().unwrap();
            CString::new(str).unwrap()
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

fn to_ptr_const_c_char(path: PathBuf) -> *const c_char {
    let c_string = {
        let str = path.to_str().unwrap();
        CString::new(str).unwrap()
    };

    let ptr = c_string.as_ptr();
    std::mem::forget(c_string);
    ptr
}
