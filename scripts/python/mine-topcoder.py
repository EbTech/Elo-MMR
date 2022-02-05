#!/usr/bin/env python

import requests
from bs4 import BeautifulSoup

topcoder_dir = cache_dir + "../cache/topcoder"
os.makedirs(topcoder_dir, exist_ok=True)

# Mine list of contests
index_link = "https://www.topcoder.com/tc?module=MatchList&sc=&sd=&nr=200&sr={}"
round_ids = []
for page_id in range(1, 999999, 200):
    url = index_link.format(page_id)
    html = requests.get(url)
    soup = BeautifulSoup(html.text, 'html.parser')
    results = soup.find_all("td", {"class": "value"})
    for result in results:
        ref = result.find('a', href=True)['href']
        round_id = int(ref.split('=')[-1])
        round_ids.append(round_id)
    if len(results) < 200:
        break

magic_link = "https://community.topcoder.com/stat?c=round_stats&rd={}&sm=1&em=100&nm=99&dn={}"
def get_round_html(round_id, div):
    url = magic_link.format(round_id, div)
    html = requests.get(url)
    return html.text
    
def reconstruct_rankings(round_id, div):    
    text = get_round_html(round_id, div)
    soup = BeautifulSoup(text, 'html.parser')
    # Lines with statText are the ones with ranks
    results = soup.find_all("td", {"class": "statText"})
    decoded_text = [res.get_text(strip=True) for res in results]
    # First 20 lines are junk
    decoded_text = decoded_text[19:]
    
    # Every 18 lines is a new participant
    def parse_rank(text):
        name = text[1]
        points = float(text[8])
        return (points, name)
    ranks = []
    i = 0
    while i + 18 <= len(decoded_text):
        if 'Room' in decoded_text[i]:
            i += 1
            continue
            
        try:
            ranks.append(parse_rank(decoded_text[i:i+18]))
        except:
            pass
        i += 18
        
    return sorted(ranks, reverse=True)

contests = []
for round_id in round_ids:
    for div in range(2):
        ranks = reconstruct_rankings(round_id, div+1)
        if ranks:
            contests.append(ranks)
    print('Completed round id', round_id)

import json
from collections import defaultdict

for r, contest in enumerate(reversed(contests)):
    data = {}
    data['id'] = r
    data['name'] = "To be filled."
    data['time_seconds'] = 0
    
    standings = []
    data['standings'] = standings
    i = 0
    while i < len(contest):
        j = 0
        while i + j < len(contest) and contest[i + j][0] == contest[i][0]:
            j += 1
        
        for k in range(j):
            standings.append([contest[i+k][1], i, i + j - 1])
        i += j
        
    ranking_file = open(f'{topcoder_dir}/{r}.json', 'w')
    json.dump(data, ranking_file)
    ranking_file.close()
    
#contest_ids = list(range(len(contests)))
#with open(f'{topcoder_dir}/contest_ids.json', 'w') as out:
#    out.write(str(contest_ids))
#    out.close()



