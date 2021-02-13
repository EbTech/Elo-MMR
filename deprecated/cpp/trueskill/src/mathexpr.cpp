 // for the sanity check
#include <iostream>     // std::cout, std::fixed
#include <iomanip>      // std::setprecision
#include <stdexcept>    // std::invalid_argument
#include <sstream>      // std::ostringstream

#include "ndtr.h"
#include "ndtri.h"
#include "pdf.h"

void mathexpr_sanity_check() {
  std::ostringstream strs;

  if (cdf(1.4) != 0.919243340766228934) {
    strs << cdf(1.4);
    std::string str = strs.str();
    throw std::invalid_argument("invalid output for cdf " + str);
  }

  if (pdf(1.4) != 0.149727465635744877) {
    strs << pdf(1.4);
    std::string str = strs.str();
    throw std::invalid_argument("invalid output for pdf " + str);
  }

  if (ppf(.4) != -0.253347103135799723) {
    strs << ppf(.4);
    std::string str = strs.str();
    throw std::invalid_argument("invalid output for cdf " + str);
  }
}
