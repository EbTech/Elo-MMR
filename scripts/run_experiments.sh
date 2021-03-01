#!/bin/bash

# This script should be run from the scripts directory
echo "Running hparam search..."

for dataset in codeforces topcoder reddit #synth-sm synth-la
do
	echo "Processing dataset ${dataset}..."
	cargo run --release --manifest-path=../multi-skill/Cargo.toml --bin hparam_search $dataset | tee | python3 python/parse-hparams.py --dataset $dataset
done

# echo "Running experiments for optimal hparams..."
# cargo run --release --manifest-path=../multi-skill/Cargo.toml --bin eval | tee || python3 python/parse-hparams.py --save_configs false
