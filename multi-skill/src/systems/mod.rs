mod bar;
mod codeforces_sys;
mod common;
mod elo_mmr;
mod endure_elo;
mod glicko;
mod simple_elo_mmr;
mod topcoder_sys;
mod true_skill;

pub use bar::BAR;
pub use codeforces_sys::CodeforcesSys;
pub use common::{
    Player, PlayerEvent, PlayersByName, Rating, RatingSystem, TanhTerm, get_participant_ratings,
    outcome_free, robust_average, simulate_contest,
};
pub use elo_mmr::{EloMMR, EloMMRVariant};
pub use endure_elo::EndureElo;
pub use glicko::Glicko;
pub use simple_elo_mmr::SimpleEloMMR;
pub use topcoder_sys::TopcoderSys;
pub use true_skill::TrueSkillSPb;

pub static SECS_PER_DAY: f64 = 86_400.;

// TODO: add a version that can take parameters, like in experiment_config but polymorphic
pub fn get_rating_system_by_name(
    system_name: &str,
) -> Result<Box<dyn RatingSystem + Send>, String> {
    match system_name {
        "bar" => Ok(Box::new(BAR::default())),
        "glicko" => Ok(Box::new(Glicko::default())),
        "endure" => Ok(Box::new(EndureElo::default())),
        "cfsys" => Ok(Box::new(CodeforcesSys::default())),
        "tcsys" => Ok(Box::new(TopcoderSys::default())),
        "trueskill" => Ok(Box::new(TrueSkillSPb::default())),
        "mmx" => Ok(Box::new(EloMMR::default_gaussian())),
        "mmx-fast" => Ok(Box::new(EloMMR::default_gaussian_fast())),
        "mmr" => Ok(Box::new(EloMMR::default())),
        "mmr-fast" => Ok(Box::new(EloMMR::default_fast())),
        "mmr-simple" => Ok(Box::new(SimpleEloMMR::default())),
        name => Err(format!(
            "{} is not a valid rating system. Must be one of: bar, glicko, cfsys, tcsys, trueskill, mmx, mmx-fast, mmr, mmr-fast, mmr-simple",
            name
        )),
    }
}
