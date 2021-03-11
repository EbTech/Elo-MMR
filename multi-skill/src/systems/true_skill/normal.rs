use super::to_hp;
use rug::Float;
use std::ops::{Add, Deref, Div, Mul, Sub};

use std::f64::consts::PI;

#[derive(Clone, Debug)]
pub struct Gaussian {
    pub mu: Float,
    pub sigma: Float,
}

impl Add<&Gaussian> for &Gaussian {
    type Output = Gaussian;
    fn add(self, rhs: &Gaussian) -> Gaussian {
        Gaussian {
            mu: to_hp(&self.mu + &rhs.mu),
            sigma: to_hp(self.sigma.hypot_ref(&rhs.sigma)),
        }
    }
}

impl Sub<&Gaussian> for &Gaussian {
    type Output = Gaussian;
    fn sub(self, rhs: &Gaussian) -> Gaussian {
        Gaussian {
            mu: to_hp(&self.mu - &rhs.mu),
            sigma: to_hp(self.sigma.hypot_ref(&rhs.sigma)),
        }
    }
}

impl Mul<&Gaussian> for &Gaussian {
    type Output = Gaussian;
    fn mul(self, rhs: &Gaussian) -> Gaussian {
        if self.sigma.is_infinite() {
            return rhs.clone();
        }
        if rhs.sigma.is_infinite() {
            return self.clone();
        }

        let ssigma1 = self.sigma.clone().square();
        let ssigma2 = rhs.sigma.clone().square();
        let ss_total = to_hp(&ssigma1 + &ssigma2);
        Gaussian {
            mu: (&self.mu * ssigma2 + &rhs.mu * ssigma1) / &ss_total,
            sigma: &self.sigma / ss_total.sqrt() * &rhs.sigma,
        }
    }
}

impl Div<&Gaussian> for &Gaussian {
    type Output = Gaussian;
    fn div(self, rhs: &Gaussian) -> Gaussian {
        if rhs.sigma.is_infinite() {
            return self.clone();
        }
        if self.sigma.is_infinite() {
            return Gaussian {
                mu: -rhs.mu.clone(),
                sigma: rhs.sigma.clone(),
            };
        }

        let ssigma1 = self.sigma.clone().square();
        let ssigma2 = rhs.sigma.clone().square();
        let ss_diff = to_hp(&ssigma2 - &ssigma1).abs();

        Gaussian {
            mu: (&self.mu * ssigma2 - &rhs.mu * ssigma1) / &ss_diff,
            sigma: &self.sigma / ss_diff.sqrt() * &rhs.sigma,
        }
    }
}

fn gauss_exponent(mu: &Float, sigma: &Float, t: &Float) -> Float {
    let z = to_hp(t - mu) / sigma;
    (-z.square()).exp()
}

fn moment0(mu: &Float, sigma: &Float, t: &Float) -> Float {
    let z = to_hp(t - mu) / sigma;
    z.erfc() * sigma * (PI.sqrt() / 2.)
}

fn moment1(mu: &Float, sigma: &Float, t: &Float) -> Float {
    mu * moment0(&to_hp(0.), sigma, &to_hp(t - mu))
        + sigma.clone().square() / 2. * gauss_exponent(mu, sigma, t)
}

fn moment2(mu: &Float, sigma: &Float, t: &Float) -> Float {
    let t_minus_mu = to_hp(t - mu);
    mu.clone().square() * moment0(&to_hp(0.), sigma, &t_minus_mu)
        + moment1(&to_hp(0.), sigma, &t_minus_mu) * mu * 2.
        + sigma.clone().square() / 4.
            * (2. * gauss_exponent(mu, sigma, t) * &t_minus_mu
                + to_hp(&t_minus_mu / sigma).erfc() * sigma * PI.sqrt())
}

impl Gaussian {
    pub fn zero() -> Self {
        Self {
            mu: to_hp(0.),
            sigma: to_hp(0.),
        }
    }

    pub fn one() -> Self {
        Self {
            mu: to_hp(0.),
            sigma: to_hp(std::f64::INFINITY),
        }
    }

    pub fn leq_eps(&self, eps: &Float) -> Gaussian {
        assert!(!eps.is_sign_negative());
        assert!(!self.sigma.is_infinite());
        let neg_eps = to_hp(-eps);

        let alpha = moment0(&self.mu, &self.sigma, &neg_eps) - moment0(&self.mu, &self.sigma, &eps);

        let mut mu = (moment1(&self.mu, &self.sigma, &neg_eps) - moment1(&self.mu, &self.sigma, &eps))
            / &alpha;
        if alpha == 0 {
            mu = eps.clone();
        }

        let mut sigma2 = (moment2(&self.mu, &self.sigma, &neg_eps)
            - moment2(&self.mu, &self.sigma, &eps))
            / &alpha
            - mu.clone().square();
        // sigma2 can only be negative due to numerical errors
        if alpha == 0 {
            sigma2 = eps.clone().square();
        }
        let sigma = sigma2.max(&to_hp(0.)).sqrt();

        assert!(
            !mu.is_nan() && !sigma.is_nan(),
            "{:?}\teps {} {} {} {} {}",
            self,
            eps,
            neg_eps,
            mu,
            sigma,
            alpha
        );

        let ans = &Gaussian { mu, sigma } / self;

        assert!(ans.mu.as_abs().deref() <= eps);

        ans
    }

    pub fn greater_eps(&self, eps: &Float) -> Gaussian {
        assert!(!eps.is_sign_negative());
        assert!(!self.sigma.is_infinite());

        let alpha = moment0(&self.mu, &self.sigma, &eps);

        let mu = moment1(&self.mu, &self.sigma, &eps) / &alpha;
        let sigma2 = moment2(&self.mu, &self.sigma, &eps) / &alpha - mu.clone().square();
        // sigma2 can only be negative due to numerical errors
        let sigma = sigma2.max(&to_hp(0.)).sqrt();

        assert!(!mu.is_nan() && !sigma.is_nan(), "{:?}\teps {}", self, eps);

        let ans = &Gaussian { mu, sigma } / self;

        //assert!(ans.mu >= to_hp(2. * eps));

        ans
    }
}
