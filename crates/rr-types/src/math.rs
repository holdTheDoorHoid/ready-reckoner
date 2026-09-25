//! Deterministic floating-point functions for the engine.
//!
//! `f64::exp`, `f64::ln`, `f64::powf` and friends call the platform maths library. On native Linux
//! that is glibc; in WebAssembly it is Rust's port of musl (the `libm` crate). The two disagree in
//! the last bit for some inputs (`exp(1.0)` is one of them), so the same plan could print
//! differently in the CLI and in the browser. The functions here call the pure-Rust `libm` crate
//! on every target, so every engine build produces the same bits.
//!
//! Engine crates should use these instead of the `f64` methods for anything transcendental.
//! `+ - * /`, `sqrt`, `abs`, `min`, `max`, `floor` and `ceil` are exact in IEEE 754 and safe to
//! use directly.

use core::f64::consts::SQRT_2;

/// The 0.9 quantile of the standard normal distribution, z such that Φ(z) = 0.9
/// (1.2815515655446004669…; DESIGN and the brief round it to 1.2816).
pub const Z_90: f64 = 1.2815515655446004;

/// e raised to `x`.
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// `exp(x) - 1`, accurate for `x` near zero. Use `-exp_m1(-x)` for `1 - exp(-x)` when `x` is
/// small (for example the chance of at least one event in a Poisson process).
pub fn exp_m1(x: f64) -> f64 {
    libm::expm1(x)
}

/// Natural logarithm. `ln(0) = -inf`; negative input gives NaN.
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

/// `ln(1 + x)`, accurate for `x` near zero.
pub fn ln_1p(x: f64) -> f64 {
    libm::log1p(x)
}

/// `x` raised to `y`.
pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

/// Standard normal cumulative distribution Φ(z) = P(Z ≤ z), computed as `erfc(-z/√2) / 2` so the
/// lower tail keeps full relative precision. Φ(0) is exactly 0.5.
pub fn norm_cdf(z: f64) -> f64 {
    0.5 * libm::erfc(-z / SQRT_2)
}

/// Standard normal survival function 1 − Φ(z) = P(Z > z), computed directly so the upper tail keeps
/// full relative precision.
pub fn norm_sf(z: f64) -> f64 {
    0.5 * libm::erfc(z / SQRT_2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn exp_is_libm_on_every_target() {
        // libm's exp(1) is 0x4005BF0A8B14576A; glibc returns 0x4005BF0A8B145769. Pinning the bits
        // catches anyone swapping in `f64::exp`, which would make native and wasm output differ.
        assert_eq!(exp(1.0).to_bits(), 0x4005_BF0A_8B14_576A);
    }

    #[test]
    fn basic_values() {
        assert_eq!(exp(0.0), 1.0);
        assert_eq!(ln(1.0), 0.0);
        assert!(close(ln(10.0), core::f64::consts::LN_10, 1e-15));
        assert!(close(exp(ln(7.0)), 7.0, 1e-14));
        assert_eq!(ln(0.0), f64::NEG_INFINITY);
        assert!(ln(-1.0).is_nan());
        // expm1(x) = x + x²/2 + …, ln(1 + x) = x − x²/2 + …; at x = 1e-10 the x² term is 5e-21,
        // far below what `exp(x) - 1` could resolve.
        assert!(close(exp_m1(1e-10), 1.00000000005e-10, 1e-25));
        assert!(close(ln_1p(1e-10), 0.99999999995e-10, 1e-25));
        assert!(close(-exp_m1(-1e-12), 9.999999999995e-13, 1e-27));
        assert_eq!(pow(2.0, 10.0), 1024.0);
        assert!(close(pow(10.0, -2.0), 0.01, 1e-17));
    }

    #[test]
    fn normal_cdf_reference_values() {
        assert_eq!(norm_cdf(0.0), 0.5);
        assert_eq!(norm_sf(0.0), 0.5);
        // Quantiles to 16 digits (computed at 50-digit precision).
        assert!(close(norm_cdf(Z_90), 0.9, 1e-15));
        assert!(close(norm_cdf(1.6448536269514726), 0.95, 1e-15));
        assert!(close(norm_cdf(2.326347874040841), 0.99, 1e-15));
        assert!(close(norm_cdf(-Z_90), 0.1, 1e-15));
        assert!(close(norm_sf(Z_90), 0.1, 1e-15));
        // Deep tails keep relative precision: Φ(-10) = 7.619853024160526e-24.
        assert!(close(
            norm_cdf(-10.0) / 7.619_853_024_160_526e-24,
            1.0,
            1e-12
        ));
        assert!(close(norm_sf(10.0) / 7.619_853_024_160_526e-24, 1.0, 1e-12));
        assert_eq!(norm_cdf(f64::INFINITY), 1.0);
        assert_eq!(norm_cdf(f64::NEG_INFINITY), 0.0);
        assert!(norm_cdf(f64::NAN).is_nan());
    }

    #[test]
    fn normal_cdf_is_monotone_and_symmetric() {
        let mut prev = 0.0;
        for i in -4000..=4000 {
            let z = f64::from(i) * 0.002;
            let p = norm_cdf(z);
            assert!(p >= prev, "not monotone at {z}");
            assert!((0.0..=1.0).contains(&p));
            assert!(close(p + norm_sf(z), 1.0, 1e-15), "cdf + sf != 1 at {z}");
            assert!(close(p, norm_sf(-z), 1e-16), "not symmetric at {z}");
            prev = p;
        }
    }
}
