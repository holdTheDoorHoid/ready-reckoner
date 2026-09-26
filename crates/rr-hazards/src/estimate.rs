//! A number with a plausible range, the evidence behind it, and where it comes from.
//!
//! Every household rate is built as a product of factors (county frequency × footprint ×
//! household modifier × climate multiplier) or a sum of parts. [`Estimate`] carries each factor's
//! value, its plausible low and high, whether it rests on data or on expert judgement, and its
//! citation ids, and combines them:
//!
//! - **Products** multiply the values and add the log-spreads in quadrature:
//!   `ln(high/value) = sqrt(Σ ln(high_i/value_i)²)`, and likewise for the low side. Independent
//!   "factor of k" uncertainties compound like this; multiplying every factor's extreme would
//!   give a range far wider than any plausible combination.
//! - **Sums** add values, lows and highs (the parts are treated as moving together, which is the
//!   cautious choice).
//!
//! All transcendental maths goes through [`rr_types::math`] so native and WebAssembly builds give
//! the same bits.

use rr_types::{CitationId, Evidence, math};

/// A number with a plausible range, its evidence and its sources.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Estimate {
    /// The central value.
    pub value: f64,
    /// A plausible low (at most `value`).
    pub low: f64,
    /// A plausible high (at least `value`).
    pub high: f64,
    /// Data or expert judgement.
    pub evidence: Evidence,
    /// Citation ids, in the order they were first added, without repeats.
    pub sources: Vec<CitationId>,
}

impl Estimate {
    /// A value with a range and sources.
    pub fn new(value: f64, low: f64, high: f64, evidence: Evidence, sources: &[&str]) -> Self {
        debug_assert!(
            low <= value && value <= high,
            "{low} <= {value} <= {high} does not hold"
        );
        Estimate {
            value,
            low,
            high,
            evidence,
            sources: sources.iter().map(|s| CitationId::from(*s)).collect(),
        }
    }

    /// Measured data with a range.
    pub fn data(value: f64, low: f64, high: f64, sources: &[&str]) -> Self {
        Self::new(value, low, high, Evidence::Empirical, sources)
    }

    /// Expert judgement with a range.
    pub fn prior(value: f64, low: f64, high: f64, sources: &[&str]) -> Self {
        Self::new(value, low, high, Evidence::Prior, sources)
    }

    /// A known multiplier with no uncertainty (a count of people, a definition). Multiplying by it
    /// changes neither the evidence nor the relative range.
    pub fn exact(value: f64) -> Self {
        Estimate {
            value,
            low: value,
            high: value,
            evidence: Evidence::Empirical,
            sources: Vec::new(),
        }
    }

    /// True when the range is wider than the value itself.
    pub fn is_uncertain(&self) -> bool {
        self.low < self.value || self.high > self.value
    }

    /// Adds citation ids (skipping ones already present).
    pub fn cite(mut self, ids: &[&str]) -> Self {
        for id in ids {
            push_unique(&mut self.sources, CitationId::from(*id));
        }
        self
    }

    /// Adds another estimate's citation ids (skipping ones already present).
    fn cite_all(&mut self, ids: &[CitationId]) {
        for id in ids {
            push_unique(&mut self.sources, id.clone());
        }
    }

    /// The product of two estimates (log-spreads added in quadrature).
    pub fn times(&self, other: &Estimate) -> Estimate {
        let value = self.value * other.value;
        let evidence = combine_evidence(self, other);
        let mut out = if value == 0.0 {
            Estimate {
                value: 0.0,
                low: 0.0,
                high: 0.0,
                evidence,
                sources: Vec::new(),
            }
        } else {
            let down = quadrature(log_spread_down(self), log_spread_down(other));
            let up = quadrature(log_spread_up(self), log_spread_up(other));
            let low = if down.is_finite() {
                value * math::exp(-down)
            } else {
                0.0
            };
            let high = value * math::exp(up);
            Estimate {
                value,
                // Rounding in exp/ln can leave the bounds a hair on the wrong side of the value.
                low: low.min(value),
                high: high.max(value),
                evidence,
                sources: Vec::new(),
            }
        };
        out.cite_all(&self.sources);
        out.cite_all(&other.sources);
        out
    }

    /// The sum of two estimates (values, lows and highs added).
    pub fn plus(&self, other: &Estimate) -> Estimate {
        let evidence = if self.value == 0.0 {
            other.evidence
        } else if other.value == 0.0 {
            self.evidence
        } else {
            combine_evidence(self, other)
        };
        let mut out = Estimate {
            value: self.value + other.value,
            low: self.low + other.low,
            high: self.high + other.high,
            evidence,
            sources: Vec::new(),
        };
        out.cite_all(&self.sources);
        out.cite_all(&other.sources);
        out
    }

    /// The estimate multiplied by a known, exact number (a count of people, of earners).
    pub fn scaled(&self, k: f64) -> Estimate {
        self.times(&Estimate::exact(k))
    }

    /// The product whose range spans every combination of the two ranges: low × low to
    /// high × high. Used for the rare families, where every factor is expert judgement stacked
    /// on expert judgement and only the range is ever shown (REVIEW §2.3 computes its ranges
    /// this way); the ranked rates use [`Estimate::times`].
    pub fn times_span(&self, other: &Estimate) -> Estimate {
        let mut out = Estimate {
            value: self.value * other.value,
            low: self.low * other.low,
            high: self.high * other.high,
            evidence: combine_evidence(self, other),
            sources: Vec::new(),
        };
        out.low = out.low.min(out.value);
        out.high = out.high.max(out.value);
        out.cite_all(&self.sources);
        out.cite_all(&other.sources);
        out
    }

    /// max(high/value, value/low): how many times the range reaches away from the value.
    /// 1 for an exact number; infinite when the low end is zero but the value is not.
    pub fn range_factor(&self) -> f64 {
        if self.value <= 0.0 {
            return 1.0;
        }
        let up = self.high / self.value;
        let down = if self.low > 0.0 {
            self.value / self.low
        } else {
            f64::INFINITY
        };
        up.max(down)
    }
}

/// The evidence of a combination: expert judgement if any materially uncertain part is expert
/// judgement. Exact parts (counts, definitions) do not change it.
fn combine_evidence(a: &Estimate, b: &Estimate) -> Evidence {
    let prior =
        |e: &Estimate| e.evidence == Evidence::Prior && (e.is_uncertain() || !e.sources.is_empty());
    if prior(a) || prior(b) {
        Evidence::Prior
    } else {
        Evidence::Empirical
    }
}

fn log_spread_up(e: &Estimate) -> f64 {
    if e.high > e.value && e.value > 0.0 {
        math::ln(e.high / e.value)
    } else {
        0.0
    }
}

fn log_spread_down(e: &Estimate) -> f64 {
    if e.value <= 0.0 || e.low >= e.value {
        0.0
    } else if e.low <= 0.0 {
        f64::INFINITY
    } else {
        math::ln(e.value / e.low)
    }
}

fn quadrature(a: f64, b: f64) -> f64 {
    if a.is_infinite() || b.is_infinite() {
        f64::INFINITY
    } else {
        (a * a + b * b).sqrt()
    }
}

pub(crate) fn push_unique(list: &mut Vec<CitationId>, id: CitationId) {
    if !list.contains(&id) {
        list.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-12 * a.abs().max(b.abs()).max(1.0)
    }

    #[test]
    fn product_adds_log_spreads_in_quadrature() {
        // A factor of 2 either way times a factor of 2 either way is a factor of 2^√2 ≈ 2.665.
        let a = Estimate::prior(0.1, 0.05, 0.2, &["a"]);
        let b = Estimate::data(3.0, 1.5, 6.0, &["b"]);
        let p = a.times(&b);
        assert!(close(p.value, 0.3));
        let k = math::exp(2.0_f64.sqrt() * math::ln(2.0));
        assert!(close(p.high, 0.3 * k), "{}", p.high);
        assert!(close(p.low, 0.3 / k), "{}", p.low);
        assert_eq!(p.evidence, Evidence::Prior);
        assert_eq!(
            p.sources,
            vec![CitationId::from("a"), CitationId::from("b")]
        );
    }

    #[test]
    fn exact_factors_keep_the_relative_range_and_evidence() {
        let a = Estimate::data(0.083, 0.06, 0.126, &["bls"]);
        let two = a.scaled(2.0);
        assert!(close(two.value, 0.166));
        assert!(close(two.low, 0.12));
        assert!(close(two.high, 0.252));
        assert_eq!(two.evidence, Evidence::Empirical);
        assert_eq!(two.sources, a.sources);
    }

    #[test]
    fn zero_and_sums() {
        let a = Estimate::prior(0.05, 0.02, 0.1, &["p"]);
        let z = a.scaled(0.0);
        assert_eq!((z.value, z.low, z.high), (0.0, 0.0, 0.0));
        let b = Estimate::data(0.1, 0.07, 0.3, &["d"]);
        let s = a.plus(&b);
        assert!(close(s.value, 0.15) && close(s.low, 0.09) && close(s.high, 0.4));
        assert_eq!(s.evidence, Evidence::Prior);
        // Adding a zero part keeps the other part's evidence.
        let s2 = b.plus(&z);
        assert_eq!(s2.evidence, Evidence::Empirical);
    }

    #[test]
    fn a_low_end_of_zero_propagates() {
        let a = Estimate::prior(0.01, 0.0, 0.03, &["p"]);
        let b = Estimate::data(2.0, 1.0, 4.0, &["d"]);
        let p = a.times(&b);
        assert_eq!(p.low, 0.0);
        assert!(p.high > 0.06);
        assert!(p.range_factor().is_infinite());
    }

    #[test]
    fn duplicate_sources_are_listed_once() {
        let a = Estimate::data(1.0, 0.5, 2.0, &["x", "y"]);
        let b = Estimate::data(1.0, 0.5, 2.0, &["y", "z"]);
        let product = a.times(&b);
        let ids: Vec<&str> = product.sources.iter().map(|s| s.as_str()).collect();
        assert_eq!(ids, ["x", "y", "z"]);
    }
}
