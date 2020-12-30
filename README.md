# Elo-MMR: A Rating System for Massive Multiplayer Competitions

This is a package containing implementations of several rating systems for multi-player competitions: pairwise-Glicko, Codeforces, TopCoder, TrueSkill-SPb, and the new system Elo-MMR.

## Getting started

First, [install Rust](https://www.rust-lang.org/tools/install). Then,
```
cd ranking/
cargo run --release --bin run mmr codeforces
```

## What does this command mean?

`cargo run` compiles and runs a Rust project.

`--release` creates a release build, which takes longer to compile but executes faster than a debug build.

`--bin run` selects the entry-point `ranking/src/bin/run.rs`.

`mmr` is a command-line argument specifying the rating system.

`codeforces` is a command-line argument specifying the dataset.

An optional integer argument may follow, to specify how many contests to process.

## What does this command do?

It pulls data from the Codeforces contests specified in `data/contest_ids.txt`. If a contest is not already stored in `cache/codeforces/`, then it is downloaded there via the Codeforces online API. Finally, the resulting skill ratings of all the contestants are saved in `data/ratings_output.txt`.

Please note that your first Codeforces run will be slower, as the data is pulled from the Codeforces API, or may even fail if the Codeforces site experiences downtime. However, subsequent runs should be much faster.

## How can I produce ratings from my own dataset?

Run the basic Codeforces command for at least a few seconds to download sample contest standings files. Follow their format when creating your own files. Number your files with consecutive numbers, the first contest being `0.json`, then `1.json`, and so on. Then, place these files in `cache/{contest_name}/`. Finally, run the above command with `codeforces` replaced by `{contest_name}`.

## Rating system details

Please see the [paper](paper/EloR_updated.pdf).
