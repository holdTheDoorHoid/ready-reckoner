//! Deterministic number formatting for pack files.
//!
//! Every non-integer value written to a pack is rounded to four significant figures and printed
//! in the shortest decimal form that round-trips (no exponent), so the same input always yields
//! the same bytes on every platform. Rust's float formatting is exact (correctly rounded), which
//! is what makes this reproducible.

/// Round `x` to four significant figures and print it in plain decimal notation.
///
/// Non-finite values print as an empty string (a missing cell). Negative zero prints as `0`.
///
/// ```
/// use rr_etl::num::sig4;
/// assert_eq!(sig4(0.000091476), "0.00009148");
/// assert_eq!(sig4(1603797.0), "1604000");
/// assert_eq!(sig4(5.821428571428571), "5.821");
/// assert_eq!(sig4(0.0), "0");
/// assert_eq!(sig4(f64::NAN), "");
/// ```
pub fn sig4(x: f64) -> String {
    sig(x, 4)
}

/// Round `x` to `digits` significant figures (1..=17) and print it in plain decimal notation.
pub fn sig(x: f64, digits: usize) -> String {
    if !x.is_finite() {
        return String::new();
    }
    if x == 0.0 {
        return "0".to_string();
    }
    let digits = digits.clamp(1, 17);
    let rounded = round_sig(x, digits);
    if rounded == 0.0 {
        return "0".to_string();
    }
    format!("{rounded}")
}

/// Round `x` to `digits` significant figures and return it as an `f64` (the nearest double to
/// the rounded decimal value).
pub fn round_sig(x: f64, digits: usize) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let digits = digits.clamp(1, 17);
    let s = format!("{:.*e}", digits - 1, x);
    s.parse::<f64>().unwrap_or(x)
}

/// Format an optional value with [`sig4`]; `None` becomes an empty cell.
pub fn opt4(x: Option<f64>) -> String {
    x.map(sig4).unwrap_or_default()
}

/// Round to a fixed number of decimal places (used for coordinates) and print plainly.
///
/// ```
/// use rr_etl::num::fixed;
/// assert_eq!(fixed(-75.16349, 3), "-75.163");
/// assert_eq!(fixed(40.0, 3), "40");
/// assert_eq!(fixed(-0.0004, 3), "0");
/// ```
pub fn fixed(x: f64, places: usize) -> String {
    if !x.is_finite() {
        return String::new();
    }
    let s = format!("{x:.places$}");
    let v: f64 = s.parse().unwrap_or(x);
    if v == 0.0 {
        return "0".to_string();
    }
    format!("{v}")
}

/// Round to a fixed number of decimal places, returning the value.
pub fn round_places(x: f64, places: usize) -> f64 {
    if !x.is_finite() {
        return x;
    }
    format!("{x:.places$}").parse().unwrap_or(x)
}

/// Linear interpolation of the `q` quantile (0..=1) of an already sorted slice of
/// `(value, weight)` pairs, using the weighted empirical CDF. Returns `None` for an empty slice
/// or zero total weight.
pub fn weighted_quantile(sorted: &[(f64, f64)], q: f64) -> Option<f64> {
    let total: f64 = sorted.iter().map(|(_, w)| *w).sum();
    if sorted.is_empty() || total <= 0.0 {
        return None;
    }
    let target = q.clamp(0.0, 1.0) * total;
    let mut acc = 0.0;
    for (v, w) in sorted {
        acc += *w;
        if acc >= target {
            return Some(*v);
        }
    }
    sorted.last().map(|(v, _)| *v)
}

/// Unweighted quantile of a sorted slice using the "lower median" nearest-rank rule
/// (the smallest value whose cumulative share reaches `q`).
pub fn quantile_sorted(sorted: &[f64], q: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let n = sorted.len();
    let rank = (q.clamp(0.0, 1.0) * n as f64).ceil() as usize;
    let idx = rank.clamp(1, n) - 1;
    Some(sorted[idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sig4_examples() {
        assert_eq!(sig4(35.735766419465484), "35.74");
        assert_eq!(sig4(9.147627346633526e-05), "0.00009148");
        assert_eq!(sig4(-3.796590599637702), "-3.797");
        assert_eq!(sig4(10241406107.0), "10240000000");
        assert_eq!(sig4(1.0), "1");
        assert_eq!(sig4(0.99995), "1");
        assert_eq!(sig4(-0.0), "0");
        assert_eq!(sig4(f64::INFINITY), "");
        assert_eq!(sig4(123.45), "123.5");
    }

    #[test]
    fn sig4_is_idempotent() {
        for x in [0.1234567, 98765.4321, 1e-9 * 3.14159, 2.5e12, 0.0625, 7.0] {
            let once = sig4(x);
            let twice = sig4(once.parse().unwrap());
            assert_eq!(once, twice);
        }
    }

    #[test]
    fn quantiles() {
        let v = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(quantile_sorted(&v, 0.5), Some(2.0));
        assert_eq!(quantile_sorted(&v, 0.9), Some(4.0));
        let w = [(1.0, 1.0), (10.0, 9.0)];
        assert_eq!(weighted_quantile(&w, 0.5), Some(10.0));
        assert_eq!(weighted_quantile(&w, 0.05), Some(1.0));
        assert_eq!(weighted_quantile(&[], 0.5), None);
    }
}
