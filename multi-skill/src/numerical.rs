pub const TANH_MULTIPLIER: f64 = std::f64::consts::PI / 1.7320508075688772;

#[allow(dead_code)]
pub fn standard_logistic_pdf(z: f64) -> f64 {
    0.25 * TANH_MULTIPLIER * (0.5 * TANH_MULTIPLIER * z).cosh().powi(-2)
}

pub fn standard_logistic_cdf(z: f64) -> f64 {
    0.5 + 0.5 * (0.5 * TANH_MULTIPLIER * z).tanh()
}

#[allow(dead_code)]
pub fn standard_logistic_cdf_inv(prob: f64) -> f64 {
    (2. * prob - 1.).atanh() * 2. / TANH_MULTIPLIER
}

pub fn standard_normal_pdf(z: f64) -> f64 {
    const NORMALIZE: f64 = 0.5 * std::f64::consts::FRAC_2_SQRT_PI / std::f64::consts::SQRT_2;
    NORMALIZE * (-0.5 * z * z).exp()
}

pub fn standard_normal_cdf(z: f64) -> f64 {
    0.5 * statrs::function::erf::erfc(-z / std::f64::consts::SQRT_2)
    // Less numerically stable: 0.5 + 0.5 * statrs::function::erf::erf(z / std::f64::consts::SQRT_2)
}

pub fn standard_normal_cdf_inv(prob: f64) -> f64 {
    -std::f64::consts::SQRT_2 * statrs::function::erf::erfc_inv(2. * prob)
    // Equivalently: std::f64::consts::SQRT_2 * statrs::function::erf::erf_inv(2. * prob - 1.)
}

#[allow(dead_code)]
pub fn solve_bisection((mut lo, mut hi): (f64, f64), f: impl Fn(f64) -> f64) -> f64 {
    loop {
        let flo = f(lo);
        let guess = 0.5 * (lo + hi);
        if lo >= guess || guess >= hi {
            return guess;
        }
        if f(guess) * flo > 0. {
            lo = guess;
        } else {
            hi = guess;
        }
    }
}

#[allow(dead_code)]
pub fn solve_illinois((mut lo, mut hi): (f64, f64), f: impl Fn(f64) -> f64) -> f64 {
    let (mut flo, mut fhi, mut side) = (f(lo), f(hi), 0i8);
    loop {
        let guess = (flo * hi - fhi * lo) / (flo - fhi);
        if lo >= guess || guess >= hi {
            return 0.5 * (lo + hi);
        }
        let fguess = f(guess);
        if fguess * flo > 0. {
            lo = guess;
            flo = fguess;
            if side == -1 {
                fhi *= 0.5;
            }
            side = -1;
        } else if fguess * fhi > 0. {
            hi = guess;
            fhi = fguess;
            if side == 1 {
                flo *= 0.5;
            }
            side = 1;
        } else {
            return guess;
        }
    }
}

pub fn solve_newton((mut lo, mut hi): (f64, f64), f: impl Fn(f64) -> (f64, f64)) -> f64 {
    let mut guess = 0.5 * (lo + hi);
    loop {
        let (sum, sum_prime) = f(guess);
        let extrapolate = guess - sum / sum_prime;
        if extrapolate < guess {
            hi = guess;
            guess = extrapolate.clamp(hi - 0.75 * (hi - lo), hi);
        } else {
            lo = guess;
            guess = extrapolate.clamp(lo, lo + 0.75 * (hi - lo));
        }
        if lo >= guess || guess >= hi {
            if sum.abs() > 1e-10 {
                tracing::warn!(
                    "Possible failure to converge @ {}: s={}, s'={}",
                    guess,
                    sum,
                    sum_prime
                );
            }
            return guess;
        }
    }
}
