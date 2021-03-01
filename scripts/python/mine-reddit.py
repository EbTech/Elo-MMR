#!/usr/bin/env python
# coding: utf-8

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
import praw

reddit = praw.Reddit(
    client_id="8MguWY6Vv2iwDw",  
    client_secret="GyXPBkZaNZn1GT8o_K3lNXqx8mU",
    user_agent="Chrome:EloR Miner:v0.1 (by u/inutard)")

# From a glance at the daily top threads, gaming seems to be most popular
# We'll mine from a specific subreddit to make sure theres some overlap in the user base
thread_scores = []
subreddit_name = "SubredditSimulator"
# Each thread of O(1000) comments takes about 3min to process
count = 0
for submission in reddit.subreddit("SubredditSimulator").top(limit=2000):
    if count % 100 == 0:
        print(submission.title, submission.num_comments, submission.score)
    count += 1
    submission.comments.replace_more(limit=None)
    scores = []
    seen = set()
    for comment in submission.comments:
        try:
            scores.append((comment.score, comment.author.name))
            seen.add(comment.author.name)
        except:
            # Likely deleted comments
            pass
        
        # Include second-level and third-level comments too
        for reply in comment.replies:
            try:
                if reply.author.name not in seen:
                    scores.append((reply.score, reply.author.name))
                    seen.add(reply.author.name)
            except:
                # Likely deleted comments
                pass
    thread_scores.append(scores)


import json
from collections import defaultdict

for tid, thread in enumerate(thread_scores):
    data = {}
    data['id'] = tid
    data['name'] = "To be filled."
    data['time_seconds'] = 0
    
    standings = []
    data['standings'] = standings
    
    names = set()
    # Remove duplicate comments by consolidating votes
    together = defaultdict(int)
    for user in thread:
        together[user[1]] += user[0]
    thread = [(x[1], x[0]) for x in together.items()]
    
    thread = sorted(thread, reverse=True)
    # Use -1 as placeholder value
    lscore, lo, hi = -1, -1, -1
    backlog = []
    for user in thread:
        if user[0] != lscore:
            for name in backlog:
                standings.append([name, lo, hi])
            lo = hi = hi + 1
            backlog = []
        else:
            hi += 1
        backlog.append(user[1])
        lscore = user[0]
        
    for name in backlog:
        standings.append([name, lo, hi])
    
    with open('../cache/reddit/' + str(tid) + '.json', 'w') as out:
        json.dump(data, out)


contest_ids = list(range(len(thread_scores)))
with open('../data/reddit/contest_ids.json', 'w') as out:
    out.write(str(contest_ids))