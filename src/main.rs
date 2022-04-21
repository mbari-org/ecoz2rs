#[macro_use]
extern crate assert_approx_eq;
extern crate byteorder;
extern crate colored;
extern crate itertools;
extern crate ndarray;
extern crate num_cpus;
extern crate openmp_sys;
extern crate serde;
extern crate serde_json;
extern crate structopt;

#[macro_use]
extern crate lazy_static;

use structopt::clap::{self, crate_version};
use structopt::StructOpt;

mod c12n;
mod comet_client;
mod csvutil;
mod ecoz2_lib;
mod hmm;
mod jld2;
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
#[structopt(name = "ecoz2", about = "ECOZ2 System")]
#[structopt(version = crate_version!())]
enum Ecoz {
    #[structopt(about = "Show version of C code")]
    Cversion,

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

    #[structopt(about = "Basic display of jld2 file")]
    Jld2Show(jld2::Jld2ShowOpts),
}

fn main() {
    match Ecoz::from_args() {
        Ecoz::Cversion => {
            println!("ecoz2/C {}", ecoz2_lib::version().unwrap());
        }

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

        Ecoz::Jld2Show(opts) => {
            jld2::main_jld2_show(opts);
        }
    }
}
