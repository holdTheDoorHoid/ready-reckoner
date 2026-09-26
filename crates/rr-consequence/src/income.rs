//! The income bucket (research §3.5): months of household income gap after unemployment
//! insurance, at the household's return period.
//!
//! Each earner holds a share `s = 1 / earners` of household income. A spell of `D` weeks costs
//!
//! ```text
//! gap(D) = [ s·(1−ρ)·min(D, W) + s·max(D − W, 0) ] / 4.345      months of household income
//! ```
//!
//! with unemployment insurance replacing `ρ` of wages for `W` weeks (streams without insurance use
//! ρ = 0). Λ_income(g) = Σ streams R · P(D > gap⁻¹(g)), and the target is the smallest g with
//! Λ_income(g) ≤ the dial rate, rounded up to [`MONTHS_LADDER`] (3 % tolerance, as for days).

use rr_types::math;

use crate::model::{IncomeTerm, UParam};

/// The month values an income target is rounded up to.
pub const MONTHS_LADDER: [f32; 18] = [
    0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 15.0, 18.0, 24.0, 36.0,
];

/// Largest income gap the solver looks at, in months.
const MAX_MONTHS: f64 = 120.0;

/// Unemployment-insurance and unit settings for the gap formula.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GapRule {
    /// Each earner's share of household income, s.
    pub share: f64,
    /// Share of wages unemployment insurance replaces, ρ.
    pub replacement: f64,
    /// Weeks of unemployment insurance, W.
    pub ui_weeks: f64,
    /// Weeks per month (4.345).
    pub weeks_per_month: f64,
}

impl GapRule {
    /// Months of household income lost in a spell of `weeks` weeks.
    pub fn gap_months(&self, weeks: f64, insured: bool) -> f64 {
        let d = weeks.max(0.0);
        let (rho, w) = if insured {
            (self.replacement, self.ui_weeks)
        } else {
            (0.0, 0.0)
        };
        (self.share * (1.0 - rho) * d.min(w) + self.share * (d - w).max(0.0)) / self.weeks_per_month
    }

    /// The spell length in weeks that produces a gap of `months` (the inverse of
    /// [`GapRule::gap_months`]).
    pub fn weeks_for_gap(&self, months: f64, insured: bool) -> f64 {
        let g = months.max(0.0) * self.weeks_per_month;
        let (rho, w) = if insured {
            (self.replacement, self.ui_weeks)
        } else {
            (0.0, 0.0)
        };
        let covered = self.share * (1.0 - rho) * w; // gap·wpm reached after W weeks
        if g <= covered && rho < 1.0 {
            g / (self.share * (1.0 - rho))
        } else {
            w + (g - covered) / self.share
        }
    }
}

/// The income streams under one draw.
pub(crate) struct IncomeEval<'m> {
    terms: &'m [IncomeTerm],
    rate: Vec<f64>,
    ln_scale: Vec<f64>,
    rule: GapRule,
}

impl<'m> IncomeEval<'m> {
    pub fn central(terms: &'m [IncomeTerm], rule: GapRule) -> IncomeEval<'m> {
        IncomeEval {
            terms,
            rate: terms.iter().map(|t| t.rate).collect(),
            ln_scale: vec![0.0; terms.len()],
            rule,
        }
    }

    pub fn set_draw(&mut self, params: &[UParam], z: impl Fn(usize) -> f64) {
        for (i, t) in self.terms.iter().enumerate() {
            self.rate[i] = t.rate
                * t.rate_param
                    .map_or(1.0, |p| math::exp(params[p].ln_mult(z(p))));
            self.ln_scale[i] = t.dur_param.map_or(0.0, |p| params[p].ln_mult(z(p)));
        }
    }

    /// Back to central values.
    pub fn reset(&mut self) {
        for (k, t) in self.terms.iter().enumerate() {
            self.rate[k] = t.rate;
            self.ln_scale[k] = 0.0;
        }
    }

    pub fn set_draw_from(&mut self, draws: &crate::ranges::Draws, i: usize) {
        for (k, t) in self.terms.iter().enumerate() {
            self.rate[k] = t.rate * t.rate_param.map_or(1.0, |p| draws.mult(p, i));
            self.ln_scale[k] = t.dur_param.map_or(0.0, |p| draws.lnm(p, i));
        }
    }

    pub fn terms(&self) -> &[IncomeTerm] {
        self.terms
    }

    /// Spells per year leaving a gap of more than `months`.
    pub fn lambda(&self, months: f64) -> f64 {
        self.terms
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let weeks = self.rule.weeks_for_gap(months, t.unemployment_insurance);
                self.rate[i] * t.spell.sf(weeks, self.ln_scale[i])
            })
            .sum()
    }

    /// Each stream's share of Λ(months).
    pub fn shares(&self, months: f64) -> Vec<f64> {
        let parts: Vec<f64> = self
            .terms
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let weeks = self.rule.weeks_for_gap(months, t.unemployment_insurance);
                self.rate[i] * t.spell.sf(weeks, self.ln_scale[i])
            })
            .collect();
        let total: f64 = parts.iter().sum();
        if total > 0.0 {
            parts.iter().map(|p| p / total).collect()
        } else {
            vec![0.0; parts.len()]
        }
    }

    /// Continuous target in months (0 when spells of any length are rarer than `rate`).
    pub fn target(&self, rate: f64, iterations: u32) -> f64 {
        let total: f64 = self.rate.iter().sum();
        if total <= rate {
            return 0.0;
        }
        if self.lambda(MAX_MONTHS) > rate {
            return MAX_MONTHS;
        }
        let (mut lo, mut hi) = (0.0, MAX_MONTHS);
        for _ in 0..iterations {
            let mid = 0.5 * (lo + hi);
            if self.lambda(mid) > rate {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        hi
    }

    /// The target on [`MONTHS_LADDER`], rounded up with the 3 % tolerance (the smallest step g
    /// with Λ(g · 1.03) ≤ `rate`).
    pub fn ladder_target(&self, rate: f64) -> f32 {
        let total: f64 = self.rate.iter().sum();
        if total <= rate {
            return 0.0;
        }
        let (mut lo, mut hi) = (0usize, MONTHS_LADDER.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.lambda(month_limit(MONTHS_LADDER[mid])) <= rate {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        MONTHS_LADDER[lo.min(MONTHS_LADDER.len() - 1)]
    }
}

/// The household's income curve at central values, self-contained (for the calibration report
/// and the savings track).
#[derive(Debug, Clone, PartialEq)]
pub struct IncomeCurve {
    /// Streams: spells per year, spell length in weeks, and whether unemployment insurance
    /// applies.
    pub streams: Vec<(f64, crate::survival::Survival, bool)>,
    /// The gap formula's settings.
    pub rule: GapRule,
}

impl IncomeCurve {
    pub(crate) fn from_terms(terms: &[IncomeTerm], rule: GapRule) -> IncomeCurve {
        IncomeCurve {
            streams: terms
                .iter()
                .map(|t| (t.rate, t.spell.clone(), t.unemployment_insurance))
                .collect(),
            rule,
        }
    }

    /// Spells per year leaving a gap of more than `months`.
    pub fn lambda(&self, months: f64) -> f64 {
        self.streams
            .iter()
            .map(|(rate, spell, insured)| {
                rate * spell.sf(self.rule.weeks_for_gap(months, *insured), 0.0)
            })
            .sum()
    }

    /// Months of income gap at any yearly rate (continuous).
    pub fn target_at(&self, rate: f64) -> f64 {
        let total: f64 = self.streams.iter().map(|s| s.0).sum();
        if total <= rate {
            return 0.0;
        }
        if self.lambda(MAX_MONTHS) > rate {
            return MAX_MONTHS;
        }
        let (mut lo, mut hi) = (0.0, MAX_MONTHS);
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            if self.lambda(mid) > rate {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        hi
    }
}

/// The largest raw value that still rounds to months-ladder step `step` (the same 3 % tolerance
/// as the day ladder).
fn month_limit(step: f32) -> f64 {
    f64::from(step) * (1.0 + crate::curve::LADDER_TOLERANCE)
}

/// Rounds months up to [`MONTHS_LADDER`], a value within 3 % above a step counting as that step.
pub fn round_up_months(months: f64) -> f32 {
    if months.is_nan() || months <= 0.0 {
        return 0.0;
    }
    MONTHS_LADDER
        .iter()
        .copied()
        .find(|&m| month_limit(m) >= months)
        .unwrap_or(MONTHS_LADDER[MONTHS_LADDER.len() - 1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survival::Survival;

    fn rule(share: f64) -> GapRule {
        GapRule {
            share,
            replacement: 0.5,
            ui_weeks: 26.0,
            weeks_per_month: 4.345,
        }
    }

    #[test]
    fn gap_formula_matches_research_section_3_5() {
        let r = rule(0.5);
        // 20 weeks, all insured: 0.5·0.5·20 / 4.345.
        assert!((r.gap_months(20.0, true) - 5.0 / 4.345).abs() < 1e-12);
        // 46.07 weeks: [0.5·0.5·26 + 0.5·20.07] / 4.345 = 3.806 months (the research's 3.8).
        assert!((r.gap_months(46.07, true) - 3.806).abs() < 1e-3);
        for g in [0.1, 1.0, 1.4959, 3.0, 8.0] {
            for insured in [true, false] {
                let w = r.weeks_for_gap(g, insured);
                assert!((r.gap_months(w, insured) - g).abs() < 1e-9, "{g} {insured}");
            }
        }
    }

    #[test]
    fn philadelphia_income_target_matches_the_research() {
        // Two earners, 0.083 spells a year each, spells median 10 / p90 36 weeks.
        let terms = vec![IncomeTerm {
            hazard: rr_types::HazardId::JobLoss,
            label: "losing a job".into(),
            rate: 0.166,
            rate_param: None,
            spell: Survival::log_normal(10.0, 36.0),
            dur_param: None,
            unemployment_insurance: true,
            sources: vec![],
        }];
        let e = IncomeEval::central(&terms, rule(0.5));
        let research = [(0.1, 0.44), (0.02, 2.22), (0.0105, 3.81), (0.002, 9.48)];
        for (rate, want) in research {
            let got = e.target(rate, 60);
            assert!((got - want).abs() < 0.02, "at {rate}: {got} vs {want}");
        }
        assert_eq!(e.ladder_target(0.01), 4.0);
        assert_eq!(round_up_months(3.94), 4.0);
        assert_eq!(round_up_months(4.1), 4.0);
        assert_eq!(round_up_months(4.2), 5.0);
        assert_eq!(round_up_months(0.0), 0.0);
    }
}
