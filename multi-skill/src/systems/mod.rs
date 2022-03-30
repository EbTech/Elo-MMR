mod bar;
mod codeforces_sys;
mod elo_mmr;
mod glicko;
mod simple_elo_mmr;
mod topcoder_sys;
mod true_skill;
mod common;

pub use bar::BAR;
pub use codeforces_sys::CodeforcesSys;
pub use elo_mmr::{EloMMR, EloMMRVariant};
pub use glicko::Glicko;
pub use simple_elo_mmr::SimpleEloMMR;
pub use topcoder_sys::TopcoderSys;
pub use true_skill::TrueSkillSPb;
pub use common::{
    get_participant_ratings, outcome_free, simulate_contest, PlayersByName,
    Rating, RatingSystem, robust_average, TanhTerm, Player, PlayerEvent
};

// TODO: add a version that can take parameters, like in experiment_config but polymorphic
pub fn get_rating_system_by_name(
    system_name: &str,
) -> Result<Box<dyn RatingSystem + Send>, String> {
    match system_name {
        "bar" => Ok(Box::new(BAR::default())),
        "glicko" => Ok(Box::new(Glicko::default())),
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
