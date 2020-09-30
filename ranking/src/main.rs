mod compute_ratings;
mod contest_config;
mod read_codeforces;

mod cf_system;
mod elor_system;
mod tc_system;
mod ts_system;

use compute_ratings::{predict_performance, print_ratings, simulate_contest};
use contest_config::{get_contest, get_contest_config, get_contest_ids, ContestSource};
use std::collections::HashMap;
use std::time::{Instant};

#[allow(unused_imports)]
use cf_system::CodeforcesSystem as CFSys;
#[allow(unused_imports)]
use elor_system::EloRSystem as EloRSys;
#[allow(unused_imports)]
use tc_system::TopCoderSystem as TCSys;
#[allow(unused_imports)]
use ts_system::TrueSkillSPBSystem as TSSys;

/// simulates the entire history of Codeforces; runs on my laptop in an hour,
/// somewhat longer if the Codeforces API data isn't cached
fn main() {
    // let mut players = HashMap::new();
    // let config = get_contest_config(ContestSource::Codeforces);
    // let mut system = EloRSys::default();
    // let mut last_contest_time = 0;
    // for contest_id in get_contest_ids(&config.contest_id_file) {
    //     let contest = get_contest(&config.contest_cache_folder, contest_id);
    //     println!(
    //         "Processing {:5} contestants in contest/{:4}: {}",
    //         contest.standings.len(),
    //         contest.id,
    //         contest.name
    //     );
    //     simulate_contest(&mut players, &contest, &mut system);
    //     log_performance(&mut players, &contest, &mut system);
    //     last_contest_time = contest.time_seconds;
    // }
    // print_ratings(&players, last_contest_time - 183 * 86_400);

    let max_contests = 100;
    let config = get_contest_config(ContestSource::Codeforces);
    let mut players = HashMap::new();
    let contest_ids = get_contest_ids(&config.contest_id_file);
    let topk = 50;

    println!("CodeForces average performance ({} contests, top-{}):", max_contests, topk);
    for si in -10..10 {
        for wi in 1..20 {
            let sig_perf = (si as f64) * 15. + 800. / std::f64::consts::LN_10;
            let weight = (wi as f64) * 0.05;

            players.clear();
            let now = Instant::now();
            let mut system = CFSys {sig_perf: sig_perf, weight: weight};
            let mut avg_perf = 0.;

            for (i, contest_id) in contest_ids.iter().enumerate() {
                if i >= max_contests {
                    break;
                }
                let contest = get_contest(&config.contest_cache_folder, *contest_id);

                // Predict performance must be run before simulate contest
                // since we don't want to make predictions after we've seen the contest
                avg_perf += predict_performance(&mut players, &contest, &system, topk);
                simulate_contest(&mut players, &contest, &mut system);
            }
            avg_perf /= max_contests as f64;
            println!(
                "{}, {}: {}, {}s",
                sig_perf, weight, avg_perf, now.elapsed().as_millis() as f64 / 1000.
            );
        }
    }

    println!("EloR average performance ({} contests, top-50):", max_contests);
    for pi in -15..15 {
        for li in -15..15 {
            let sig_perf = (pi as f64) * 5. + 170.;
            let sig_limit = (li as f64) * 1.5 + 60.;

            players.clear();
            let now = Instant::now();
            let mut system = EloRSys {sig_perf: sig_perf, sig_limit: sig_limit, variant: elor_system::EloRVariant::LogisticFull};
            let mut avg_perf = 0.;

            for (i, contest_id) in contest_ids.iter().enumerate() {
                if i >= max_contests {
                    break;
                }
                let contest = get_contest(&config.contest_cache_folder, *contest_id);

                // Predict performance must be run before simulate contest
                // since we don't want to make predictions after we've seen the contest
                avg_perf += predict_performance(&mut players, &contest, &system, topk);
                simulate_contest(&mut players, &contest, &mut system);
            }
            avg_perf /= max_contests as f64;
            println!(
                "{}, {}: {}, {}s",
                sig_perf, sig_limit, avg_perf, now.elapsed().as_millis() as f64 / 1000.
            );
        }
    }

    println!("TopCoder average performance ({} contests, top-50):", max_contests);
    for wi in 1..20 {
        let weight = (wi as f64) * 0.05;

        players.clear();
        let now = Instant::now();
        let mut system = TCSys {weight_multiplier: weight};
        let mut avg_perf = 0.;

        for (i, contest_id) in contest_ids.iter().enumerate() {
            if i >= max_contests {
                break;
            }
            let contest = get_contest(&config.contest_cache_folder, *contest_id);

            // Predict performance must be run before simulate contest
            // since we don't want to make predictions after we've seen the contest
            avg_perf += predict_performance(&mut players, &contest, &system, topk);
            simulate_contest(&mut players, &contest, &mut system);
        }
        avg_perf /= max_contests as f64;
        println!(
            "{}: {}, {}s",
            weight, avg_perf, now.elapsed().as_millis() as f64 / 1000.
        );
    }

    println!("TrueSkill average performance ({} contests, top-50):", max_contests);
    for ei in 1..6 {
        for bi in -3..3 {
            for si in -1..5 {
                let eps = (ei as f64) * 0.1;
                let beta = (bi as f64) * 30. + 250.;
                let sigma_growth = (si as f64) * 2.5 + 10.;

                players.clear();
                let now = Instant::now();
                let mut system = TSSys {eps: eps, beta: beta, convergence_eps: 2e-4, sigma_growth: sigma_growth};
                let mut avg_perf = 0.;

                for (i, contest_id) in contest_ids.iter().enumerate() {
                    if i >= max_contests {
                        break;
                    }
                    let contest = get_contest(&config.contest_cache_folder, *contest_id);

                    // Predict performance must be run before simulate contest
                    // since we don't want to make predictions after we've seen the contest
                    avg_perf += predict_performance(&mut players, &contest, &system, topk);
                    simulate_contest(&mut players, &contest, &mut system);
                }
                avg_perf /= max_contests as f64;
                println!(
                    "{}, {}, {}: {}, {}s",
                    eps, beta, sigma_growth, avg_perf, now.elapsed().as_millis() as f64 / 1000.
                );
            }
        }
    }
}
