extern crate overload;

use overload::overload;
use std::ops;

use statrs::function::erf::erfc;
use std::f64::consts::PI;
use std::f64::INFINITY;

const PREC: f64 = 1e-4;

#[derive(Clone, Debug)]
pub struct Gaussian {
    pub mu: f64,
    pub sigma: f64,
}

pub const ZERO: Gaussian = Gaussian { mu: 0., sigma: 0. };

pub const ONE: Gaussian = Gaussian {
    mu: 0.,
    sigma: INFINITY,
};

overload!((a: ?Gaussian) + (b: ?Gaussian) -> Gaussian {
    Gaussian {
        mu: a.mu + b.mu,
        sigma: a.sigma.hypot(b.sigma),
    }
});

overload!((a: &mut Gaussian) += (b: ?Gaussian) {
    a.mu += b.mu;
    a.sigma = a.sigma.hypot(b.sigma);
});

overload!((a: ?Gaussian) - (b: ?Gaussian) -> Gaussian {
    Gaussian {
        mu: a.mu - b.mu,
        sigma: a.sigma.hypot(b.sigma),
    }
});

overload!((a: &mut Gaussian) -= (b: ?Gaussian) {
    a.mu -= b.mu;
    a.sigma = a.sigma.hypot(b.sigma);
});

overload!(-(a: &mut Gaussian) -> Gaussian {
    Gaussian {
        mu: -a.mu,
        sigma: a.sigma,
    }
});

overload!((a: ?Gaussian) * (b: ?f64) -> Gaussian {
    Gaussian {
        mu: a.mu * b,
        sigma: a.sigma * b.abs(),
    }
});

overload!((a: &mut Gaussian) *= (b: ?f64) {
    a.mu *= b;
    a.sigma *= b.abs();
});

overload!((a: ?Gaussian) / (b: ?f64) -> Gaussian {
    Gaussian {
        mu: a.mu / b,
        sigma: a.sigma / b.abs(),
    }
});

overload!((a: &mut Gaussian) /= (b: ?f64) {
    a.mu /= b;
    a.sigma /= b.abs();
});

overload!((a: ?Gaussian) * (b: ?Gaussian) -> Gaussian {
    if a.sigma.is_infinite() {
        return b.clone();
    }
    if b.sigma.is_infinite() {
        return a.clone();
    }

    let ssigma1 = a.sigma.powi(2);
    let ssigma2 = b.sigma.powi(2);
    Gaussian {
        mu: (a.mu * ssigma2 + b.mu * ssigma1) / (ssigma1 + ssigma2),
        sigma: a.sigma * b.sigma / (ssigma1 + ssigma2).sqrt(),
    }
});

overload!((a: &mut Gaussian) *= (b: ?Gaussian) {
    *a = a.clone() * b;
});

overload!((a: ?Gaussian) / (b: ?Gaussian) -> Gaussian {
    if b.sigma.is_infinite() {
        return a.clone();
    }

    let ssigma1 = a.sigma.powi(2);
    let ssigma2 = b.sigma.powi(2);
    Gaussian {
        mu: (a.mu * ssigma2 - b.mu * ssigma1) / (ssigma2 - ssigma1),
        sigma: a.sigma * b.sigma / (ssigma2 - ssigma1).abs().sqrt(),
    }
});

overload!((a: &mut Gaussian) /= (b: ?Gaussian) {
    *a = a.clone() / b;
});

fn gauss_exponent(mu: f64, sigma: f64, t: f64) -> f64 {
    (-((t - mu) / sigma).powi(2)).exp()
}

fn moment0(mu: f64, sigma: f64, t: f64) -> f64 {
    sigma * PI.sqrt() / 2. * erfc((t - mu) / sigma)
}

fn moment1(mu: f64, sigma: f64, t: f64) -> f64 {
    mu * moment0(0., sigma, t - mu) + sigma.powi(2) / 2. * gauss_exponent(mu, sigma, t)
}

fn moment2(mu: f64, sigma: f64, t: f64) -> f64 {
    mu.powi(2) * moment0(0., sigma, t - mu)
        + 2. * mu * moment1(0., sigma, t - mu)
        + sigma.powi(2) / 4.
            * (2. * gauss_exponent(mu, sigma, t) * (t - mu)
                + sigma * PI.sqrt() * erfc((t - mu) / sigma))
}

impl Gaussian {
    pub fn leq_eps(&self, eps: f64) -> Gaussian {
        assert!(eps >= 0.);

        let alpha = moment0(self.mu, self.sigma, -eps) - moment0(self.mu, self.sigma, eps);

        if alpha < PREC {
            return Gaussian {
                mu: 0.,
                sigma: (1. / 3. as f64).sqrt(),
            } / self;
        }

        let mu =
            1. / alpha * (moment1(self.mu, self.sigma, -eps) - moment1(self.mu, self.sigma, eps));
        let mut sigma2 = 1. / alpha
            * (moment2(self.mu, self.sigma, -eps) - moment2(self.mu, self.sigma, eps))
            - mu.powi(2);
        if sigma2 < 0. {
            // sigma2 can only be < 0 due to numerical errors
            sigma2 = 0.;
        }
        let sigma = sigma2.sqrt();

        assert!(!mu.is_nan() && !sigma.is_nan(), "{:?}\teps {} {} {}", self, eps, mu, sigma2);

        let ans = Gaussian { mu, sigma } / self;

        assert!(ans.mu.abs() <= eps);

        ans
    }

    pub fn greater_eps(&self, eps: f64) -> Gaussian {
        assert!(eps >= 0.);

        let alpha = moment0(self.mu, self.sigma, eps);

        if alpha < PREC {
            return Gaussian {
                mu: eps,
                sigma: self.sigma / (2. as f64).sqrt(),
            } / self;
        }

        let mu = 1. / alpha * moment1(self.mu, self.sigma, eps);
        let mut sigma2 = 1. / alpha * moment2(self.mu, self.sigma, eps) - mu.powi(2);
        if sigma2 < 0. {
            // sigma2 can only be < 0 due to numerical errors
            sigma2 = 0.;
        }
        let sigma = sigma2.sqrt();

        assert!(!mu.is_nan() && !sigma.is_nan(), "{:?}\teps {}", self, eps);

        let ans = Gaussian { mu, sigma } / self;

        assert!(ans.mu >= 2. * eps);

        ans
    }
}
