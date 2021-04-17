use crate::domain::{ContestSummary, HistoryPoint, PlayerSummary, UserName};
use csv::Reader;
use serde::de::DeserializeOwned;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::slice::SliceIndex;
use superslice::Ext;

fn read_csv<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<Vec<T>, csv::Error> {
    Reader::from_path(path)?.deserialize().collect()
}

pub struct ImmutableSportDatabase {
    players_path: PathBuf,
    top_list: Vec<PlayerSummary>,
    contest_list: Vec<ContestSummary>,
}

impl ImmutableSportDatabase {
    pub fn new(data_path: impl AsRef<Path>) -> Result<Self, csv::Error> {
        let data_path = data_path.as_ref();
        let players_path = data_path.join("players");
        let top_list = read_csv(data_path.join(&"all_players.csv"))?;
        let contest_list = read_csv(data_path.join(&"all_contests.csv"))?;
        Ok(Self {
            players_path,
            top_list,
            contest_list,
        })
    }

    pub fn num_players(&self) -> usize {
        self.top_list.len()
    }

    pub fn index_by_rank<I: SliceIndex<[PlayerSummary]>>(&self, index: I) -> Option<&I::Output> {
        self.top_list.get(index)
    }

    pub fn count_rating_range(&self, min: i32, max: i32) -> usize {
        let reverse_key = |player: &PlayerSummary| Reverse(player.display_rating);
        let idx_lo = self.top_list.lower_bound_by_key(&Reverse(max), reverse_key);
        let idx_hi = self.top_list.upper_bound_by_key(&Reverse(min), reverse_key);
        idx_hi - idx_lo
    }

    pub fn player_history(&self, handle: &UserName) -> Result<Vec<HistoryPoint>, csv::Error> {
        let filename = self.players_path.join(format!("{}.csv", handle.as_ref()));
        let history = read_csv(filename)?;
        let history_with_contest_data = history
            .into_iter()
            .map(|ev| {
                HistoryPoint::new(
                    &ev,
                    self.contest_list.get(ev.contest_index).expect(
                        "Inconsistent database: PlayerEvent pointing to invalid contest index",
                    ),
                )
            })
            .collect();
        Ok(history_with_contest_data)
    }

    pub fn autocomplete(&self, prefix: &UserName, max_suggestions: usize) -> Vec<String> {
        // Note: this can be made case-insensitive, and sped up using a trie.
        let mut filtered_list: Vec<_> = self
            .top_list
            .iter()
            .filter(|player| player.handle.starts_with(prefix.as_ref()))
            .take(max_suggestions + 1)
            .collect();

        // If there are too many candidates, yield no candidates.
        if filtered_list.len() > max_suggestions {
            filtered_list.clear();
        }

        // Sort the candidates alphabetically.
        filtered_list.sort_unstable_by_key(|player| &player.handle);

        // Return just the names.
        filtered_list
            .into_iter()
            .map(|player| player.handle.clone())
            .collect()
    }
}

pub struct SportDatabases {
    databases: HashMap<String, ImmutableSportDatabase>,
}

impl SportDatabases {
    pub fn new(dir: impl AsRef<Path>, sources: Vec<String>) -> Result<Self, csv::Error> {
        let mut databases = HashMap::with_capacity(sources.len());
        for source in sources {
            let path = dir.as_ref().join(&source);
            let db = ImmutableSportDatabase::new(&path)?;
            databases.insert(source, db);
        }
        Ok(Self { databases })
    }

    pub fn get(&self, source: &str) -> Option<&ImmutableSportDatabase> {
        self.databases.get(source)
    }
}
