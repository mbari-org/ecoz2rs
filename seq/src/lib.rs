extern crate ecoz2_lib;
extern crate structopt;
extern crate utl;

use std::error::Error;
use std::path::PathBuf;

use ecoz2_lib::ecoz2_seq_show_files;
use structopt::StructOpt;
use EcozSeqCommand::Show;

#[derive(StructOpt, Debug)]
pub struct SeqMainOpts {
    #[structopt(subcommand)]
    cmd: EcozSeqCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "seq", about = "Sequence file operations")]
enum EcozSeqCommand {
    #[structopt(about = "Sequence file info")]
    Show(SeqShowOpts),
}

#[derive(StructOpt, Debug)]
pub struct SeqShowOpts {
    /// Show associated probabilities
    #[structopt(short = "P")]
    with_prob: bool,

    /// Show most likely state sequence
    #[structopt(short = "Q")]
    gen_q_opt: bool,

    /// Do not show sequence
    #[structopt(short = "c")]
    no_sequence: bool,

    /// HMM model
    #[structopt(long, parse(from_os_str))]
    hmm: Option<PathBuf>,

    /// Sequences.
    /// If a directory is given, then all `.seq` under it will be used.
    #[structopt(required = true, min_values = 1, parse(from_os_str))]
    seq_filenames: Vec<PathBuf>,
}

pub fn main(opts: SeqMainOpts) {
    let res = match opts.cmd {
        Show(opts) => seq_show(opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn seq_show(opts: SeqShowOpts) -> Result<(), Box<dyn Error>> {
    let SeqShowOpts {
        with_prob,
        gen_q_opt,
        no_sequence,
        hmm,
        seq_filenames,
    } = opts;

    ecoz2_seq_show_files(
        with_prob,
        gen_q_opt,
        no_sequence,
        hmm,
        utl::get_actual_filenames(seq_filenames, ".seq")?,
    );

    Ok(())
}
