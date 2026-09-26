//! rr-budget v0.2.0 through the real pipeline and catalogue (brief-budget.md), on every fixture
//! household and the seven pending ones: prerequisites, the two done months, bare-minimum mode,
//! decisions by month 1, the rare allowance by family, the long-horizon section, the season rule
//! and the practitioner's orderings (practitioner review P-05, P-06, P-12).

mod common;

use std::sync::OnceLock;

use common::{assess, engine, household};
use rr_plan::Assessment;
use rr_types::{PlanInput, PlanItemKind, PlanOutput};

/// Every fixture and pending household with its assessment (computed once).
fn every_household() -> &'static Vec<(&'static str, PlanInput, Assessment, PlanOutput)> {
    static ALL: OnceLock<Vec<(&'static str, PlanInput, Assessment, PlanOutput)>> = OnceLock::new();
    ALL.get_or_init(|| {
        rr_types::fixtures::all()
            .into_iter()
            .chain(rr_types::fixtures::pending())
            .map(|(name, input)| {
                let a = engine()
                    .run(&input)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                let out = engine()
                    .assess(&input)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                (name, input, a, out)
            })
            .collect()
    })
}

/// The first month an item is in the plan (bought, owned, done or a free step).
fn first_month(out: &PlanOutput, id: &str) -> Option<u16> {
    out.plan
        .months
        .iter()
        .find(|m| {
            m.items
                .iter()
                .any(|i| i.item_id == id && i.kind != PlanItemKind::Reserve)
        })
        .map(|m| m.index)
}

/// P-12: no accessory is bought before one of the devices it names (a headlamp, lantern or radio
/// for batteries; the bucket for toilet bags; diapers for wipes; the stove for its fuel), unless
/// the catalogue offers none of them, when the household's own device (its generator) is why the
/// accessory is offered.
#[test]
fn accessories_never_precede_their_devices_on_the_real_catalogue() {
    let mut checked = 0;
    for (name, _, a, out) in every_household() {
        for m in &out.plan.months {
            for (pos, it) in m.items.iter().enumerate() {
                if it.kind != PlanItemKind::Purchase || it.requires.is_empty() || it.done {
                    continue;
                }
                checked += 1;
                if it
                    .requires
                    .iter()
                    .all(|d| a.offers.get(d.as_str()).is_none())
                {
                    continue;
                }
                let earlier = out.plan.months.iter().filter(|x| x.index < m.index);
                let before = earlier
                    .flat_map(|x| &x.items)
                    .chain(m.items[..pos].iter())
                    .any(|x| it.requires.contains(&x.item_id) && x.kind != PlanItemKind::Reserve);
                assert!(
                    before,
                    "{name}: {} in month {} before any of {:?}",
                    it.item_id, m.index, it.requires
                );
            }
        }
    }
    assert!(checked >= 20, "only {checked} accessory purchases");
}

/// Every plan reports its two done months, in order, and bare-minimum mode follows its rule: on
/// with the dial or (warning `plan_too_long`) when the full plan would run past three years; what
/// the warning names is not bought by month 36.
#[test]
fn two_done_months_and_the_bare_minimum_rule() {
    for (name, input, _, out) in every_household() {
        let p = &out.plan;
        if let (Some(k), Some(d)) = (p.minimum_done_month, p.done_month) {
            assert!(k <= d, "{name}: kit month {k} after the done month {d}");
        }
        if input.finances.monthly_budget_usd + input.finances.one_off_budget_usd > 0.0 {
            assert!(p.minimum_done_month.is_some(), "{name}: no kit month");
        }
        let too_long = out.warnings.iter().find(|w| w.id == "plan_too_long");
        assert_eq!(
            p.minimum_kit,
            input.dials.minimum_kit || too_long.is_some(),
            "{name}"
        );
        if let Some(w) = too_long {
            for id in &w.related {
                let bought_by_36 = p
                    .months
                    .iter()
                    .filter(|m| m.index <= 36)
                    .flat_map(|m| &m.items)
                    .any(|i| i.item_id.as_str() == id.as_str() && i.kind == PlanItemKind::Purchase);
                let bought_later = p
                    .months
                    .iter()
                    .filter(|m| m.index > 36)
                    .flat_map(|m| &m.items)
                    .any(|i| i.item_id.as_str() == id.as_str() && i.kind == PlanItemKind::Purchase);
                assert!(
                    bought_later || !bought_by_36,
                    "{name}: {id} is named as waiting but bought by month 36"
                );
            }
        }
    }
    // The SNAP household at $10 a month (model review M-12) asked for the bare minimum: its kit
    // is complete within the first year, although the whole plan never finishes.
    let (_, _, _, detroit) = every_household()
        .iter()
        .find(|(n, ..)| *n == "detroit-snap-3")
        .unwrap();
    assert!(detroit.plan.minimum_kit && detroit.plan.done_month.is_none());
    assert!(detroit.plan.minimum_done_month.is_some_and(|m| m <= 12));
    assert!(detroit.warnings.iter().any(|w| w.id == "benefit_lapse"));
}

/// Decisions (insurance, ID, home repairs) are free steps scheduled by month 1 and marked as
/// decisions; no plan lists more than eight other free steps in a month.
#[test]
fn decisions_come_by_month_1_marked_as_decisions() {
    let content = rr_content::content();
    for (name, _, _, out) in every_household() {
        let mut decisions = 0;
        for m in &out.plan.months {
            let mut ordinary = 0;
            for it in &m.items {
                let decision = content
                    .item(it.item_id.as_str())
                    .is_some_and(|i| i.decision);
                assert_eq!(it.decision, decision, "{name}: {}", it.item_id);
                if it.decision {
                    decisions += 1;
                    assert!(m.index <= 1, "{name}: {} in month {}", it.item_id, m.index);
                } else if it.kind == PlanItemKind::FreeAction
                    && !it.done
                    && !rr_budget::EXEMPT_FREE_STEPS.contains(&it.item_id.as_str())
                    && !content
                        .item(it.item_id.as_str())
                        .is_some_and(|i| i.long_horizon)
                {
                    ordinary += 1;
                }
            }
            assert!(
                ordinary <= 8,
                "{name}: {ordinary} free steps in month {}",
                m.index
            );
        }
        assert!(decisions >= 1, "{name}: no decisions");
    }
}

/// P-05, P-06, P-12 on the practitioner's two households. Philadelphia ($60 a month) has stored
/// water, three days of medicine, bleach, a light and its batteries (after it) by month 1, the
/// extinguishers by month 2 and the first-aid kit by month 6; Hays gets emergency alerts in month 0,
/// and its wipes wait for the diapers and the toilet bags for the bucket.
#[test]
fn the_practitioners_orderings() {
    let phl = assess(&household("philadelphia-renters-4"));
    let by =
        |out: &PlanOutput, id: &str, month: u16| first_month(out, id).is_some_and(|m| m <= month);
    for id in [
        "water_stored_bottled",
        "med_reserve_supply",
        "water_bleach_unscented",
        "power_headlamp",
        "power_batteries",
    ] {
        assert!(by(&phl, id, 1), "Philadelphia: {id} by month 1");
    }
    assert!(by(&phl, "fire_extinguisher", 2));
    assert!(by(&phl, "med_first_aid_kit", 6));
    assert!(first_month(&phl, "san_toilet_bags") >= first_month(&phl, "san_twin_bucket_toilet"));

    let hays = assess(&household("hays-kansas-farm-5"));
    assert_eq!(first_month(&hays, "comms_wea_alerts_on"), Some(0));
    assert!(first_month(&hays, "san_baby_wipes") >= first_month(&hays, "san_diapers"));
    assert!(first_month(&hays, "san_toilet_bags") >= first_month(&hays, "san_twin_bucket_toilet"));
    assert!(first_month(&hays, "power_batteries") >= first_month(&hays, "power_headlamp"));
}

/// The season rule: with a 1 October planning date, the battery fan lands before summer (June is
/// month 8) where the cost order allows.
#[test]
fn fans_arrive_before_summer_where_the_cost_order_allows() {
    for name in ["philadelphia-renters-4", "hays-kansas-farm-5"] {
        let out = assess(&household(name));
        let m = first_month(&out, "thermal_battery_fan").expect("a fan");
        assert!(m < 8, "{name}: fan in month {m}");
    }
}

/// REVIEW §2.4: the allowance buys only for ticked families likely enough here. The dosimeter card
/// goes with the nuclear family; Faraday storage with the months-long blackout family (an
/// electromagnetic pulse reaches far beyond any blast zone; planner, 2026-09-26). Minot (a missile
/// field, class A) ticked the nuclear and solar-storm families: the card comes after the three-day
/// life-safety purchases and says what it is for, and there is no Faraday storage (the blackout
/// family is not ticked). Coos Bay ticking every family buys nothing (the nuclear family about 1.2
/// and the blackout family about 6.9 in 10,000 over ten years). Philadelphia ticking every family
/// buys both (the blackout family about 1.07 in 1,000).
#[test]
fn the_rare_allowance_by_family_on_the_fixtures() {
    let (_, _, a, minot) = every_household()
        .iter()
        .find(|(n, ..)| *n == "minot-missile-field-3")
        .unwrap();
    let card = minot
        .plan
        .months
        .iter()
        .find_map(|m| {
            m.items
                .iter()
                .find(|i| i.item_id == "rare_radiation_meter" && i.kind == PlanItemKind::Purchase)
                .map(|i| (m.index, i))
        })
        .expect("the dosimeter card");
    assert!(
        card.1.why.contains("nuclear attack row you ticked"),
        "{}",
        card.1.why
    );
    let basics = a
        .budget
        .sequence
        .iter()
        .filter(|p| !p.rare_catastrophic && p.tier == rr_types::TierId::H72)
        .filter(|p| {
            a.offers
                .get(p.item_id.as_str())
                .is_some_and(|o| o.item.life_safety)
        })
        .map(|p| p.month)
        .max()
        .unwrap_or(0);
    assert!(
        card.0 > basics,
        "card in month {}, basics until {basics}",
        card.0
    );

    let mut coos = household("coos-bay-well-owner-2");
    coos.dials.rare_opt_in = vec!["all".into()];
    let out = assess(&coos);
    assert!(
        !out.plan.months.iter().flat_map(|m| &m.items).any(|i| i
            .item_id
            .as_str()
            .starts_with("rare_")
            && i.kind == PlanItemKind::Purchase),
        "class E: the nuclear family is under 1 in 1,000 over ten years"
    );

    assert_eq!(first_month(minot, "rare_faraday_storage"), None);

    let mut phl = household("philadelphia-renters-4");
    phl.dials.rare_opt_in = vec!["all".into()];
    let out = assess(&phl);
    for id in ["rare_radiation_meter", "rare_faraday_storage"] {
        assert!(first_month(&out, id).is_some(), "Philadelphia: {id}");
    }
    let bag = out
        .plan
        .months
        .iter()
        .flat_map(|m| &m.items)
        .find(|i| i.item_id == "rare_faraday_storage" && i.kind == PlanItemKind::Purchase)
        .unwrap();
    assert_eq!(bag.hazards, [rr_types::HazardId::MultiMonthBlackout]);
    assert!(
        bag.why.contains("power out for months row you ticked"),
        "{}",
        bag.why
    );
    // Ticking the nuclear family alone no longer buys it.
    let mut nuclear_only = household("philadelphia-renters-4");
    nuclear_only.dials.rare_opt_in = vec!["nuclear_attack".into()];
    let out = assess(&nuclear_only);
    assert!(first_month(&out, "rare_radiation_meter").is_some());
    assert_eq!(first_month(&out, "rare_faraday_storage"), None);
}

/// The long-horizon section lists the flagged items when a target reaches 30 days or the
/// household asks, and every one of them is still in the months.
#[test]
fn the_long_horizon_section_groups_without_dropping() {
    let content = rr_content::content();
    for (name, input, _, out) in every_household() {
        let long = input.dials.long_horizon
            || out
                .buckets
                .iter()
                .any(|b| matches!(b.target, rr_types::Target::Days { value, .. } if value >= 30.0));
        if !long {
            assert!(out.plan.long_horizon.is_empty(), "{name}");
            continue;
        }
        for it in &out.plan.long_horizon {
            assert!(
                content
                    .item(it.item_id.as_str())
                    .is_some_and(|i| i.long_horizon),
                "{name}: {}",
                it.item_id
            );
            assert!(
                first_month(out, it.item_id.as_str()).is_some(),
                "{name}: {}",
                it.item_id
            );
        }
    }
    let coos = every_household()
        .iter()
        .find(|(n, ..)| *n == "coos-bay-well-owner-2")
        .unwrap();
    assert!(!coos.3.plan.long_horizon.is_empty());
}
