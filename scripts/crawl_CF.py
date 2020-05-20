from bs4 import BeautifulSoup
from pathlib import Path
from multiprocessing import Pool
import requests
import logging
import time
logging.basicConfig(level=logging.INFO, format="%(asctime)s -%(levelname)s - %(message)s")

def get_soup(url):
    headers = {
        "User-Agent": "Pied Piper (www.arameb.com)",
        "From": "Aram Ebtekar"
    }

    while True:
        try:
            response = requests.get(url, headers=headers)
            response.raise_for_status()
            break
        except Exception as err:
            logging.info("Retrying after ", err)
            time.sleep(10)
    
    return BeautifulSoup(response.content, "html5lib")

def participant_info(participant):
    rank, handle = participant.find_all("td")[:2]
    return handle.a.text, int(rank.text)

def crawler_thread(arg):
    num, url = arg
    participants = get_soup(url).find_all(lambda tag: tag.name == "tr" and tag.has_attr("id"))
    info = list(map(participant_info, participants))
    return num, info

def all_pages(soup, prefix, extra_suffix):
    urls = []
    for href_tag in soup.find_all(lambda tag: tag.name == "a" and tag.has_attr("href")):
        href = href_tag["href"]
        if href.startswith(prefix) and href.find("#") == -1:
            suffix = href[len(prefix):]
            if suffix.find("/") == -1:
                if suffix.startswith("1?"):
                    suffix = "1"
                urls.append((suffix, f"https://codeforces.com{prefix}{suffix}/{extra_suffix}"))

    with Pool(processes=50) as pool:
        return pool.map(crawler_thread, urls)

def get_rated_contests(num_pages):
    contests = []
    for page in range(1, 1 + num_pages):
            # Use ru because contests [541,648,649,780,904,1319] were only made available in Russian
            page_soup = get_soup(f"https://codeforces.com/contests/page/{page}?locale=ru")
            
            for contest, participants in all_pages(page_soup, "/contest/", "ratings"):
                # Check that there is at least one *rated* participant
                if len(participants) != 0:
                    contests.append(int(contest))
                    logging.info(contest)
    
    list.reverse(contests)
    logging.info(f"The full list of {len(contests)} contests is {contests}")
    return contests

def save_contest_standings(contests, directory):
    for contest in contests:
        standings = []
        tie_intervals = dict()

        page_soup = get_soup(f"https://codeforces.com/contest/{contest}/ratings/page/1")
        title = page_soup.find(attrs={"class": "title"}).a.text.strip()
        
        for page, participants in all_pages(page_soup, f"/contest/{contest}/ratings/page/", ""):
            for r, participant in enumerate(participants, len(standings) + 1):
                handle, rank = participant
                
                if len(standings) > 0 and standings[-1][1] == rank:
                    assert rank < r
                else:
                    assert rank == r
                
                standings.append((handle, rank))
                tie_intervals[rank] = r
        
        with open(directory / f"{contest}.txt", "w", encoding="utf-8") as standings_file:
            standings_file.write(f"{len(standings)} {title}\n")
            for handle, rank in standings:
                standings_file.write(f"{handle} {rank} {tie_intervals[rank]}\n")
        logging.info(f"Standings saved to {contest}.txt")

def save_cached_contests(contests, file):
    with open(file, "w") as contests_file:
        contests_file.write(f"{len(contests)}\n")
        for contest in contests:
            contests_file.write(f"{contest}\n")
    logging.info(f"List of contests saved to {file}")

def get_cached_contests(file):
    contests_file = open(file, 'r')
    return [int(contest) for contest in contests_file][1:]

if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()

    parser.add_argument('--pages', type=int, default=1,
                        help='Number of pages of users whose histories to search.')

    args = parser.parse_args()
    
    # contests = get_rated_contests(args.pages)
    contests = get_cached_contests(Path("..") / "data"/ "all_contests.txt")[-3:]
    # save_contest_standings(contests, Path("..") / "standings")
