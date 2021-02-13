#ifndef INC_TRUESKILL_H
#define INC_TRUESKILL_H

#include <map>
#include <vector>
#include <ostream>
#include <iostream>

class Player {
public:
  double mu;
  double sigma;
  int rank;
};

class Gaussian {
public:
  double pi;
  double tau;

  Gaussian() : pi(0.0), tau(0.0) {};

  void init_pi_tau(double pi, double tau);
  void init_mu_sigma(double mu, double sigma);
  double get_mu();
  double get_sigma();
  Gaussian* operator* (Gaussian* other) {
    Gaussian* gaussian = new Gaussian();
    gaussian->init_pi_tau(this->pi + other->pi, this->tau + other->tau);
    return gaussian;
  }
  Gaussian* operator/ (Gaussian* other) {
    Gaussian* gaussian = new Gaussian();
    gaussian->init_pi_tau(this->pi - other->pi, this->tau - other->tau);
    return gaussian;
  }
};

class Variable;

class Factor {
private:
  static int s_id;
public:
  std::vector<Variable*>* variables;
  int id;

  Factor() : id(s_id++) {}
  ~Factor();

  void set_variables(std::vector<Variable*>* variables);
};

struct FactorKeyMapper {
  bool operator()(const Factor& lhs, const Factor& rhs) const {
	return lhs.id < rhs.id;
  }
};

class Variable {
public:
  Gaussian* value;
  std::map<Factor, Gaussian*, FactorKeyMapper> factors;

  Variable() : value(new Gaussian) {};
  ~Variable();

  void attach_factor(Factor* factor);
  void update_message(Factor* factor, Gaussian* message);
  void update_value(Factor* factor, Gaussian* value);
  Gaussian* get_message(Factor* factor);
};

class PriorFactor : public Factor {
public:
  Gaussian* gaussian;

  PriorFactor(Variable* variable, Gaussian* gaussian) : gaussian(gaussian) {
    std::vector<Variable*>* variables = new std::vector<Variable*>;
    variables->push_back(variable);
    this->set_variables(variables);
  };
  ~PriorFactor();

  void start();
};

class LikelihoodFactor : public Factor {
public:
  Variable* mean;
  Variable* value;
  double variance;

  LikelihoodFactor(Variable* mean, Variable* value, double variance) {
	std::vector<Variable*>* variables = new std::vector<Variable*>;
    variables->push_back(mean);
    variables->push_back(value);
    this->set_variables(variables);

    this->mean = mean;
    this->value = value;
    this->variance = variance;
  }
  ~LikelihoodFactor();

  void update_value();
  void update_mean();
};

class SumFactor : public Factor {
private:
  void _internal_update(
    Variable* var,
    std::vector<Gaussian*> y,
    std::vector<Gaussian*> fy,
    std::vector<double>* a);
public:
  Variable* sum;
  std::vector<Variable*>* terms;
  std::vector<double>* coeffs;

  SumFactor(Variable* var, std::vector<Variable*>* terms, std::vector<double>* coeffs);
  ~SumFactor();

  void update_sum();
  void update_term(unsigned int index);
};

class TruncateFactor : public Factor {
public:
  Variable* variable;
  double epsilon;

  TruncateFactor(Variable* variable, double epsilon) : variable(variable), epsilon(epsilon) {}
  virtual ~TruncateFactor() {}

  virtual void update() {}
};

class TruncateFactorDraw : public TruncateFactor {
public:

  TruncateFactorDraw(Variable* variable, double epsilon) : TruncateFactor(variable, epsilon) {
	std::vector<Variable*>* variables = new std::vector<Variable*>;
    variables->push_back(variable);
    this->set_variables(variables);
  }
  ~TruncateFactorDraw();

  void update();
};

class TruncateFactorWin : public TruncateFactor {
public:

  TruncateFactorWin(Variable* variable, double epsilon) : TruncateFactor(variable, epsilon) {
	std::vector<Variable*>* variables = new std::vector<Variable*>;
    variables->push_back(variable);
    this->set_variables(variables);
  }
  ~TruncateFactorWin();

  void update();
};

class Constants {
public:
  double BETA;
  double EPSILON;
  double GAMMA;
  Constants();
};

class TrueSkill {
public:
  void adjust_players(std::vector<Player*> players);
};

void simple_example();

#endif /* INC_TRUESKILL_H */
