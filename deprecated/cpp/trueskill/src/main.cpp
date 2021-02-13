#include <iostream>
#include <fstream>
#include <unordered_map>

#include "mathexpr.h"
#include "trueskill.h"
#include "json.h"

using namespace std;
using namespace nlohmann;

int main() {
  //mathexpr_sanity_check();
  //simple_example();
  // Store the players' priors
  unordered_map<string, Player*> ratings;  

  string contest_dir = "../../../cache/codeforces/";
  const int max_contests = 150;
  double mu_noob = 1500, sig_noob = 300;

  TrueSkill ts;
  for (int cid = 0; cid < max_contests; cid++) {
  	cerr << "Processing contest #" << cid << endl;
  	
  	ifstream file(contest_dir + to_string(cid) + ".json");
  	json js;
  	file >> js;

  	// cout << js << endl;
  	
  	vector<Player*> players;
  	int rk = 1;
  	for (auto rank : js["standings"]) {
  	  string name = rank[0].get<string>();
  	  if (!ratings.count(name)) {
  	  	ratings[name] = new Player({mu_noob, sig_noob, rk});
  	  }
  	  players.push_back(ratings[name]);
  	  rk++;
  	}
  	ts.adjust_players(players);
  }

  for (auto kv : ratings) {
  	cout << kv.second->mu << endl;
  }
  return 0;
}
