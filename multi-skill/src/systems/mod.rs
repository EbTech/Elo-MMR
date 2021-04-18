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
    get_participant_ratings, outcome_free, simulate_contest, Player, PlayerEvent, PlayersByName,
    Rating, RatingSystem,
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
        name => Err(format!(
            "{} is not a valid rating system. Must be one of: bar, glicko, cfsys, tcsys, trueskill, mmx, mmx-fast, mmr, mmr-fast",
            name
        )),
    }
}
