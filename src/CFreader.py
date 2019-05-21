from bs4 import BeautifulSoup
import requests
import itertools

"""
#Does a topological sort of members' histories to determine the order in which
#contests were administered. In cases of ambiguity, contests are ordered numerically.
class ContestOrder:
    def __init__(self):
        self.adj = dict()
        self.in_deg = dict()
    
    def add_history(self, contest_history):
        for a, b in zip(contest_history, contest_history[1:]):
            self.adj.setdefault(a, set())
            if not b in self.adj[a]:
                self.adj.setdefault(b, set())
                self.adj[a].add(b)
                
                self.in_deg.setdefault(a, 0)
                self.in_deg.setdefault(b, 0)
                self.in_deg[b] += 1
    
    def consume(self):
        first = []
        for a, deg in self.in_deg.items():
            if deg == 0:
                first.append(a)
        
        contests = []
        while len(first) > 0:
            if len(first) > 1:
                list.sort(first, reverse=True)
                print(f"WARNING: Can't tell which of {first} comes first", flush=True)
            
            a = first.pop()
            contests.append(a)
            
            for b in self.adj[a]:
                self.in_deg[b] -= 1
                if self.in_deg[b] == 0:
                    first.append(b)
        
        return contests

def get_educational(contests):
    for contest in contests:
        if contest % 2 == 0:
            print(contest, flush=True)
        page_soup = get_soup(f"https://codeforces.com/contest/{contest}/standings")
        title = page_soup.find(attrs={"property": "og:title"})["content"]
        if title.find("ated") != -1:
            print(f"{contest} {title}", flush=True)

def get_rated_contests_deprecated(num_pages):
    contest_order = ContestOrder()
    
    for page in range(1, 1 + num_pages):
        page_soup = get_soup(f"https://codeforces.com/problemset/standings/page/{page}")
        
        for handle in generate_hrefs(page_soup, "/profile/"):
            handle_soup = get_soup(f"https://codeforces.com/contests/with/{handle}")
            
            contest_history = []
            for contest_num in generate_hrefs(handle_soup, "/contest/"):
                contest_history.append(int(contest_num))
            contest_history.reverse()
            contest_order.add_history(contest_history)
            print(f"Processed {handle:<20}{len(contest_order.in_deg)} rated contests found so far", flush=True)
    
    contests = contest_order.consume()
    print(f"The full list of contests is {contests}", flush=True)
    return contests
"""

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

def get_rated_contests(num_pages = 12):
    contests = []
    for page in range(1, 1 + num_pages):
            # Contests 541, 648, 649, 780, and 904 were only made available in Russian
            page_soup = get_soup(f"https://codeforces.com/contests/page/{page}?locale=ru")
            
            for contest in generate_hrefs(page_soup, "/contest/"):
                ratings_soup = get_soup(f"https://codeforces.com/contest/{contest}/ratings")
                participants = ratings_soup.find_all(lambda tag: tag.name == "tr" and tag.has_attr("id"))
                if len(participants) != 0:
                    contests.append(int(contest))
                    print(contest, flush=True)
    
    list.reverse(contests)
    print(f"The full list of {len(contests)} contests is {contests}", flush=True)
    return contests

def participant_info(participant):
    rank, handle = participant.find_all("td")[:2]
    return handle.a.text, int(rank.text)

def save_contest_standings(contests):
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
        
        with open(f"../standings/{contest}.txt","w+") as standings_file:
            standings_file.write(f"{len(standings)} {title}\n")
            for handle, rank in standings:
                standings_file.write(f"{handle} {rank} {tie_intervals[rank]}\n")
        print(f"Standings saved to {contest}.txt")

if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()

    parser.add_argument('--pages', type=int, default=12,
                        help='Number of pages of users whose histories to search.')

    args = parser.parse_args()
    
    from rated_contests import all_contests
    #contests = get_rated_contests(args.pages)
    #save_contest_standings(all_contests)

