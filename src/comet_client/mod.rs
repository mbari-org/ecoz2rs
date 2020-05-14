use std::collections::HashMap;

#[repr(C)]
pub struct CometClient {
    api_key: Option<String>,
    experiment_key: Option<String>,
}

impl CometClient {
    pub fn new(experiment_key: Option<String>) -> CometClient {
        let api_key = match experiment_key {
            Some(_) => match std::env::var("COMET_API_KEY") {
                Ok(key) => Some(key),
                Err(_) => {
                    eprintln!("WARN: CometClient: no COMET_API_KEY defined");
                    None
                }
            },
            None => None,
        };

        CometClient {
            api_key,
            experiment_key,
        }
    }

    pub fn log_vq_learn(&self, m: i32, avg_distortion: f64, sigma: f64, inertia: f64) {
        println!(
            " CometClient.log_vq_learn: M={} avg_distortion={} sigma={} inertia={}",
            m, avg_distortion, sigma, inertia
        );

        match &self.experiment_key {
            Some(exp_key) => {
                let metric_name = &String::from("avg_distortion");
                let metric_value = &format!("{}", avg_distortion);
                let authorization = self.api_key.as_ref().unwrap();

                let client = reqwest::blocking::Client::new();
                let mut map = HashMap::new();
                map.insert("experimentKey", exp_key);
                map.insert("metricName", metric_name);
                map.insert("metricValue", metric_value);

                let res = client
                    .post("https://www.comet.ml/api/rest/v2/write/experiment/metric")
                    .json(&map)
                    .header("Authorization", authorization)
                    .send();

                println!("POST metric response: {:?}", res)
            }
            None => (),
        }
    }
}
