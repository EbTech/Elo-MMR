# Elo-MMR: A Rating System for Massive Multiplayer Competitions

[![Crates.io Version](https://img.shields.io/crates/v/multi-skill.svg)](https://crates.io/crates/multi-skill)
[![Documentation](https://docs.rs/multi-skill/badge.svg)](https://docs.rs/multi-skill)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/EbTech/Elo-MMR/blob/master/LICENSE)
[![Crates.io Downloads](https://img.shields.io/crates/d/multi-skill.svg)](https://crates.io/crates/multi-skill)
[![Gitter](https://badges.gitter.im/multi-skill/community.svg)](https://gitter.im/multi-skill/community?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)

This is a package containing implementations of several rating systems for multi-player competitions: Glicko all-pairs, BAR BT-all-pairs, Codeforces, Topcoder, TrueSkill-SPb, and the new system Elo-MMR. All under MIT license except for the contents of deprecated/cpp/trueskill.

Rating systems estimate the skills of players who participate in a common activity. The Elo-MMR algorithm was designed for activities in which moderate to large numbers of players are ranked at competitive events, and results cannot be standardized across different events for any of the following reasons:

- Each event features novel challenges, such as in obstacle course races, rock climbing, and academic olympiads.

- The contestants are evaluated relative to others at the same event, perhaps by a panel of judges with some degree of subjectivity, as in competitive ballroom, figure skating, and gymnastics.

- The contestants interact heavily with others at the same event, as in most board games.

In these settings, it's often useful to quantify how good a player is. Ratings could be used to track a player's progress over a training programme, set motivational objectives, predict likely champions, or create invitational beginner-only or expert-only events. Three properties of Elo-MMR make it particularly well-suited to these aims:

- Massively Multiplayer: the algorithm is fast and numerically stable, even with thousands or millions of individually ranked contestants.

- Incentive-Compatible: the better you do in competitions, the higher your rating will be.

- Robust Response: one very bad (or very good) event cannot change your rating too much.

Note: in theory, Elo-MMR can be applied in team competitions as well, but additional research is needed to determine the best ways to do this.

## Getting Started

First, [install Rust](https://www.rust-lang.org/tools/install). From the `multi-skill/` directory, run the command
```
RUST_LOG=debug cargo run --release --bin rate mmr-fast codeforces
```

UPDATE: it's now also possible to run the rating system from a config file. Here's an example: (this config file is out of date, we'll update it shortly.)
```
RUST_LOG=debug cargo run --release --bin rate file: ../experiments/codeforces/mmr-fast-acc.json
```

To test the new checkpointing feature, try
```
RUST_LOG=debug cargo run --release --bin rate_from_configs ../experiments/testing/mmr-cf-1to10.json ../experiments/testing/mmr-cf-11to20.json
```
which produces state checkpoint files in the `experiments/testing/` directory.

### What does the first command mean?

`RUST_LOG=debug` sets an environment variable to print additional information to the terminal during execution. Note that environment variables are [set differently on Windows](https://stackoverflow.com/questions/18433840/logging-rust-programs).

`cargo run` compiles and runs a Rust project.

`--release` creates a release build, which takes longer to compile but executes faster than a dev build.

`--bin rate` selects the entry-point `multi-skill/src/bin/rate.rs`.

`mmr-fast` is a command-line argument specifying the rating system. Try `mmr` for a slower but more precise version of Elo-MMR.

`codeforces` is a command-line argument specifying the dataset.

An optional integer argument may follow, to specify how many contests to process.

### What does this command do?

It pulls data from the Codeforces contests specified in `data/codeforces/contest_ids.json`. If a contest is not already stored in `cache/codeforces/`, then it is downloaded there via the Codeforces online API. Finally, the resulting skill ratings of all the contestants are saved in `data/codeforces/ratings_output.csv`.

Please note that your first Codeforces run will be slower, as the contest standings are pulled from the Codeforces API. It may even fail if Codeforces.com experiences downtime, or decides that you've used too much bandwidth; if this happens, please wait a few minutes to try again.

### How can I rate contestants of my own games?

Contests are stored in JSON format, with the standings listed in order from first to last place. Here is a sample contest file, where the angled brackets and ellipsis should be replaced with your own data:
```
{
    "name": <str, human-readable name of the contest>, 
    "time_seconds": <int, seconds since the Unix epoch>, 
    "standings": [[<str, player 0's name>, <int, low rank>, <int, high rank>], 
                  [<str, player 1's name>, <int, low rank>, <int, high rank>],
                  ...]]
    "weight": <optional float, defaults to 1 if not included>,
    "perf_ceiling": <optional float, defaults to infinity if not included>
}
```
The low and high ranks are 0-indexed and will differ for players who are involved in a tie. They specify the range of players with whom this player tied. For example, if there is a three-way tie at the top, players 0, 1 and 2 will each have a low rank of 0 and a high rank of 2.

If you ran the above Codeforces command for at least a few seconds, then you will have downloaded some example contest files in `cache/codeforces/`, which you may use as a reference.

With this file format in mind, you can run your own contests as follows:

- Number your files with consecutive integers, the first contest being saved in `0.json`, the second in `1.json`, and so on.

- Place your files in `cache/{dataset_name}/`.

- Finally, run the same command, but with `codeforces` replaced by `{dataset_name}`.

## Mathematical Details

Please see the [full paper](paper/EloMMR.pdf) published at the Web Conference 2021. If you use this crate in your research, please consider citing our paper.
