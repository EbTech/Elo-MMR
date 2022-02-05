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
        offset = 7
        name = tokens[offset]
        rest = ' '.join(tokens[offset:])
       
        if name not in algs or args.dataset.lower() not in tokens[offset-1].lower():
            continue

        if name == 'EloMMR' and 'Gaussian' in rest:
            name = 'EloMMX'

        tokens = rest.split(':')
        params, results = ":".join(tokens[:-1]), tokens[-1]

        vals = extract_numbers(results)

        for i in range(6):
            metrics[metric_names[i]][name][params] = (vals[i], vals[-1])
        metrics['time'][name][params] = vals[-1]

    contest_source = args.dataset
    mu_noob = 1500
    sig_noob = 350

    # Iterate through our results and take the best
    for alg, shortname, expname in zip(algs, shortnames, expnames):
        print(f"Algorithm type {alg}:")
        for name in metric_names:
            if name == 'time':
                continue
            values = metrics[name][alg].values()
            if values:
                if 'pair' in name:
                    idx, best = max(enumerate(values), key=itemgetter(1))
                else:
                    idx, best = min(enumerate(values), key=itemgetter(1))

                best_params = list(metrics[name][alg])[idx]

                param_values = extract_numbers(best_params)
                print(f"  Best {name} (metric, time): {best} {best_params}")
