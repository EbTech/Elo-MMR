#include <math.h>

double pdf(double a) {
  static const double inv_sqrt_2pi = 0.3989422804014327;
  return inv_sqrt_2pi * exp(-0.5 * a * a);
}
