extern crate clap;
extern crate hound;
extern crate itertools;

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use clap::StructOpt;
use regex::Regex;

use crate::csvutil::{load_instance_info, InstanceInfo};

use self::hound::WavSpec;
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
    selection_ranges: Vec<String>,

    /// Desired time ranges. Each string of the form `start-end`
    /// indicating initial (inclusive) and final (exclusive) times in seconds.
    /// Only segments fully contained in a given range are extracted.
    #[structopt(long)]
    time_ranges: Vec<String>,

    /// Only extract a class if it has at least this number of instances
    #[structopt(short = 'm', long, default_value = "0")]
    minpc: usize,

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

fn print_sgn_info(num_samples: usize, spec: &WavSpec) {
    println!(
        "\n\
num_samples      : {}\n\
sample_rate      : {}\n\
bits_per_sample  : {}\n\
channels         : {}\n\
sample_format    : {:?}\n",
        num_samples, spec.sample_rate, spec.bits_per_sample, spec.channels, spec.sample_format,
    );
}

pub fn sgn_show(opts: SgnShowOpts) -> Result<(), Box<dyn Error>> {
    let SgnShowOpts { file } = opts;

    let filename: &str = file.to_str().unwrap();

    let reader = hound::WavReader::open(filename)?;
    let spec = reader.spec();
    print_sgn_info(reader.len() as usize, &spec);
    Ok(())
}

struct SgnExtractor {
    sgn: Sgn,

    sample_period: f32,

    sgn_filename: String,

    selection_ranges: Vec<std::ops::Range<i32>>,
    time_ranges: Vec<(f32, f32)>,

    minpc: usize,

    out_dir: String,
}

/// True if the instance passes the range filters given on the command line.
///
/// An empty filter list means "no restriction"; when both filters are given,
/// both must hold.  A time range selects the segments *fully contained* in it,
/// as documented for `--time-ranges`.
fn instance_in_ranges(
    selection_ranges: &[std::ops::Range<i32>],
    time_ranges: &[(f32, f32)],
    i: &InstanceInfo,
) -> bool {
    let in_selection =
        selection_ranges.is_empty() || selection_ranges.iter().any(|r| r.contains(&i.selection));

    let in_time = time_ranges.is_empty()
        || time_ranges
            .iter()
            .any(|r| r.0 <= i.begin_time && i.end_time <= r.1);

    in_selection && in_time
}

impl SgnExtractor {
    fn new(opts: SgnExtractOpts) -> SgnExtractor {
        let SgnExtractOpts {
            wav,
            segments,
            selection_ranges,
            time_ranges,
            minpc,
            out_dir,
        } = opts;

        let wav_filename: &str = wav.to_str().unwrap();

        println!("SgnExtractor: Loading {}", wav_filename);
        let sgn = load(wav_filename);
        sgn.show();

        let duration = sgn.num_samples / sgn.sample_rate;
        let sample_period = 1.0 / sgn.sample_rate as f32;
        println!("duration: {}  sample_period: {}", duration, sample_period);

        let sgn_filename = segments.to_str().unwrap().into();

        let sel_range_re: Regex = Regex::new(r"(?x)(?P<start>\d+)-(?P<end>-?\d+)").unwrap();
        let selection_ranges: Vec<std::ops::Range<i32>> = selection_ranges
            .iter()
            .filter_map(|s| {
                sel_range_re.captures(s).map(|caps| {
                    let start: i32 = caps["start"].parse().unwrap();
                    let end: i32 = caps["end"].parse().unwrap();
                    start..end
                })
            })
            .collect();
        println!("parsed selection_ranges = {:?}", selection_ranges);

        let time_range_re: Regex =
            Regex::new(r"(?x)(?P<start>(\d|\.)+)-(?P<end>(\d|\.)+)").unwrap();
        let time_ranges: Vec<(f32, f32)> = time_ranges
            .iter()
            .filter_map(|s| {
                time_range_re.captures(s).map(|caps| {
                    let start: f32 = caps["start"].parse().unwrap();
                    let end: f32 = caps["end"].parse().unwrap();
                    if start > end {
                        panic!("invalid time range: start={} > end={}", start, end);
                    }
                    (start, end)
                })
            })
            .collect();
        println!("parsed time_ranges = {:?}", time_ranges);

        SgnExtractor {
            sgn,
            sample_period,
            sgn_filename,
            selection_ranges,
            time_ranges,
            minpc,
            out_dir,
        }
    }

    fn in_ranges(&self, i: &InstanceInfo) -> bool {
        instance_in_ranges(&self.selection_ranges, &self.time_ranges, i)
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
            if self.minpc > 0 && instances.len() < self.minpc {
                continue;
            }
            for i in instances {
                if self.in_ranges(i) {
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

        let out_name = format!("{}/{:05}.wav", out_dir.to_str().unwrap(), i.selection,);

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
        (time_secs / self.sample_period) as usize
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
        print_sgn_info(self.num_samples, &self.spec);
    }
}

pub fn load(filename: &str) -> Sgn {
    let mut reader = hound::WavReader::open(filename).unwrap();
    let samples: Vec<i32> = reader.samples().map(|s| s.unwrap()).collect();
    let num_samples = samples.len();

    // convert samples to f64:
    let samples = samples.iter().map(|s| *s as f64).collect();

    let spec = reader.spec();
    let sample_rate = spec.sample_rate as usize;

    Sgn {
        sample_rate,
        num_samples,
        samples,
        spec,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst(selection: i32, begin_time: f32, end_time: f32) -> InstanceInfo {
        InstanceInfo {
            selection,
            begin_time,
            end_time,
            type_: "A".to_string(),
        }
    }

    #[test]
    fn no_ranges_accepts_everything() {
        assert!(instance_in_ranges(&[], &[], &inst(1, 263.5, 265.0)));
    }

    /// A time range selects segments fully inside it.  A segment that starts
    /// before the range must be excluded, not included.
    #[test]
    fn time_range_selects_fully_contained_segments() {
        let ranges = [(300.0f32, 1800.0f32)];
        assert!(instance_in_ranges(&[], &ranges, &inst(1, 400.0, 500.0)));
        assert!(instance_in_ranges(&[], &ranges, &inst(2, 300.0, 1800.0))); // inclusive
        assert!(!instance_in_ranges(&[], &ranges, &inst(3, 263.0, 265.0))); // starts before
        assert!(!instance_in_ranges(&[], &ranges, &inst(4, 1700.0, 1900.0))); // ends after
        assert!(!instance_in_ranges(&[], &ranges, &inst(5, 200.0, 2000.0))); // spans it
    }

    #[test]
    fn selection_range_is_half_open() {
        let ranges = [10..20];
        assert!(instance_in_ranges(&ranges, &[], &inst(10, 0.0, 1.0)));
        assert!(instance_in_ranges(&ranges, &[], &inst(19, 0.0, 1.0)));
        assert!(!instance_in_ranges(&ranges, &[], &inst(20, 0.0, 1.0)));
    }

    /// Both filters given: an instance must satisfy both, rather than the
    /// selection verdict being discarded.
    #[test]
    fn both_filters_must_hold() {
        let sel = [10..20];
        let time = [(300.0f32, 1800.0f32)];
        assert!(instance_in_ranges(&sel, &time, &inst(15, 400.0, 500.0)));
        assert!(!instance_in_ranges(&sel, &time, &inst(99, 400.0, 500.0)));
        assert!(!instance_in_ranges(&sel, &time, &inst(15, 100.0, 200.0)));
    }
}
