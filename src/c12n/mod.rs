use crate::utl;
use colored::*;
use std::io::Write;

// note: just a quick direct translation of my C code from the early 90s ;)

/// Classification results
pub struct C12nResults {
    model_class_names: Vec<String>,

    result: Vec<Vec<i32>>,
    confusion: Vec<Vec<i32>>,
    // TODO eventually remove some to the above
    y_true: Vec<String>,
    y_pred: Vec<String>,
}

impl C12nResults {
    pub fn new(model_class_names: Vec<String>) -> C12nResults {
        let num_models = model_class_names.len();
        let result = vec![vec![0i32; num_models + 1]; num_models + 1];
        let confusion = vec![vec![0i32; num_models + 1]; num_models + 1];

        let y_true = Vec::new();
        let y_pred = Vec::new();

        C12nResults {
            model_class_names,
            result,
            confusion,
            y_true,
            y_pred,
        }
    }

    pub fn add_case<F>(
        &mut self,
        class_id: usize,
        seq_classname: &str,
        probs: Vec<f64>,
        show_ranked: bool,
        f: F,
    ) where
        F: FnOnce() -> String,
    {
        let num_models = self.model_class_names.len();

        self.result[num_models][0] += 1_i32;
        self.result[class_id][0] += 1_i32;

        // sort given probabilities:
        let mut probs: Vec<(usize, &f64)> = probs.iter().enumerate().collect();
        probs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let predicted_id = probs[num_models - 1].0;
        let correct = class_id == predicted_id;
        print!("{}", if correct { "*".green() } else { "_".red() });
        std::io::stdout().flush().unwrap();

        self.y_true.push(seq_classname.to_string());

        let predicted_class_name = &self.model_class_names[predicted_id];
        self.y_pred.push(predicted_class_name.to_string());

        if show_ranked && !correct {
            let header_line = f();
            println!("{}", header_line);

            let mut index = 0;
            for r in (0..num_models).rev() {
                let model_id = probs[r].0;
                let model_class_name = &self.model_class_names[r];

                let mark = if class_id == model_id { "*" } else { "" };

                println!(
                    "  [{:>2}] {:1} model: <{:>2}>  {:e}  : '{}'  r={}",
                    index, mark, model_id, probs[model_id].1, model_class_name, r
                );

                // only show until corresponding model:
                if class_id == model_id {
                    break;
                }
                index += 1;
            }
            println!();
        }

        self.confusion[class_id][probs[num_models - 1].0] += 1_i32;

        // did best candidate correctly classify the instance?
        if correct {
            self.result[num_models][1] += 1_i32;
            self.result[class_id][1] += 1_i32;
        } else {
            // update order of recognized candidate:
            for i in 1..num_models {
                if probs[num_models - 1 - i].0 == class_id {
                    self.result[num_models][i + 1] += 1_i32;
                    self.result[class_id][i + 1] += 1_i32;
                    break;
                }
            }
        }
    }

    pub fn report_results(&mut self, class_names: Vec<&String>, out_base_name: String) {
        let num_models = self.model_class_names.len();

        //    println!("result = {:?}\n", self.result);
        //    println!("confusion = {:?}\n", self.confusion);

        if self.result[num_models][0] == 0 {
            return;
        }

        let mut margin = 0;
        for i in 0..num_models {
            if self.result[i][0] > 0 {
                let len = class_names[i].len();
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
            if self.result[j][0] > 0 {
                print!("{:>3} ", j);
            }
        }
        println!("    tests   errors");

        for i in 0..num_models {
            if self.result[i][0] == 0 {
                continue;
            }
            println!();
            print!("{:margin$} ", class_names[i], margin = margin);
            print!("{:>3}  ", i);

            let mut num_errs = 0; // in row
            for j in 0..num_models {
                if self.result[j][0] > 0 {
                    print!("{:>3} ", self.confusion[i][j]);
                    if i != j {
                        num_errs += self.confusion[i][j];
                    }
                }
            }
            print!("{:>8}{:>8}", self.result[i][0], num_errs);
        }

        println!("\n");
        print!("{:margin$} ", "", margin = margin);
        println!("class     accuracy   tests       candidate order");

        let mut num_classes = 0;

        let mut summary = Summary {
            accuracy: 0_f32,
            avg_accuracy: 0_f32,
        };

        for class_id in 0..=num_models {
            if self.result[class_id][0] == 0 {
                continue;
            }

            let num_tests = self.result[class_id][0];
            let correct_tests = self.result[class_id][1];

            let acc = correct_tests as f32 / num_tests as f32;

            if class_id < num_models {
                num_classes += 1;
                summary.avg_accuracy += acc;

                print!("{:margin$} ", class_names[class_id], margin = margin);
                print!("  {:3}    ", class_id);
            } else {
                println!();
                print!("{:margin$} ", "", margin = margin);
                print!("  TOTAL  ");
                summary.accuracy = acc;
            }

            print!("  {:6.2}%    {:4}       ", 100_f32 * acc, num_tests);

            for i in 1..=num_models {
                print!("{:4} ", self.result[class_id][i]);
            }
            println!();
        }

        summary.accuracy *= 100_f32;
        summary.avg_accuracy = summary.avg_accuracy * 100_f32 / num_classes as f32;

        println!("  avg_accuracy  {:6.2}%", summary.avg_accuracy);
        //println!("    error_rate  {:6.2}%", 100_f32 - summary.avg_accuracy);
        println!();

        let out_summary = format!("{}_classification.json", &out_base_name);
        utl::save_json(&summary, &out_summary).unwrap();
        println!("{} saved", out_summary);

        let y_true = &self.y_true;
        let y_pred = &self.y_pred;
        let true_pred = TruePred {
            y_true: y_true.to_vec(),
            y_pred: y_pred.to_vec(),
        };
        let out_true_pred = format!("{}_y_true_pred.json", &out_base_name);
        utl::save_json(&true_pred, &out_true_pred).unwrap();
        println!("{} saved", out_true_pred);
    }
}

#[derive(serde::Serialize)]
struct Summary {
    accuracy: f32,
    avg_accuracy: f32,
}

// TODO instead, generate a c12n CSV as the HMM C code does.
#[derive(serde::Serialize)]
struct TruePred {
    y_true: Vec<String>,
    y_pred: Vec<String>,
}
