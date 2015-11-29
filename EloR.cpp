// author: Aram Ebtekar
// last updated: Nov 28, 2015
#include <iostream>
#include <fstream>
#include <cassert>
#include <cmath>
#include <sstream>
#include <string>
#include <vector>
#include <map>
#include <unordered_map>
#include <algorithm>
using namespace std;

const double sig_newbie = 400; // starting uncertainty for new player
const double sig_limit = 200; // limiting uncertainty for a player who competed a lot
const double sig_perf = 500; // variation in individual performances
//const double sig_noise = sig_limit*sig_limit / sqrt(sig_limit*sig_limit + sig_perf*sig_perf);
const double sig_noise = sqrt(1.0 / (1.0/sig_limit/sig_limit - 1.0/sig_perf/sig_perf) - sig_limit*sig_limit);
const double tfactor = sqrt(12.0)/M_PI;

struct Rating
{
    double mu; // mean of skill belief
    double sig; // uncertainty of skill belief
    double tig; // uncertainty converted into units suitable for the tanh function
    Rating(double m, double s) : mu(m), sig(s), tig(tfactor*s) {} // create player
    Rating() : Rating(1500, sig_newbie) {} // create new player
    int conservativeRating() { return int(mu + 2*(sig_limit - sig)); } // displayed rating
    void add_noise() // prepare a prior for the current match
    {
        double varSkill = sig*sig + sig_noise*sig_noise;
        sig = sqrt(varSkill);
        tig = tfactor * sqrt(varSkill + sig_perf*sig_perf);
    }
};

ostream& operator<<(ostream& os, const Rating& r)
{
    os << int(r.mu) << "+/-" << int(r.sig);
    return os;
}

// apply noise to one variable for which we have many estimates
void add_noise_uniform(vector<Rating>& measurements)
{
    double sum_sigInvSq = 0;
    for (Rating& r : measurements)
        sum_sigInvSq += 1.0 / r.sig / r.sig;
    double decay = sqrt(1.0 + sig_noise*sig_noise*sum_sigInvSq);
    for (Rating& r : measurements)
    {
        r.sig *= decay;
        r.tig *= decay;
    }
}

// returns something near the mean if the ratings are consistent; near the median if they're far apart
double robustMean(vector<Rating> ratings, double offset = 0.0)
{
    double lo = 0, hi = 4000;
    while (hi - lo > 1e-9)
    {
        double mid = (lo + hi) / 2;
        double sum = 0;
        for (Rating& r : ratings)
            sum += tanh((mid-r.mu)/r.tig) / r.tig;
        if (sum > offset)
            hi = mid;
        else
            lo = mid;
    }
    return (lo + hi) / 2;
}

// ratings is a list of the participants, ordered from first to last place
// returns: performance of the player in ratings[id] who tied against ratings[lo..hi]
double performance(vector<Rating> ratings, int id, int lo, int hi)
{
    int N = ratings.size();
    assert(0 <= lo && lo <= id && id <= hi && hi <= N-1);
    double offset = 0;
    for (int i = 0; i < lo; ++i)
        offset -= 1.0 / ratings[i].tig;
    for (int i = hi+1; i < N; ++i)
        offset += 1.0 / ratings[i].tig;
    ratings.push_back(ratings[id]);
    return robustMean(ratings, offset);
}

struct Player
{
    vector<Rating> perfs;
    Rating posterior;
    void updatePosterior()
    {
        double sum_sigInvSq = 0.0;
        for (Rating& p : perfs)
            sum_sigInvSq += 1.0 / p.sig / p.sig;
        posterior = Rating(robustMean(perfs), 1.0/sqrt(sum_sigInvSq));
    }
    Player()
    {
        perfs.push_back(posterior);
    }
};

void simulateCodeforcesHistory()
{
    map<string, Player> players;
    
    for (int roundNum = 1; roundNum <= 602; ++roundNum)
    {
        if (roundNum == 589 || roundNum == 598 || roundNum == 600)
            continue;
        stringstream ssFileName;
        ssFileName << "Standings/" << roundNum << ".txt";
        ifstream standingsFile;
        int N;
        standingsFile.open(ssFileName.str());
        standingsFile >> N;
        cerr << "Processing Codeforces round " << roundNum;
        cerr << " with " << N << " contestants..." << endl;
        
        vector<Rating> ratings(N);
        vector<string> names(N);
        vector<int> lo(N), hi(N);
        for (int i = 0; i < N; ++i)
        {
            standingsFile >> names[i] >> lo[i] >> hi[i];
            --lo[i]; --hi[i];
            ratings[i] = players[names[i]].posterior;
        }
        standingsFile.close();
        for (int i = 0; i < N; ++i)
        {
            Player& player = players[names[i]];
            add_noise_uniform(player.perfs);
            
            double perf = performance(ratings, i, lo[i], hi[i]);
            player.perfs.emplace_back(perf, sig_perf);
            
            player.updatePosterior();
        }
    }
    vector<pair<double,string> > conservativeRatings;
    for (pair<const string,Player>& player : players)
    {
        Rating& r = player.second.posterior;
        conservativeRatings.push_back(make_pair(r.conservativeRating(), player.first));
    }
    sort(conservativeRatings.begin(), conservativeRatings.end());
    reverse(conservativeRatings.begin(), conservativeRatings.end());
    for (pair<double,string>& entry: conservativeRatings)
    {
        cout << entry.first << ' ' << entry.second << endl;
    }
}

int main()
{
    simulateCodeforcesHistory(); // takes about 45 mins on my PC
}
