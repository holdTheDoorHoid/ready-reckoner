//! Documents, insurance and savings (research §8). The emergency fund feeds the savings track,
//! never the supplies budget.

use rr_types::{Finances, Housing, Per, Tenure};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{count, num, usd};

/// The Emergency Financial First Aid Kit: copies of the household's key papers, kept safe and
/// updated. Rule `document_kit`.
pub fn document_kit() -> Sizing {
    let mut b = Basis::new();
    let parts = b.k(keys::EFFAK_PARTS);
    let text = format!(
        "Copy the {} parts of FEMA's Emergency Financial First Aid Kit: IDs, financial and legal papers (including insurance), medical information, and household contacts. Keep copies in a waterproof, fire-resistant place or with someone you trust, with photos of your home and belongings, and update them once a year.",
        num(parts, 0)
    );
    Sizing::new(
        &b,
        "document_kit",
        "document_kit",
        1.0,
        "kit",
        Per::Household,
        text,
    )
}

/// Decide on homeowners or renters insurance when the household has none. Rule
/// `insurance_home_or_renters`.
pub fn insurance_home_or_renters(finances: &Finances, housing: &Housing) -> Option<Sizing> {
    if finances.insurance.home_or_renters {
        return None;
    }
    let mut b = Basis::new();
    b.cite("fema_effak");
    b.cite("ready_gov_financial");
    let kind = if housing.tenure == Tenure::Rent {
        "renters"
    } else {
        "homeowners"
    };
    let text = format!(
        "You have no {kind} insurance yet: get a quote and decide. Check what it pays if you have to live somewhere else for a while."
    );
    Some(Sizing::new(
        &b,
        "insurance_home_or_renters",
        "insurance_home_or_renters",
        1.0,
        "decision",
        Per::Household,
        text,
    ))
}

/// Decide on flood insurance when the household has none: standard policies exclude floods, and a
/// new policy usually waits 30 days. Rule `insurance_flood`.
pub fn insurance_flood(finances: &Finances, housing: &Housing) -> Option<Sizing> {
    if finances.insurance.flood {
        return None;
    }
    let mut b = Basis::new();
    let wait = b.k(keys::NFIP_WAIT_DAYS);
    let building = b.k(keys::NFIP_BUILDING_LIMIT_USD);
    let contents = b.k(keys::NFIP_CONTENTS_LIMIT_USD);
    b.cite("ready_gov_financial");
    let renter = housing.tenure == Tenure::Rent;
    let text = format!(
        "Home and renters policies usually don't cover floods. Decide on flood insurance before you need it: a new policy usually starts {} days after you buy it. It pays up to {} for the building and {} for belongings{}.",
        num(wait, 0),
        usd(building),
        usd(contents),
        if renter {
            ", and renters can buy cover for belongings only"
        } else {
            ""
        }
    );
    Some(Sizing::new(
        &b,
        "insurance_flood",
        "insurance_flood",
        1.0,
        "decision",
        Per::Household,
        text,
    ))
}

/// Decide on earthquake insurance when earthquakes drive the household's risk and it has none.
/// Rule `insurance_earthquake`.
pub fn insurance_earthquake(finances: &Finances) -> Option<Sizing> {
    if finances.insurance.earthquake {
        return None;
    }
    let mut b = Basis::new();
    b.cite("ready_gov_earthquakes");
    let text = "A standard home policy does not cover earthquake damage. Decide whether earthquake insurance is worth its deductible where you live.".to_owned();
    Some(Sizing::new(
        &b,
        "insurance_earthquake",
        "insurance_earthquake",
        1.0,
        "decision",
        Per::Household,
        text,
    ))
}

/// Months of expenses to hold in savings: the household's income target, or 3 months when there is
/// none. Rule `emergency_fund_months` (the savings track).
pub fn emergency_fund_months(target_months: Option<f64>, finances: &Finances) -> Sizing {
    let mut b = Basis::new();
    let default = b.k(keys::EMERGENCY_FUND_MONTHS);
    let (lo, hi) = b.range(keys::EMERGENCY_FUND_MONTHS);
    b.cite("cfpb_emergency_fund");
    let months = target_months.unwrap_or(default);
    let money = finances
        .monthly_expenses_usd
        .map(f64::from)
        .filter(|e| e.is_finite() && *e > 0.0)
        .map_or(String::new(), |e| {
            format!(" ({} at {} a month)", usd(months * e), usd(e))
        });
    let have = f64::from(finances.emergency_fund_months).max(0.0);
    let text = format!(
        "Savings goal: about {} of expenses{money}, built up over time and kept apart from the supplies budget. You have about {} saved now. Planners often suggest {} to {} months; the CFPB says it depends on your situation.",
        count(crate::format::round_dp(months, 1), "month", "months"),
        count(crate::format::round_dp(have, 1), "month", "months"),
        num(lo, 0),
        num(hi, 0)
    );
    Sizing::new(
        &b,
        "emergency_fund_months",
        "emergency_fund",
        months,
        "month",
        Per::Household,
        text,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    #[test]
    fn philadelphia_documents_and_insurance() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        assert_eq!(document_kit().quantity, 1.0);
        let r = insurance_home_or_renters(&p.finances, &p.housing).unwrap();
        assert!(r.plain.contains("renters insurance"));
        let f = insurance_flood(&p.finances, &p.housing).unwrap();
        assert!(
            f.plain.contains("30 days")
                && f.plain.contains("$250,000")
                && f.plain.contains("$100,000")
        );
        assert!(f.plain.contains("belongings only"));
        let e = emergency_fund_months(Some(3.8), &p.finances);
        assert_eq!(e.quantity, 3.8);
        assert!(e.plain.contains("$15,960"), "{}", e.plain);
        assert!(e.plain.contains("0.5 months saved"));
    }

    #[test]
    fn held_insurance_needs_no_decision() {
        let m = fixtures::get("miami-condo-retiree-1").unwrap();
        assert!(insurance_home_or_renters(&m.finances, &m.housing).is_none());
        assert!(insurance_flood(&m.finances, &m.housing).is_none());
        assert!(insurance_earthquake(&m.finances).is_some());
        assert_eq!(emergency_fund_months(None, &m.finances).quantity, 3.0);
    }
}
