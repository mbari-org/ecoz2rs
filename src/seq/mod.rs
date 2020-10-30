extern crate structopt;

use std::error::Error;
use std::path::PathBuf;

use structopt::StructOpt;

//use ecoz2_lib::seq_show_files;
//use utl;

use self::EcozSeqCommand::Show;

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
    // The TODO's because of pending impl on rust side.
    /// (TODO) Show associated probabilities
    #[structopt(short = "P")]
    pub with_prob: bool,

    /// (TODO) Show most likely state sequence
    #[structopt(short = "Q")]
    pub gen_q_opt: bool,

    /// Do not show sequence
    #[structopt(short = "c")]
    pub no_sequence: bool,

    /// (TODO) HMM model
    #[structopt(long, parse(from_os_str))]
    pub hmm: Option<PathBuf>,

    /// Only show length of the sequence
    #[structopt(short = "L")]
    pub only_length: bool,

    /// Show full sequence (by default, abbreviated unless very short)
    #[structopt(long)]
    pub full: bool,

    /// Export to the given file in pickle format.
    #[structopt(long, parse(from_os_str))]
    pub pickle: Option<PathBuf>,

    /// Sequences.
    /// If directories are included, then all `.seq` under them will be used.
    #[structopt(required = true, min_values = 1, parse(from_os_str))]
    pub seq_filenames: Vec<PathBuf>,
}

pub fn main(opts: SeqMainOpts) {
    let res = match opts.cmd {
        Show(opts) => seq_show(&opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn seq_show(opts: &SeqShowOpts) -> Result<(), Box<dyn Error>> {
    use crate::sequence::load;
    for seq_filename in &opts.seq_filenames {
        let mut seq = load(seq_filename.to_str().unwrap())?;
        seq.show(opts);
    }

    // let SeqShowOpts {
    //     with_prob: _,
    //     gen_q_opt: _,
    //     no_sequence: _,
    //     hmm: _,
    //     seq_filenames,
    // } = opts;
    //    seq_show_files(
    //        with_prob,
    //        gen_q_opt,
    //        no_sequence,
    //        hmm,
    //        utl::get_actual_filenames(seq_filenames, ".seq", "sequences")?,
    //    );

    Ok(())
}
