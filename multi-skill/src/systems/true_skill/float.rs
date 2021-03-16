#[allow(dead_code)]
mod f64_module {
    pub type MyFloat = f64;
    pub const ZERO: MyFloat = 0.;
    pub const TWO: MyFloat = 2.;
    pub use statrs::function::erf::erfc;
    pub use std::f64::consts::PI;
}

#[allow(dead_code)]
mod f128_module {
    pub type MyFloat = f128::f128;
    pub const ZERO: MyFloat = MyFloat::ZERO;
    pub const TWO: MyFloat = MyFloat::TWO;
    pub const PI: MyFloat = MyFloat::PI;
    pub fn erfc(a: MyFloat) -> MyFloat {
        unsafe { f128::ffi::erfcq_f(a) }
    }
}

// Choose between f64 and f128
pub use f64_module::*;
pub use num_traits::float::Float;
