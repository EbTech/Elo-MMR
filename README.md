# Elo-MMR: A Rating System for Massive Multiplayer Competitions

This is a package containing implementations of several rating systems for multi-player competitions: pairwise-Glicko, Codeforces, TopCoder, TrueSkill-SPb, and the new system Elo-MMR.

## Getting started

First, [install Rust](https://www.rust-lang.org/tools/install). Then,
```
cd ranking/
cargo run --release --bin run elor
```

## What does this command mean?

`cargo run` compiles and runs a Rust project.

`--release` creates a release build, which takes longer to compile but executes faster than a debug build.

`--bin run` selects the entry-point `ranking/src/bin/run.rs`.

`elor` is a command-line argument specifying the rating system.

An optional integer argument may follow, to specify how many contests to process.

## What does this command do?

It pulls data from the Codeforces contests specified in `data/contest_ids.txt`. If a contest is not already stored in `cache/codeforces/`, then it is downloaded there via the Codeforces online API. Alternatively, you can manually populate `cache/codeforces` with standings from your own contests. Finally, the resulting skill ratings of all the contestants are saved in `data/CFratings_temp.txt`.

## Rating system details

Please see the [paper](paper/EloR_updated.pdf).
