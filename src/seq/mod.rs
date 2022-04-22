extern crate structopt;

use std::error::Error;
use std::path::PathBuf;

use structopt::StructOpt;

//use ecoz2_lib::seq_show_files;
//use utl;

use self::EcozSeqCommand::Show;
use crate::utl;

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

    /// Export all the given sequences to the given file (in pickle format).
    #[structopt(long, name = "filename", parse(from_os_str))]
    pub pickle: Option<PathBuf>,

    /// Desired class name when `--pickle` is given
    #[structopt(long, name = "class")]
    class_name: Option<String>,

    /// TRAIN or TEST when `--pickle` is given
    #[structopt(long)]
    tt: Option<String>,

    /// Codebook size when `--pickle` is given
    #[structopt(short = "M", long, name = "#")]
    codebook_size: Option<usize>,

    /// Sequences, gathered according to various parameters.
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

    if let Some(pickle_filename) = &opts.pickle {
        if let (Some(codebook_size), Some(tt)) = (&opts.codebook_size, &opts.tt) {
            let seq_filenames = utl::resolve_files2(
                &opts.seq_filenames,
                tt,
                &opts.class_name,
                format!("sequences/M{}", codebook_size),
                ".seq",
            )?;

            let list_of_sequences = &seq_filenames
                .iter()
                .map(|seq_filename| {
                    let str = seq_filename.to_str().unwrap();
                    load(str).unwrap()
                })
                .map(|sequence| sequence.symbols)
                .collect::<Vec<_>>();

            utl::to_pickle(&list_of_sequences, pickle_filename)?;
            println!(
                "{} sequence(s) saved to {:?}",
                list_of_sequences.len(),
                pickle_filename
            );
            return Ok(());
        } else {
            // TODO more elegant handing
            panic!("--codebook-size and --tt required when --pickle given")
        }
    }

    // NOTE here the gathered sequences are just as explicitly given
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
