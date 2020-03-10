extern crate structopt;
use structopt::StructOpt;

extern crate csvutil;
extern crate lpc;
extern crate prd;
extern crate sgn;
extern crate vq;

#[derive(StructOpt, Debug)]
#[structopt(name = "ecoz2", about = "ECOZ System")]
enum Ecoz {
    #[structopt(about = "Basic csv selection info")]
    CsvShow(csvutil::CsvShowOpts),

    #[structopt(about = "Basic info about signal file")]
    SgnShow(sgn::SgnShowOpts),

    #[structopt(about = "Linear prediction coding")]
    Lpc(lpc::LpcOpts),

    #[structopt(about = "Show predictor")]
    PrdShow(prd::PrdShowOpts),

    #[structopt(about = "Codebook training")]
    VqLearn(vq::VqLearnOpts),
}

fn main() {
    match Ecoz::from_args() {
        Ecoz::CsvShow(opts) => {
            csvutil::main_csv_show(opts);
        }

        Ecoz::SgnShow(opts) => {
            sgn::main_sgn_show(opts);
        }

        Ecoz::Lpc(opts) => {
            lpc::main_lpc(opts);
        }

        Ecoz::PrdShow(opts) => {
            prd::main_prd_show(opts);
        }

        Ecoz::VqLearn(opts) => {
            vq::main_vq_learn(opts);
        }
    }
}
