extern crate csvutil;
extern crate lpc;
extern crate prd;
extern crate sgn;
extern crate structopt;
extern crate vq;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "ecoz2", about = "ECOZ System")]
enum Ecoz {
    #[structopt(about = "Basic csv selection info")]
    CsvShow(csvutil::CsvShowOpts),

    #[structopt(about = "Signal operations")]
    Sgn(sgn::SgnMainOpts),

    #[structopt(about = "Linear prediction coding")]
    Lpc(lpc::LpcOpts),

    #[structopt(about = "Show predictor")]
    PrdShow(prd::PrdShowOpts),

    #[structopt(about = "VQ operations")]
    Vq(vq::VqMainOpts),
}

fn main() {
    match Ecoz::from_args() {
        Ecoz::CsvShow(opts) => {
            csvutil::main_csv_show(opts);
        }

        Ecoz::Sgn(opts) => {
            sgn::main(opts);
        }

        Ecoz::Lpc(opts) => {
            lpc::main_lpc(opts);
        }

        Ecoz::PrdShow(opts) => {
            prd::main_prd_show(opts);
        }

        Ecoz::Vq(opts) => {
            vq::main(opts);
        }
    }
}
