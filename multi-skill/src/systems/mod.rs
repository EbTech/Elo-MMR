mod bar;
mod codeforces_sys;
mod elo_mmr;
mod glicko;
mod topcoder_sys;
mod true_skill;
mod util;

pub use bar::BAR;
pub use codeforces_sys::CodeforcesSys;
pub use elo_mmr::{EloMMR, EloMMRVariant};
pub use glicko::Glicko;
pub use topcoder_sys::TopcoderSys;
pub use true_skill::TrueSkillSPb;
pub use util::{
    get_participant_ratings, outcome_free, simulate_contest, Player, PlayersByName, Rating,
    RatingSystem,
};

// TODO: add a version that can take parameters, like in experiment_config but polymorphic
pub fn get_rating_system_by_name(
    system_name: &str,
) -> Result<Box<dyn RatingSystem + Send>, String> {
    match system_name {
        "bar" => Ok(Box::new(BAR::default())),
        "glicko" => Ok(Box::new(Glicko::default())),
        "cf" => Ok(Box::new(CodeforcesSys::default())),
        "tc" => Ok(Box::new(TopcoderSys::default())),
        "ts" => Ok(Box::new(TrueSkillSPb::default())),
        "mmx" => Ok(Box::new(EloMMR::default_gaussian())),
        "mmr" => Ok(Box::new(EloMMR::default())),
        name => Err(format!(
            "{} is not a valid rating system. Must be one of: bar, glicko, cf, tc, ts, mmx, mmr",
            name
        )),
    }
}
