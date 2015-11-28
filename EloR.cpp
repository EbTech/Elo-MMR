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

const double sig_newbie = 450; // starting uncertainty for new player
const double sig_limit = 150; // limiting uncertainty for a player who competed a lot
const double sig_perf = 375; // variation in individual performances
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
    int conservativeRating() { return int(mu + 1*(sig_limit - sig)); } // displayed rating
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

/* don't use this
void add_noise_suffix(vector<Rating>& measurements)
{
    double sum_sigInvSq = 0;
    double sum_newsigInvSq = 0;
    for (auto it = measurements.rbegin(); it != measurements.rend(); ++it)
    {
        Rating& r = *it;
        sum_sigInvSq += 1.0 / r.sig / r.sig;
        r.sig = 1.0 / sqrt(1.0 / (1.0/sum_sigInvSq + sig_noise*sig_noise) - sum_newsigInvSq);
        r.tig = tfactor * r.sig;
        sum_newsigInvSq += 1.0 / r.sig / r.sig;
    }
}*/

/* don't use this either
vector<Rating> add_noise_deprecated(vector<Rating> measurements)
{
    vector<Rating> ret;
    double sum_sigInvSq = 0;
    double sum_mu_sigInvSq = 0;
    for (Rating& r : measurements)
    {
        sum_sigInvSq += 1.0 / r.sig / r.sig;
        sum_mu_sigInvSq += r.mu / r.sig / r.sig;
    }
    double sum_all_sigInvSq = sum_sigInvSq + 1.0 / sig_noise / sig_noise;
    ret.emplace_back(sum_mu_sigInvSq / sum_sigInvSq,
                    sum_all_sigInvSq * sig_noise / sum_sigInvSq);
    for (Rating& r : measurements)
    {
        ret.emplace_back(r.mu + sig_noise*sig_noise*(r.mu*sum_sigInvSq - sum_mu_sigInvSq),
                        sum_all_sigInvSq * sig_noise * sig_noise * r.sig);
    }
    return ret;
}*/

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

void CFtest()
{
    map<string, Player> players;

    for (int roundNum = 1; roundNum <= 220; ++roundNum)
    {
        stringstream ssFileName;
        ssFileName << "Standings/" << roundNum << ".txt";
        ifstream standingsFile;
        int N;
        standingsFile.open(ssFileName.str());
        standingsFile >> N;
        
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

// ratings is a list of the participants, ordered from first to last place
// returns: updated rating of the player in ratings[id]
// PROPORTIONAL UPDATE: all performances weighted equally
Rating update1(vector<Rating> ratings, int id, int lo, int hi)
{
    for (Rating& r : ratings)
        r.add_noise();
    double perf = performance(ratings, id, lo, hi);
    double varSkill = ratings[id].sig * ratings[id].sig;
    double varPerf = sig_perf * sig_perf;
    
    double mu_upd = (varSkill*perf + varPerf*ratings[id].mu) / (varSkill + varPerf);
    //double sig_upd = sqrt(varSkill*varPerf/(varSkill+varPerf) + sig_noise*sig_noise);
    double sig_upd = 1.0 / sqrt(1.0/varSkill + 1.0/varPerf);
    return Rating(mu_upd, sig_upd);
}

// ratings is a list of the participants, ordered from first to last place
// returns: updated rating of the player in ratings[id]
// SINGLE-STEP ROBUST UPDATE: reduces weight of extreme performances
// maximum rating change for an experienced player is about 87 = sqrt(3)/pi*s*(log(sig_perf+s)-log(sig_perf-s))
Rating update2(vector<Rating> ratings, int id, int lo, int hi)
{
    for (Rating& r : ratings)
        r.add_noise();
    double perf = performance(ratings, id, lo, hi);
    
    vector<Rating> posterior;
    posterior.emplace_back(ratings[id].mu, ratings[id].sig);
    posterior.emplace_back(perf, sig_perf);
    double mu_upd = robustMean(posterior);
    double sig_upd = 1.0 / sqrt(1.0/ratings[id].sig/ratings[id].sig + 1.0/sig_perf/sig_perf);
    return Rating(mu_upd, sig_upd);
}

// prior is the rating from R rounds ago, where R = performances.size()
// MULTI-STEP ROBUST UPDATE: reduces weight of extreme performances, dynamically adjusting the weights
Rating update3(Rating prior, vector<double> performances)
{
    vector<Rating> posterior;
    posterior.emplace_back(prior.mu, prior.sig);
    double varPosterior = prior.sig * prior.sig;
    for (double p : performances)
    {
        add_noise_uniform(posterior);
        varPosterior += sig_noise*sig_noise;
        
        posterior.emplace_back(p, sig_perf);
        varPosterior = 1.0 / (1.0/varPosterior + 1.0/sig_perf/sig_perf);
    }
    return Rating(robustMean(posterior), sqrt(varPosterior));
}

int main()
{
    CFtest(); return 0;
    // test of update3()
    vector<double> p = {1000,1000,1000,1000,1000,1000,1000,1000,1000,1000};
    cout << update3(Rating(), p) << endl; // after 10 matches, the rating converges to 1010
    p.push_back(3000);
    cout << update3(Rating(), p) << endl; // 1 strong performance only raises it to 1091
    p.push_back(3000);
    cout << update3(Rating(), p) << endl; // however, now the rating jumps to 1213
    p.push_back(3000);
    cout << update3(Rating(), p) << endl; // now it's 1410
    p.push_back(3000);
    cout << update3(Rating(), p) << endl; // now it's 1803
    p.push_back(3000);
    cout << update3(Rating(), p) << endl; // now it's 2467. at this point the rating is temporarily unstable
                                          // because the system is unsure about which results to believe:
                                          // depending on future performances, it may either quickly bounce
                                          // back to the 1000 range, or it may converge near 3000.
    p = {1000,1000,1000,1000,1000,1000,1000,1000,1000,1000,3000,-1000,-1000};
    cout << update3(Rating(), p) << endl; // 868. unlike on TopCoder, follow-up bad performances don't get
                                          // extra weight due to the good performance (nor vice-versa).
                                          // it's guaranteed you'll never wish to have done worse on a round.
    // main test
    map<string,Rating> ratings;
    ratings["tourist"] = Rating(3374, sig_limit);
    string line, name;
    cout << "On each line, output a space-separated list of distinct handles" << endl;
    cout << "representing the final standings of a contest" << endl;
    while (getline(cin, line) && !line.empty())
    {
        stringstream ss(line);
        vector<string> names;
        vector<Rating> preRound, postRound;
        while (ss >> name)
        {
            names.push_back(name);
            preRound.push_back(ratings[name]);
        }
        for (int i = 0; i < names.size(); ++i)
        {
            postRound.push_back(update1(preRound, i, i, i));
            //use the following rule if you want to bound rating change to experienced members:
            //postRound.push_back(update2(preRound, i, i, i));
        }
        for (int i = 0; i < names.size(); ++i)
        {
            ratings[names[i]] = postRound[i];
            cout << names[i] << ": " << preRound[i].conservativeRating();
            cout << "->" << postRound[i].conservativeRating() << " internal details: ";
            cout << preRound[i] << " -> " << postRound[i] << endl;
        }
    }
}
