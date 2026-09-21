#[macro_use]
extern crate assert_approx_eq;
extern crate byteorder;
extern crate clap;
extern crate colored;
extern crate itertools;
extern crate ndarray;
extern crate num_cpus;
extern crate serde;
extern crate serde_json;

use clap::Parser;
use clap::StructOpt;

mod c12n;
mod csvutil;
mod hmm;
mod lpc;
mod mm;
mod nb;
mod prd;
mod seq;
mod sequence;
mod sgn;
mod util;
mod utl;
mod vq;

#[derive(StructOpt, Debug)]
#[structopt(global_setting(clap::AppSettings::ColoredHelp))]
#[clap(version, about = "ECOZ2 System", long_about = None)]
enum Ecoz {
    #[structopt(about = "Basic csv selection info")]
    CsvShow(csvutil::CsvShowOpts),

    #[structopt(about = "Signal operations")]
    Sgn(sgn::SgnMainOpts),

    #[structopt(about = "Linear prediction coding")]
    Lpc(lpc::LpcOpts),

    #[structopt(about = "Predictor file operations")]
    Prd(prd::PrdMainOpts),

    #[structopt(about = "VQ operations")]
    Vq(vq::VqMainOpts),

    #[structopt(about = "HMM operations")]
    Hmm(hmm::HmmMainOpts),

    #[structopt(about = "Sequence file operations")]
    Seq(seq::SeqMainOpts),

    #[structopt(about = "MM operations")]
    Mm(mm::MMMainOpts),

    #[structopt(about = "NBayes operations")]
    Nb(nb::NBayesMainOpts),

    #[structopt(about = "Utilities")]
    Util(util::UtilMainOpts),
}

fn main() {
    match Ecoz::parse() {
        Ecoz::CsvShow(opts) => {
            csvutil::main_csv_show(opts);
        }

        Ecoz::Sgn(opts) => {
            sgn::main(opts);
        }

        Ecoz::Lpc(opts) => {
            lpc::main(opts);
        }

        Ecoz::Prd(opts) => {
            prd::main(opts);
        }

        Ecoz::Vq(opts) => {
            vq::main(opts);
        }

        Ecoz::Hmm(opts) => {
            hmm::main(opts);
        }

        Ecoz::Seq(opts) => {
            seq::main(opts);
        }

        Ecoz::Mm(opts) => {
            mm::main(opts);
        }

        Ecoz::Nb(opts) => {
            nb::main(opts);
        }

        Ecoz::Util(opts) => {
            util::main(opts);
        }
    }
}
