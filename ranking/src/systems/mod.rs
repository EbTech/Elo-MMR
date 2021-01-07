mod cf_sys;
mod elo_mmr;
mod glicko;
mod tc_sys;
mod true_skill;
mod util;

pub use cf_sys::CFSys;
pub use elo_mmr::{EloMMR, EloMMRVariant};
pub use glicko::Glicko;
pub use tc_sys::TCSys;
pub use true_skill::TrueSkillSPb;
pub use util::{
    get_participant_ratings, simulate_contest, Player, PlayersByName, Rating, RatingSystem,
};

// TODO: add a version that can take parameters, like in experiment_config but polymorphic
pub fn get_rating_system_by_name(system_name: &str) -> Result<Box<dyn RatingSystem>, String> {
    match system_name {
        "glicko" => Ok(Box::new(Glicko::default())),
        "cf" => Ok(Box::new(CFSys::default())),
        "tc" => Ok(Box::new(TCSys::default())),
        "ts" => Ok(Box::new(TrueSkillSPb::default())),
        "mmx" => Ok(Box::new(EloMMR::default_gaussian())),
        "mmr" => Ok(Box::new(EloMMR::default())),
        name => Err(format!(
            "{} is not a valid rating system. Must be one of: glicko, cf, tc, ts, mmx, mmr",
            name
        )),
    }
}
