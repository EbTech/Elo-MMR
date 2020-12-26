# Elo-MMR: A Rating System for Massive Multiplayer Competitions

Instructions to compute ratings after installing Rust:
```
cd ranking/
cargo run --release --bin run
```

`data/contest_ids.txt` tells the program which contests to look for in cache/
You can populate the corresponding cache/ entries manually with your own contests.
If the corresponding cache/ entry doesn't exist, the program searches Codeforces for it.
