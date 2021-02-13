#include <algorithm> // std::sort
#include <map>
#include <vector>
#include <iomanip>
#include <iostream>
#include "mathexpr.h"
#include "trueskill.h"

int GMID = 0;

static double Vwin(double t, double e) {
  return pdf(t - e) / cdf(t - e);
}

static double Wwin(double t, double e) {
  double vwin = Vwin(t, e);
  return vwin * (vwin + t - e);
}

static double Vdraw(double t, double e) {
  return (pdf(-e - t) - pdf(e - t)) / (cdf(e - t) - cdf(-e - t));
}

static double Wdraw(double t, double e) {
  double vdraw = Vdraw(t, e);
  double n = (vdraw * vdraw) + ((e - t) * pdf(e - t) + (e + t) * pdf(e + t));
  double d = cdf(e - t) - cdf(-e - t);
  return n / d;
}

struct player_sorter {
  inline bool operator() (const Player* player1, const Player* player2) {
    return player1->rank < player2->rank;
  }
};

std::ostream& operator<<(std::ostream &strm, const Player &p) {
  strm << std::fixed;
  strm << "Player(mu=" << std::setprecision(3) << p.mu <<
    ";sigma=" << std::setprecision(3) << p.sigma <<
    ";rank=" << p.rank << ")";
  return strm;
}

void Gaussian::init_pi_tau(double pi, double tau) {
  this->pi = pi;
  this->tau = tau;
}

void Gaussian::init_mu_sigma(double mu, double sigma) {
  this->pi = 1.0 / (sigma * sigma);
  this->tau = this->pi * mu;
}

double Gaussian::get_mu() {
  if (this->pi == 0.0) {
    return 0.0;
  } else {
    return this->tau / this->pi;
  }
}

double Gaussian::get_sigma() {
  return sqrt(1.0 / this->pi);
}

std::ostream& operator<<(std::ostream &strm, const Gaussian &g) {
  double mu = 0.0;
  double sigma = 1.0 / 0.0;
  if (g.pi != 0.0) {
	sigma = sqrt(1.0 / g.pi);
	mu = g.tau / g.pi;
  }
  strm << std::fixed;
  strm << "N(mu=" << std::setprecision(3) << mu << ",sigma=" << std::setprecision(3) << sigma <<
		     ",pi=" << std::setprecision(3) << g.pi << ",tau=" << std::setprecision(3) << g.tau << ")";
  return strm;
}

Variable::~Variable() {
	delete this->value;
	for (std::map<Factor, Gaussian*>::iterator it = this->factors.begin(); it != this->factors.end(); ++it) {
		delete it->second;
	}
	this->factors.clear();
}

void Variable::attach_factor(Factor* factor) {
  Gaussian* gaussian = new Gaussian();
  this->factors[*factor] = gaussian;
}

void Variable::update_message(Factor* factor, Gaussian* message) {
  Gaussian* old = this->factors[*factor];

  Gaussian* intermediate = *this->value / old;
  Gaussian* value = *intermediate * message;
  delete intermediate;
  delete this->value;
  this->value = value;

  delete this->factors[*factor];
  this->factors[*factor] = message;
}

void Variable::update_value(Factor* factor, Gaussian* value) {
  Gaussian* old = this->factors[*factor];

  Gaussian* intermediate = *value * old;
  this->factors[*factor] = *intermediate / this->value;
  delete intermediate;
  delete old;

  delete this->value;
  this->value = value;
}

Gaussian* Variable::get_message(Factor* factor) {
//#if DEBUG
//  std::cout << "gm[" << GMID++ << "]=" << *this->factors[*factor] << std::endl;
//#endif
  return this->factors[*factor];
}

std::ostream& operator<<(std::ostream &strm, const Variable &v) {
  return strm << "Variable(" << *v.value << ")";
}

std::ostream& operator<<(std::ostream &strm, const std::vector<Variable*> &v) {
  for(std::vector<Variable*>::const_iterator it = v.begin(); it != v.end(); ++it) {
    if ((it != v.end()) && (++(v.begin() = it) == v.end())) {
      // last item
      strm << **it;
    } else {
      strm << **it << ",";
    }
  }
  return strm;
}

Factor::~Factor() {}

void Factor::set_variables(std::vector<Variable*>* variables) {
  this->variables = variables;
  for(std::vector<Variable*>::iterator it = variables->begin(); it != variables->end(); ++it) {
    (*it)->attach_factor(this);
  }
}

int Factor::s_id;

std::ostream& operator<<(std::ostream &strm, const Factor &f) {
  strm << "<-Factor([" << *f.variables << "])";
  return strm;
}

PriorFactor::~PriorFactor() {
  while(!this->variables->empty()) {
    this->variables->pop_back();
  }
  delete this->variables;
}

void PriorFactor::start() {
  (*this->variables)[0]->update_value(this, this->gaussian);
}

std::ostream& operator<<(std::ostream &strm, const PriorFactor &f) {
  strm << static_cast<const Factor &>(f) << "<-Prior(" << *f.gaussian << ")";
  return strm;
}

LikelihoodFactor::~LikelihoodFactor() {
  while(!this->variables->empty()) {
	this->variables->pop_back();
  }
  delete this->variables;
}

void LikelihoodFactor::update_value() {
  Gaussian y = *this->mean->value;
  Gaussian fy = *this->mean->get_message(this);
  double a = 1.0 / (1.0 + this->variance * (y.pi - fy.pi));

  Gaussian* gaussian = new Gaussian();
  gaussian->init_pi_tau(
    a * (y.pi - fy.pi),
    a * (y.tau - fy.tau)
  );

  this->value->update_message(this, gaussian);
}

void LikelihoodFactor::update_mean() {
  Gaussian x = *this->value->value;
  Gaussian fx = *this->value->get_message(this);
  double a = 1.0 / (1.0 + this->variance * (x.pi - fx.pi));

  Gaussian* gaussian = new Gaussian();
  gaussian->init_pi_tau(
    a * (x.pi - fx.pi),
    a * (x.tau - fx.tau)
  );
  this->mean->update_message(this, gaussian);
}

std::ostream& operator<<(std::ostream &strm, const LikelihoodFactor &f) {
  strm << static_cast<const Factor &>(f) << "<-Likelihood(" << *f.mean << "," << *f.value << "," << f.variance << ")";
  return strm;
}

SumFactor::~SumFactor() {
  while(!this->terms->empty()) {
	this->terms->pop_back();
  }
  delete this->terms;

  while(!this->coeffs->empty()) {
	this->coeffs->pop_back();
  }
  delete this->coeffs;

  while(!this->variables->empty()) {
    this->variables->pop_back();
  }
  delete this->variables;
}

SumFactor::SumFactor(Variable* sum, std::vector<Variable*>* terms, std::vector<double>* coeffs) {
  std::vector<Variable*>* variables = new std::vector<Variable*>;
  variables->push_back(sum);

  for(std::vector<Variable*>::iterator it = terms->begin(); it != terms->end(); ++it) {
    variables->push_back(*it);
  }

  this->set_variables(variables);

  this->sum = sum;
  this->terms = terms;
  this->coeffs = coeffs;
}

void SumFactor::_internal_update(
  Variable* var,
  std::vector<Gaussian*> y,
  std::vector<Gaussian*> fy,
  std::vector<double>* a) {

  double sum_pi = 0.0, sum_tau = 0.0, new_pi, new_tau, da;
  unsigned int i = 0, size = a->size();
  Gaussian gy, gfy;

  for (i = 0; i < size; ++i) {
    da = (*a)[i];
    gy = *y[i];
    gfy = *fy[i];

    sum_pi = sum_pi + ((da * da) / (gy.pi - gfy.pi));
    sum_tau = sum_tau + (da * (gy.tau - gfy.tau) / (gy.pi - gfy.pi));
  }

  new_pi = 1.0 / sum_pi;
  new_tau = new_pi * sum_tau;

  Gaussian* gaussian = new Gaussian();
  gaussian->init_pi_tau(new_pi, new_tau);
  var->update_message(this, gaussian);
}

void SumFactor::update_sum() {
  std::vector<Gaussian*>y;
  for(std::vector<Variable*>::iterator it = this->terms->begin(); it != this->terms->end(); ++it) {
    y.push_back((*it)->value);
  }

  std::vector<Gaussian*> fy;
  for(std::vector<Variable*>::iterator it = this->terms->begin(); it != this->terms->end(); ++it) {
    fy.push_back((*it)->get_message(this));
  }
  this->_internal_update(this->sum, y, fy, this->coeffs);
}

void SumFactor::update_term(unsigned int index) {
  unsigned int i = 0, size = this->coeffs->size();
  double idxcoeff = (*this->coeffs)[index];
  std::vector<double> a(size);

  for (i = 0; i < size; ++i) {
	if (i != index) {
      a[i] = -(*this->coeffs)[i] / idxcoeff;
    }
  }
  a[index] = 1.0 / idxcoeff;

  Variable* idxterm = (*this->terms)[index];
  std::vector<Gaussian*> y;
  std::vector<Gaussian*> fy;

  std::vector<Variable*> v = *this->terms;
  v[index] = this->sum;
  for(std::vector<Variable*>::iterator it = v.begin(); it != v.end(); ++it) {
	y.push_back((*it)->value);
    fy.push_back((*it)->get_message(this));
  }
  this->_internal_update(idxterm, y, fy, &a);
}

std::ostream& operator<<(std::ostream &strm, const SumFactor &f) {
  strm << static_cast<const Factor &>(f) << "<-Sum(" << *f.sum << ",[" << *f.terms << "],[";

  for(std::vector<double>::iterator it = f.coeffs->begin(); it != f.coeffs->end(); ++it) {
    if ((it != f.coeffs->end()) && (++(f.coeffs->begin() = it) == f.coeffs->end())) {
      // last item
      strm << (int)*it;
    } else {
      strm << (int)*it << ",";
    }
  }

  strm << "])";
  return strm;
}

TruncateFactorWin::~TruncateFactorWin() {
  while(!this->variables->empty()) {
	this->variables->pop_back();
  }
  delete this->variables;
}

TruncateFactorDraw::~TruncateFactorDraw() {
  while(!this->variables->empty()) {
	this->variables->pop_back();
  }
  delete this->variables;
}

void TruncateFactorWin::update() {
  Gaussian* x = this->variable->value;
  Gaussian* fx = this->variable->get_message(this);

  double c, d, sqrt_c, V, W, t, e, mW;
  c = x->pi - fx->pi;
  d = x->tau - fx->tau;
  sqrt_c = sqrt(c);

  t = d / sqrt_c;
  e = this->epsilon * sqrt_c;

  V = Vwin(t, e);
  W = Wwin(t, e);
  mW = 1.0 - W;

  Gaussian* gaussian = new Gaussian();
  gaussian->init_pi_tau(c / mW, (d + sqrt_c * V) / mW);
  this->variable->update_value(this, gaussian);
}

void TruncateFactorDraw::update() {
  Gaussian* x = this->variable->value;
  Gaussian* fx = this->variable->get_message(this);

  double c, d, sqrt_c, V, W, t, e, mW;
  c = x->pi - fx->pi;
  d = x->tau = fx->tau;
  sqrt_c = sqrt(c);

  t = d / sqrt_c;
  e = epsilon * sqrt_c;

  V = Vdraw(t, e);
  W = Wdraw(t, e);
  mW = 1.0 - W;

  Gaussian* gaussian = new Gaussian();
  gaussian->init_pi_tau(c / mW, (d + sqrt_c * V) / mW);
  this->variable->update_value(this, gaussian);
}

std::ostream& operator<<(std::ostream &strm, const TruncateFactor &f) {
  strm << static_cast<const Factor &>(f) << "<-TruncateFactor(" << *f.variable << "," << f.epsilon << ")";
  return strm;
}

double draw_margin(double p, double beta, double total_players) {
  return icdf((p + 1.0) / 2) * sqrt(total_players) * beta;
}

Constants::Constants() {
  double INITIAL_MU = 25.0;
  double INITIAL_SIGMA = INITIAL_MU / 3.0;
  double TOTAL_PLAYERS = 2.0;

  this->BETA = INITIAL_SIGMA / 2.0;
  this->EPSILON = draw_margin(0.1, this->BETA, TOTAL_PLAYERS);
  this->GAMMA = INITIAL_SIGMA / 100.0;
}

void TrueSkill::adjust_players(std::vector<Player*> players) {
  Constants constants;

  std::sort(players.begin(), players.end(), player_sorter());

  std::vector<Variable*> ss, ps, ts, ds;
  unsigned int i = 0, size = players.size();
  double gammasqr = constants.GAMMA * constants.GAMMA,
    betasqr = constants.BETA * constants.BETA;

  for (i = 0; i < size; ++i) {
    ss.push_back(new Variable());
    ps.push_back(new Variable());
    ts.push_back(new Variable());
  }

  for (i = 0; i < size - 1; ++i) {
    ds.push_back(new Variable());
  }

  std::vector<PriorFactor*> skill;
  for (i = 0; i < size; ++i) {
	Player* pl = players[i];
	Variable* s = ss[i];
    Gaussian* gaussian = new Gaussian();
    gaussian->init_mu_sigma(pl->mu, sqrt((pl->sigma * pl->sigma) + gammasqr));
    skill.push_back(new PriorFactor(s, gaussian));
  }

  std::vector<LikelihoodFactor*> skill_to_perf;
  for (i = 0; i < size; ++i) {
	Variable* s = ss[i];
	Variable* p = ps[i];
    skill_to_perf.push_back(new LikelihoodFactor(s, p, betasqr));
  }

  std::vector<SumFactor*> perf_to_team;
  for (i = 0; i < size; ++i) {
    std::vector<Variable*>* p = new std::vector<Variable*>();
    std::vector<double>* c = new std::vector<double>;
    p->push_back(ps[i]);
    c->push_back(1.0);
    perf_to_team.push_back(new SumFactor(ts[i], p, c));
  }

  std::vector<SumFactor*> team_diff;
  for (i = 0; i < size - 1; ++i) {
    std::vector<Variable*>* p = new std::vector<Variable*>();
    p->push_back(ts[i]);
    p->push_back(ts[i + 1]);
    std::vector<double>* c = new std::vector<double>;
    c->push_back(1.0);
    c->push_back(-1.0);
    team_diff.push_back(new SumFactor(ds[i], p, c));
  }

  std::vector<TruncateFactor*> trunc;
  for (i = 0; i < size - 1; ++i) {
    TruncateFactor* tf;
    if (players[i]->rank == players[i + 1]->rank) {
      tf = new TruncateFactorDraw(ds[i], constants.EPSILON);
    } else {
      tf = new TruncateFactorWin(ds[i], constants.EPSILON);
    }
    trunc.push_back(tf);
  }

  for(std::vector<PriorFactor*>::iterator it = skill.begin(); it != skill.end(); ++it) {
    (*it)->start();
  }

  for(std::vector<LikelihoodFactor*>::iterator it = skill_to_perf.begin(); it != skill_to_perf.end(); ++it) {
	(*it)->update_value();
  }

  for(std::vector<SumFactor*>::iterator it = perf_to_team.begin(); it != perf_to_team.end(); ++it) {
	(*it)->update_sum();
  }

  for (i = 0; i < 5; ++i) {
    for(std::vector<SumFactor*>::iterator it = team_diff.begin(); it != team_diff.end(); ++it) {
      (*it)->update_sum();
    }

    for(std::vector<TruncateFactor*>::iterator it = trunc.begin(); it != trunc.end(); ++it) {
      (*it)->update();
    }

    for(std::vector<SumFactor*>::iterator it = team_diff.begin(); it != team_diff.end(); ++it) {
	  (*it)->update_term(0);
	  (*it)->update_term(1);
    }
  }

  for(std::vector<SumFactor*>::iterator it = perf_to_team.begin(); it != perf_to_team.end(); ++it) {
	(*it)->update_term(0);
  }

  for(std::vector<LikelihoodFactor*>::iterator it = skill_to_perf.begin(); it != skill_to_perf.end(); ++it) {
	(*it)->update_mean();
  }

  for (i = 0; i < size; ++i) {
    players[i]->mu = ss[i]->value->get_mu();
    players[i]->sigma = ss[i]->value->get_sigma();
  }

  while(!skill.empty()) {
	delete skill.back();
	skill.pop_back();
  }

  while(!skill_to_perf.empty()) {
	delete skill_to_perf.back();
	skill_to_perf.pop_back();
  }

  while(!perf_to_team.empty()) {
	delete perf_to_team.back();
	perf_to_team.pop_back();
  }

  while(!team_diff.empty()) {
	delete team_diff.back();
	team_diff.pop_back();
  }

  while(!trunc.empty()) {
	delete trunc.back();
	trunc.pop_back();
  }

  while(!ss.empty()) {
	delete ss.back();
	ss.pop_back();
  }

  while(!ts.empty()) {
	delete ts.back();
	ts.pop_back();
  }

  while(!ds.empty()) {
	delete ds.back();
	ds.pop_back();
  }

  while(!ps.empty()) {
	delete ps.back();
	ps.pop_back();
  }
}

void simple_example() {
  Player* alice = new Player();
  alice->mu = 25.0;
  alice->sigma = 25.0 / 3.0;

  Player* bob = new Player();
  bob->mu = 25.0;
  bob->sigma = 25.0 / 3.0;

  Player* chris = new Player();
  chris->mu = 25.0;
  chris->sigma = 25.0 / 3.0;

  Player* darren = new Player();
  darren->mu = 25.0;
  darren->sigma = 25.0 / 3.0;

  std::vector<Player*> players;
  players.push_back(alice);
  players.push_back(bob);
  players.push_back(chris);
  players.push_back(darren);

  // set player ranks for the match
  alice->rank = 1;
  bob->rank = 2;
  chris->rank = 3;
  darren->rank = 4;

  // Do the computation to find each player's new skill estimate.

  TrueSkill ts;
  ts.adjust_players(players);

  // Print the results.

  std::cout << " Alice: " << *alice << std::endl;
  std::cout << "   Bob: " << *bob << std::endl;
  std::cout << " Chris: " << *chris << std::endl;
  std::cout << "Darren: " << *darren << std::endl;

  delete alice;
  delete bob;
  delete chris;
  delete darren;
  players.clear();
}
