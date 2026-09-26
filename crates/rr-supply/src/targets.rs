//! Reading bucket targets from [`BucketAssessment`]s.

use std::collections::BTreeMap;

use rr_types::{BucketAssessment, BucketId, CitationId, HazardId, Target};

use crate::format::round_dp;

/// A duration target in days, cleaned of `f32` noise (two decimals) and never negative.
pub(crate) fn clean_days(v: f32) -> f64 {
    let d = round_dp(f64::from(v), 2);
    if d.is_finite() && d > 0.0 { d } else { 0.0 }
}

/// The evacuation target's shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Evac {
    pub notice_hours_low: f64,
    pub notice_hours_high: f64,
    pub days_away: f64,
}

/// The household's bucket targets, keyed by bucket (the first assessment for a bucket wins).
#[derive(Debug, Clone)]
pub(crate) struct Targets<'a> {
    by_bucket: BTreeMap<BucketId, &'a BucketAssessment>,
}

impl<'a> Targets<'a> {
    pub(crate) fn new(buckets: &'a [BucketAssessment]) -> Self {
        let mut by_bucket = BTreeMap::new();
        for b in buckets {
            by_bucket.entry(b.id).or_insert(b);
        }
        Self { by_bucket }
    }

    pub(crate) fn get(&self, b: BucketId) -> Option<&'a BucketAssessment> {
        self.by_bucket.get(&b).copied()
    }

    /// The design target in days, for a duration bucket.
    pub(crate) fn days(&self, b: BucketId) -> Option<f64> {
        match self.get(b)?.target {
            Target::Days { value, .. } => Some(clean_days(value)),
            _ => None,
        }
    }

    /// The income target in months.
    pub(crate) fn months(&self, b: BucketId) -> Option<f64> {
        match self.get(b)?.target {
            Target::Months { value, .. } => {
                let m = round_dp(f64::from(value), 2);
                (m.is_finite() && m > 0.0).then_some(m)
            }
            _ => None,
        }
    }

    pub(crate) fn evacuate(&self) -> Option<Evac> {
        match self.get(BucketId::Evacuate)?.target {
            Target::Evacuate {
                notice_hours_low,
                notice_hours_high,
                days_away,
                ..
            } => Some(Evac {
                notice_hours_low: clean_days(notice_hours_low),
                notice_hours_high: clean_days(notice_hours_high),
                days_away: clean_days(days_away),
            }),
            _ => None,
        }
    }

    /// Whether any of `hazards` drives the bucket; `None` when the assessment lists no
    /// contributions (then the rules cover every case).
    pub(crate) fn driven_by(&self, b: BucketId, hazards: &[HazardId]) -> Option<bool> {
        let a = self.get(b)?;
        if a.contributions.is_empty() {
            return None;
        }
        Some(
            a.contributions
                .iter()
                .any(|c| c.share > 0.0 && hazards.contains(&c.hazard)),
        )
    }

    /// The target's own sources (hazard and duration data), cited after the supply sources.
    pub(crate) fn sources(&self, b: BucketId) -> &'a [CitationId] {
        self.get(b).map_or(&[], |a| a.sources.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_noise_is_cleaned() {
        assert_eq!(clean_days(1.4), 1.4);
        assert_eq!(clean_days(2.8), 2.8);
        assert_eq!(clean_days(-3.0), 0.0);
        assert_eq!(clean_days(f32::NAN), 0.0);
        assert_eq!(clean_days(f32::INFINITY), 0.0);
    }
}
