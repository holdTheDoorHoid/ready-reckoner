//! Property tests on random households (seeded, so every run checks the same cases): more people
//! or more days never mean less, hot never less than temperate, survival ≤ basic ≤ comfortable,
//! and the same input always gives the same lines.

mod common;

use std::collections::BTreeMap;

use rr_supply::{LineKind, SizedLine, SupplyContext, sized_requirements};
use rr_types::rng::SplitMix64;
use rr_types::{
    AgeBand, BucketAssessment, BucketId, Commute, CommuteMode, Medical, Mobility, Person, Pets,
    PlanInput, PoweredDevice, Target, WaterLevel,
};

const CASES: u64 = 300;

fn pick<T: Copy>(r: &mut SplitMix64, xs: &[T]) -> T {
    xs[r.next_below(xs.len() as u64) as usize]
}

fn person(r: &mut SplitMix64) -> Person {
    let band = pick(r, AgeBand::ALL);
    let adult = matches!(band, AgeBand::Adult);
    let device = pick(
        r,
        &[
            PoweredDevice::None,
            PoweredDevice::None,
            PoweredDevice::Cpap,
            PoweredDevice::Oxygen,
            PoweredDevice::Other { watts: 45.0 },
        ],
    );
    let commute =
        (matches!(band, AgeBand::Adult | AgeBand::Teen) && r.next_f64() < 0.6).then(|| Commute {
            distance_km: (r.next_f64() * 40.0) as f32,
            mode: pick(
                r,
                &[
                    CommuteMode::Car,
                    CommuteMode::Transit,
                    CommuteMode::Walk,
                    CommuteMode::Bike,
                ],
            ),
            remote_possible: r.next_f64() < 0.5,
        });
    Person {
        age_band: band,
        pregnant_or_nursing: adult && r.next_f64() < 0.2,
        medical: Medical {
            daily_rx: r.next_f64() < 0.3,
            refrigerated_rx: r.next_f64() < 0.1,
            powered_device: device,
            mobility: pick(
                r,
                &[
                    Mobility::None,
                    Mobility::None,
                    Mobility::Limited,
                    Mobility::Wheelchair,
                ],
            ),
            dietary: if band == AgeBand::Infant && r.next_f64() < 0.5 {
                vec!["formula".into()]
            } else {
                Vec::new()
            },
            epinephrine: r.next_f64() < 0.1,
        },
        earner: false,
        commute,
        access_needs: Vec::new(),
    }
}

fn household(r: &mut SplitMix64) -> PlanInput {
    let mut input = rr_types::fixtures::get("philadelphia-renters-4").unwrap();
    let n = 1 + r.next_below(7) as usize;
    input.people = (0..n).map(|_| person(r)).collect();
    input.pets = Pets {
        dogs: r.next_below(3) as u8,
        cats: r.next_below(3) as u8,
        small: r.next_below(2) as u8,
        large_animals: if r.next_f64() < 0.2 {
            r.next_below(10) as u8
        } else {
            0
        },
    };
    input.dials.water_level = pick(r, WaterLevel::ALL);
    input.housing = rr_types::fixtures::get(pick(
        r,
        &[
            "philadelphia-renters-4",
            "coos-bay-well-owner-2",
            "miami-condo-retiree-1",
            "phoenix-apartment-cpap-1",
        ],
    ))
    .unwrap()
    .housing;
    input
}

fn targets(r: &mut SplitMix64) -> Vec<BucketAssessment> {
    let ladder = rr_types::TARGET_LADDER_DAYS;
    let d = |r: &mut SplitMix64| ladder[r.next_below(ladder.len() as u64) as usize];
    vec![
        common::days(BucketId::Power, d(r)),
        common::days(BucketId::WaterBoil, d(r)),
        common::days(BucketId::WaterOut, d(r)),
        common::days(BucketId::Supplies, d(r)),
        common::days(BucketId::Thermal, d(r)),
        common::days(BucketId::Medication, d(r)),
        common::days(BucketId::Comms, d(r)),
        common::evacuate(r.next_f64() * 0.2, (r.next_f64() * 48.0) as f32, 72.0, d(r)),
        common::readiness(BucketId::GetHome, r.next_f64()),
        common::readiness(BucketId::MedicalEmergency, r.next_f64()),
        common::readiness(BucketId::Fire, r.next_f64()),
        common::readiness(BucketId::Security, r.next_f64()),
        common::months((r.next_f64() * 12.0) as f32),
        common::readiness(BucketId::HomeLoss, r.next_f64()),
    ]
}

fn needs(lines: &[SizedLine]) -> BTreeMap<String, f32> {
    lines
        .iter()
        .filter(|l| l.kind == LineKind::Need)
        .map(|l| (l.line.id.clone(), l.line.quantity))
        .collect()
}

/// Every need in `small` is also in `big`, with at least the same quantity.
fn no_less(small: &[SizedLine], big: &[SizedLine], what: &str) {
    let (s, b) = (needs(small), needs(big));
    for (id, q) in &s {
        let Some(qb) = b.get(id) else {
            panic!("{what}: line {id} disappeared");
        };
        assert!(qb >= q, "{what}: {id} fell from {q} to {qb}");
    }
}

fn scale_days(ts: &[BucketAssessment], f: f32) -> Vec<BucketAssessment> {
    ts.iter()
        .cloned()
        .map(|mut b| {
            if let Target::Days { value, .. } = b.target {
                let v = value * f;
                b.target = Target::Days {
                    value: v,
                    low: v,
                    high: v,
                };
            }
            b
        })
        .collect()
}

#[test]
fn more_people_never_mean_less() {
    let mut r = SplitMix64::new(0x5eed_0001);
    let ctx = SupplyContext::default();
    for case in 0..CASES {
        let input = household(&mut r);
        let ts = targets(&mut r);
        let mut bigger = input.clone();
        bigger.people.push(person(&mut r));
        no_less(
            &sized_requirements(&input, &ts, &ctx),
            &sized_requirements(&bigger, &ts, &ctx),
            &format!("case {case}"),
        );
        let mut more_pets = input.clone();
        more_pets.pets.dogs += 1;
        more_pets.pets.large_animals += 1;
        no_less(
            &sized_requirements(&input, &ts, &ctx),
            &sized_requirements(&more_pets, &ts, &ctx),
            &format!("case {case} pets"),
        );
    }
}

#[test]
fn more_days_never_mean_less() {
    let mut r = SplitMix64::new(0x5eed_0002);
    let ctx = SupplyContext::default();
    for case in 0..CASES {
        let input = household(&mut r);
        let ts = targets(&mut r);
        let longer = scale_days(&ts, 1.0 + (r.next_f64() * 3.0) as f32);
        no_less(
            &sized_requirements(&input, &ts, &ctx),
            &sized_requirements(&input, &longer, &ctx),
            &format!("case {case}"),
        );
    }
}

#[test]
fn hot_is_never_less_than_temperate() {
    let mut r = SplitMix64::new(0x5eed_0003);
    for case in 0..CASES {
        let input = household(&mut r);
        let ts = targets(&mut r);
        let temperate = sized_requirements(&input, &ts, &SupplyContext::default());
        let hot = sized_requirements(
            &input,
            &ts,
            &SupplyContext {
                days_at_or_above_95f: Some(100.0),
                latitude: None,
                ..SupplyContext::default()
            },
        );
        no_less(&temperate, &hot, &format!("case {case}"));
        let water = |ls: &[SizedLine]| needs(ls).get("water_out.water_gallons").copied();
        assert!(water(&hot) >= water(&temperate), "case {case}");
    }
}

#[test]
fn survival_basic_comfortable_are_ordered() {
    let mut r = SplitMix64::new(0x5eed_0004);
    let ctx = SupplyContext::default();
    for case in 0..CASES {
        let mut input = household(&mut r);
        let ts = targets(&mut r);
        let mut by_level = Vec::new();
        for level in [
            WaterLevel::Survival,
            WaterLevel::Basic,
            WaterLevel::Comfortable,
        ] {
            input.dials.water_level = level;
            by_level.push(sized_requirements(&input, &ts, &ctx));
        }
        for w in by_level.windows(2) {
            // Every water quantity (stored, treated beyond the stored days, the go-bags) rises
            // with the level; nothing else changes.
            no_less(&w[0], &w[1], &format!("case {case}"));
        }
    }
}

#[test]
fn same_input_same_lines() {
    let mut r = SplitMix64::new(0x5eed_0005);
    for _ in 0..50 {
        let input = household(&mut r);
        let ts = targets(&mut r);
        let ctx = SupplyContext {
            days_at_or_above_95f: Some(r.next_f64() * 100.0),
            latitude: Some(r.next_f64() * 60.0 + 20.0),
            ..SupplyContext::default()
        };
        let a = sized_requirements(&input, &ts, &ctx);
        let b = sized_requirements(&input, &ts, &ctx);
        assert_eq!(a, b);
        assert_eq!(
            serde_json::to_string(&a.iter().map(|l| &l.line).collect::<Vec<_>>()).unwrap(),
            serde_json::to_string(&b.iter().map(|l| &l.line).collect::<Vec<_>>()).unwrap()
        );
    }
}

#[test]
fn every_line_cites_on_random_households() {
    let mut r = SplitMix64::new(0x5eed_0006);
    let known = rr_supply::citations_used();
    for case in 0..CASES {
        let input = household(&mut r);
        let ts = targets(&mut r);
        let ctx = SupplyContext {
            days_at_or_above_95f: Some(r.next_f64() * 100.0),
            latitude: Some(r.next_f64() * 60.0 + 20.0),
            ..SupplyContext::default()
        };
        let lines = sized_requirements(&input, &ts, &ctx);
        let mut ids = std::collections::BTreeSet::new();
        for l in &lines {
            assert!(
                ids.insert(&l.line.id),
                "case {case}: duplicate {}",
                l.line.id
            );
            assert!(!l.line.citations.is_empty(), "case {case}: {}", l.line.id);
            assert!(
                l.line
                    .citations
                    .iter()
                    .all(|c| known.contains(c) || c == common::TARGET_SOURCE),
                "case {case}: {}",
                l.line.id
            );
            assert!(
                l.line.quantity.is_finite() && l.line.quantity >= 0.0,
                "case {case}: {}",
                l.line.id
            );
        }
        let notes = lines
            .iter()
            .filter(|l| l.line.plain.contains(rr_supply::ONE_MONTH_NOTE))
            .count();
        let m1 = lines
            .iter()
            .any(|l| l.kind == LineKind::Need && l.tier == rr_types::TierId::M1);
        assert_eq!(
            notes,
            usize::from(m1),
            "case {case}: the one-month note is said once when needed"
        );
    }
}
