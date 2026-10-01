//! End to end on every fixture household: the pipeline runs, every plan item cites, spending
//! never outruns the money, and the outputs are well formed. The binder's and the Prepare
//! sheet's own checks are in `tests/binder.rs`.

mod common;

use std::collections::BTreeSet;

use common::outputs;
use rr_types::{BucketId, BucketKind, PlanItemKind, Target};

#[test]
fn every_fixture_assesses() {
    assert_eq!(outputs().len(), rr_types::fixtures::RAW.len());
    for (name, _, out) in outputs() {
        assert_eq!(out.api_version, rr_types::ENGINE_API_VERSION, "{name}");
        assert_eq!(out.content_version, rr_content::CONTENT_VERSION, "{name}");
        assert_eq!(
            Some(out.data_pack_version.as_str()),
            rr_data::DataStore::pack_version(common::engine().store()),
            "{name}: planned against the data packs"
        );
        assert!(
            out.location.data_note.as_deref() != Some(rr_plan::source::FIXTURE_DATA_NOTE),
            "{name}: real county records, not the sample counties"
        );
        assert_eq!(out.buckets.len(), 15, "{name}"); // 15 since contract v2 (clean_air)
        let ids: Vec<BucketId> = out.buckets.iter().map(|b| b.id).collect();
        assert_eq!(ids, BucketId::ALL, "{name}: buckets in BucketId order");
        assert!(!out.register.is_empty(), "{name}");
        assert!(!out.requirements.is_empty(), "{name}");
        assert!(!out.plan.months.is_empty(), "{name}");
    }
}

#[test]
fn every_plan_item_cites_a_source_in_the_provenance() {
    let content = rr_content::content();
    for (name, _, out) in outputs() {
        let provenance: BTreeSet<&str> = out.provenance.iter().map(|c| c.id.as_str()).collect();
        for m in &out.plan.months {
            for it in &m.items {
                let item = content
                    .item(it.item_id.as_str())
                    .unwrap_or_else(|| panic!("{name}: {} is not a catalogue item", it.item_id));
                assert!(
                    !item.citations.is_empty(),
                    "{name}: {} has no citation",
                    item.id
                );
                for c in &item.citations {
                    assert!(
                        provenance.contains(c.as_str()),
                        "{name}: {} cites {c}, missing from provenance",
                        item.id
                    );
                }
                assert!(!it.why.is_empty(), "{name}: {} has no why", it.item_id);
            }
        }
    }
}

#[test]
fn every_referenced_citation_is_in_the_provenance_once() {
    for (name, _, out) in outputs() {
        let ids: Vec<&str> = out.provenance.iter().map(|c| c.id.as_str()).collect();
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(
            unique.len(),
            ids.len(),
            "{name}: duplicate provenance entries"
        );
        let mut referenced: Vec<&str> = Vec::new();
        for p in &out.register {
            referenced.extend(p.sources.iter().map(|c| c.as_str()));
        }
        for b in &out.buckets {
            referenced.extend(b.sources.iter().map(|c| c.as_str()));
            if let Some(r) = &b.relief {
                referenced.extend(r.sources.iter().map(|c| c.as_str()));
            }
        }
        for s in &out.scenarios {
            referenced.extend(s.sources.iter().map(|c| c.as_str()));
        }
        for l in &out.requirements {
            referenced.extend(l.citations.iter().map(|c| c.as_str()));
        }
        for id in referenced {
            assert!(
                unique.contains(id),
                "{name}: {id} is referenced but not in provenance"
            );
        }
        for c in &out.provenance {
            assert!(!c.url.is_empty() && !c.title.is_empty(), "{name}: {}", c.id);
        }
    }
}

#[test]
fn spending_never_outruns_the_money() {
    for (name, input, out) in outputs() {
        let mut money = 0.0_f64;
        let mut spent = 0.0_f64;
        for m in &out.plan.months {
            money += f64::from(m.budget_usd);
            for it in &m.items {
                if it.kind == PlanItemKind::Purchase && !it.done {
                    spent += f64::from(it.est_cost_usd);
                }
            }
            assert!(
                spent <= money + 0.05,
                "{name}: by month {} spent ${spent:.2} of ${money:.2}",
                m.index
            );
        }
        let f = &input.finances;
        let months = out.plan.months.len().saturating_sub(1) as f64;
        let most = f64::from(f.one_off_budget_usd) + f64::from(f.monthly_budget_usd) * months;
        assert!(spent <= most + 0.05, "{name}: ${spent:.2} over ${most:.2}");
    }
}

#[test]
fn targets_come_from_consequence_and_tiers_from_supply() {
    for (name, input, out) in outputs() {
        let a = common::run(input);
        for (b, c) in out.buckets.iter().zip(&a.consequence.buckets) {
            assert_eq!(b.id, c.id);
            match (b.target, c.target) {
                (
                    Target::Days { value, low, high },
                    Target::Days {
                        value: v,
                        low: l,
                        high: h,
                    },
                ) => {
                    assert_eq!((value, low, high), (v, l, h), "{name} {}", b.id)
                }
                (Target::Months { value, .. }, Target::Months { value: v, .. }) => {
                    assert_eq!(value, v, "{name} {}", b.id)
                }
                _ => {}
            }
            assert_eq!(b.tier_enough, rr_supply::tier_enough(b), "{name} {}", b.id);
            assert_eq!(b.covered.kind(), b.target.kind(), "{name} {}", b.id);
            if let (Target::Days { value, .. }, Target::Days { value: c, .. }) =
                (b.target, b.covered)
            {
                assert!(
                    c <= value + 1e-6,
                    "{name} {}: covered {c} > target {value}",
                    b.id
                );
            }
        }
        assert_eq!(
            out.tier_recommended,
            rr_supply::tier_recommended(&out.buckets),
            "{name}"
        );
        let ids: Vec<&str> = out.warnings.iter().map(|w| w.id.as_str()).collect();
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(
            unique.len(),
            ids.len(),
            "{name}: duplicate warning ids {ids:?}"
        );
        let envelopes: Vec<&str> = out
            .plan
            .envelopes
            .iter()
            .map(|e| e.item_id.as_str())
            .collect();
        let unique: BTreeSet<&str> = envelopes.iter().copied().collect();
        assert_eq!(
            unique.len(),
            envelopes.len(),
            "{name}: one envelope per item"
        );
    }
}

/// `docs/RISK_MODEL.md` § "End to end with rr-hazards' rates": Philadelphia at the default dial
/// gets power 3 d (the research's 2.8; the pack's pooled restoration curves, passed to the model
/// since v0.2.0, bring it back from 5), boil-water 5 d, no tap water 5 d, food 14 d, heat or cold
/// 3 d, medicine 3 weeks (a daily prescription, v0.2.0), phone 3 d, income 4 months; one month is
/// the step that is enough.
#[test]
fn philadelphia_targets_match_the_risk_model_report() {
    let (_, _, out) = outputs()
        .iter()
        .find(|(n, _, _)| *n == "philadelphia-renters-4")
        .unwrap();
    let days = |b: BucketId| match out.buckets.iter().find(|x| x.id == b).unwrap().target {
        Target::Days { value, .. } => value,
        Target::Months { value, .. } => value,
        _ => f32::NAN,
    };
    assert_eq!(days(BucketId::Power), 3.0);
    assert_eq!(days(BucketId::WaterBoil), 5.0);
    assert_eq!(days(BucketId::WaterOut), 5.0);
    assert_eq!(days(BucketId::Supplies), 14.0);
    assert_eq!(days(BucketId::Thermal), 3.0);
    assert_eq!(days(BucketId::Medication), 21.0);
    assert_eq!(days(BucketId::Comms), 3.0);
    assert_eq!(days(BucketId::Income), 4.0);
    assert_eq!(out.tier_recommended, rr_types::TierId::M1);
    // Water: 21.6 gallons for four people and a dog over 5 days (4.32 a day, as the 12.9 over 3
    // days of v0.1.1; docs/RISK_MODEL.md, Supply sizing), met by the free reused bottles and
    // bottled water.
    let water = out
        .requirements
        .iter()
        .find(|l| l.id == "water_out.water_gallons")
        .unwrap();
    assert!((water.quantity - 21.6).abs() < 0.05, "{}", water.quantity);
}

#[test]
fn duration_buckets_the_plan_covers_reach_their_targets() {
    for (name, input, out) in outputs() {
        if out.plan.done_month.is_none() {
            continue;
        }
        let _ = input;
        for b in out
            .buckets
            .iter()
            .filter(|b| b.id.kind() == BucketKind::Duration)
        {
            if let (Target::Days { value, .. }, Target::Days { value: c, .. }) =
                (b.target, b.covered)
            {
                assert!(
                    (c - value).abs() < 0.11,
                    "{name} {}: done but covered {c} of {value}",
                    b.id
                );
            }
        }
    }
}

/// The binder keeps only the hazards that apply here: a Philadelphia rowhouse gets no avalanche or
/// tsunami page and no avalanche advice; Coos Bay, in a tsunami zone, gets the tsunami page.
#[test]
fn the_binder_keeps_only_the_hazards_that_apply_here() {
    let binder = |name: &str| {
        outputs()
            .iter()
            .find(|(n, _, _)| *n == name)
            .map(|(_, _, o)| o.binder.clone())
            .unwrap()
    };
    let phl = binder("philadelphia-renters-4");
    let md = rr_plan::binder::markdown::render(&phl).to_lowercase();
    assert!(
        !md.contains("avalanche"),
        "avalanche advice in Philadelphia"
    );
    assert!(phl.page("check_tsunami").is_none() && phl.page("check_avalanche").is_none());
    let coos = binder("coos-bay-well-owner-2");
    assert!(coos.page("check_tsunami").is_some(), "the tsunami page");
}
