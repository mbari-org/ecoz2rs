extern crate structopt;
use std::path::PathBuf;
use structopt::StructOpt;
extern crate hound;

#[derive(StructOpt, Debug)]
pub struct SgnShowOpts {
    /// File to read
    #[structopt(short, long, parse(from_os_str))]
    file: PathBuf,
}

pub fn main_sgn_show(opts: SgnShowOpts) {
    let SgnShowOpts { file } = opts;

    let filename: &str = file.to_str().unwrap();

    let mut s = load(&filename);
    println!("Signal loaded: {}", filename);
    s.show();
}

pub struct Sgn {
    pub sample_rate: usize,
    pub num_samples: usize,
    pub samples: Vec<i32>,

    spec: hound::WavSpec,
}

impl Sgn {
    pub fn save(&mut self, filename: &str) {
        println!("save: filename = {}", filename);

        let spec = self.spec;
        let mut writer = hound::WavWriter::create(filename, spec).unwrap();

        for sample in &self.samples {
            writer.write_sample(*sample as i16).unwrap();
        }
        println!("Duration: {} secs", writer.duration() / spec.sample_rate);
        writer.finalize().unwrap();
    }

    pub fn show(&mut self) {
        println!(
            "num_samples: {}  sample_rate: {}  bits_per_sample: {}  sample_format = {:?}",
            self.num_samples, self.sample_rate, self.spec.bits_per_sample, self.spec.sample_format
        );
    }
}

pub fn load(filename: &str) -> Sgn {
    let mut reader = hound::WavReader::open(&filename).unwrap();
    let samples: Vec<i32> = reader.samples().map(|s| s.unwrap()).collect();

    let spec = reader.spec();
    let sample_rate = spec.sample_rate as usize;
    let num_samples = samples.len();

    Sgn {
        sample_rate,
        num_samples,
        samples,
        spec,
    }
}
