//! Properties: the same input gives the same bytes; a more cautious dial never lowers a target;
//! more money never ends the plan with less coverage; more people never need less water; a zero
//! budget gives free steps only, with a savings suggestion.

mod common;

use common::{assess, engine, household};
use rr_types::{
    AgeBand, BucketId, BucketKind, Medical, Mobility, Person, PlanItemKind, PoweredDevice,
    ReturnPeriod, Target,
};

fn fresh() -> &'static rr_plan::Engine<rr_data::DataStore> {
    static FRESH: std::sync::OnceLock<rr_plan::Engine<rr_data::DataStore>> =
        std::sync::OnceLock::new();
    FRESH.get_or_init(|| rr_plan::Engine::with_data_dir(rr_plan::golden::data_dir()).unwrap())
}

#[test]
fn the_same_input_gives_byte_identical_output() {
    for (name, input) in rr_types::fixtures::all() {
        let a = rr_plan::to_json(&assess(&input));
        let b = rr_plan::to_json(&assess(&input));
        assert!(a == b, "{name}: two runs differ");
        // A freshly loaded engine too (no state carried between calls or loads).
        let c = rr_plan::to_json(&fresh().assess(&input).unwrap());
        assert!(a == c, "{name}: a second engine differs");
    }
}

#[test]
fn the_json_parses_back_and_has_no_widened_floats() {
    let out = assess(&household("philadelphia-renters-4"));
    let json = rr_plan::to_json(&out);
    let back: rr_types::PlanOutput = serde_json::from_str(&json).expect("round trip");
    assert_eq!(back.buckets.len(), out.buckets.len());
    assert_eq!(back.packet_markdown, out.packet_markdown);
    // f32 fields (money, days) print at their own width: "7.03", never "7.03000020980835".
    for line in json.lines().filter(|l| l.contains("\"est_cost_usd\"")) {
        let number = line.split(':').nth(1).unwrap().trim().trim_end_matches(',');
        assert!(number.len() <= 9, "an f32 printed as f64: {line}");
    }
    // No object in the contract is a map, so field order is the struct's.
    assert!(json.find("\"engine_version\"").unwrap() < json.find("\"provenance\"").unwrap());
}

fn targets(out: &rr_types::PlanOutput) -> Vec<(BucketId, f32)> {
    out.buckets
        .iter()
        .filter_map(|b| match b.target {
            Target::Days { value, .. } | Target::Months { value, .. } => Some((b.id, value)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_more_cautious_dial_never_lowers_a_target() {
    for (name, input) in rr_types::fixtures::all() {
        let mut previous: Option<Vec<(BucketId, f32)>> = None;
        for rp in ReturnPeriod::ALL {
            let mut i = input.clone();
            i.dials.return_period = *rp;
            let t = targets(&assess(&i));
            if let Some(prev) = &previous {
                for ((b, before), (_, now)) in prev.iter().zip(&t) {
                    assert!(
                        now >= before,
                        "{name} {b}: {before} at the setting before, {now} at {rp}"
                    );
                }
            }
            previous = Some(t);
        }
    }
}

fn covered(out: &rr_types::PlanOutput) -> Vec<(BucketId, f32)> {
    out.buckets
        .iter()
        .filter(|b| b.id.kind() == BucketKind::Duration)
        .map(|b| match b.covered {
            Target::Days { value, .. } => (b.id, value),
            _ => (b.id, 0.0),
        })
        .collect()
}

#[test]
fn more_money_never_ends_the_plan_with_less_coverage() {
    for name in [
        "philadelphia-renters-4",
        "phoenix-apartment-cpap-1",
        "chicago-student-zero-budget-1",
    ] {
        let base = household(name);
        let mut previous: Option<Vec<(BucketId, f32)>> = None;
        for monthly in [0.0_f32, 25.0, 60.0, 150.0, 400.0] {
            let mut i = base.clone();
            i.finances.monthly_budget_usd = monthly;
            let c = covered(&assess(&i));
            if let Some(prev) = &previous {
                for ((b, before), (_, now)) in prev.iter().zip(&c) {
                    assert!(
                        *now + 1e-4 >= *before,
                        "{name} {b}: {before} days covered at the smaller budget, {now} at ${monthly}"
                    );
                }
            }
            previous = Some(c);
        }
    }
}

#[test]
fn more_people_never_need_less_water() {
    let base = household("philadelphia-renters-4");
    let water = |i: &rr_types::PlanInput| -> f32 {
        assess(i)
            .requirements
            .iter()
            .find(|l| l.id == "water_out.water_gallons")
            .map_or(0.0, |l| l.quantity)
    };
    let before = water(&base);
    let mut more = base.clone();
    more.people.push(Person {
        age_band: AgeBand::Adult,
        pregnant_or_nursing: false,
        medical: Medical {
            daily_rx: false,
            refrigerated_rx: false,
            powered_device: PoweredDevice::None,
            mobility: Mobility::None,
            dietary: Vec::new(),
            epinephrine: false,
        },
        earner: false,
        commute: None,
    });
    assert!(water(&more) > before);
}

#[test]
fn a_zero_budget_gives_free_steps_only_and_a_savings_suggestion() {
    let input = household("chicago-student-zero-budget-1");
    assert_eq!(input.finances.monthly_budget_usd, 0.0);
    assert_eq!(input.finances.one_off_budget_usd, 0.0);
    let out = assess(&input);
    let mut free = 0;
    for m in &out.plan.months {
        for it in &m.items {
            assert_eq!(
                it.kind,
                PlanItemKind::FreeAction,
                "month {}: {} is not free",
                m.index,
                it.item_id
            );
            assert_eq!(it.est_cost_usd, 0.0);
            free += 1;
        }
    }
    assert!(free >= 8, "only {free} free steps");
    assert!(out.plan.envelopes.is_empty());
    // The savings suggestion: the plan's own step and the guardrail note.
    assert!(
        out.plan
            .months
            .iter()
            .flat_map(|m| &m.items)
            .any(|i| i.item_id == "docs_start_emergency_fund"),
        "no emergency-fund step"
    );
    assert!(out.warnings.iter().any(|w| w.id == "zero_budget"));
    assert!(out.packet_markdown.contains("even a few dollars a month"));
}

#[test]
fn free_steps_are_at_most_eight_a_month_and_life_safety_first() {
    for (name, input) in rr_types::fixtures::all() {
        let out = assess(&input);
        for m in &out.plan.months {
            let free: Vec<&rr_types::PlanItem> = m
                .items
                .iter()
                .filter(|i| i.kind == PlanItemKind::FreeAction && !i.done)
                .collect();
            assert!(
                free.len() <= 8,
                "{name} month {}: {} free steps",
                m.index,
                free.len()
            );
        }
        let first: Vec<&str> = out.plan.months[0]
            .items
            .iter()
            .filter(|i| i.kind == PlanItemKind::FreeAction)
            .map(|i| i.item_id.as_str())
            .collect();
        let content = rr_content::content();
        // Life-safety free steps come before the others in month 0.
        let flags: Vec<bool> = first
            .iter()
            .map(|id| content.item(id).is_some_and(|i| i.life_safety))
            .collect();
        let first_other = flags.iter().position(|f| !f).unwrap_or(flags.len());
        assert!(
            flags[first_other..].iter().all(|f| !f),
            "{name}: life-safety steps not first: {first:?}"
        );
    }
}

#[test]
fn a_powered_device_makes_its_backup_life_safety() {
    let a = common::run(&household("phoenix-apartment-cpap-1"));
    for id in ["power_device_battery", "power_station"] {
        let o = a
            .offers
            .get(id)
            .unwrap_or_else(|| panic!("{id} not offered"));
        assert!(o.item.life_safety, "{id}");
    }
    let _ = engine();
}
