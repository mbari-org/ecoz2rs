extern crate clap;

use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use clap::StructOpt;
use colored::*;

use crate::c12n;
use crate::sequence;
use crate::utl;

mod hmm_learn_rs;
mod hmm_rs;

use self::EcozHmmCommand::{Classify, Learn, Show};

#[derive(StructOpt, Debug)]
pub struct HmmMainOpts {
    #[structopt(subcommand)]
    cmd: EcozHmmCommand,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "hmm", about = "HMM operations")]
enum EcozHmmCommand {
    #[structopt(about = "HMM training")]
    Learn(HmmLearnOpts),

    #[structopt(about = "HMM based classification")]
    Classify(HmmClassifyOpts),

    #[structopt(about = "Show HMM model")]
    Show(HmmShowOpts),
}

#[derive(StructOpt, Debug)]
pub struct HmmLearnOpts {
    /// Number of states
    #[structopt(short = 'N', long, name = "N", default_value = "5")]
    num_states: usize,

    /// Number of symbols (codebook size)
    #[structopt(short = 'M', long, name = "M", required = true)]
    codebook_size: usize,

    /// Type of model to generate:
    ///    0: random values for pi, A, and B
    ///    1: uniform distributions
    ///    2: cascade-2; random B
    ///    3: cascade-3; random B
    #[structopt(short = 't', default_value = "3")]
    type_: usize,

    /// Maximum number of iterations. Default (-1) means no limit.
    #[structopt(short = 'I', long, name = "I", default_value = "-1")]
    max_iterations: i32,

    /// epsilon restriction on B.
    /// 0 means do not apply this restriction
    #[structopt(short = 'e', default_value = "1e-05")]
    epsilon: f64,

    /// val_auto.
    #[structopt(short = 'a', default_value = "0.3")]
    val_auto: f64,

    /// Seed for random numbers. Negative means random seed.
    /// Otherwise, the given seed is used, which will allow for reproducibility.
    #[structopt(short = 's', long, default_value = "-1")]
    seed: i64,

    /// Training sequences.
    /// If a single `.csv` file is given, then the "TRAIN" files indicated there will be used,
    /// and only the ones corresponding to a class name if `--class-name` is given.
    /// Otherwise, if directories are included, then all `.seq` under them will be used.
    #[structopt(long, parse(from_os_str), name = "files")]
    sequences: Vec<PathBuf>,

    /// If training sequences are given via a `.csv` file,
    /// only select the ones with this class name.
    #[structopt(long, name = "class")]
    class_name: Option<String>,
}

#[derive(StructOpt, Debug)]
pub struct HmmClassifyOpts {
    /// Show ranked models for incorrect classifications
    #[structopt(short = 'r', long)]
    show_ranked: bool,

    /// File to report classification results for each sequence.
    #[structopt(short, long = "c12n", parse(from_os_str))]
    classification_filename: Option<PathBuf>,

    /// HMM models.
    /// If directories are included, then all `.hmm` under them will be used.
    #[structopt(short, long, required = true, min_values = 1, parse(from_os_str))]
    models: Vec<PathBuf>,

    /// TRAIN or TEST
    #[structopt(long, required = true)]
    tt: String,

    /// If sequences or predictors are given via a `.csv` file,
    /// only select the ones with this class name.
    #[structopt(long, name = "class")]
    class_name: Option<String>,

    /// Sequences to classify.
    /// If directories are included, then all `.seq` under them will be used.
    #[structopt(
        short,
        long,
        required_unless("predictors"),
        min_values = 1,
        parse(from_os_str)
    )]
    sequences: Vec<PathBuf>,

    /// Number of symbols (codebook size) when `--sequences` with
    /// a `.csv` file is given. Helps determine the path to the sequences.
    #[structopt(short = 'M', long, required = true)]
    codebook_size: usize,

    /// Predictor files to classify.
    /// If a single `.csv` file is given, then only the ones indicated with `--tt` will be used.
    /// Otherwise, if directories are included, then all `.prd` under them will be used.
    #[structopt(long, min_values = 1, parse(from_os_str))]
    predictors: Vec<PathBuf>,

    #[structopt(long, default_value = "data/predictors")]
    predictors_dir_template: String,

    /// Codebook models when `--predictors` is given.
    /// If directories are included, then all `.cb` under them will be used.
    #[structopt(long, required_unless("sequences"), min_values = 1, parse(from_os_str))]
    codebooks: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
pub struct HmmShowOpts {
    /// HMM model.
    ///
    /// No short form: deriving one from the field name gives `-h`, which clap
    /// rejects as conflicting with help, and made `hmm show` panic on every
    /// invocation.
    #[structopt(long, parse(from_os_str))]
    hmm: PathBuf,

    /// Number format, a printf-like `%[-][0][width][.prec][L]{g,f,e}`.
    #[structopt(short, long, default_value = "%Lg ")]
    format: String,
}

pub fn main(opts: HmmMainOpts) {
    let res = match opts.cmd {
        Learn(opts) => main_hmm_learn(opts),

        Classify(opts) => main_hmm_classify(opts),

        Show(opts) => main_hmm_show(opts),
    };

    if let Err(err) = res {
        println!("{}", err.to_string().red());
    }
}

pub fn main_hmm_learn(opts: HmmLearnOpts) -> Result<(), Box<dyn Error>> {
    let HmmLearnOpts {
        num_states,
        codebook_size,
        type_,
        max_iterations,
        epsilon,
        val_auto,
        seed,
        sequences,
        class_name,
    } = opts;

    let seq_filenames = utl::resolve_files(
        sequences,
        "TRAIN",
        class_name,
        format!("sequences/M{}", codebook_size),
        ".seq",
    )?;

    println!("sequences: {}", seq_filenames.len());
    println!("val_auto = {}", val_auto);

    hmm_learn_rs_driver(
        num_states,
        type_,
        &seq_filenames,
        codebook_size,
        epsilon,
        val_auto,
        max_iterations,
        seed,
    )
}

pub fn main_hmm_classify(opts: HmmClassifyOpts) -> Result<(), Box<dyn Error>> {
    let HmmClassifyOpts {
        show_ranked,
        classification_filename,
        models,
        tt,
        class_name,
        sequences,
        codebook_size,
        predictors,
        predictors_dir_template,
        codebooks,
    } = opts;

    assert_ne!(predictors.is_empty(), sequences.is_empty());

    let hmm_filenames = utl::resolve_filenames(models, ".hmm", "models")?;

    if !sequences.is_empty() {
        let seq_filenames = utl::resolve_files(
            sequences,
            tt.as_str(),
            class_name,
            format!("sequences/M{}", codebook_size),
            ".seq",
        )?;

        println!(
            "number of HMM models: {}  number of sequences: {}",
            hmm_filenames.len(),
            seq_filenames.len()
        );
        println!("show_ranked = {}", show_ranked);

        return hmm_classify_sequences_rs(
            &hmm_filenames,
            &seq_filenames,
            show_ranked,
            classification_filename,
            codebook_size,
        );
    } else {
        let cb_filenames = utl::resolve_filenames(codebooks, ".cbook", "codebooks")?;

        let prd_filenames = utl::resolve_files3(
            &predictors,
            tt.as_str(),
            &class_name,
            "".to_string(),
            predictors_dir_template,
            ".prd",
        )?;

        hmm_classify_predictors_rs(
            &hmm_filenames,
            &cb_filenames,
            &prd_filenames,
            show_ranked,
            classification_filename,
        )?;
    }

    Ok(())
}

pub fn main_hmm_show(opts: HmmShowOpts) -> Result<(), Box<dyn Error>> {
    let HmmShowOpts { hmm, format } = opts;

    hmm_show_rs(&hmm, &format)?;

    Ok(())
}

/// The Rust implementation of `hmm show` (`hmm_show_model` in
/// `ecoz2/src/hmm/hmm.c`).
///
/// The C hands `format` straight to `printf`, which lets the caller inject an
/// arbitrary conversion; `utl::pf::render` parses the subset in use instead and
/// rejects the rest.
fn hmm_show_rs(filename: &Path, format: &str) -> Result<(), Box<dyn Error>> {
    use crate::utl::cfmt;
    use crate::utl::pf;

    let hmm = cfmt::load_hmm(filename)?;
    let n = hmm.num_states();
    let m = hmm.num_symbols();
    println!("className: '{}'  N={}  M={}", hmm.class_name, n, m);

    // Each row is printed with its own sum, and flagged if it is not 1.
    let row = |label: String, values: &[f64]| -> Result<(), Box<dyn Error>> {
        print!("{}", label);
        let mut sum = 0f64;
        for v in values {
            print!("{}", pf::render(format, *v)?);
            sum += *v;
        }
        print!(" Σ = {}", pf::render(format, sum)?);
        println!();
        if (sum - 1.0).abs() > 1e-10 {
            println!(
                "{}",
                format!(
                    "***ERROR** Σ = {:e} != 1
",
                    sum
                )
                .red()
            );
        }
        Ok(())
    };

    println!();
    row("pi = ".to_string(), &hmm.pi)?;
    println!();
    print!("A =  ");
    // As the C does: the label is printed once, and rows after the first start
    // at column 0.
    for a_row in &hmm.a {
        row(String::new(), a_row)?;
    }
    println!();
    print!("B =  ");
    // As the C does: the label is printed once, and rows after the first start
    // at column 0.
    for b_row in &hmm.b {
        row(String::new(), b_row)?;
    }
    Ok(())
}

/// The Rust implementation of `hmm_classify` for the direct-sequence case
/// (`ecoz2/src/hmm/hmm_classify.c`).
///
/// Each sequence is scored against every model by the scaled forward
/// recursion, and the models ranked by log-probability.
fn hmm_classify_sequences_rs(
    hmm_filenames: &[PathBuf],
    seq_filenames: &[PathBuf],
    show_ranked: bool,
    classification_filename: Option<PathBuf>,
    codebook_size: usize,
) -> Result<(), Box<dyn Error>> {
    use crate::utl::cfmt;

    println!("\nLoading models:");
    let mut models: Vec<hmm_rs::Hmm> = Vec::new();
    for (i, f) in hmm_filenames.iter().enumerate() {
        println!("{:2}: {}", i, f.display());
        let hmm = hmm_rs::Hmm::from_data(cfmt::load_hmm(f)?);
        if let Some(first) = models.first() {
            if hmm.m != first.m {
                return Err(format!(
                    "{}: conformity error: M={}, expected {}",
                    f.display(),
                    hmm.m,
                    first.m
                )
                .into());
            }
        }
        models.push(hmm);
    }
    let class_names: Vec<String> = models.iter().map(|m| m.class_name.clone()).collect();

    let mut c12n_file = C12nCsv::create(
        &classification_filename,
        models.len(),
        codebook_size,
        seq_filenames.len(),
    )?;

    let mut c12n = c12n::C12nResults::new(class_names.clone());
    let mut work = hmm_rs::Work::new();
    println!();

    for filename in seq_filenames {
        let name = filename.to_str().unwrap();
        let seq = sequence::load(name)?;
        let class_id = match class_names.iter().position(|n| *n == seq.class_name) {
            Some(i) => i,
            None => continue,
        };

        let probs: Vec<f64> = models
            .iter()
            .map(|m| {
                m.log_prob(&seq.symbols, &mut work)
                    .unwrap_or(f64::NEG_INFINITY)
            })
            .collect();

        c12n_file.add_case(name, &seq.class_name, class_id, &probs, &class_names)?;

        c12n.add_case(class_id, &seq.class_name, probs, show_ranked, || {
            format!("\n{}: '{}'\n", name, seq.class_name)
        });
    }
    println!();

    let names: Vec<&String> = class_names.iter().collect();
    c12n.report_results(names, format!("hmm_{}", codebook_size));
    Ok(())
}

/// The per-instance classification CSV, in the shape `c12n_prepare` and
/// `c12n_add_case` write it (`ecoz2/src/utl/c12n.c`).
struct C12nCsv(Option<std::io::BufWriter<std::fs::File>>);

impl C12nCsv {
    fn create(
        path: &Option<PathBuf>,
        num_models: usize,
        codebook_size: usize,
        num_instances: usize,
    ) -> Result<C12nCsv, Box<dyn Error>> {
        use std::io::Write;

        let path = match path {
            Some(p) => p,
            None => return Ok(C12nCsv(None)),
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
        writeln!(
            f,
            "# num_models={}  M={}  num_seqs={}",
            num_models, codebook_size, num_instances
        )?;
        write!(f, "seq_filename,seq_class_name,correct,rank")?;
        for r in 1..=num_models {
            write!(f, ",r{}", r)?;
        }
        writeln!(f)?;
        Ok(C12nCsv(Some(f)))
    }

    fn add_case(
        &mut self,
        filename: &str,
        class_name: &str,
        class_id: usize,
        probs: &[f64],
        class_names: &[String],
    ) -> Result<(), Box<dyn Error>> {
        use std::io::Write;

        let f = match self.0.as_mut() {
            Some(f) => f,
            None => return Ok(()),
        };
        let ranked = hmm_rs::rank_descending(probs);
        let correct = ranked[0] == class_id;
        // `rank` is where the correct model landed, 1-based.
        let rank = 1 + ranked.iter().position(|&id| id == class_id).unwrap_or(0);
        write!(
            f,
            "{},{},{},{}",
            filename,
            class_name,
            if correct { "*" } else { "!" },
            rank
        )?;
        for id in &ranked {
            write!(f, ",{}", class_names[*id])?;
        }
        writeln!(f)?;
        Ok(())
    }
}

/// The Rust implementation of `hmm_classify` for the predictor case
/// (`ecoz2/src/hmm/hmm_classify.c`, feeding off `seq_provider`).
///
/// Instead of reading `.seq` files, each `.prd` is quantized on the fly with
/// every model's own codebook and the resulting sequence scored against that
/// model.  Codebook `r` belongs to model `r` -- the C matches them positionally
/// and asserts the class names agree, which is checked here too.
fn hmm_classify_predictors_rs(
    hmm_filenames: &[PathBuf],
    cb_filenames: &[PathBuf],
    prd_filenames: &[PathBuf],
    show_ranked: bool,
    classification_filename: Option<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    use crate::utl::cfmt;
    use crate::vq::vq_rs;
    use std::collections::HashSet;

    if hmm_filenames.len() != cb_filenames.len() {
        return Err(format!(
            "number of codebooks ({}) does not match number of models ({})",
            cb_filenames.len(),
            hmm_filenames.len()
        )
        .into());
    }

    println!("\nLoading HMM models:");
    let mut models: Vec<hmm_rs::Hmm> = Vec::new();
    for (i, f) in hmm_filenames.iter().enumerate() {
        println!("{:2}: {}", i, f.display());
        let hmm = hmm_rs::Hmm::from_data(cfmt::load_hmm(f)?);
        if let Some(first) = models.first() {
            if hmm.m != first.m {
                return Err(format!(
                    "{}: conformity error: M={}, expected {}",
                    f.display(),
                    hmm.m,
                    first.m
                )
                .into());
            }
        }
        models.push(hmm);
    }
    let class_names: Vec<String> = models.iter().map(|m| m.class_name.clone()).collect();
    let codebook_size = models[0].m;

    println!("\nLoading codebooks:");
    let mut quantizers: Vec<vq_rs::Codebook> = Vec::new();
    let mut prediction_order: Option<usize> = None;
    for (i, f) in cb_filenames.iter().enumerate() {
        println!("{:2}: {}", i, f.display());
        let cb = cfmt::load_codebook(f)?;
        // The C asserts both of these; a mismatch means the codebooks were
        // given in a different order than the models.
        if cb.vectors.len() != models[i].m {
            return Err(format!(
                "{}: conformity error: {} entries, but model {} has M={}",
                f.display(),
                cb.vectors.len(),
                hmm_filenames[i].display(),
                models[i].m
            )
            .into());
        }
        if cb.class_name != class_names[i] {
            return Err(format!(
                "{}: class '{}' does not correspond to model {} of class '{}'",
                f.display(),
                cb.class_name,
                hmm_filenames[i].display(),
                class_names[i]
            )
            .into());
        }
        match prediction_order {
            None => prediction_order = Some(cb.prediction_order),
            Some(p) if p != cb.prediction_order => {
                return Err(format!(
                    "{}: prediction order {}, expected {}",
                    f.display(),
                    cb.prediction_order,
                    p
                )
                .into())
            }
            _ => {}
        }
        quantizers.push(vq_rs::Codebook::from_reflections(
            &cb.vectors,
            cb.prediction_order,
        ));
    }
    let prediction_order = prediction_order.unwrap();

    let mut c12n_file = C12nCsv::create(
        &classification_filename,
        models.len(),
        codebook_size,
        prd_filenames.len(),
    )?;

    let mut c12n = c12n::C12nResults::new(class_names.clone());
    let mut work = hmm_rs::Work::new();
    // As the C does, report an unmodelled class only the first time it is seen.
    let mut unmodelled: HashSet<String> = HashSet::new();
    println!();

    for filename in prd_filenames {
        let name = filename.to_str().unwrap();
        let prd = cfmt::load_predictor(filename)?;
        if prd.prediction_order != prediction_order {
            return Err(format!(
                "{}: prediction order {} does not match the codebooks' {}",
                filename.display(),
                prd.prediction_order,
                prediction_order
            )
            .into());
        }

        let class_id = match class_names.iter().position(|n| *n == prd.class_name) {
            Some(i) => i,
            None => {
                if unmodelled.insert(prd.class_name.clone()) {
                    eprintln!("\nNo model loaded for '{}'", prd.class_name);
                }
                continue;
            }
        };

        // Each model scores the sequence its own codebook produced.
        let probs: Vec<f64> = models
            .iter()
            .zip(&quantizers)
            .map(|(m, q)| {
                let (symbols, _) = vq_rs::quantize(q, &prd.vectors);
                m.log_prob(&symbols, &mut work).unwrap_or(f64::NEG_INFINITY)
            })
            .collect();

        c12n_file.add_case(name, &prd.class_name, class_id, &probs, &class_names)?;

        c12n.add_case(class_id, &prd.class_name, probs, show_ranked, || {
            format!("\n{}: '{}'\n", name, prd.class_name)
        });
    }
    println!();

    let names: Vec<&String> = class_names.iter().collect();
    c12n.report_results(names, format!("hmm_{}", codebook_size));
    Ok(())
}

/// The Rust implementation of `hmm learn` (`ecoz2/src/hmm/hmm_learn.c`).
///
/// Writes the model plus the `.rpt` and `.csv` the C emits, into the same
/// `data/hmms/N<N>__M<M>_t<type>__a<auto>[_I<iters>]/` directory.
#[allow(clippy::too_many_arguments)]
fn hmm_learn_rs_driver(
    num_states: usize,
    model_type: usize,
    seq_filenames: &[PathBuf],
    codebook_size: usize,
    epsilon: f64,
    val_auto: f64,
    max_iterations: i32,
    seed: i64,
) -> Result<(), Box<dyn Error>> {
    use crate::utl::pf;
    use rand::{rng, RngExt};
    use std::io::Write;

    if seq_filenames.is_empty() {
        return Err("no training sequences".into());
    }

    // As with `util split`: an unseeded run draws one and reports it, so the
    // result stays reproducible after the fact.
    let seed_used: u64 = if seed < 0 {
        rng().random()
    } else {
        seed as u64
    };
    println!("hmm_learn: seed={}", seed_used);

    let mut seqs: Vec<Vec<u16>> = Vec::with_capacity(seq_filenames.len());
    let mut model_class_name = String::new();
    let mut m = 0usize;
    for (r, f) in seq_filenames.iter().enumerate() {
        let seq = sequence::load(f.to_str().unwrap())?;
        if r == 0 {
            // the first sequence's class names the model, as in the C
            model_class_name = seq.class_name.clone();
            m = seq.codebook_size as usize;
        } else if seq.codebook_size as usize != m {
            eprintln!("{}: not conformant.", f.display());
            continue;
        } else if seq.class_name != model_class_name {
            println!(
                "WARNING: model '{}' trained with sequence '{}'",
                model_class_name, seq.class_name
            );
        }
        seqs.push(seq.symbols);
    }
    if m == 0 {
        return Err("M == 0".into());
    }
    if m != codebook_size {
        println!(
            "note: sequences carry M={}, --codebook-size said {}",
            m, codebook_size
        );
    }

    let dir = if max_iterations >= 0 {
        format!(
            "data/hmms/N{}__M{}_t{}__a{}_I{}",
            num_states,
            m,
            model_type,
            pf::g(val_auto),
            max_iterations
        )
    } else {
        format!(
            "data/hmms/N{}__M{}_t{}__a{}",
            num_states,
            m,
            model_type,
            pf::g(val_auto)
        )
    };
    std::fs::create_dir_all(&dir)?;
    let model_path = PathBuf::from(&dir).join(format!("{}.hmm", model_class_name));

    let outcome = hmm_learn_rs::learn(
        &model_class_name,
        num_states,
        model_type,
        &seqs,
        m,
        epsilon,
        val_auto,
        max_iterations,
        seed_used,
        &model_path,
    )?;

    // the per-iteration trace
    let csv_path = PathBuf::from(&dir).join(format!("{}.csv", model_class_name));
    let mut csv = std::io::BufWriter::new(std::fs::File::create(csv_path)?);
    writeln!(
        csv,
        "# N={} M={} type={}  #sequences = {}  max_T={}  (seed={})",
        num_states, m, model_type, outcome.num_seqs, outcome.max_t, seed_used
    )?;
    writeln!(csv, "I,Σ log(P)")?;
    for (i, slp) in &outcome.trace {
        writeln!(csv, "{},{}", i, pf::g(*slp))?;
    }

    // the summary, to stdout and to <model>.rpt
    let type_name = match model_type {
        0 => "no restriction",
        1 => "uniform distributions",
        2 => "cascade-2",
        _ => "cascade-3",
    };
    let restriction = if epsilon == 0.0 {
        "No".to_string()
    } else {
        pf::g(epsilon)
    };
    let summary = |prefix: &str| -> String {
        format!(
            "{p} Model: {}   (seed={})'\n\
             {p} className: '{}'\n\
             {p} N={} M={} type: {}\n\
             {p} restriction: {}\n\
             {p}     #sequences: {}\n\
             {p}     auto value: {}\n\
             {p}   #refinements: {}\n\
             {p}       Σ log(P): {}\n",
            model_path.display(),
            seed_used,
            model_class_name,
            num_states,
            m,
            type_name,
            restriction,
            outcome.num_seqs,
            pf::g(val_auto),
            outcome.num_refinements,
            pf::g(outcome.sum_log_prob),
            p = prefix
        )
    };
    println!("\n{}", summary("    "));
    let rpt_path = model_path.with_extension("rpt");
    std::fs::write(rpt_path, summary(""))?;

    println!("=> training complete     class={}\n", model_class_name);
    Ok(())
}
