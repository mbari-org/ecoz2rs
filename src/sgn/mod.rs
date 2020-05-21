extern crate hound;
extern crate itertools;
extern crate structopt;

use std::error::Error;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use regex::Regex;
use structopt::StructOpt;

use crate::csvutil::{load_instance_info, InstanceInfo};

use self::itertools::Itertools;
use self::EcozSgnCommand::{Extract, Show};

#[derive(StructOpt, Debug)]
pub struct SgnMainOpts {
    #[structopt(subcommand)]
    cmd: EcozSgnCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "sgn", about = "Signal operations")]
enum EcozSgnCommand {
    #[structopt(about = "Basic signal info")]
    Show(SgnShowOpts),

    #[structopt(about = "Extract segments from audio file")]
    Extract(SgnExtractOpts),
}

#[derive(StructOpt, Debug)]
pub struct SgnShowOpts {
    /// File to read
    #[structopt(short, long, parse(from_os_str))]
    file: PathBuf,
}

#[derive(StructOpt, Debug)]
pub struct SgnExtractOpts {
    /// Source wave file
    #[structopt(short, long, parse(from_os_str))]
    wav: PathBuf,

    /// Segments file
    #[structopt(short, long, parse(from_os_str))]
    segments: PathBuf,

    /// Desired selection ranges. Each string of the form `start-end`
    /// indicating initial (inclusive) and final (exclusive) selection numbers
    /// as given in the segments file.
    #[structopt(long)]
    select: Vec<String>,

    /// Base directory for output wave files
    #[structopt(short, long)]
    out_dir: String,
}

pub fn main(opts: SgnMainOpts) {
    let res = match opts.cmd {
        Show(opts) => sgn_show(opts),

        Extract(opts) => SgnExtractor::new(opts).sgn_extract(),
    };

    if let Err(err) = res {
        println!("{}", err);
    }
}

pub fn sgn_show(opts: SgnShowOpts) -> Result<(), Box<dyn Error>> {
    let SgnShowOpts { file } = opts;

    let filename: &str = file.to_str().unwrap();

    let s = load(&filename);
    println!("Signal loaded: {}", filename);
    s.show();
    Ok(())
}

struct SgnExtractor {
    wav_simple_name: String,
    sgn: Sgn,

    sample_period: f32,

    sgn_filename: String,

    ranges: Vec<std::ops::Range<i32>>,

    out_dir: String,
}

lazy_static! {
    static ref RANGE_RE: Regex = Regex::new(r"(?x)(?P<start>\d+)-(?P<end>-?\d+)").unwrap();
}

impl SgnExtractor {
    fn new(opts: SgnExtractOpts) -> SgnExtractor {
        let SgnExtractOpts {
            wav,
            segments,
            select,
            out_dir,
        } = opts;

        let wav_filename: &str = wav.to_str().unwrap();

        println!("Loading {}", wav_filename);
        let sgn = load(&wav_filename);
        sgn.show();

        let wav_simple_name = Path::new(wav_filename)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .into();

        let duration = sgn.num_samples / sgn.sample_rate;
        let sample_period = 1.0 / sgn.sample_rate as f32;
        println!("duration: {}  sample_period: {}", duration, sample_period);

        let sgn_filename = segments.to_str().unwrap().into();

        let ranges: Vec<std::ops::Range<i32>> = select
            .iter()
            .map(|s| {
                RANGE_RE.captures(s).map(|caps| {
                    let start: i32 = caps["start"].parse().unwrap();
                    let end: i32 = caps["end"].parse().unwrap();
                    start..end
                })
            })
            .flatten()
            .collect();

        println!("parsed ranges = {:?}", ranges);

        SgnExtractor {
            wav_simple_name,
            sgn,
            sample_period,
            sgn_filename,
            ranges,
            out_dir,
        }
    }

    fn in_ranges(&mut self, selection: i32) -> bool {
        if self.ranges.is_empty() {
            true
        } else {
            self.ranges.iter().any(|r| r.contains(&selection))
        }
    }

    pub fn sgn_extract(&mut self) -> Result<(), Box<dyn Error>> {
        let instances = load_instance_info(self.sgn_filename.as_str())?;

        let lookup = &instances
            .iter()
            .map(|instance| (instance.type_.to_string(), instance))
            .into_group_map();

        let mut tot_instances = 0;
        for (type_, instances) in lookup {
            let mut type_instances = 0;
            for i in instances {
                if self.in_ranges(i.selection) {
                    self.extract_instance(i)?;
                    type_instances += 1;
                    tot_instances += 1;
                }
            }
            if type_instances > 0 {
                println!("{0: >8}  {1: >3} instances", type_, type_instances);
            }
        }
        println!(
            "{0: >8}  {1: >3} total extracted instances",
            "", tot_instances
        );
        //    println!("Bmh = {:?}", lookup["Bmh"][0]);

        Ok(())
    }

    fn extract_instance(&mut self, i: &InstanceInfo) -> Result<(), Box<dyn Error>> {
        let out_dir: PathBuf = [&self.out_dir, &i.type_].iter().collect();
        fs::create_dir_all(&out_dir)?;

        let out_name = format!(
            "{}/from_{}__s{:04}__{}_{}.wav",
            &out_dir.to_str().unwrap(),
            self.wav_simple_name,
            i.selection,
            i.begin_time,
            i.end_time
        );

        //println!("\t\t extract_instance {} => {}", i.selection, out_name);

        let pos_beg = self.position(i.begin_time);
        let pos_end = self.position(i.end_time);

        /*
                println!("\t\tbegin_time={} end_time={}", i.begin_time, i.end_time);
                println!("\t\tpos_beg={} pos_end={}", pos_beg, pos_end);
        */

        let samples = self.sgn.samples[pos_beg..pos_end].to_vec();

        let spec = self.sgn.spec;
        let sample_rate = spec.sample_rate as usize;
        let num_samples = samples.len();

        let segment = Sgn {
            sample_rate,
            num_samples,
            samples,
            spec,
        };

        let _dur_secs = segment.save(out_name.as_str());
        //println!("saved {}  Duration: {:.3} secs", filename, dur_secs);

        Ok(())
    }

    fn position(&mut self, time_secs: f32) -> usize {
        (time_secs / self.sample_period as f32) as usize
    }
}

pub struct Sgn {
    pub sample_rate: usize,
    pub num_samples: usize,
    pub samples: Vec<f64>,

    spec: hound::WavSpec,
}

impl Sgn {
    /// returns duration in seconds
    pub fn save(&self, filename: &str) -> f32 {
        let spec = self.spec;
        let mut writer = hound::WavWriter::create(filename, spec).unwrap();

        for sample in &self.samples {
            writer.write_sample(*sample as i16).unwrap();
        }
        let dur_secs = writer.duration() as f32 / spec.sample_rate as f32;
        writer.finalize().unwrap();
        dur_secs
    }

    pub fn show(&self) {
        println!(
            "num_samples: {}  sample_rate: {}  bits_per_sample: {}  sample_format = {:?}",
            self.num_samples, self.sample_rate, self.spec.bits_per_sample, self.spec.sample_format
        );
    }
}

pub fn load(filename: &str) -> Sgn {
    let mut reader = hound::WavReader::open(&filename).unwrap();
    let i32s: Vec<i32> = reader.samples().map(|s| s.unwrap()).collect();
    let num_samples = i32s.len();

    // convert samples to f64:
    let mut samples = vec![0f64; num_samples];
    for (dst, src) in samples.iter_mut().zip(i32s.as_slice()) {
        *dst = f64::from(*src);
    }

    let spec = reader.spec();
    let sample_rate = spec.sample_rate as usize;

    Sgn {
        sample_rate,
        num_samples,
        samples,
        spec,
    }
}
