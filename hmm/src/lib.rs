extern crate libc;
extern crate structopt;
extern crate utl;

use std::error::Error;
use std::ffi::CString;
use std::path::PathBuf;

use libc::c_char;
use libc::c_double;
use libc::c_int;
use structopt::StructOpt;
use EcozHmmCommand::Learn;

extern "C" {
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

#[derive(StructOpt, Debug)]
pub struct HmmMainOpts {
    #[structopt(subcommand)]
    cmd: EcozHmmCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "hmm", about = "HMM operations")]
enum EcozHmmCommand {
    #[structopt(about = "Codebook training")]
    Learn(HmmLearnOpts),
}

#[derive(StructOpt, Debug)]
pub struct HmmLearnOpts {
    /// Number of states
    #[structopt(short = "N", long, default_value = "5")]
    num_states: usize,

    /// Type of model to generate:
    ///    0: random values for pi, A, and B
    ///    1: uniform distributions
    ///    2: cascade-2; random B
    ///    3: cascade-3; random B
    #[structopt(short = "t", default_value = "3")]
    type_: usize,

    /// Maximum number of iterations
    #[structopt(short = "I", long)]
    max_iterations: usize,

    /// epsilon restriction on B.
    /// 0 means do not apply this restriction
    #[structopt(short = "e", default_value = "1e-05")]
    epsilon: f64,

    /// val_auto.
    #[structopt(short = "a", default_value = "0.3")]
    val_auto: f64,

    /// Training sequences.
    /// If a directory is given, then all `.seq` under it will be used.
    #[structopt(parse(from_os_str))]
    sequence_filenames: Vec<PathBuf>,
}

pub fn main(opts: HmmMainOpts) {
    let res = match opts.cmd {
        Learn(opts) => main_hmm_learn(opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn main_hmm_learn(opts: HmmLearnOpts) -> Result<(), Box<dyn Error>> {
    let HmmLearnOpts {
        num_states,
        type_,
        max_iterations,
        epsilon,
        val_auto,
        sequence_filenames,
    } = opts;

    let actual_sequence_filenames = utl::get_actual_filenames(sequence_filenames, ".seq")?;

    let num_actual_sequences = actual_sequence_filenames.len() as c_int;
    println!("num_actual_sequences: {}", num_actual_sequences);
    println!("val_auto = {}", val_auto);

    let c_strings: Vec<CString> = utl::to_cstrings(actual_sequence_filenames);

    unsafe {
        let c_sequence_filenames: Vec<*const c_char> = c_strings
            .into_iter()
            .map(|c_string| {
                let ptr = c_string.as_ptr();
                std::mem::forget(c_string);
                ptr
            })
            .collect();

        hmm_learn(
            num_states as c_int,
            type_ as c_int,
            c_sequence_filenames.as_ptr(),
            num_actual_sequences,
            epsilon as c_double,
            val_auto as c_double,
            max_iterations as c_int,
        );
    }

    Ok(())
}
