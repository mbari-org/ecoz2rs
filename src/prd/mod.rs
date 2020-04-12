extern crate structopt;

use std::error::Error;
use std::path::PathBuf;

use structopt::StructOpt;

use ecoz2_lib::prd_show_file;

use self::EcozPrdCommand::Show;

#[derive(StructOpt, Debug)]
pub struct PrdMainOpts {
    #[structopt(subcommand)]
    cmd: EcozPrdCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "prd", about = "Predictor file operations")]
enum EcozPrdCommand {
    #[structopt(about = "Predictor file info")]
    Show(PrdShowOpts),
}

#[derive(StructOpt, Debug)]
pub struct PrdShowOpts {
    /// Show reflection coefficients
    #[structopt(short = "k", long = "reflection")]
    show_reflection: bool,

    /// Start for coefficient range selection
    #[structopt(short = "f", long, default_value = "-1")]
    from: i32,

    /// End for coefficient range selection
    #[structopt(short = "t", long, default_value = "-1")]
    to: i32,

    /// File to read
    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

pub fn main(opts: PrdMainOpts) {
    let res = match opts.cmd {
        Show(opts) => prd_show(opts),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn prd_show(opts: PrdShowOpts) -> Result<(), Box<dyn Error>> {
    let PrdShowOpts {
        show_reflection,
        from,
        to,
        file,
    } = opts;

    prd_show_file(file, show_reflection, from, to);

    Ok(())
}
