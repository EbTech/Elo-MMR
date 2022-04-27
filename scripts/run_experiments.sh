#!/bin/bash

# This script should be run from the scripts directory
run_hparam_search() {
	echo "Running hparam search..."
	for dataset in codechef #dance ctf codeforces topcoder reddit synth-sm #synth-la
	do
		echo "Processing dataset ${dataset}..."
		export RUST_LOG=info
		cargo run --release --manifest-path=../multi-skill/Cargo.toml --bin hparam_search $dataset | tee log-$dataset.txt
		cat log-$dataset.txt | python3 python/parse-hparams.py --dataset $dataset --output_dir ../experiments
		#rm log-$dataset.txt
	done
}

echo "Do you wish to run the hparam search from scratch? (This may take several hours.)"
select yn in "Yes" "No"; do
	case $yn in
		Yes ) 
		run_hparam_search
		break;;
		No ) 
		echo "Skipping hparam search..."
		break;;
	esac
done

echo "Running experiments with existing hparams from ../experiments:"
RUST_LOG=info cargo run --release --manifest-path=../multi-skill/Cargo.toml --bin eval | tee results.txt

for dataset in codechef #dance ctf codeforces topcoder reddit synth-sm synth-la
do
	echo "Running optimal set of experiments for ${dataset}..."
	cat results.txt | python3 python/parse-results.py --dataset $dataset | tee results-$dataset.txt
done

echo "Experiment results are saved in results-*.txt"
