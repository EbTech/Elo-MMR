use super::{erfc, Float, MyFloat, PI, TWO, ZERO};
use overload::overload;
use std::ops;

#[derive(Clone, Debug)]
pub struct Gaussian {
    pub mu: MyFloat,
    pub sigma: MyFloat,
}

pub const G_ZERO: Gaussian = Gaussian {
    mu: ZERO,
    sigma: ZERO,
};

pub const G_ONE: Gaussian = Gaussian {
    mu: ZERO,
    sigma: MyFloat::INFINITY,
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

overload!((a: ?Gaussian) * (b: ?MyFloat) -> Gaussian {
    Gaussian {
        mu: a.mu * b,
        sigma: a.sigma * b.abs(),
    }
});

overload!((a: &mut Gaussian) *= (b: ?MyFloat) {
    a.mu *= b;
    a.sigma *= b.abs();
});

overload!((a: ?Gaussian) / (b: ?MyFloat) -> Gaussian {
    Gaussian {
        mu: a.mu / b,
        sigma: a.sigma / b.abs(),
    }
});

overload!((a: &mut Gaussian) /= (b: ?MyFloat) {
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
    if a.sigma.is_infinite() {
        return Gaussian {
            mu: -b.mu,
            sigma: b.sigma,
        }
    }
    let ssigma1 = a.sigma.powi(2);
    let ssigma2 = b.sigma.powi(2);

    Gaussian {
        mu: (a.mu * ssigma2 - b.mu * ssigma1) / (ssigma2 - ssigma1).abs(),
        sigma: a.sigma * b.sigma / (ssigma2 - ssigma1).abs().sqrt(),
    }
});

overload!((a: &mut Gaussian) /= (b: ?Gaussian) {
    *a = a.clone() / b;
});

fn gauss_exponent(mu: MyFloat, sigma: MyFloat, t: MyFloat) -> MyFloat {
    (-((t - mu) / sigma).powi(2)).exp()
}

fn moment0(mu: MyFloat, sigma: MyFloat, t: MyFloat) -> MyFloat {
    sigma * PI.sqrt() / TWO * erfc((t - mu) / sigma)
}

fn moment1(mu: MyFloat, sigma: MyFloat, t: MyFloat) -> MyFloat {
    mu * moment0(ZERO, sigma, t - mu) + sigma.powi(2) / TWO * gauss_exponent(mu, sigma, t)
}

fn moment2(mu: MyFloat, sigma: MyFloat, t: MyFloat) -> MyFloat {
    mu.powi(2) * moment0(ZERO, sigma, t - mu)
        + TWO * mu * moment1(ZERO, sigma, t - mu)
        + (sigma / TWO).powi(2)
            * (TWO * gauss_exponent(mu, sigma, t) * (t - mu)
                + sigma * PI.sqrt() * erfc((t - mu) / sigma))
}

impl Gaussian {
    pub fn leq_eps(&self, eps: MyFloat) -> Gaussian {
        assert!(eps >= ZERO);
        assert!(!self.sigma.is_infinite());

        let alpha = moment0(self.mu, self.sigma, -eps) - moment0(self.mu, self.sigma, eps);

        const FLOAT_CMP_EPS: f64 = 1e-8;
        let (mu, sigma) = if alpha < FLOAT_CMP_EPS.into() {
            (eps, eps)
        } else {
            let mu =
                (moment1(self.mu, self.sigma, -eps) - moment1(self.mu, self.sigma, eps)) / alpha;
            let sigma2 = (moment2(self.mu, self.sigma, -eps) - moment2(self.mu, self.sigma, eps))
                / alpha
                - mu.powi(2);
            // sigma2 can only be < 0 due to numerical errors
            (mu, sigma2.max(ZERO).sqrt())
        };

        assert!(
            !mu.is_nan() && !sigma.is_nan(),
            "{:?}\teps {} {} {}",
            self,
            eps,
            mu,
            sigma
        );

        Gaussian { mu, sigma } / self
    }

    pub fn greater_eps(&self, eps: MyFloat) -> Gaussian {
        assert!(eps >= ZERO);
        assert!(!self.sigma.is_infinite());

        let alpha = moment0(self.mu, self.sigma, eps);

        let mu = moment1(self.mu, self.sigma, eps) / alpha;
        let sigma2 = moment2(self.mu, self.sigma, eps) / alpha - mu.powi(2);
        // sigma2 can only be < 0 due to numerical errors
        let sigma = sigma2.max(ZERO).sqrt();

        assert!(!mu.is_nan() && !sigma.is_nan(), "{:?}\teps {}", self, eps);

        Gaussian { mu, sigma } / self
    }
}
