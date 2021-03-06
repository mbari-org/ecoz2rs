use reqwest::blocking::Client;
use std::collections::HashMap;

pub struct CometClient {
    api_key: Option<String>,
    experiment_key: Option<String>,
    client: Option<Client>,
}

impl CometClient {
    pub fn new(experiment_key: Option<String>) -> CometClient {
        let (api_key, client) = match experiment_key {
            Some(_) => match std::env::var("COMET_API_KEY") {
                Ok(key) => (Some(key), Some(Client::new())),
                Err(_) => {
                    eprintln!("WARN: CometClient: no COMET_API_KEY defined");
                    (None, None)
                }
            },
            None => (None, None),
        };

        CometClient {
            api_key,
            experiment_key,
            client,
        }
    }

    pub fn log_vq_learn(&self, m: i32, avg_distortion: f64, sigma: f64, inertia: f64) {
        //        println!(
        //            " CometClient.log_vq_learn: M={} avg_distortion={} sigma={} inertia={}",
        //            m, avg_distortion, sigma, inertia
        //        );

        if let (Some(exp_key), Some(client)) = (&self.experiment_key, &self.client) {
            self.log_metric(exp_key, "M", &format!("{}", m), client);

            self.log_metric(
                exp_key,
                "avg_distortion",
                &format!("{}", avg_distortion),
                client,
            );

            self.log_metric(exp_key, "sigma", &format!("{}", sigma), client);

            self.log_metric(exp_key, "inertia", &format!("{}", inertia), client);
        }
    }

    pub fn log_parameter(&self, name: &str, value: &str) {
        if let (Some(exp_key), Some(client)) = (&self.experiment_key, &self.client) {
            self._log_parameter(exp_key, name, value, client);
        }
    }

    fn _log_parameter(&self, exp_key: &str, name: &str, value: &str, client: &Client) {
        let authorization = self.api_key.as_ref().unwrap();
        let mut map = HashMap::new();
        map.insert("experimentKey", exp_key);
        map.insert("parameterName", name);
        map.insert("parameterValue", value);

        let res = client
            .post("https://www.comet.ml/api/rest/v2/write/experiment/parameter")
            .json(&map)
            .header("Authorization", authorization)
            .send()
            .unwrap();

        println!("POST metric response: status={}", res.status())
    }

    fn log_metric(&self, exp_key: &str, name: &str, value: &str, client: &Client) {
        let authorization = self.api_key.as_ref().unwrap();
        let mut map = HashMap::new();
        map.insert("experimentKey", exp_key);
        map.insert("metricName", name);
        map.insert("metricValue", value);

        let res = client
            .post("https://www.comet.ml/api/rest/v2/write/experiment/metric")
            .json(&map)
            .header("Authorization", authorization)
            .send()
            .unwrap();

        println!("POST metric response: status={}", res.status())
    }
}
