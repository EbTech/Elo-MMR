from bs4 import BeautifulSoup
import requests
import itertools

def get_soup(url):
    headers = {
        "User-Agent": "Pied Piper (www.arameb.com)",
        "From": "Aram Ebtekar"
    }
    
    response = requests.get(url, headers=headers)
    soup = BeautifulSoup(response.content, "html5lib")
    return soup

def generate_hrefs(soup, prefix):
    for href_tag in soup.find_all("a"):
        href = href_tag["href"]
        if href.startswith(prefix) and href.find("#") == -1:
            href_suffix = href[len(prefix):]
            if href_suffix.find("/") == -1:
                yield href_suffix

def get_rated_contests(num_pages):
    contests = []
    for page in range(1, 1 + num_pages):
            # Use ru because contests [541,648,649,780,904] were only made available in Russian
            page_soup = get_soup(f"https://codeforces.com/contests/page/{page}?locale=ru")
            
            for contest in generate_hrefs(page_soup, "/contest/"):
                ratings_soup = get_soup(f"https://codeforces.com/contest/{contest}/ratings")
                participants = ratings_soup.find_all(lambda tag: tag.name == "tr" and tag.has_attr("id"))
                
                # Check that there is at least one *rated* participant
                if len(participants) != 0:
                    contests.append(int(contest))
                    print(contest, flush=True)
    
    list.reverse(contests)
    print(f"The full list of {len(contests)} contests is {contests}", flush=True)
    return contests

def participant_info(participant):
    rank, handle = participant.find_all("td")[:2]
    return handle.a.text, int(rank.text)

def save_contest_standings(contests, directory):
    for contest in contests:
        standings = []
        tie_intervals = dict()
        
        for page in itertools.count(1):
            page_soup = get_soup(f"https://codeforces.com/contest/{contest}/ratings/page/{page}")
            participants = page_soup.find_all(lambda tag: tag.name == "tr" and tag.has_attr("id"))
            
            if page == 1:
                title = page_soup.find(attrs={"class": "title"}).a.text.strip()
            elif participant_info(participants[0]) == standings[100 * page - 200]:
                break
            
            for r, participant in enumerate(participants, len(standings) + 1):
                handle, rank = participant_info(participant)
                
                if len(standings) > 0 and standings[-1][1] == rank:
                    assert rank < r
                else:
                    assert rank == r
                
                standings.append((handle, rank))
                tie_intervals[rank] = r
        
        with open(f"{directory}/{contest}.txt", "w+") as standings_file:
            standings_file.write(f"{len(standings)} {title}\n")
            for handle, rank in standings:
                standings_file.write(f"{handle} {rank} {tie_intervals[rank]}\n")
        print(f"Standings saved to {contest}.txt")

def save_contests(contests, file):
    with open(file, "w+") as contests_file:
        contests_file.write(f"{len(contests)}\n")
        for contest in contests:
            contests_file.write(f"{contest}\n")
    print(f"List of contests saved to {file}")

def get_contests(file):
    contests_file = open(file, 'r')
    return [int(contest) for contest in contests_file][1:]

if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()

    parser.add_argument('--pages', type=int, default=1,
                        help='Number of pages of users whose histories to search.')

    args = parser.parse_args()
    
    # all_contests = get_contests("../data/all_contests.txt")
    contests = get_rated_contests(args.pages)
    save_contest_standings(contests, "../standings")

