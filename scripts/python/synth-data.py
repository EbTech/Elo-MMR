#!/usr/bin/env python
# coding: utf-8

# In[3]:


import os
import numpy as np
import math
import json

from math import sqrt

cache_dir = "../cache/"

mu_noob = 1500
sig_noob = 350
sig_perf = 200
sig_noise = 35
logistic = True

def np_random_custom(mu, sig, size=None):
    if logistic:
        return np.random.logistic(mu, sig * sqrt(3) / math.pi, size)
    else:
        return np.random.normal(mu, sig, size)

def make_dataset(dataset_name, pool_players, num_players, num_rounds):
    synth_dir = cache_dir + dataset_name
    os.makedirs(synth_dir, exist_ok=True)
    skills = np.random.normal(mu_noob, sig_noob, pool_players)
    
    for idx in range(num_rounds):
        participants = np.random.choice(pool_players, num_players, replace=False)
        
        skills[participants] += np.random.normal(0, sig_noise, num_players)
        perfs = skills[participants] + np_random_custom(0, sig_perf, num_players)

        rankings = zip(perfs, participants)
        rankings = reversed(sorted(rankings))

        data = {}
        data['id'] = idx
        data['name'] = "Round #{}".format(idx)
        data['time_seconds'] = idx * 86400

        standings = []
        data['standings'] = standings
        for i, rank in enumerate(rankings):
            standings.append(["P{}".format(rank[1]), i, i])
        with open(f'{synth_dir}/{idx}.json', 'w') as out:
            json.dump(data, out)


# In[4]:


make_dataset("synth-sm", 1000, 5, 15000)
make_dataset("synth-la", 100000, 100000, 60)

