mod player;

use crate::data_processing::Contest;
use serde::{Deserialize, Serialize};
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
pub use player::{Player, PlayerEvent};
use crate::numerical::{TANH_MULTIPLIER,solve_newton};

pub type PlayersByName = HashMap<String, RefCell<Player>>;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rating {
    pub mu: f64,
    pub sig: f64,
}

impl Rating {
    pub fn with_noise(self, sig_noise: f64) -> Self {
        Self {
            mu: self.mu,
            sig: self.sig.hypot(sig_noise),
        }
    }

    // TODO: consider making time_decay head towards a limit (mu_noob, sig_moob),
    //       or alternatively keep mu the same while having sig -> sig_noob.
    pub fn towards_noise(self, decay: f64, limit: Self) -> Self {
        let mu_diff = self.mu - limit.mu;
        let sig_sq_diff = self.sig * self.sig - limit.sig * limit.sig;
        Self {
            mu: limit.mu + mu_diff * decay,
            sig: (limit.sig * limit.sig + sig_sq_diff * decay * decay).sqrt(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TanhTerm {
    pub mu: f64,
    pub w_arg: f64,
    pub w_out: f64,
}

impl From<Rating> for TanhTerm {
    fn from(rating: Rating) -> Self {
        let w = TANH_MULTIPLIER / rating.sig;
        Self {
            mu: rating.mu,
            w_arg: w * 0.5,
            w_out: w,
        }
    }
}

impl TanhTerm {
    pub fn get_weight(&self) -> f64 {
        self.w_arg * self.w_out * 2. / TANH_MULTIPLIER.powi(2)
    }

    pub fn base_values(&self, x: f64) -> (f64, f64) {
        let z = (x - self.mu) * self.w_arg;
        let val = -z.tanh() * self.w_out;
        let val_prime = -z.cosh().powi(-2) * self.w_arg * self.w_out;
        (val, val_prime)
    }
}

// Returns the unique zero of the following strictly increasing function of x:
// offset + slope * x + sum_i weight_i * tanh((x-mu_i)/sig_i)
// We must have slope != 0 or |offset| < sum_i weight_i in order for the zero to exist.
// If offset == slope == 0, we get a robust weighted average of the mu_i's.
pub fn robust_average(
    all_ratings: impl Iterator<Item = TanhTerm> + Clone,
    offset: f64,
    slope: f64,
) -> f64 {
    let bounds = (-6000.0, 9000.0);
    let f = |x: f64| -> (f64, f64) {
        all_ratings
            .clone()
            .map(|term| {
                let tanh_z = ((x - term.mu) * term.w_arg).tanh();
                (
                    tanh_z * term.w_out,
                    (1. - tanh_z * tanh_z) * term.w_arg * term.w_out,
                )
            })
            .fold((offset + slope * x, slope), |(s, sp), (v, vp)| {
                (s + v, sp + vp)
            })
    };
    solve_newton(bounds, f)
}

pub trait RatingSystem: std::fmt::Debug {
    fn round_update(&self, contest_weight: f64, standings: Vec<(&mut Player, usize, usize)>);
}

pub fn outcome_free<T>(standings: &[(T, usize, usize)]) -> bool {
    standings.is_empty() || standings[0].2 + 1 >= standings.len()
}

pub fn simulate_contest(
    players: &mut PlayersByName,
    contest: &Contest,
    system: &dyn RatingSystem,
    mu_newbie: f64,
    sig_newbie: f64,
    contest_index: usize,
) {
    if outcome_free(&contest.standings) {
        tracing::warn!(
            "Ignoring contest {} because all players tied",
            contest_index
        );
        return;
    }

    // If a player is competing for the first time, initialize with a default rating
    contest.standings.iter().for_each(|&(ref handle, _, _)| {
        // TODO TEAMS: make an entry for every member of the team, then make the team object
        //             in teams: PlayersByName with system.make_team(players)
        players.entry(handle.clone()).or_insert_with(|| {
            RefCell::new(Player::with_rating(
                mu_newbie,
                sig_newbie,
                contest.time_seconds,
            ))
        });
    });

    // Low-level magic: verify that handles are distinct and store guards so that the cells
    // can be released later. This setup enables safe parallel processing.
    let mut guards: Vec<RefMut<Player>> = contest
        .standings
        .iter()
        // TODO TEAMS: if individual, get guard to that, else get guard to its team
        .map(|(handle, _, _)| {
            players
                .get(handle)
                .expect("Uninitialized handle")
                .try_borrow_mut()
                .expect("Duplicate handle")
        })
        .collect();

    // Update player metadata and get &mut references to all requested players
    let standings: Vec<(&mut Player, usize, usize)> = guards
        .iter_mut()
        .map(std::ops::DerefMut::deref_mut)
        .zip(contest.standings.iter())
        .map(|(player, &(_, lo, hi))| {
            player.event_history.push(PlayerEvent {
                contest_index,
                rating_mu: 0,  // will be filled by system.round_update()
                rating_sig: 0, // will be filled by system.round_update()
                perf_score: 0, // will be filled by system.round_update()
                place: lo,
            });
            player.delta_time = contest.time_seconds - player.update_time;
            player.update_time = contest.time_seconds;
            (player, lo, hi)
        })
        .collect();

    system.round_update(contest.weight, standings);

    // TODO TEAMS: each participant uses its team's update using system.infer_from_team(),
    // making sure to copy the team's event_history metadata as well
}

pub fn get_participant_ratings(
    players: &mut PlayersByName,
    contest_standings: &[(String, usize, usize)],
    min_history: usize,
) -> Vec<(Rating, usize, usize)> {
    let mut standings: Vec<(Rating, usize, usize)> = vec![];

    for &(ref handle, lo, hi) in contest_standings {
        if let Some(player) = players.get(handle).map(RefCell::borrow) {
            if player.event_history.len() >= min_history {
                standings.push((player.approx_posterior, lo, hi));
            }
        }
    }

    // Normalizing the ranks is very annoying, I probably should've just represented
    // standings as an Vec of Vec of players
    let (mut last_k, mut last_v) = (usize::MAX, usize::MAX);
    for (i, (_, lo, _)) in standings.iter_mut().enumerate() {
        if *lo != last_k {
            last_k = *lo;
            last_v = i;
        }
        *lo = last_v;
    }
    let (mut last_k, mut last_v) = (usize::MAX, usize::MAX);
    for (i, (_, _, hi)) in standings.iter_mut().enumerate().rev() {
        if *hi != last_k {
            last_k = *hi;
            last_v = i;
        }
        *hi = last_v;
    }
    standings
}
