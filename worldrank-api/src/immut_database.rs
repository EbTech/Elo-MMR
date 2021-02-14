use crate::domain::{PlayerEvent, PlayerSummary, UserName};
use csv::Reader;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::slice::SliceIndex;

fn read_csv<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<Vec<T>, csv::Error> {
    Reader::from_path(&path)?.deserialize().collect()
}

pub struct ImmutableSportDatabase {
    players_path: PathBuf,
    top_list: Vec<PlayerSummary>,
}

impl ImmutableSportDatabase {
    pub fn new(data_path: impl AsRef<Path>) -> Result<Self, csv::Error> {
        let data_path = data_path.as_ref();
        let players_path = data_path.join("players");
        let top_list = read_csv(data_path.join(&"all_players.csv"))?;
        Ok(Self {
            players_path,
            top_list,
        })
    }

    pub fn num_players(&self) -> usize {
        self.top_list.len()
    }

    pub fn index_by_rank<I: SliceIndex<[PlayerSummary]>>(&self, index: I) -> Option<&I::Output> {
        self.top_list.get(index)
    }

    pub fn player_history(&self, handle: &UserName) -> Result<Vec<PlayerEvent>, csv::Error> {
        let filename = self.players_path.join(format!("{}.csv", handle.as_ref()));
        read_csv(filename)
    }
}
