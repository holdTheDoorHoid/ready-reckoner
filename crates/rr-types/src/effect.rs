//! Hazard effects (DESIGN §4.3): what a hazard does to each consequence bucket, and for how long.

use serde::{Deserialize, Serialize};

use crate::math::{self, Z_90};
use crate::{BucketId, CitationId, HazardId};

string_enum! {
    /// What a duration or probability rests on.
    pub enum Evidence: "evidence" {
        /// Measured data (outage records, boil-water notices, displacement studies).
        Empirical = "empirical",
        /// Expert judgement. Shown to the user as an estimate.
        Prior = "prior",
    }
}

/// Why a duration distribution was rejected.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DurationError {
    /// The median is not a positive, finite number of days.
    #[error("median_days must be a positive number of days, got {0}")]
    Median(f64),
    /// The 90th percentile is below the median or not finite.
    #[error("p90_days ({p90_days}) must be a finite number at least median_days ({median_days})")]
    P90 {
        /// The median given.
        median_days: f64,
        /// The 90th percentile given.
        p90_days: f64,
    },
    /// A fixed duration that is negative or not finite.
    #[error("days must be zero or more, got {0}")]
    Fixed(f64),
}

/// How long a consequence lasts once it happens.
///
/// JSON and TOML use a `kind` tag: `{ kind = "log_normal", median_days = 2.0, p90_days = 7.0 }` or
/// `{ kind = "fixed", days = 1.0 }`. Deserialising rejects invalid parameters (see
/// [`DurationDist::validate`]).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    deny_unknown_fields,
    try_from = "RawDurationDist"
)]
pub enum DurationDist {
    /// Log-normal, given by its median and 90th percentile in days. The log-scale spread is
    /// σ = ln(p90 / median) / z₀.₉ with z₀.₉ = 1.2815515655446004 ([`math::Z_90`]).
    LogNormal {
        /// Half of events last less than this many days.
        median_days: f64,
        /// Nine in ten events last less than this many days.
        p90_days: f64,
    },
    /// Always exactly this many days.
    Fixed {
        /// The duration in days.
        days: f64,
    },
}

/// The unchecked twin of [`DurationDist`] that serde fills before validation.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RawDurationDist {
    LogNormal { median_days: f64, p90_days: f64 },
    Fixed { days: f64 },
}

impl TryFrom<RawDurationDist> for DurationDist {
    type Error = DurationError;

    fn try_from(raw: RawDurationDist) -> Result<Self, Self::Error> {
        let dist = match raw {
            RawDurationDist::LogNormal {
                median_days,
                p90_days,
            } => DurationDist::LogNormal {
                median_days,
                p90_days,
            },
            RawDurationDist::Fixed { days } => DurationDist::Fixed { days },
        };
        dist.validate()?;
        Ok(dist)
    }
}

impl DurationDist {
    /// Checks the parameters: a log-normal needs a positive, finite median and a finite p90 at
    /// least the median (equal means every event lasts exactly the median); a fixed duration
    /// needs a finite value of zero or more.
    pub fn validate(&self) -> Result<(), DurationError> {
        match *self {
            DurationDist::LogNormal {
                median_days,
                p90_days,
            } => {
                if !(median_days.is_finite() && median_days > 0.0) {
                    Err(DurationError::Median(median_days))
                } else if !(p90_days.is_finite() && p90_days >= median_days) {
                    Err(DurationError::P90 {
                        median_days,
                        p90_days,
                    })
                } else {
                    Ok(())
                }
            }
            DurationDist::Fixed { days } => {
                if days.is_finite() && days >= 0.0 {
                    Ok(())
                } else {
                    Err(DurationError::Fixed(days))
                }
            }
        }
    }

    /// P(duration ≤ `days`): the chance that an event is over within `days`.
    ///
    /// Non-decreasing in `days`, 0 for `days` ≤ 0 (log-normal), 0.5 at the median and 0.9 at the
    /// 90th percentile. NaN in gives NaN out. The parameters must be valid (deserialisation
    /// guarantees it; check with [`DurationDist::validate`] when building one in code).
    pub fn cdf(&self, days: f64) -> f64 {
        if days.is_nan() {
            return f64::NAN;
        }
        match *self {
            DurationDist::Fixed { days: fixed } => {
                if days >= fixed {
                    1.0
                } else {
                    0.0
                }
            }
            DurationDist::LogNormal {
                median_days,
                p90_days,
            } => match log_normal_z(median_days, p90_days, days) {
                Z::Below => 0.0,
                Z::Above => 1.0,
                Z::At(z) => math::norm_cdf(z),
            },
        }
    }

    /// P(duration > `days`) = 1 − [`DurationDist::cdf`], computed directly so the upper tail keeps
    /// its precision. This is the 1 − F(d) of the design-event formula (DESIGN §4.4).
    pub fn survival(&self, days: f64) -> f64 {
        if days.is_nan() {
            return f64::NAN;
        }
        match *self {
            DurationDist::Fixed { days: fixed } => {
                if days >= fixed {
                    0.0
                } else {
                    1.0
                }
            }
            DurationDist::LogNormal {
                median_days,
                p90_days,
            } => match log_normal_z(median_days, p90_days, days) {
                Z::Below => 1.0,
                Z::Above => 0.0,
                Z::At(z) => math::norm_sf(z),
            },
        }
    }
}

/// Where `days` falls on a log-normal: certainly below, certainly above, or at standard score z.
enum Z {
    Below,
    Above,
    At(f64),
}

fn log_normal_z(median_days: f64, p90_days: f64, days: f64) -> Z {
    if days <= 0.0 {
        return Z::Below;
    }
    if days == f64::INFINITY {
        return Z::Above;
    }
    let mu = math::ln(median_days);
    // σ·z₀.₉: the log-distance from the median to the 90th percentile.
    let spread = math::ln(p90_days) - mu;
    if spread <= 0.0 {
        // p90 == median: every event lasts exactly the median.
        return if days >= median_days {
            Z::Above
        } else {
            Z::Below
        };
    }
    // z = (ln d − μ) / σ with σ = spread / z₀.₉. Written this way, d = p90 gives exactly z₀.₉ and
    // d = median gives exactly 0.
    Z::At(Z_90 * ((math::ln(days) - mu) / spread))
}

/// Why an effect was rejected.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EffectError {
    /// `p_given_event` is not a probability.
    #[error("p_given_event must be between 0 and 1, got {0}")]
    Probability(f64),
    /// The duration is invalid.
    #[error(transparent)]
    Duration(#[from] DurationError),
    /// No sources: every number needs a citation.
    #[error("an effect needs at least one source")]
    NoSources,
}

/// What one hazard does to one bucket when it happens (DESIGN §4.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    /// The hazard.
    pub hazard: HazardId,
    /// The consequence.
    pub bucket: BucketId,
    /// Chance the consequence happens to the household, given the hazard happens (0 to 1).
    pub p_given_event: f64,
    /// How long the consequence lasts.
    pub duration: DurationDist,
    /// What the numbers rest on.
    pub evidence: Evidence,
    /// Where they come from.
    pub sources: Vec<CitationId>,
}

impl Effect {
    /// Checks that `p_given_event` is a probability, the duration is valid and there is a source.
    pub fn validate(&self) -> Result<(), EffectError> {
        if !(0.0..=1.0).contains(&self.p_given_event) {
            return Err(EffectError::Probability(self.p_given_event));
        }
        self.duration.validate()?;
        if self.sources.is_empty() {
            return Err(EffectError::NoSources);
        }
        Ok(())
    }
}

/// Why a household event rate was rejected.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RateError {
    /// A rate that is negative or not finite, or a range that does not contain it.
    #[error(
        "rates must be finite, zero or more, with low <= rate_per_year <= high; got {low} <= {rate} <= {high}"
    )]
    Range {
        /// The low end given.
        low: f64,
        /// The rate given.
        rate: f64,
        /// The high end given.
        high: f64,
    },
    /// No sources: every number needs a citation.
    #[error("a rate needs at least one source")]
    NoSources,
}

/// How often a hazard reaches this household: the interface from `rr-hazards` to
/// `rr-consequence`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseholdEventRate {
    /// The hazard.
    pub hazard: HazardId,
    /// Household-significant events per year, r_h (climate multiplier applied).
    pub rate_per_year: f64,
    /// Low end of the plausible range.
    pub low: f64,
    /// High end of the plausible range.
    pub high: f64,
    /// What the rate rests on.
    pub evidence: Evidence,
    /// Where it comes from.
    pub sources: Vec<CitationId>,
}

impl HouseholdEventRate {
    /// Checks that the rate and range are finite, not negative, and ordered
    /// `low <= rate_per_year <= high`, and that there is a source.
    pub fn validate(&self) -> Result<(), RateError> {
        let (low, rate, high) = (self.low, self.rate_per_year, self.high);
        let finite = low.is_finite() && rate.is_finite() && high.is_finite();
        if !(finite && 0.0 <= low && low <= rate && rate <= high) {
            return Err(RateError::Range { low, rate, high });
        }
        if self.sources.is_empty() {
            return Err(RateError::NoSources);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn household_event_rate_validation() {
        let good = HouseholdEventRate {
            hazard: HazardId::HouseFire,
            rate_per_year: 0.003,
            low: 0.002,
            high: 0.004,
            evidence: Evidence::Empirical,
            sources: vec!["nfpa_home_fires".into()],
        };
        assert_eq!(good.validate(), Ok(()));
        let json = serde_json::to_value(&good).unwrap();
        assert_eq!(json["evidence"], "empirical");
        assert_eq!(
            serde_json::from_value::<HouseholdEventRate>(json).unwrap(),
            good
        );
        for (low, rate, high) in [
            (0.004, 0.003, 0.005),
            (0.001, 0.003, 0.002),
            (-0.1, 0.0, 0.1),
            (0.0, f64::NAN, 1.0),
        ] {
            let mut r = good.clone();
            (r.low, r.rate_per_year, r.high) = (low, rate, high);
            assert!(
                matches!(r.validate(), Err(RateError::Range { .. })),
                "{low} {rate} {high}"
            );
        }
        let mut r = good;
        r.sources.clear();
        assert_eq!(r.validate(), Err(RateError::NoSources));
    }

    fn ln(median_days: f64, p90_days: f64) -> DurationDist {
        DurationDist::LogNormal {
            median_days,
            p90_days,
        }
    }

    #[test]
    fn cdf_is_one_half_at_the_median_and_nine_tenths_at_p90() {
        for (m, p) in [
            (2.0, 7.0),
            (0.5, 3.0),
            (14.0, 60.0),
            (1.0, 1.5),
            (3.0, 3.0001),
            (90.0, 400.0),
        ] {
            let d = ln(m, p);
            assert_eq!(d.cdf(m), 0.5, "median of ({m}, {p})");
            assert!(
                (d.cdf(p) - 0.9).abs() < 1e-15,
                "p90 of ({m}, {p}): {}",
                d.cdf(p)
            );
            assert_eq!(d.survival(m), 0.5);
            assert!((d.survival(p) - 0.1).abs() < 1e-15);
        }
    }

    #[test]
    fn cdf_matches_independent_reference_values() {
        // Φ((ln d − ln m) / σ), σ = ln(p90/m) / z₀.₉, computed in Python (statistics.NormalDist).
        let cases = [
            (2.0, 7.0, 1.0, 0.23913873211007597),
            (2.0, 7.0, 14.0, 0.9767390631098625),
            (2.0, 7.0, 30.0, 0.9971996060957541),
            (0.5, 3.0, 0.1, 0.12483598138014823),
            (0.5, 3.0, 1.0, 0.6899722957846335),
            (0.5, 3.0, 10.0, 0.9839310070139753),
            (14.0, 60.0, 30.0, 0.7489387699033713),
            (14.0, 60.0, 90.0, 0.9493528792152196),
            (14.0, 60.0, 365.0, 0.997957699563554),
            (1.0, 1.5, 1.2, 0.7177812335104511),
        ];
        for (m, p, d, want) in cases {
            let got = ln(m, p).cdf(d);
            assert!(
                (got - want).abs() < 1e-12,
                "cdf({d}) for ({m}, {p}) = {got}, want {want}"
            );
            let tail = ln(m, p).survival(d);
            assert!((tail - (1.0 - want)).abs() < 1e-12);
        }
    }

    #[test]
    fn cdf_is_monotone_and_bounded() {
        for d in [
            ln(2.0, 7.0),
            ln(0.25, 30.0),
            ln(14.0, 60.0),
            ln(5.0, 5.0),
            DurationDist::Fixed { days: 3.0 },
        ] {
            let mut prev_cdf = 0.0;
            let mut prev_sf = 1.0;
            for i in 0..=20_000 {
                let days = f64::from(i) * 0.05;
                let (c, s) = (d.cdf(days), d.survival(days));
                assert!((0.0..=1.0).contains(&c) && (0.0..=1.0).contains(&s));
                assert!(c >= prev_cdf, "{d:?}: cdf fell at {days}");
                assert!(s <= prev_sf, "{d:?}: survival rose at {days}");
                assert!(
                    (c + s - 1.0).abs() < 1e-15,
                    "{d:?}: cdf + survival != 1 at {days}"
                );
                prev_cdf = c;
                prev_sf = s;
            }
        }
    }

    #[test]
    fn cdf_edge_cases() {
        let d = ln(2.0, 7.0);
        assert_eq!(d.cdf(0.0), 0.0);
        assert_eq!(d.cdf(-1.0), 0.0);
        assert_eq!(d.cdf(f64::INFINITY), 1.0);
        assert_eq!(d.survival(0.0), 1.0);
        assert_eq!(d.survival(f64::INFINITY), 0.0);
        assert!(d.cdf(f64::NAN).is_nan());
        assert!(d.survival(f64::NAN).is_nan());
        assert!(d.cdf(1e-300) < 1e-100);
        assert!(d.cdf(1e6) > 1.0 - 1e-15);

        // p90 == median: a point mass at the median.
        let point = ln(5.0, 5.0);
        assert_eq!(point.cdf(4.999), 0.0);
        assert_eq!(point.cdf(5.0), 1.0);
        assert_eq!(point.survival(5.0), 0.0);

        let fixed = DurationDist::Fixed { days: 3.0 };
        assert_eq!(fixed.cdf(2.999), 0.0);
        assert_eq!(fixed.cdf(3.0), 1.0);
        assert_eq!(fixed.survival(2.999), 1.0);
        assert_eq!(fixed.survival(3.0), 0.0);
        let zero = DurationDist::Fixed { days: 0.0 };
        assert_eq!(zero.cdf(0.0), 1.0);
        assert!(zero.cdf(f64::NAN).is_nan());
    }

    #[test]
    fn wider_spread_means_fatter_tail() {
        let narrow = ln(2.0, 4.0);
        let wide = ln(2.0, 20.0);
        for days in [5.0, 10.0, 30.0] {
            assert!(wide.survival(days) > narrow.survival(days));
        }
        for days in [0.5, 1.0, 1.5] {
            assert!(wide.cdf(days) > narrow.cdf(days));
        }
    }

    #[test]
    fn serde_shape_and_validation() {
        let d = ln(2.0, 7.0);
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"log_normal","median_days":2.0,"p90_days":7.0}"#
        );
        assert_eq!(serde_json::from_str::<DurationDist>(&json).unwrap(), d);
        let fixed = serde_json::to_string(&DurationDist::Fixed { days: 1.5 }).unwrap();
        assert_eq!(fixed, r#"{"kind":"fixed","days":1.5}"#);

        for bad in [
            r#"{"kind":"log_normal","median_days":2,"p90_days":1}"#,
            r#"{"kind":"log_normal","median_days":0,"p90_days":1}"#,
            r#"{"kind":"log_normal","median_days":-2,"p90_days":1}"#,
            r#"{"kind":"fixed","days":-1}"#,
            r#"{"kind":"fixed","days":1,"extra":2}"#,
            r#"{"kind":"weibull","shape":2}"#,
            r#"{"median_days":2,"p90_days":7}"#,
        ] {
            assert!(serde_json::from_str::<DurationDist>(bad).is_err(), "{bad}");
        }
        assert_eq!(
            ln(2.0, 1.0).validate(),
            Err(DurationError::P90 {
                median_days: 2.0,
                p90_days: 1.0
            })
        );
        assert!(ln(f64::NAN, 1.0).validate().is_err());
        assert!(ln(1.0, f64::INFINITY).validate().is_err());
        assert!(DurationDist::Fixed { days: f64::NAN }.validate().is_err());
    }

    #[test]
    fn effect_validation() {
        let good = Effect {
            hazard: HazardId::IceStorm,
            bucket: BucketId::Power,
            p_given_event: 0.4,
            duration: ln(2.0, 7.0),
            evidence: Evidence::Empirical,
            sources: vec!["eagle_i_outages".into()],
        };
        assert_eq!(good.validate(), Ok(()));
        let json = serde_json::to_value(&good).unwrap();
        assert_eq!(json["duration"]["kind"], "log_normal");
        assert_eq!(serde_json::from_value::<Effect>(json).unwrap(), good);

        let mut e = good.clone();
        e.p_given_event = 1.5;
        assert_eq!(e.validate(), Err(EffectError::Probability(1.5)));
        let mut e = good.clone();
        e.p_given_event = f64::NAN;
        assert!(e.validate().is_err());
        let mut e = good.clone();
        e.sources.clear();
        assert_eq!(e.validate(), Err(EffectError::NoSources));
        let mut e = good;
        e.duration = ln(3.0, 1.0);
        assert!(matches!(e.validate(), Err(EffectError::Duration(_))));
    }
}
