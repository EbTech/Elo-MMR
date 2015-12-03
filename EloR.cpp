// author: Aram Ebtekar
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

const double sig_limit = 100; // limiting uncertainty for a player who competed a lot
const double sig_perf = 250; // variation in individual performances
const double sig_noise = sqrt( 1.0 / (1.0/sig_limit/sig_limit - 1.0/sig_perf/sig_perf)
                               - sig_limit*sig_limit );
const double tfactor = sqrt(12.0)/M_PI;
// ignore this: 400*PI / (ln(10)*sqrt(3)) = sqrt(1 / (1/sig_limit^2 - 1/sig_perf^2) + sig_perf^2)

struct Rating
{
    double mu; // mean of skill belief
    double sig; // uncertainty of skill belief
    double tig; // uncertainty converted into units suitable for the tanh function
    Rating(double m, double s) : mu(m), sig(s), tig(tfactor*s) {} // create player
};

ostream& operator<<(ostream& os, const Rating& r)
{
    os << int(r.mu) << "+/-" << int(r.sig);
    return os;
}

// returns something near the mean if the ratings are consistent; near the median if they're far apart
// offC and offM are constant and slope offsets, respectively
double robustMean(const vector<Rating>& ratings, double offC = 0, double offM = 0)
{
    double lo = -1000, hi = 5000;
    while (hi - lo > 1e-9)
    {
        double mid = (lo + hi) / 2;
        double sum = offC + offM * mid;
        for (const Rating& r : ratings)
            sum += tanh((mid-r.mu)/r.tig) / r.tig;
        if (sum > 0)
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
        offset += 1.0 / ratings[i].tig;
    for (int i = hi+1; i < N; ++i)
        offset -= 1.0 / ratings[i].tig;
    ratings.push_back(ratings[id]);
    return robustMean(ratings, offset);
}

struct Player
{
    vector<Rating> perfs;
    Rating strongPrior; // future optimization: if perfs gets too long, merge results into strongPrior
    Rating posterior;
    int prevRating, maxRating;
    // apply noise to one variable for which we have many estimates
    void add_noise_uniform()
    {
        double decay = sqrt(1.0 + sig_noise*sig_noise/posterior.sig/posterior.sig);
        strongPrior.sig *= decay;
        strongPrior.tig *= decay;
        for (Rating& r : perfs)
        {
            r.sig *= decay;
            r.tig *= decay;
        }
    }
    void updatePosterior()
    {
        double tigInvSq = 1.0 / strongPrior.tig / strongPrior.tig;
        double mu = robustMean(perfs, -strongPrior.mu*tigInvSq, tigInvSq);
        double sigInvSq = 1.0 / strongPrior.sig / strongPrior.sig;
        for (const Rating& r : perfs)
            sigInvSq += 1.0 / r.sig / r.sig;
        posterior = Rating(mu, 1.0 / sqrt(sigInvSq));
    }
    int conservativeRating() const // displayed rating
    {
        return int(posterior.mu - 2*(posterior.sig - sig_limit) + 0.5);
    }
    Player() : maxRating(0), strongPrior(1500,350), posterior(1500,350) { }
};

void simulateCodeforcesHistory()
{
    map<string, Player> players;
    
    // 2011 ends at round 139, 2013 ends at round 379
    for (int roundNum = 1; roundNum <= 604; ++roundNum)
    {
        if (roundNum == 589 || roundNum == 598 || roundNum == 600)
            continue;
        
        // read the standings
        stringstream ssFileName;
        ssFileName << "Standings/" << roundNum << ".txt";
        ifstream standingsFile;
        standingsFile.open(ssFileName.str());
        int N;
        standingsFile >> N;
        cerr << "Processing Codeforces Round " << roundNum;
        cerr << " with " << N << " contestants..." << endl;
        
        vector<string> names(N);
        vector<int> lo(N), hi(N);
        vector<Rating> compRatings;
        compRatings.reserve(N);
        for (int i = 0; i < N; ++i)
        {
            standingsFile >> names[i] >> lo[i] >> hi[i];
            --lo[i]; --hi[i];
            
            Player& player = players[names[i]];
            Rating& r = player.posterior;
            double compVar = r.sig*r.sig + sig_noise*sig_noise + sig_perf*sig_perf;
            compRatings.emplace_back(r.mu, sqrt(compVar));
        }
        standingsFile.close();
        
        // begin rating updates
        for (int i = 0; i < N; ++i)
        {
            Player& player = players[names[i]];
            player.add_noise_uniform();
            
            double perf = performance(compRatings, i, lo[i], hi[i]);
            player.perfs.emplace_back(perf, sig_perf);
            
            player.prevRating = player.conservativeRating();
            player.updatePosterior();
            player.maxRating = max(player.maxRating, player.conservativeRating());
        }
        // end rating updates
    }
    // output the result
    double sumRatings = 0;
    vector<tuple<int,string,int,int,int> > conservativeRatings;
    for (pair<const string,Player>& entry : players)
    {
        Player& player = entry.second;
        Rating& r = player.posterior;
        conservativeRatings.push_back(make_tuple(player.conservativeRating(), entry.first,
            player.maxRating, player.prevRating, player.perfs.back().mu));
        sumRatings += r.mu;
    }
    cout << "Mean rating.mu = " << (sumRatings/players.size()) << endl;
    sort(conservativeRatings.begin(), conservativeRatings.end());
    reverse(conservativeRatings.begin(), conservativeRatings.end());
    for (tuple<int,string,int,int,int>& entry: conservativeRatings)
    {
        int delta = get<0>(entry) - get<3>(entry);
        cout << get<0>(entry) << "(" << get<2>(entry) << ") " << get<1>(entry);
        cout << "\t\t\t\tp=" << get<4>(entry);
        cout << "\tdelta=" << delta << endl;
    }
}

int main()
{
    simulateCodeforcesHistory(); // takes about 40 mins on my PC
}
