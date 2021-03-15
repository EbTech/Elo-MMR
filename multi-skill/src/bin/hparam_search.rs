use multi_skill::data_processing::{get_dataset_by_name, BoxedDataset, Dataset};
use multi_skill::experiment_config::Experiment;
use multi_skill::systems::{self, RatingSystem};
use rayon::prelude::*;

fn log_space(
    mut lo: f64,
    mut hi: f64,
    steps: usize,
    resolution: f64,
) -> impl Iterator<Item = f64> + Clone {
    assert!(lo < hi && steps > 1);
    lo = lo.ln();
    hi = hi.ln();
    let mult = ((steps - 1) as f64).recip();
    let exp_with_rounding = move |x: f64| (x.exp() / resolution).round() * resolution;
    (0..steps).map(move |i| exp_with_rounding(lo + i as f64 * mult * (hi - lo)))
}

fn main() {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        tracing::error!("Usage: {} dataset_name", args[0]);
        return;
    }

    // Prepare the contest system parameters
    let beta_range = log_space(50., 400., 10, 5.);
    let drift_range = log_space(10., 80., 10, 1.);
    let mut systems: Vec<Box<dyn RatingSystem + Send>> = vec![];

    for beta in beta_range.clone() {
        for weight_multiplier in log_space(0.01, 10., 16, 1e-3) {
            let system = systems::CodeforcesSys {
                beta,
                weight_multiplier,
            };
            systems.push(Box::new(system));
        }
    }
    for weight_multiplier in log_space(0.02, 50., 40, 1e-3) {
        let system = systems::TopcoderSys { weight_multiplier };
        systems.push(Box::new(system));
    }
    for eps in log_space(0.1, 10., 9, 0.1) {
        for beta in beta_range.clone() {
            for sig_drift in drift_range.clone() {
                let system = systems::TrueSkillSPb {
                    eps,
                    beta,
                    convergence_eps: 1e-4,
                    sig_drift,
                };
                systems.push(Box::new(system));
            }
        }
    }
    for beta in beta_range.clone() {
        for sig_limit in log_space(20., 0.75 * beta, 10, 1.) {
            for &split_ties in &[false, true] {
                // make the algorithm fast
                let subsample_size = 100;
                let subsample_bucket = 2.;

                // Gaussian performance model
                let system = systems::EloMMR {
                    beta,
                    sig_limit,
                    drift_per_sec: 0.,
                    split_ties,
                    subsample_size,
                    subsample_bucket,
                    variant: systems::EloMMRVariant::Gaussian,
                };
                systems.push(Box::new(system));

                // Logistic performance model with pseudodiffusion
                let rho_vals = &[0., 0.04, 0.2, 1., 5., f64::INFINITY];
                for &rho in rho_vals {
                    let system = systems::EloMMR {
                        beta,
                        sig_limit,
                        drift_per_sec: 0.,
                        split_ties,
                        subsample_size,
                        subsample_bucket,
                        variant: systems::EloMMRVariant::Logistic(rho),
                    };
                    systems.push(Box::new(system));
                }
            }
        }
    }
    for beta in beta_range.clone() {
        for sig_drift in drift_range.clone() {
            let system = systems::Glicko { beta, sig_drift };
            systems.push(Box::new(system));

            let system = systems::BAR {
                beta,
                sig_drift,
                kappa: 1e-4,
            };
            systems.push(Box::new(system));
        }
    }

    systems.into_par_iter().for_each(|system| {
        // We're repeatedly loading the same dataset's metadata but this is cheap anyway
        let dataset = get_dataset_by_name(&args[1]).unwrap();
        let dataset_len = dataset.len();
        let train_set_len = dataset_len / 10;
        let boxed: BoxedDataset = Box::new(dataset.subrange(..train_set_len));
        let dataset = boxed.wrap();

        let experiment = Experiment {
            mu_noob: 1500.,
            sig_noob: 350.,
            system,
            dataset,
        };
        let results = experiment.eval(0);

        let horizontal = "============================================================";
        tracing::info!(
            "{:?}: {}, {}s, {} contests\n{}",
            experiment.system,
            results.avg_perf,
            results.secs_elapsed,
            dataset_len,
            horizontal
        );
    });
}
