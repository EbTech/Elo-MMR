extern crate ranking;

use ranking::data_processing::get_dataset_by_name;
use ranking::metrics::compute_metrics_custom;
use ranking::systems;
use ranking::systems::{simulate_contest, RatingSystem};
use std::collections::HashMap;
use std::time::Instant;

fn main() {
    // Prepare the contest system parameters
    let mut systems: Vec<Box<dyn RatingSystem>> = vec![];
    for si in (100..=500).step_by(40) {
        for wi in -8..=4 {
            let sig_perf = si as f64;
            let weight = 10f64.powf((wi as f64) * 0.25);
            let system = systems::CodeforcesSys { sig_perf, weight };
            systems.push(Box::new(system));
        }
    }
    for pi in (100..=500).step_by(50) {
        for li in (0..=120).step_by(20) {
            let sig_perf = pi as f64;
            let sig_drift = li as f64;
            let system = systems::EloMMR {
                sig_perf,
                sig_drift,
                split_ties: false,
                variant: systems::EloMMRVariant::Gaussian,
            };
            systems.push(Box::new(system));
        }
    }
    for ri in -1..=1 {
        for pi in (100..=500).step_by(50) {
            for li in (0..=120).step_by(20) {
                let sig_perf = pi as f64;
                let sig_drift = li as f64;
                let system = systems::EloMMR {
                    sig_perf,
                    sig_drift,
                    split_ties: false,
                    variant: systems::EloMMRVariant::Logistic(2f64.powi(ri)),
                };
                systems.push(Box::new(system));
            }
        }
    }
    for wi in -15..=15 {
        let weight_multiplier = 10f64.powf((wi as f64) * 0.1);
        let system = systems::TopcoderSys { weight_multiplier };
        systems.push(Box::new(system));
    }
    for ei in 1..=5 {
        for bi in (140..=360).step_by(40) {
            for si in (0..=20).step_by(4) {
                let eps = (ei as f64) * 0.1;
                let beta = bi as f64;
                let sigma_growth = si as f64;
                let system = systems::TrueSkillSPb {
                    eps,
                    beta,
                    convergence_eps: 2e-4,
                    sigma_growth,
                };
                systems.push(Box::new(system));
            }
        }
    }

    // Run the contest histories and measure
    let dataset = get_dataset_by_name("codeforces").unwrap();
    let max_contests = usize::MAX;
    let mu_noob = 1500.;
    let sig_noob = 350.;
    for system in systems {
        let mut players = HashMap::new();
        let mut avg_perf = compute_metrics_custom(&mut players, &[]);
        let now = Instant::now();

        for contest in dataset.iter().take(max_contests) {
            // Predict performance must be run before simulate contest
            // since we don't want to make predictions after we've seen the contest
            avg_perf += compute_metrics_custom(&mut players, &contest.standings);

            // Now run the actual rating update
            simulate_contest(&mut players, &contest, &*system, mu_noob, sig_noob);
        }
        println!(
            "{:?}: {}, {}s",
            system,
            avg_perf,
            now.elapsed().as_millis() as f64 / 1000.
        );
    }
}
