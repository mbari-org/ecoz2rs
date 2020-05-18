use std::fs::File;
use std::io::Write;

/// Classification results
pub struct C12nResults {
    num_models: usize,
    result: Vec<Vec<i32>>,
    confusion: Vec<Vec<i32>>,
}

impl C12nResults {
    pub fn new(num_models: usize) -> C12nResults {
        let result = vec![vec![0i32; num_models + 1]; num_models + 1];
        let confusion = vec![vec![0i32; num_models + 1]; num_models + 1];

        C12nResults {
            num_models,
            result,
            confusion,
        }
    }

    pub fn add_case(&mut self, class_id: usize, probs: Vec<f64>) {
        let num_models = self.num_models as usize;

        self.result[num_models][0] += 1_i32;
        self.result[class_id][0] += 1_i32;

        // sort given probabilities:
        let mut probs: Vec<(usize, &f64)> = probs.iter().enumerate().collect();
        probs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let correct = class_id == probs[num_models - 1].0;
        print!("{}", if correct { "*" } else { "_" });
        std::io::stdout().flush().unwrap();

        //TODO if show_ranked && !correct {}

        self.confusion[class_id][probs[num_models - 1].0] += 1_i32;

        // did best candidate correctly classify the instance?
        if probs[num_models - 1].0 == class_id {
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

    pub fn report_results(&mut self, class_names: Vec<&String>, summary_name: String) {
        let num_models = self.num_models as usize;

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
        println!("class     accuracy    tests      candidate order");

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

            print!("  {:6.2}%   {:3}        ", 100_f32 * acc, num_tests);

            for i in 1..=num_models {
                print!("{:3} ", self.result[class_id][i]);
            }
            println!();
        }

        summary.accuracy *= 100_f32;
        summary.avg_accuracy = summary.avg_accuracy * 100_f32 / num_classes as f32;

        println!("  avg_accuracy  {}%", summary.avg_accuracy);
        println!("    error_rate  {}%", 100_f32 - summary.avg_accuracy);
        println!();

        report_summary(summary, summary_name);
    }
}

fn report_summary(summary: Summary, summary_name: String) {
    let f = File::create(summary_name).unwrap();
    serde_json::to_writer(f, &summary).unwrap();
}

#[derive(serde::Serialize)]
struct Summary {
    accuracy: f32,
    avg_accuracy: f32,
}
