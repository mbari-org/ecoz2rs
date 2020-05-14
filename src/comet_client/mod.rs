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

    pub fn log_vq_learn(self, m: i32, avg_distortion: f64, sigma: f64, inertia: f64) {
        println!(
            " CometClient.log_vq_learn: M={} avg_distortion={} sigma={} inertia={}",
            m, avg_distortion, sigma, inertia
        );
        //        match self.experiment_key {
        //            Some(_exp_key) => {
        //            }
        //            None => (),
        //        }
    }
}
