extern crate structopt;

use std::error::Error;
use std::path::PathBuf;

use structopt::StructOpt;

use ecoz2_lib::ecoz2_lpc_signals;
use utl;

#[derive(StructOpt, Debug)]
pub struct LpcOpts {
    /// Prediction order
    #[structopt(short = "P", long, default_value = "36")]
    prediction_order: usize,

    /// Analysis window length in milliseconds
    #[structopt(short = "W", long, default_value = "45")]
    window_length_ms: usize,

    /// Window offset length in milliseconds
    #[structopt(short = "O", long, default_value = "15")]
    offset_length_ms: usize,

    /// Only process a class if it has at least this number of signals
    #[structopt(short = "m", long, default_value = "0")]
    minpc: usize,

    /// Put the generated predictors into two different training
    /// and test subsets (with the given approx ratio)
    #[structopt(short = "s", long, default_value = "0")]
    split: f32,

    /// Signal files to process. If directories are included, then
    /// all `.wav` under them will be used.
    #[structopt(parse(from_os_str))]
    sgn_filenames: Vec<PathBuf>,
}

pub fn main(opts: LpcOpts) {
    let res = main_lpc(opts);

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn main_lpc(opts: LpcOpts) -> Result<(), Box<dyn Error>> {
    let LpcOpts {
        prediction_order,
        window_length_ms,
        offset_length_ms,
        minpc,
        split,
        sgn_filenames,
    } = opts;

    let actual_sgn_filenames = utl::get_actual_filenames(sgn_filenames, ".wav")?;

    ecoz2_lpc_signals(
        prediction_order,
        window_length_ms,
        offset_length_ms,
        minpc,
        split,
        actual_sgn_filenames,
    );

    Ok(())
}
