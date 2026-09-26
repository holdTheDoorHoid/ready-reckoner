//! Documents, insurance and savings (research §8). The emergency fund feeds the savings track,
//! never the supplies budget.

use rr_types::{Finances, Housing, HousingKind, Per, Person, Tenure};

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

/// Whether the home is an apartment the household owns (a condominium or co-op unit).
fn owned_apartment(housing: &Housing) -> bool {
    housing.tenure == Tenure::Own
        && matches!(
            housing.kind,
            HousingKind::ApartmentHighRise | HousingKind::ApartmentLowRise
        )
}

/// Decide on homeowners, condominium or renters insurance when the household has none. Rule
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
    } else if owned_apartment(housing) {
        b.cite("naic_home_insurance_guide");
        "condominium unit-owners"
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

/// Decide on flood insurance when the household has none: standard policies exclude floods, a new
/// policy usually waits 30 days, and nearly a third of flood claims (29 %, 2014–2024) come from
/// outside high-risk flood areas (FloodSmart), which is why the decision is shown to every
/// household without a policy, not only those in a flood zone (round-2 review RR-P09). Rule
/// `insurance_flood`.
pub fn insurance_flood(
    finances: &Finances,
    housing: &Housing,
    sfha_home_share: Option<f64>,
) -> Option<Sizing> {
    if finances.insurance.flood {
        return None;
    }
    let mut b = Basis::new();
    let wait = b.k(keys::NFIP_WAIT_DAYS);
    let building = b.k(keys::NFIP_BUILDING_LIMIT_USD);
    let contents = b.k(keys::NFIP_CONTENTS_LIMIT_USD);
    let outside = b.k(keys::NFIP_CLAIMS_OUTSIDE_HIGH_RISK_SHARE);
    b.cite("ready_gov_financial");
    let renter = housing.tenure == Tenure::Rent;
    // Most homes outside the mapped zone: say so, because that is where the surprise comes from.
    let zone = match sfha_home_share {
        Some(s) if s.is_finite() && s < 0.5 => {
            b.cite("openfema_nfip");
            " Most homes in your county are outside the mapped high-risk flood zone, and that is where many uninsured floods happen:"
        }
        _ => "",
    };
    let text = format!(
        "Home and renters policies usually don't cover floods. Decide on flood insurance before you need it:{zone} about {} in 10 flood insurance claims come from outside high-risk flood areas. A quote costs nothing, and a new policy usually starts {} days after you buy it. It pays up to {} for the building and {} for belongings{}.",
        num((outside * 10.0).round(), 0),
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

/// Check the hurricane or windstorm deductible when hurricanes drive the household's home-loss risk
/// and it has a home or renters policy: in some places it is a percentage of the home's insured
/// value, not a dollar amount, and coastal policies can leave out wind entirely (NAIC; round-2
/// review RR-P09). Rule `insurance_wind_deductible`.
pub fn insurance_wind_deductible(finances: &Finances) -> Option<Sizing> {
    if !finances.insurance.home_or_renters {
        return None;
    }
    let mut b = Basis::new();
    b.cite("naic_home_insurance_guide");
    let text = "Hurricanes drive your home-loss risk: check your policy for a separate hurricane or windstorm deductible. In some places it is a percentage of the home's insured value rather than a dollar amount, and coastal policies can leave out wind damage altogether. Find the amount on the declarations page and add it to your savings goal.".to_owned();
    Some(Sizing::new(
        &b,
        "insurance_wind_deductible",
        "insurance_wind_deductible",
        1.0,
        "decision",
        Per::Household,
        text,
    ))
}

/// Decide on sewer and drain backup cover when the home has a basement, the household has a home
/// or renters policy, and it has not said it holds the cover: most policies pay little or nothing
/// for water that backs up through drains or an overflowing sump pump (NAIC; round-2 review RR-P09).
/// Rule `insurance_sewer_backup`.
pub fn insurance_sewer_backup(finances: &Finances, housing: &Housing) -> Option<Sizing> {
    let held = finances.insurance.sewer_backup == Some(true);
    if !housing.basement || !finances.insurance.home_or_renters || held {
        return None;
    }
    let mut b = Basis::new();
    b.cite("naic_home_insurance_guide");
    let text = "Your home has a basement. Most home policies pay little or nothing for water that backs up through the drains or a sump pump that overflows: ask your insurer what a sewer and drain backup endorsement would cost, and decide.".to_owned();
    Some(Sizing::new(
        &b,
        "insurance_sewer_backup",
        "insurance_sewer_backup",
        1.0,
        "decision",
        Per::Household,
        text,
    ))
}

/// Check a condominium unit-owners policy when the household owns an apartment and has a policy:
/// the unit-owners form covers belongings and the unit's own walls, floors and ceilings (NAIC), so
/// check that is the form it holds and what the association's policy leaves to the owner (round-2
/// review RR-P09). Rule `insurance_condo_unit`.
pub fn insurance_condo_unit(finances: &Finances, housing: &Housing) -> Option<Sizing> {
    if !owned_apartment(housing) || !finances.insurance.home_or_renters {
        return None;
    }
    let mut b = Basis::new();
    b.cite("naic_home_insurance_guide");
    let text = "You own an apartment: check that your policy is a condominium unit-owners policy, which covers your belongings and your own walls, floors and ceilings, and ask your association what its building policy leaves to you after a disaster.".to_owned();
    Some(Sizing::new(
        &b,
        "insurance_condo_unit",
        "insurance_condo_unit",
        1.0,
        "decision",
        Per::Household,
        text,
    ))
}

/// Decide on disability and life insurance when someone earns money for the household and it has
/// not said it holds them: the register's "death or disability of an earner" needs a paired action
/// (round-2 review RR-P09). A 20-year-old worker has about a 1-in-4 chance of a disability before
/// retirement age (SSA). Rule `insurance_life_disability` (bucket `income`).
pub fn insurance_life_disability(people_list: &[Person], finances: &Finances) -> Option<Sizing> {
    let earners = people_list.iter().filter(|p| p.earner).count();
    let held = finances.insurance.life_or_disability == Some(true);
    if earners == 0 || held {
        return None;
    }
    let mut b = Basis::new();
    let share = b.k(keys::DISABILITY_BEFORE_RETIREMENT_SHARE);
    b.cite("naic_life_insurance_guide");
    let others = people_list.len() > earners;
    let who = match (earners, others) {
        (1, true) => "Someone in your household earns money the others rely on",
        (1, false) => "You earn your household's money",
        (_, true) => "More than one of you earns money the household relies on",
        (_, false) => "You each earn money the household relies on",
    };
    let life = if others {
        ", and whether life insurance would carry the others through the years they would need"
    } else {
        ""
    };
    let text = format!(
        "{who}: check whether your job offers disability insurance{life}. About {} in 4 workers who start at 20 become disabled before retirement age. NAIC's buyer's guide helps you work out how much cover is enough.",
        num((share * 4.0).round(), 0)
    );
    Some(Sizing::new(
        &b,
        "insurance_life_disability",
        "insurance_life_disability",
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
        let f = insurance_flood(&p.finances, &p.housing, None).unwrap();
        assert!(
            f.plain.contains("30 days")
                && f.plain.contains("$250,000")
                && f.plain.contains("$100,000")
        );
        assert!(f.plain.contains("belongings only"));
        // Round-2 review RR-P09: nearly a third of claims come from outside high-risk areas.
        assert!(
            f.plain.contains("about 3 in 10 flood insurance claims"),
            "{}",
            f.plain
        );
        assert!(f.citations.iter().any(|c| c == "floodsmart_flood_risk"));
        let low = insurance_flood(&p.finances, &p.housing, Some(0.05)).unwrap();
        assert!(
            low.plain
                .contains("outside the mapped high-risk flood zone")
        );
        let e = emergency_fund_months(Some(3.8), &p.finances);
        assert_eq!(e.quantity, 3.8);
        assert!(e.plain.contains("$15,960"), "{}", e.plain);
        assert!(e.plain.contains("0.5 months saved"));
    }

    #[test]
    fn held_insurance_needs_no_decision() {
        let m = fixtures::get("miami-condo-retiree-1").unwrap();
        assert!(insurance_home_or_renters(&m.finances, &m.housing).is_none());
        assert!(insurance_flood(&m.finances, &m.housing, None).is_none());
        assert!(insurance_earthquake(&m.finances).is_some());
        assert_eq!(emergency_fund_months(None, &m.finances).quantity, 3.0);
        // A condo owner with a policy checks it is the unit-owners form; nobody earns: no
        // disability decision.
        let c = insurance_condo_unit(&m.finances, &m.housing).unwrap();
        assert!(
            c.plain.contains("walls, floors and ceilings"),
            "{}",
            c.plain
        );
        assert!(insurance_life_disability(&m.people, &m.finances).is_none());
        assert!(insurance_wind_deductible(&m.finances).is_some());
    }

    /// Round-2 review RR-P09: four new decisions, each gated on the household's own facts.
    #[test]
    fn insurance_decisions_follow_the_household() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        // Renters with no policy: the home decision first; no sewer, wind or condo check yet.
        assert!(insurance_sewer_backup(&p.finances, &p.housing).is_none());
        assert!(insurance_wind_deductible(&p.finances).is_none());
        assert!(insurance_condo_unit(&p.finances, &p.housing).is_none());
        // Two earners and a child: the disability and life decision, with SSA's 1 in 4.
        let l = insurance_life_disability(&p.people, &p.finances).unwrap();
        assert!(l.plain.contains("About 1 in 4"), "{}", l.plain);
        assert!(l.citations.iter().any(|c| c == "ssa_disability_facts"));
        // Hays owns a house with a basement and a policy: the sewer-backup decision.
        let hays = fixtures::get("hays-kansas-farm-5").unwrap();
        let s = insurance_sewer_backup(&hays.finances, &hays.housing).unwrap();
        assert!(s.plain.contains("sump pump"), "{}", s.plain);
        let mut held = hays.finances.clone();
        held.insurance.sewer_backup = Some(true);
        held.insurance.life_or_disability = Some(true);
        assert!(insurance_sewer_backup(&held, &hays.housing).is_none());
        assert!(insurance_life_disability(&hays.people, &held).is_none());
        // A condo owner with no policy is asked about unit-owners insurance by name.
        let mut m = fixtures::get("miami-condo-retiree-1").unwrap();
        m.finances.insurance.home_or_renters = false;
        let h = insurance_home_or_renters(&m.finances, &m.housing).unwrap();
        assert!(h.plain.contains("condominium unit-owners"), "{}", h.plain);
    }
}
