from operator import itemgetter
from collections import defaultdict

import json
import os
import sys
import re
import argparse

def extract_numbers(line):
    res = re.findall(r'[^0-9](\d+\.*\d*)[^0-9]', line)
    return [float(x) for x in res]

if __name__ == "__main__": 
    parser = argparse.ArgumentParser()
    parser.add_argument("--output_dir", type=str, default="experiments", help="Output directory for the config files.")
    parser.add_argument("--dataset", type=str, default="codeforces", help="The name of the dataset being processed.")
    args = parser.parse_args()

    algs = ["CodeforcesSys", "EloMMR", "EloMMX", "TopcoderSys", "TrueSkillSPb", "Glicko", "BAR"]
    shortnames = ["cfsys", "mmr", "mmx", "tcsys", "trueskill", "glicko", "bar"]
    expnames = ["cfsys", "mmr-fast", "mmx-fast", "tcsys", "trueskill", "glicko", "bar"]
    metric_names = ['pair-all', 'pair-exp', 'pair-100', 'rank-all', \
            'rank-exp', 'rank-100', 'entropy-exp', 'num-contests', 'time']
    metrics = {}


    for name in metric_names:
        metrics[name] = defaultdict(dict)

    # Read log file from stdin and parse data contents
    for line in sys.stdin:
        tokens = line.strip().split(' ')
        if len(tokens) < 5:
            continue
        offset = 4
        name = tokens[offset]
        rest = ' '.join(tokens[offset:])
        
        if name not in algs:
            continue

        if name == 'EloMMR' and 'Gaussian' in rest:
            name = 'EloMMX'

        tokens = rest.split(':')
        params, results = ":".join(tokens[:-1]), tokens[-1]

        vals = extract_numbers(results)
        for i in range(6):
            metrics[metric_names[i]][name][params] = vals[i]
        metrics['time'][name][params] = vals[-2]

        # idx, best = min(enumerate(vals[6:-1]), key=itemgetter(1))
        metrics['num-contests'][name][params] = vals[-1]

    contest_source = args.dataset
    mu_noob = 1500
    sig_noob = 350

    # Write output directory
    output_dir = f"./{args.output_dir}/{contest_source}"
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)

    # Iterate through our results and take the best
    for alg, shortname, expname in zip(algs, shortnames, expnames):
        print("Algorithm type {}:".format(alg))
        for name in metric_names:
            values = metrics[name][alg].values()
            if values:
                if 'pair' in name:
                    idx, best = max(enumerate(values), key=itemgetter(1))
                else:
                    idx, best = min(enumerate(values), key=itemgetter(1))

                best_params = list(metrics[name][alg])[idx]

                param_values = extract_numbers(best_params)
                print("  Best {}: {} {}".format(name, best, best_params))

                system = {}
                # Some ugly casing of params depending on method
                if alg == 'BAR':
                    param_values = param_values[:-1]
                elif alg == 'EloMMX':
                    split_ties = int('split_ties: true' in best_params)
                    param_values = param_values[:2] + [split_ties] + param_values[3:6]
                elif alg == 'EloMMR':
                    split_ties = int('split_ties: true' in best_params)
                    param_values = param_values[:2] + [split_ties] + param_values[3:]
                    if 'Logistic(inf)' in best_params:
                        param_values += [float('inf')]
                    
                system["method"] = shortname
                system["params"] = param_values

                config = {
                        "max_contests": int(list(metrics['num-contests'][alg].values())[0]), 
                        "mu_noob": mu_noob, 
                        "sig_noob": sig_noob,
                        "contest_source": contest_source,
                        "system": system
                        }

                tracked_metrics = defaultdict(str)
                tracked_metrics.update({"pair-exp": "acc", "rank-exp": "rnk"})
                metric = tracked_metrics[name]
                if metric:
                    with open(f"{output_dir}/{expname}-{metric}.json", "w") as outfile:
                        json.dump(config, outfile)
