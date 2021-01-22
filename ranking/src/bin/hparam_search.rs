extern crate ranking;

use ranking::data_processing::get_dataset_by_name;
use ranking::experiment_config::Experiment;
use ranking::systems::{self, RatingSystem};
use rayon::prelude::*;

fn log_space(
    mut lo: f64,
    mut hi: f64,
    steps: usize,
    resolution: f64,
) -> impl Iterator<Item = f64> + Clone {
    lo = lo.ln();
    hi = hi.ln();
    let mult = ((steps - 1) as f64).recip();
    let exp_with_rounding = move |x: f64| (x.exp() / resolution).round() * resolution;
    (0..steps).map(move |i| exp_with_rounding(lo + i as f64 * mult * (hi - lo)))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} dataset_name", args[0]);
        return;
    }

    // Prepare the contest system parameters
    let perf_range = log_space(75., 600., 10, 1.);
    let drift_range = log_space(5., 40., 10, 1.);
    let mut systems: Vec<Box<dyn RatingSystem + Send>> = vec![];

    for sig_perf in perf_range.clone() {
        for weight in log_space(0.01, 10., 16, 1e-3) {
            let system = systems::CodeforcesSys { sig_perf, weight };
            systems.push(Box::new(system));
        }
    }
    for weight_multiplier in log_space(0.02, 50., 40, 1e-3) {
        let system = systems::TopcoderSys { weight_multiplier };
        systems.push(Box::new(system));
    }
    for eps in log_space(0.5, 50., 9, 0.1) {
        for sig_perf in perf_range.clone() {
            for sig_drift in drift_range.clone() {
                let system = systems::TrueSkillSPb {
                    eps,
                    beta: sig_perf,
                    convergence_eps: 1e-4,
                    sigma_growth: sig_drift,
                };
                systems.push(Box::new(system));
            }
        }
    }
    for sig_perf in perf_range.clone() {
        for sig_drift in drift_range.clone() {
            for &split_ties in &[false, true] {
                let system = systems::EloMMR {
                    sig_perf,
                    sig_drift,
                    split_ties,
                    variant: systems::EloMMRVariant::Gaussian,
                };
                systems.push(Box::new(system));

                let rho_vals = &[0., 0.04, 0.2, 1., 5., f64::INFINITY];
                for &rho in rho_vals {
                    let system = systems::EloMMR {
                        sig_perf,
                        sig_drift,
                        split_ties: false,
                        variant: systems::EloMMRVariant::Logistic(rho),
                    };
                    systems.push(Box::new(system));
                }
            }
        }
    }
    for sig_perf in perf_range.clone() {
        for sig_drift in drift_range.clone() {
            let system = systems::Glicko {
                sig_perf,
                sig_drift,
            };
            systems.push(Box::new(system));

            let system = systems::BAR {
                sig_perf,
                sig_drift,
                kappa: 1e-4,
            };
            systems.push(Box::new(system));
        }
    }

    systems.into_par_iter().for_each(|system| {
        // We're repeatedly loading the same dataset's metadata but this is cheap anyway
        let dataset = get_dataset_by_name(&args[1]).unwrap();
        let max_contests = dataset.len() / 10;

        let experiment = Experiment {
            max_contests,
            mu_noob: 1500.,
            sig_noob: 350.,
            system,
            dataset,
        };
        experiment.eval(0, "");
    });
}
