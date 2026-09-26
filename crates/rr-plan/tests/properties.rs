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
        // What the household has (listed, or an everyday basic the plan assumes) is not bought.
        for it in m.items.iter().filter(|i| !i.done) {
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

/// Days of a days-kind coverage (0 for other kinds), with `low` and `high` checked equal to it.
fn plain_days(t: &Target) -> f32 {
    match *t {
        Target::Days { value, low, high } => {
            assert!(value == low && value == high, "{t:?}");
            value
        }
        _ => 0.0,
    }
}

/// `covered_today` is what the household has before the plan buys anything: same kind as the
/// target, never more than the plan's end point, and never more than the target in days.
#[test]
fn coverage_today_never_exceeds_the_plans() {
    for (name, _, out) in common::outputs() {
        for b in &out.buckets {
            assert_eq!(b.covered_today.kind(), b.target.kind(), "{name} {}", b.id);
            match (b.target, b.covered, b.covered_today) {
                (Target::Days { value, .. }, end, now) => {
                    let (end, now) = (plain_days(&end), plain_days(&now));
                    assert!(
                        0.0 <= now && now <= end && end <= value,
                        "{name} {}: today {now}, plan {end}, target {value}",
                        b.id
                    );
                }
                (
                    Target::Readiness { of, .. },
                    Target::Readiness { done: end, .. },
                    Target::Readiness {
                        done: now,
                        of: now_of,
                        ..
                    },
                ) => {
                    assert_eq!(now_of, of, "{name} {}", b.id);
                    assert!(now <= end, "{name} {}: {now} > {end}", b.id);
                }
                (
                    Target::Evacuate { days_away, .. },
                    Target::Evacuate { days_away: end, .. },
                    Target::Evacuate { days_away: now, .. },
                ) => {
                    assert!(now == 0.0 || now == days_away, "{name}");
                    assert!(
                        now <= end,
                        "{name}: a go-bag today but not at the plan's end"
                    );
                }
                (Target::Months { .. }, end, now) => assert_eq!(end, now, "{name} {}", b.id),
                other => panic!("{name} {}: {other:?}", b.id),
            }
        }
    }
}

/// A household that has already done every step of its plan has, today, what the plan covers;
/// a household that has done none of it has only what it owns.
#[test]
fn coverage_today_follows_what_the_household_has_done() {
    for name in ["philadelphia-renters-4", "hays-kansas-farm-5"] {
        let input = household(name);
        let out = assess(&input);
        // Every step at the quantity the plan gives it, one entry per item (the UI merges
        // check-offs the same way).
        let mut done: std::collections::BTreeMap<String, f32> = Default::default();
        for m in &out.plan.months {
            for it in m.items.iter().filter(|i| i.kind != PlanItemKind::Reserve) {
                *done.entry(it.item_id.as_str().to_owned()).or_default() += it.quantity;
            }
        }
        let mut all_done = input.clone();
        all_done.existing = done
            .into_iter()
            .map(|(id, qty)| rr_types::Owned {
                item_id: id.as_str().into(),
                qty,
                paid_usd: None,
            })
            .collect();
        let after = assess(&all_done);
        for (b, a) in out.buckets.iter().zip(&after.buckets) {
            if b.id.kind() == BucketKind::Duration {
                assert_eq!(
                    plain_days(&a.covered_today),
                    plain_days(&b.covered),
                    "{name} {}: today once every step is done",
                    b.id
                );
            }
        }
        // Nothing listed and no basics assumed: today covers only buckets with nothing to buy.
        let mut none = input.clone();
        none.existing.clear();
        none.assume_basics = false;
        let bare = common::run(&none);
        for b in &bare.buckets {
            if b.id.kind() == BucketKind::Duration && !bare.offers.rule.parts_of(b.id).is_empty() {
                assert_eq!(plain_days(&b.covered_today), 0.0, "{name} {}", b.id);
            }
        }
    }
}

/// The packet's "Spend" (verification V-18): a month's money out counts a deposit once and a
/// purchase paid from savings only for the rest, so it never exceeds the month's budget plus what
/// earlier months left unspent, and never more than the month's lines add up to.
#[test]
fn a_months_spend_never_exceeds_its_budget_plus_what_earlier_months_left() {
    for (name, input) in rr_types::fixtures::all() {
        let a = common::run(&input);
        let (mut budget, mut spent) = (0.0_f64, 0.0_f64);
        for m in &a.budget.plan.months {
            budget += f64::from(m.budget_usd);
            let s = a.month_spend(m.index);
            assert!(
                spent + s <= budget + 0.01,
                "{name} month {}: spends {s:.2} with {:.2} available",
                m.index,
                budget - spent
            );
            spent += s;
            let lines: f64 = m
                .items
                .iter()
                .filter(|i| !i.done && i.kind != PlanItemKind::FreeAction)
                .map(|i| f64::from(i.est_cost_usd))
                .sum();
            assert!(s <= lines + 1e-6, "{name} month {}", m.index);
        }
    }
    // Philadelphia, month 16 (v0.1.1 order): the last deposit toward the cash reserve, the $10 of
    // the $100 cash not yet saved, the pet food and the fans are $80 of that month's $60 plus what
    // earlier months left, not $170.
    let a = common::run(&household("philadelphia-renters-4"));
    // Displayed as $80 (the exact figure carries the price bands' cents).
    assert!(
        (a.month_spend(16) - 80.0).abs() < 0.5,
        "{}",
        a.month_spend(16)
    );
    assert!(a.input.finances.monthly_budget_usd >= 40.0);
    let packet = assess(&household("philadelphia-renters-4")).packet_markdown;
    assert!(
        packet.contains(
            "| 16 (February 2028) | save toward cash in small bills; Cash in small bills: $100, \
             $90 of it from savings; Extra pet food in an airtight container: 5 pounds of dry \
             food; Battery or rechargeable fan: 2 fans | $80 |"
        ),
        "the table row"
    );
}
