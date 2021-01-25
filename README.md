# Elo-MMR: A Rating System for Massive Multiplayer Competitions

This is a package containing implementations of several rating systems for multi-player competitions: all-pairs Glicko, all-pairs BAR, Codeforces, TopCoder, TrueSkill-SPb, and the new system Elo-MMR.

Rating systems estimate the skills of players who participate in a common activity. The Elo-MMR algorithm was designed for activities in which moderate to large numbers of players are ranked at competitive events, and results cannot be standardized across different events for any of the following reasons:

- Each event features novel challenges, such as in obstacle course races, rock climbing, and academic olympiads.

- The contestants are evaluated relative to others at the same event, perhaps by a panel of judges with some degree of subjectivity, as in competitive ballroom, figure skating, and gymnastics.

- The contestants interact heavily with others at the same event, as in most board games.

In these settings, it's often useful to quantify how good a player is. Ratings could be used to track a player's progress over a training programme, set motivational objectives, predict likely champions, or create invitational beginner-only or expert-only events. Three properties of Elo-MMR make it particularly well-suited to these aims:

- Massively Multiplayer: the algorithm is fast and numerically stable, even with thousands or millions of individually ranked contestants.

- Aligned Incentives: the better you do in competitions, the higher your rating will be.

- Robust Response: one very bad (or very good) event cannot change your rating too much.

Note: in theory, Elo-MMR can be applied in team competitions as well, but additional research is needed to determine the best ways to do this.

## Getting Started

First, [install Rust](https://www.rust-lang.org/tools/install). From the `ranking` directory, run the command
```
cargo run --release --bin rate mmr codeforces
```

UPDATE: it's now also possible to run the rating system from a config file. Here's an example:
```
cargo run --release --bin rate file: ../experiments/codeforces/mmr-acc.json
```

### What does this command mean?

`cargo run` compiles and runs a Rust project.

`--release` creates a release build, which takes longer to compile but executes faster than a debug build.

`--bin rate` selects the entry-point `ranking/src/bin/rate.rs`.

`mmr` is a command-line argument specifying the rating system.

`codeforces` is a command-line argument specifying the dataset.

An optional integer argument may follow, to specify how many contests to process.

### What does this command do?

It pulls data from the Codeforces contests specified in `data/codeforces/contest_ids.json`. If a contest is not already stored in `cache/codeforces/`, then it is downloaded there via the Codeforces online API. Finally, the resulting skill ratings of all the contestants are saved in `data/ratings_output.csv`.

Please note that your first Codeforces run will be slower, as the data is pulled from the Codeforces API, or may even fail if the Codeforces site experiences downtime. However, subsequent runs should be much faster.

### How can I rate contestants of my own games?

We use json as the basic format to storing contests. A sample json contest file is as follows:
```
{
    "id": [int, contest id], 
    "name": [str, contest name], 
    "time_seconds": [int, seconds since epoch], 
    "standings": [["player 0", lowest possible rank, highest possible rank], 
                  ["player 1", lowest possible rank, highest possible rank],
                  ...]]
}
```
The lowest and highest possible ranks for a player are 0-indexed and used in case of a tie (for example, if there is a tie in the top-two spots, the lowest possible rank for player 0 is 0 and the highest is 1).

#### For more examples of sample contest files:

- Run the basic Codeforces command for at least a few seconds to download some contest standings files.

- Follow their format when creating your own files. The `id` and `name` fields are only for debugging. `time_seconds` is measured from the Unix Epoch.

- Number your files with consecutive integers, the first contest being saved in `0.json`, the second in `1.json`, and so on.

- Place your files in `cache/{dataset_name}/`.

- Finally, run the same command, but with `codeforces` replaced by `{dataset_name}`.

## Mathematical Details

Please see the [full paper](https://arxiv.org/abs/2101.00400).
