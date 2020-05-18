use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;

use sequence;

/// A trained NBayes model.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct NBayes {
    pub class_name: String,
    pub total_symbols: usize,
    pub frequencies: Vec<usize>,
}

impl NBayes {
    pub fn save(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        let f = File::create(filename)?;
        serde_cbor::to_writer(f, &self)?;
        Ok(())
    }

    pub fn show(&mut self) {
        let codebook_size = self.frequencies.len();
        println!(
            "# class_name='{}', M={} total_symbols={}",
            self.class_name, codebook_size, self.total_symbols,
        );

        println!("{:4}, {:4}, {}", "m", "frequency", "prob");

        for (s, f) in self.frequencies.iter().enumerate() {
            let prob = self.prob_symbol(s);
            println!("{:4}, {:4}, {:.7}", s, f, prob);
        }
    }

    /// probability of generating the symbol, using an m-estimate
    pub fn prob_symbol(&self, symbol: usize) -> f64 {
        let codebook_size = self.frequencies.len();
        let f = self.frequencies[symbol] as f64;
        (f + 1.0) / (self.total_symbols + codebook_size) as f64
    }

    /// log probability of generating the symbol
    pub fn log_prob_symbol(&self, symbol: usize) -> f64 {
        self.prob_symbol(symbol).log10()
    }

    /// log probability of generating the symbol sequence
    pub fn log_prob_sequence(&self, seq: &sequence::Sequence) -> f64 {
        seq.symbols.iter().fold(0_f64, |acc, s| {
            acc + self.log_prob_symbol(*s as usize) as f64
        })
    }
}

pub fn load(filename: &str) -> Result<NBayes, Box<dyn Error>> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);
    let nbayes = serde_cbor::from_reader(br)?;
    Ok(nbayes)
}

pub fn learn(seq_filenames: Vec<PathBuf>) -> Result<NBayes, Box<dyn Error>> {
    // gotten from first given sequence:
    let mut class_name: Option<String> = None;

    let mut total_symbols: usize = 0;
    let mut frequencies = vec![];

    for seq_filename in seq_filenames {
        let filename = seq_filename.to_str().unwrap();
        let seq = sequence::load(filename)?;
        println!(" {}: '{:?}'", filename, seq.class_name);
        match &class_name {
            Some(cn) => {
                if *cn != seq.class_name {
                    return Err(format!(
                        "conformity error: Not same class: {} != {}",
                        *cn, seq.class_name
                    )
                    .into());
                }
                if frequencies.len() != seq.codebook_size as usize {
                    return Err(format!(
                        "conformity error: Not same codebook size: {} != {}",
                        frequencies.len(),
                        seq.codebook_size
                    )
                    .into());
                }
            }

            None => {
                class_name = Some(seq.class_name);
                frequencies = vec![0_usize; seq.codebook_size as usize];
            }
        }

        total_symbols += seq.symbols.len();
        for s in seq.symbols {
            frequencies[s as usize] += 1;
        }
    }

    Ok(NBayes {
        class_name: class_name.unwrap(),
        total_symbols,
        frequencies,
    })
}

pub fn classify(
    nb_filenames: Vec<PathBuf>,
    seq_filenames: Vec<PathBuf>,
    show_ranked: bool,
) -> Result<(), Box<dyn Error>> {
    println!("Loading NBayes models");
    let models: Vec<NBayes> = nb_filenames
        .iter()
        .map(|n| load(n.to_str().unwrap()).unwrap())
        .collect();

    let num_models = models.len();

    let mut result = vec![vec![0i32; num_models + 1]; num_models + 1];
    let mut confusion = vec![vec![0i32; num_models + 1]; num_models + 1];

    println!("Classifying sequences");
    for filename in seq_filenames {
        let seq = sequence::load(filename.to_str().unwrap())?;
        //        println!(
        //            " {:?}: '{}' ({})",
        //            filename,
        //            seq.class_name,
        //            seq.symbols.len()
        //        );

        let class_id_opt = &models.iter().position(|m| m.class_name == seq.class_name);
        if let Some(class_id) = *class_id_opt {
            result[num_models][0] += 1_i32;
            result[class_id][0] += 1_i32;

            // get probabilities (sorted):
            let probs: Vec<f64> = models.iter().map(|m| m.log_prob_sequence(&seq)).collect();
            let mut probs: Vec<(usize, &f64)> = probs.iter().enumerate().collect();

            probs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            let correct = class_id == probs[num_models - 1].0;
            print!("{}", if correct { "*" } else { "_" });
            std::io::stdout().flush()?;

            //TODO if show_ranked && !correct {}

            confusion[class_id][probs[num_models - 1].0] += 1_i32;

            // did best candidate correctly classify the instance?
            if probs[num_models - 1].0 == class_id {
                result[num_models][1] += 1_i32;
                result[class_id][1] += 1_i32;
            } else {
                // update order of recognized candidate:
                for i in 1..num_models {
                    if probs[num_models - 1 - i].0 == class_id {
                        result[num_models][i + 1] += 1_i32;
                        result[class_id][i + 1] += 1_i32;
                        break;
                    }
                }
            }
        }
    }

    println!();
    //    println!("result = {:?}\n", result);
    //    println!("confusion = {:?}\n", confusion);
    report_results(models, result, confusion);

    Ok(())
}

#[derive(serde::Serialize)]
struct Summary {
    accuracy: f32,
    avg_accuracy: f32,
}

fn report_results(models: Vec<NBayes>, result: Vec<Vec<i32>>, confusion: Vec<Vec<i32>>) {
    let num_models = models.len();

    if result[num_models][0] == 0 {
        return;
    }

    let mut margin = 0;
    for i in 0..num_models {
        if result[i][0] > 0 {
            let len = models[i].class_name.len();
            if margin < len {
                margin = len;
            }
        }
    }
    margin += 2;

    println!("\n");
    print!("{:margin$} ", "", margin = margin);
    println!("Confusion matrix:");

    print!("{:margin$} ", "", margin = margin);

    print!("     ");
    for j in 0..num_models {
        if result[j][0] > 0 {
            print!("{:>3} ", j);
        }
    }
    println!("    tests   errors");

    for i in 0..num_models {
        if result[i][0] == 0 {
            continue;
        }
        println!();
        print!("{:margin$} ", models[i].class_name, margin = margin);
        print!("{:>3}  ", i);

        let mut num_errs = 0; // in row
        for j in 0..num_models {
            if result[j][0] > 0 {
                print!("{:>3} ", confusion[i][j]);
                if i != j {
                    num_errs += confusion[i][j];
                }
            }
        }
        print!("{:>8}{:>8}", result[i][0], num_errs);
    }

    println!("\n");
    print!("{:margin$} ", "", margin = margin);
    println!("class     accuracy    tests      candidate order");

    let mut num_classes = 0;

    let mut summary = Summary {
        accuracy: 0_f32,
        avg_accuracy: 0_f32,
    };

    for class_id in 0..=num_models {
        if result[class_id][0] == 0 {
            continue;
        }

        let num_tests = result[class_id][0];
        let correct_tests = result[class_id][1];

        let acc = correct_tests as f32 / num_tests as f32;

        if class_id < num_models {
            num_classes += 1;
            summary.avg_accuracy += acc;

            print!("{:margin$} ", models[class_id].class_name, margin = margin);
            print!("  {:3}    ", class_id);
        } else {
            println!();
            print!("{:margin$} ", "", margin = margin);
            print!("  TOTAL  ");
            summary.accuracy = acc;
        }

        print!("  {:6.2}%   {:3}        ", 100_f32 * acc, num_tests);

        for i in 1..=num_models {
            print!("{:3} ", result[class_id][i]);
        }
        println!();
    }

    summary.accuracy *= 100_f32;
    summary.avg_accuracy = summary.avg_accuracy * 100_f32 / num_classes as f32;

    println!("  avg_accuracy  {}%", summary.avg_accuracy);
    println!("    error_rate  {}%", 100_f32 - summary.avg_accuracy);
    println!();

    report_summary(summary);
}

fn report_summary(summary: Summary) {
    let f = File::create("nb_classification.json").unwrap();
    serde_json::to_writer(f, &summary).unwrap();
}
