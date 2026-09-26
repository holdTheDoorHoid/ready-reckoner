//! Survival functions S(d) = P(duration > d) for consequence durations, set up for fast repeated
//! evaluation, plus the integral ∫ₓ^∞ S(t) dt that turns an exceedance curve into expected
//! disruption-days.
//!
//! Three shapes:
//!
//! - [`Survival::LogNormal`]: the default, parameterised exactly as [`rr_types::DurationDist`]
//!   (median and 90th percentile), with ln(median) and the log spread precomputed. For the same
//!   parameters it returns the same bits as [`rr_types::DurationDist::survival`].
//! - [`Survival::Fixed`]: every event lasts exactly `days`.
//! - [`Survival::Empirical`]: a curve through measured points (median, 90th percentile and the
//!   share of outages lasting at least 1, 3, 7 and 14 days from county outage records). Between
//!   points it is linear in (ln d, z) where z is the standard normal score of 1 − S, so a set of
//!   points that came from a log-normal gives back that log-normal exactly; beyond the last point
//!   the last segment's slope continues.
//!
//! Every function here uses [`rr_types::math`] for `exp`/`ln`/`erfc`, so native and WebAssembly
//! builds agree bit for bit.

use rr_types::DurationDist;
use rr_types::math::{self, Z_90};

/// Standard scores beyond which [`Survival::sf_ln`] returns exactly 0 or 1 (Φ̄(8.5) ≈ 9.5 × 10⁻¹⁸).
const TAIL_Z: f64 = 8.5;

/// Smallest slope (standard deviations per unit of ln d) an empirical segment may have. A flatter
/// tail would give an effectively infinite expected duration.
const MIN_SLOPE: f64 = 0.2;

/// A duration model the engine can evaluate quickly.
#[derive(Debug, Clone, PartialEq)]
pub enum Survival {
    /// Log-normal: `z = Z_90 · (ln d − mu) / spread`, where `mu = ln(median)` and
    /// `spread = ln(p90) − mu`. `spread <= 0` is a point mass at the median.
    LogNormal {
        /// ln(median days).
        mu: f64,
        /// ln(p90) − ln(median).
        spread: f64,
    },
    /// Every event lasts exactly this many days.
    Fixed {
        /// Days.
        days: f64,
    },
    /// A measured curve (see the module docs).
    Empirical(EmpiricalCurve),
}

impl Survival {
    /// The survival model for a contract [`DurationDist`]. The distribution must be valid
    /// (deserialisation and [`DurationDist::validate`] guarantee it).
    pub fn from_dist(dist: &DurationDist) -> Survival {
        match *dist {
            DurationDist::LogNormal {
                median_days,
                p90_days,
            } => {
                let mu = math::ln(median_days);
                Survival::LogNormal {
                    mu,
                    spread: math::ln(p90_days) - mu,
                }
            }
            DurationDist::Fixed { days } => Survival::Fixed { days },
        }
    }

    /// A log-normal from its median and 90th percentile in days.
    pub fn log_normal(median_days: f64, p90_days: f64) -> Survival {
        Survival::from_dist(&DurationDist::LogNormal {
            median_days,
            p90_days,
        })
    }

    /// P(D > `days`) for durations scaled by `exp(ln_scale)` (the uncertainty draw; 0 means no
    /// scaling): P(m·D > d) = S(d / m).
    pub fn sf(&self, days: f64, ln_scale: f64) -> f64 {
        if days.is_nan() {
            return f64::NAN;
        }
        if days <= 0.0 {
            return 1.0;
        }
        if days == f64::INFINITY {
            return 0.0;
        }
        match self {
            Survival::LogNormal { mu, spread } => {
                let x = math::ln(days) - ln_scale;
                if *spread <= 0.0 {
                    return if x >= *mu { 0.0 } else { 1.0 };
                }
                math::norm_sf(Z_90 * ((x - mu) / spread))
            }
            Survival::Fixed { days: fixed } => {
                let scaled = fixed * math::exp(ln_scale);
                if days >= scaled { 0.0 } else { 1.0 }
            }
            Survival::Empirical(curve) => math::norm_sf(curve.z_at(math::ln(days) - ln_scale)),
        }
    }

    /// P(D > x) given `ln_x = ln(x)` (x > 0) for durations scaled by `exp(ln_scale)`. Same bits as
    /// [`Survival::sf`] for the same x; lets a caller that evaluates many terms at one x take the
    /// logarithm once.
    ///
    /// Beyond 8.5 standard deviations (a probability below 10⁻¹⁷) it returns exactly 0 or 1
    /// without calling `erfc`, which keeps the Monte Carlo fast when most terms sit deep in a
    /// tail.
    #[inline]
    pub fn sf_ln(&self, ln_x: f64, ln_scale: f64) -> f64 {
        match self {
            Survival::LogNormal { mu, spread } => {
                let x = ln_x - ln_scale;
                if *spread <= 0.0 {
                    return if x >= *mu { 0.0 } else { 1.0 };
                }
                let z = Z_90 * ((x - mu) / spread);
                if z > TAIL_Z {
                    0.0
                } else if z < -TAIL_Z {
                    1.0
                } else {
                    math::norm_sf(z)
                }
            }
            Survival::Fixed { days } => {
                if *days <= 0.0 || ln_x - ln_scale >= math::ln(*days) {
                    0.0
                } else {
                    1.0
                }
            }
            Survival::Empirical(curve) => math::norm_sf(curve.z_at(ln_x - ln_scale)),
        }
    }

    /// The duration that 90 % of events stay under (the "time to 90 % restored" of a restoration
    /// curve), for scale 1.
    pub fn p90_days(&self) -> f64 {
        self.quantile_days(0.9)
    }

    /// The median duration in days, for scale 1.
    pub fn median_days(&self) -> f64 {
        self.quantile_days(0.5)
    }

    /// The duration `d` with P(D ≤ d) = `p` (0 < p < 1), for scale 1.
    pub fn quantile_days(&self, p: f64) -> f64 {
        match self {
            Survival::LogNormal { mu, spread } => {
                if *spread <= 0.0 {
                    return math::exp(*mu);
                }
                math::exp(mu + probit(p) * spread / Z_90)
            }
            Survival::Fixed { days } => *days,
            Survival::Empirical(curve) => math::exp(curve.ln_d_at(probit(p))),
        }
    }

    /// Expected excess E[(m·D − x)⁺] = ∫ₓ^∞ P(m·D > t) dt, in days, for durations scaled by
    /// `m = exp(ln_scale)`. For `x <= 0` this is E[m·D] − x.
    pub fn expected_excess(&self, x: f64, ln_scale: f64) -> f64 {
        let m = math::exp(ln_scale);
        if x <= 0.0 {
            return m * self.expected_excess_unscaled(0.0) - x;
        }
        m * self.expected_excess_unscaled(x / m)
    }

    /// E[(D − x)⁺] for x ≥ 0 and scale 1.
    fn expected_excess_unscaled(&self, x: f64) -> f64 {
        match self {
            Survival::LogNormal { mu, spread } => {
                if *spread <= 0.0 {
                    return (math::exp(*mu) - x).max(0.0);
                }
                log_normal_excess(*mu, spread / Z_90, x)
            }
            Survival::Fixed { days } => (days - x).max(0.0),
            Survival::Empirical(curve) => curve.expected_excess(x),
        }
    }

    /// Mean duration in days (scale 1).
    pub fn mean_days(&self) -> f64 {
        self.expected_excess_unscaled(0.0)
    }
}

/// E[(D − x)⁺] for a log-normal with log-mean `mu` and log-sd `sigma`, x ≥ 0 (the
/// Black–Scholes form): e^(μ+σ²/2)·Φ(d₁) − x·Φ(d₂), d₂ = (μ − ln x)/σ, d₁ = d₂ + σ.
fn log_normal_excess(mu: f64, sigma: f64, x: f64) -> f64 {
    let mean = math::exp(mu + 0.5 * sigma * sigma);
    if x <= 0.0 {
        return mean;
    }
    let d2 = (mu - math::ln(x)) / sigma;
    let d1 = d2 + sigma;
    // Φ(d) computed as norm_sf(-d) keeps the far tail accurate.
    let v = mean * math::norm_sf(-d1) - x * math::norm_sf(-d2);
    v.max(0.0)
}

/// ∫_{a}^{b} Φ̄(α + β ln t) dt for 0 ≤ a < b ≤ ∞ and β > 0: a piece of a log-normal survival
/// curve with μ = −α/β and σ = 1/β.
fn segment_integral(alpha: f64, beta: f64, a: f64, b: f64) -> f64 {
    let mu = -alpha / beta;
    let sigma = 1.0 / beta;
    let upper = if b == f64::INFINITY {
        0.0
    } else {
        log_normal_excess(mu, sigma, b)
    };
    (log_normal_excess(mu, sigma, a) - upper).max(0.0)
}

/// A measured survival curve: points (ln d, z) with z the standard normal score of 1 − S(d),
/// strictly increasing in both coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct EmpiricalCurve {
    ln_d: Vec<f64>,
    z: Vec<f64>,
}

impl EmpiricalCurve {
    /// Builds a curve from `(days, share lasting longer)` points. Points outside 0 < days < ∞ or
    /// 0 < share < 1 are dropped (a share of 0 in a 12-year record means "not seen", not
    /// "impossible"), as are points that would make the curve rise (a longer duration with a
    /// larger share than a shorter one). Returns the curve and the number of points dropped, or
    /// `None` when fewer than two usable points remain.
    pub fn from_points(points: &[(f64, f64)]) -> Option<(EmpiricalCurve, usize)> {
        let mut usable: Vec<(f64, f64)> = points
            .iter()
            .copied()
            .filter(|&(d, s)| d.is_finite() && d > 0.0 && s.is_finite() && s > 0.0 && s < 1.0)
            .collect();
        let mut dropped = points.len() - usable.len();
        usable.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut ln_d: Vec<f64> = Vec::with_capacity(usable.len());
        let mut z: Vec<f64> = Vec::with_capacity(usable.len());
        for (d, s) in usable {
            let zi = -probit(s); // z of 1 − S: Φ(z) = 1 − s  ⇔  z = −Φ⁻¹(s)
            let li = math::ln(d);
            match (ln_d.last(), z.last()) {
                (Some(&l0), Some(&z0)) if li <= l0 || zi <= z0 => dropped += 1,
                _ => {
                    ln_d.push(li);
                    z.push(zi);
                }
            }
        }
        if ln_d.len() < 2 {
            return None;
        }
        // Keep every slope at or above MIN_SLOPE so the tails decay.
        for i in 1..ln_d.len() {
            let min_z = z[i - 1] + MIN_SLOPE * (ln_d[i] - ln_d[i - 1]);
            if z[i] < min_z {
                z[i] = min_z;
            }
        }
        Some((EmpiricalCurve { ln_d, z }, dropped))
    }

    /// The segment index for ln d: the first segment for points before the curve, the last one
    /// after it.
    fn segment(&self, ln_d: f64) -> usize {
        let n = self.ln_d.len();
        let mut i = 0;
        while i + 2 < n && ln_d > self.ln_d[i + 1] {
            i += 1;
        }
        i
    }

    fn slope(&self, i: usize) -> f64 {
        (self.z[i + 1] - self.z[i]) / (self.ln_d[i + 1] - self.ln_d[i])
    }

    /// z at ln d (linear within segments, the end segments extended).
    fn z_at(&self, ln_d: f64) -> f64 {
        let i = self.segment(ln_d);
        self.z[i] + self.slope(i) * (ln_d - self.ln_d[i])
    }

    /// ln d at which z is reached (the inverse of [`EmpiricalCurve::z_at`]).
    fn ln_d_at(&self, z: f64) -> f64 {
        let n = self.z.len();
        let mut i = 0;
        while i + 2 < n && z > self.z[i + 1] {
            i += 1;
        }
        self.ln_d[i] + (z - self.z[i]) / self.slope(i)
    }

    /// ∫ₓ^∞ S(t) dt, exactly, segment by segment (each segment is a piece of a log-normal).
    fn expected_excess(&self, x: f64) -> f64 {
        let n = self.ln_d.len();
        let mut total = 0.0;
        for i in 0..n - 1 {
            let beta = self.slope(i);
            let alpha = self.z[i] - beta * self.ln_d[i];
            let lo = if i == 0 { 0.0 } else { math::exp(self.ln_d[i]) };
            let hi = if i + 2 == n {
                f64::INFINITY
            } else {
                math::exp(self.ln_d[i + 1])
            };
            let a = lo.max(x);
            if a < hi {
                total += segment_integral(alpha, beta, a, hi);
            }
        }
        total
    }
}

/// The standard normal quantile Φ⁻¹(p) for 0 < p < 1 (±∞ at 0 and 1, NaN outside).
///
/// Acklam's rational approximation followed by one Halley step against [`math::norm_cdf`], which
/// brings the error down to a few ulps. Only `ln`, `sqrt`, `exp` and `erfc` from
/// [`rr_types::math`] are used, so the result is deterministic across targets.
pub fn probit(p: f64) -> f64 {
    if p.is_nan() || !(0.0..=1.0).contains(&p) {
        return f64::NAN;
    }
    if p == 0.0 {
        return f64::NEG_INFINITY;
    }
    if p == 1.0 {
        return f64::INFINITY;
    }
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    const P_LOW: f64 = 0.024_25;
    let x = if p < P_LOW {
        let q = (-2.0 * math::ln(p)).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - P_LOW {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * math::ln(1.0 - p)).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    // One Halley step: e = Φ(x) − p, u = e·√(2π)·exp(x²/2).
    let e = if p < 0.5 {
        math::norm_cdf(x) - p
    } else {
        // For p near 1 work with the upper tail to keep precision.
        (1.0 - p) - math::norm_sf(x)
    };
    let u = e * (2.0 * core::f64::consts::PI).sqrt() * math::exp(0.5 * x * x);
    x - u / (1.0 + 0.5 * x * u)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol * b.abs().max(1e-300)
    }

    #[test]
    fn log_normal_matches_the_contract_bit_for_bit() {
        for (m, p) in [
            (2.0, 7.0),
            (0.125, 0.417),
            (90.0, 180.0),
            (1.5, 5.0),
            (5.0, 5.0),
        ] {
            let dist = DurationDist::LogNormal {
                median_days: m,
                p90_days: p,
            };
            let s = Survival::from_dist(&dist);
            for d in [0.01, 0.1, 0.5, 1.0, 2.9, 3.0, 13.0, 50.0, 365.0, 1e4] {
                assert_eq!(
                    s.sf(d, 0.0).to_bits(),
                    dist.survival(d).to_bits(),
                    "({m}, {p}) at {d}"
                );
            }
        }
    }

    #[test]
    fn scaling_stretches_the_time_axis() {
        let s = Survival::log_normal(2.0, 7.0);
        let ln2 = math::ln(2.0);
        for d in [1.0, 4.0, 14.0] {
            assert!(close(s.sf(d, ln2), s.sf(d / 2.0, 0.0), 1e-12));
        }
        let f = Survival::Fixed { days: 3.0 };
        assert_eq!(f.sf(5.9, ln2), 1.0);
        assert_eq!(f.sf(6.0, ln2), 0.0);
    }

    #[test]
    fn probit_inverts_the_normal_cdf() {
        for &p in &[
            1e-12,
            1e-6,
            0.001,
            0.02425,
            0.1,
            0.25,
            0.5,
            0.75,
            0.9,
            0.975,
            0.999,
            1.0 - 1e-9,
        ] {
            let x = probit(p);
            let back = if p < 0.5 {
                math::norm_cdf(x)
            } else {
                1.0 - math::norm_sf(x)
            };
            assert!(close(back, p, 1e-12), "p {p} x {x} back {back}");
        }
        assert!(close(probit(0.9), Z_90, 1e-14));
        assert_eq!(probit(0.5), 0.0);
        assert_eq!(probit(0.0), f64::NEG_INFINITY);
        assert!(probit(1.5).is_nan());
    }

    #[test]
    fn quantiles_round_trip() {
        let s = Survival::log_normal(1.8 / 24.0, 8.4 / 24.0);
        assert!(close(s.median_days(), 1.8 / 24.0, 1e-12));
        assert!(close(s.p90_days(), 8.4 / 24.0, 1e-12));
    }

    #[test]
    fn log_normal_excess_matches_numerical_integration() {
        // ∫ₓ^∞ S(t) dt by the trapezoid rule on a fine log grid.
        let s = Survival::log_normal(2.0, 7.0);
        for x in [0.0, 0.5, 2.0, 10.0] {
            let mut num = 0.0;
            let lo: f64 = if x > 0.0 { x } else { 1e-9 };
            let n = 200_000;
            let hi: f64 = 1e5;
            let (a, b) = (math::ln(lo), math::ln(hi));
            let mut prev_t = lo;
            let mut prev_s = s.sf(lo, 0.0);
            for i in 1..=n {
                let t = math::exp(a + (b - a) * f64::from(i) / f64::from(n));
                let st = s.sf(t, 0.0);
                num += 0.5 * (st + prev_s) * (t - prev_t);
                prev_t = t;
                prev_s = st;
            }
            if x == 0.0 {
                num += lo; // S = 1 on [0, lo)
            }
            let exact = s.expected_excess(x, 0.0);
            assert!(close(exact, num, 1e-5), "x {x}: {exact} vs {num}");
        }
        // Mean of a log-normal: median · exp(σ²/2).
        let sigma = math::ln(3.5) / Z_90;
        assert!(close(
            s.mean_days(),
            2.0 * math::exp(0.5 * sigma * sigma),
            1e-12
        ));
    }

    #[test]
    fn empirical_curve_reproduces_a_log_normal() {
        let ln = Survival::log_normal(2.0 / 24.0, 20.0 / 24.0);
        let pts: Vec<(f64, f64)> = [2.0 / 24.0, 20.0 / 24.0, 1.0, 3.0, 7.0, 14.0]
            .iter()
            .map(|&d| (d, ln.sf(d, 0.0)))
            .collect();
        let (curve, dropped) = EmpiricalCurve::from_points(&pts).unwrap();
        assert_eq!(dropped, 0);
        let emp = Survival::Empirical(curve);
        for d in [0.01, 0.2, 1.0, 2.0, 5.0, 20.0, 60.0] {
            assert!(close(emp.sf(d, 0.0), ln.sf(d, 0.0), 1e-9), "{d}");
        }
        for x in [0.0, 0.5, 3.0] {
            assert!(
                close(
                    emp.expected_excess(x, 0.0),
                    ln.expected_excess(x, 0.0),
                    1e-9
                ),
                "{x}"
            );
        }
        assert!(close(emp.p90_days(), 20.0 / 24.0, 1e-9));
    }

    #[test]
    fn empirical_curve_drops_bad_points() {
        // A share of 0 (not observed) and a rising point are dropped.
        let pts = [(0.1, 0.5), (0.8, 0.1), (1.0, 0.2), (3.0, 0.02), (14.0, 0.0)];
        let (curve, dropped) = EmpiricalCurve::from_points(&pts).unwrap();
        assert_eq!(dropped, 2);
        let s = Survival::Empirical(curve);
        let mut prev = 1.0;
        for i in 1..400 {
            let d = f64::from(i) * 0.1;
            let v = s.sf(d, 0.0);
            assert!(v <= prev && (0.0..=1.0).contains(&v));
            prev = v;
        }
        assert!(EmpiricalCurve::from_points(&[(1.0, 0.5)]).is_none());
        assert!(EmpiricalCurve::from_points(&[(1.0, 0.5), (0.5, 0.1)]).is_none());
    }
}
