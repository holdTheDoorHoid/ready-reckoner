//! Tier logic (DESIGN §4.5): which tier covers a number of days, which tier is enough for each
//! bucket, and the tier a household should reach.

use rr_types::{BucketAssessment, BucketId, BucketKind, Target, TierId};

use crate::basis::Basis;
use crate::constants::{constants, keys};
use crate::targets::clean_days;

/// The smallest tier that covers `days` days of self-sufficiency: 0 is `now` (free actions only),
/// up to 3 is `h72`, up to 14 `w2`, up to 30 `m1`, up to 90 `m3`, up to 180 `m6`, and anything
/// longer `y1` (DESIGN §4.5: a tier enters the plan when a target exceeds the tier before it).
pub fn tier_for_days(days: f64) -> TierId {
    if !(days > 0.0) {
        return TierId::Now;
    }
    TierId::ALL
        .iter()
        .copied()
        .find(|t| *t != TierId::Now && days <= f64::from(t.days()))
        .unwrap_or(TierId::Y1)
}

/// The tier that is enough for one bucket; the plan stops adding to the bucket there.
///
/// Duration buckets: the tier that covers the target's days. `income`: `m3`, where the savings
/// track sits (it is never funded from the supplies budget). `home_loss`: `now` (documents and
/// insurance decisions). Other readiness buckets: `h72` (the go-bag and get-home bag come right
/// after the three-day basics) when the ten-year chance of need is at least the readiness
/// threshold (2 %, an estimate), otherwise `now`.
pub fn tier_enough(b: &BucketAssessment) -> TierId {
    match b.target {
        Target::Days { value, .. } => tier_for_days(clean_days(value)),
        Target::Months { .. } => TierId::M3,
        Target::Evacuate { p_need_10yr, .. } | Target::Readiness { p_need_10yr, .. } => {
            if b.id == BucketId::HomeLoss {
                TierId::Now
            } else if p_need_10yr >= constants().value(keys::READINESS_P_NEED_THRESHOLD) {
                TierId::H72
            } else {
                TierId::Now
            }
        }
    }
}

/// The tier the household should reach: the highest tier any duration or readiness bucket needs,
/// and never less than three days (DESIGN §4.5: `h72` is always in the plan). Money buckets sit on
/// the savings track and do not raise it.
pub fn tier_recommended(buckets: &[BucketAssessment]) -> TierId {
    buckets
        .iter()
        .filter(|b| b.id.kind() != BucketKind::Money)
        .map(tier_enough)
        .max()
        .unwrap_or(TierId::H72)
        .max(TierId::H72)
}

/// Said once in the plan, on the first line that reaches the one-month tier.
pub const ONE_MONTH_NOTE: &str = "No agency sets a one-month amount: this step sits between the two-week advice (Red Cross, Oregon, Washington) and the Church's three-month pantry.";

/// The sources behind [`ONE_MONTH_NOTE`].
pub(crate) fn one_month_basis() -> Basis {
    let mut b = Basis::new();
    for id in [
        "red_cross_survival_kit",
        "oregon_2_weeks_ready",
        "wa_emd_prepare_in_a_year",
        "church_food_storage_2007",
    ] {
        b.cite(id);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::TARGET_LADDER_DAYS;

    #[test]
    fn days_map_to_the_design_tiers() {
        let cases = [
            (0.0, TierId::Now),
            (0.5, TierId::H72),
            (3.0, TierId::H72),
            (3.5, TierId::W2),
            (14.0, TierId::W2),
            (21.0, TierId::M1),
            (30.0, TierId::M1),
            (45.0, TierId::M3),
            (90.0, TierId::M3),
            (180.0, TierId::M6),
            (365.0, TierId::Y1),
            (1000.0, TierId::Y1),
            (-2.0, TierId::Now),
            (f64::NAN, TierId::Now),
        ];
        for (d, t) in cases {
            assert_eq!(tier_for_days(d), t, "{d} days");
        }
    }

    #[test]
    fn tiers_never_fall_as_days_grow() {
        let mut prev = TierId::Now;
        for d in TARGET_LADDER_DAYS {
            let t = tier_for_days(f64::from(d));
            assert!(t >= prev, "{d}");
            assert!(
                f64::from(t.days()) >= f64::from(d),
                "tier {t} covers {d} days"
            );
            prev = t;
        }
    }

    #[test]
    fn the_one_month_note_cites_its_sources() {
        let b = one_month_basis();
        assert_eq!(b.cites().len(), 4);
        assert!(ONE_MONTH_NOTE.contains("No agency sets a one-month amount"));
    }
}
