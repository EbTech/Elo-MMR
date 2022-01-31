#!/bin/bash

echo "Running full dataset collection. This may take a few hours..."

cd ../cache
mkdir -p codeforces ctf dance reddit topcoder synth-sm synth-la
cd ../scripts

#RUST_LOG=info cargo run --release --manifest-path=../multi-skill/Cargo.toml --bin summarize_dataset codeforces
#RUST_LOG=info cargo run --release --manifest-path=../multi-skill/Cargo.toml --bin summarize_dataset ctf
#RUST_LOG=info cargo run --release --manifest-path=../multi-skill/Cargo.toml --bin dance
#python3 python/mine-reddit.py
#python3 python/synth-data.py
python3 python/mine-topcoder.py

echo "Dataset collection complete."
