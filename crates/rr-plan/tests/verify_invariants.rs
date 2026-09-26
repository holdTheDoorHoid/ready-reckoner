//! Invariants under randomised households (docs/VERIFICATION.md §2).
//!
//! A seeded generator (`rr_types::rng::SplitMix64`) builds households across every county of the
//! data pack and checks, for each one: it plans without a panic; `assess` is fast (release
//! builds); money spent never exceeds money available; every plan step, requirement line,
//! hazard and target cites a source that resolves; targets never fall as the dial gets more
//! cautious; one more person never needs less water or food; a well makes the no-water target at
//! least the power target; the confidence and stage answers never move a target; assuming
//! everyday basics never credits less coverage; more money never ends with less coverage; and
//! two runs give the same bytes.
//!
//! The default run is small so `cargo test` stays quick. The verification run:
//!
//! ```text
//! RR_VERIFY_HOUSEHOLDS=500 cargo test --release -p rr-plan --test verify_invariants -- --nocapture
//! ```

mod common;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::Instant;

use common::engine;
use rr_types::rng::SplitMix64;
use rr_types::{BucketId, BucketKind, PlanInput, PlanItemKind, PlanOutput, ReturnPeriod, Target};
use serde_json::{Value, json};

fn pick<'a, T>(r: &mut SplitMix64, xs: &'a [T]) -> &'a T {
    &xs[r.next_below(xs.len() as u64) as usize]
}

fn chance(r: &mut SplitMix64, p: f64) -> bool {
    r.next_f64() < p
}

fn person(r: &mut SplitMix64, adult: bool) -> Value {
    let age = if adult {
        *pick(r, &["adult", "adult", "adult", "senior"])
    } else {
        *pick(
            r,
            &["infant", "toddler", "child", "teen", "adult", "senior"],
        )
    };
    let grown = matches!(age, "adult" | "senior");
    let device: Value = if chance(r, 0.06) {
        match r.next_below(3) {
            0 => json!("cpap"),
            1 => json!("oxygen"),
            _ => json!({ "other": { "watts": 20.0 + 200.0 * r.next_f64() } }),
        }
    } else {
        json!("none")
    };
    let earner = grown && chance(r, if age == "adult" { 0.75 } else { 0.15 });
    let mut dietary: Vec<&str> = Vec::new();
    if age == "infant" && chance(r, 0.5) {
        dietary.push("formula");
    }
    let mut p = json!({
        "age_band": age,
        "pregnant_or_nursing": age == "adult" && chance(r, 0.08),
        "medical": {
            "daily_rx": chance(r, if age == "senior" { 0.6 } else { 0.15 }),
            "refrigerated_rx": chance(r, 0.05),
            "powered_device": device,
            "mobility": *pick(r, &["none", "none", "none", "none", "limited", "wheelchair"]),
            "dietary": dietary,
            "epinephrine": chance(r, 0.04),
        },
        "earner": earner,
    });
    if earner && chance(r, 0.8) {
        p["commute"] = json!({
            "distance_km": (80.0 * r.next_f64() * r.next_f64() * 10.0).round() / 10.0,
            "mode": *pick(r, &["car", "car", "transit", "walk", "bike"]),
            "remote_possible": chance(r, 0.3),
        });
    }
    p
}

/// One random household in `fips`.
fn household(r: &mut SplitMix64, fips: &str, item_ids: &[String]) -> Value {
    let n = 1 + r.next_below(6) as usize;
    let mut people: Vec<Value> = (0..n).map(|i| person(r, i == 0)).collect();
    if chance(r, 0.05) {
        people.push(person(r, false));
    }
    let earners = people.iter().filter(|p| p["earner"] == json!(true)).count();
    let kind = *pick(
        r,
        &[
            "apartment_high_rise",
            "apartment_low_rise",
            "rowhouse",
            "detached",
            "detached",
            "mobile_home",
            "rural_property",
        ],
    );
    let floor = match kind {
        "apartment_high_rise" => 1 + r.next_below(30) as i64,
        "apartment_low_rise" => 1 + r.next_below(4) as i64,
        _ => 1 + r.next_below(2) as i64,
    };
    let rural = matches!(kind, "rural_property");
    let water = if rural || chance(r, 0.12) {
        "well"
    } else {
        "municipal"
    };
    let mut existing: Vec<Value> = Vec::new();
    let mut used: Vec<&String> = Vec::new();
    for _ in 0..r.next_below(6) {
        let id = pick(r, item_ids);
        if used.contains(&id) {
            continue;
        }
        used.push(id);
        existing.push(json!({ "item_id": id, "qty": r.next_below(5) as f64 }));
    }
    let scenarios = [
        "cascadia_m9",
        "new_madrid_m7",
        "hayward_m7",
        "local_tsunami",
        "major_hurricane_direct_hit",
    ];
    let mut overrides: Vec<Value> = Vec::new();
    for s in scenarios {
        if chance(r, 0.1) {
            overrides.push(json!({ "id": s, "on": chance(r, 0.5) }));
        }
    }
    let mut h = json!({
        "planning_date": "2026-10-01",
        "location": {
            "country": "US",
            "county_fips": fips,
            "setting": if rural { "rural" } else { *pick(r, &["urban", "suburban", "rural"]) },
        },
        "housing": {
            "kind": kind,
            "tenure": *pick(r, &["own", "rent"]),
            "floor": floor,
            "basement": matches!(kind, "detached" | "rowhouse" | "rural_property") && chance(r, 0.5),
            "water": water,
            "sewer": if water == "well" || chance(r, 0.1) { "septic" } else { "sewer" },
            "heating": *pick(r, &["gas", "gas", "electric_resistance", "heat_pump", "oil", "propane", "wood", "district", "none"]),
            "cooling": *pick(r, &["central", "central", "window", "none"]),
            "backup_power": *pick(r, &["none", "none", "none", "power_station", "generator", "solar_battery"]),
            "alarms": { "smoke": chance(r, 0.8), "co": chance(r, 0.5), "extinguisher": chance(r, 0.4) },
        },
        "people": people,
        "pets": {
            "dogs": r.next_below(3),
            "cats": r.next_below(3),
            "small": if chance(r, 0.1) { 1 } else { 0 },
            "large_animals": if rural && chance(r, 0.3) { r.next_below(12) } else { 0 },
        },
        "mobility": { "vehicles": (0..r.next_below(4)).map(|_| json!({ "fuel": *pick(r, &["gas", "gas", "diesel", "hybrid", "ev"]) })).collect::<Vec<_>>() },
        "finances": {
            "monthly_budget_usd": *pick(r, &[0.0, 10.0, 25.0, 60.0, 150.0, 400.0, 1000.0]),
            "one_off_budget_usd": *pick(r, &[0.0, 0.0, 100.0, 500.0, 2500.0]),
            "emergency_fund_months": *pick(r, &[0.0, 0.5, 1.0, 3.0, 6.0]),
            "income": { "earners": earners, "stability": *pick(r, &["very_stable", "stable", "variable", "seasonal", "gig"]) },
            "insurance": { "home_or_renters": chance(r, 0.6), "flood": chance(r, 0.2), "earthquake": chance(r, 0.1) },
        },
        "existing": existing,
        "assume_basics": chance(r, 0.8),
        "dials": {
            "return_period": *pick(r, &["one_in_10", "one_in_50", "one_in_100", "one_in_100", "one_in_500"]),
            "climate": *pick(r, &["today", "today", "y2050"]),
            "horizon_years": if chance(r, 0.8) { 10 } else { 1 + r.next_below(50) },
            "water_level": *pick(r, &["survival", "basic", "basic", "comfortable"]),
            "scenario_overrides": overrides,
            "rare_catastrophic_opt_in": chance(r, 0.1),
        },
    });
    if chance(r, 0.7) {
        h["finances"]["monthly_expenses_usd"] = json!(1500.0 + 6000.0 * r.next_f64());
    }
    if chance(r, 0.5) {
        h["stage"] = json!(*pick(
            r,
            &[
                "not_thought_about",
                "thinking",
                "have_some_things",
                "have_a_plan",
                "maintaining"
            ]
        ));
        h["confidence_1to5"] = json!(1 + r.next_below(5));
    }
    h
}

fn parse(v: &Value) -> PlanInput {
    serde_json::from_value(v.clone()).expect("the generator builds valid JSON for PlanInput")
}

fn target_values(out: &PlanOutput) -> Vec<(BucketId, f32)> {
    out.buckets
        .iter()
        .filter_map(|b| match b.target {
            Target::Days { value, .. } | Target::Months { value, .. } => Some((b.id, value)),
            _ => None,
        })
        .collect()
}

fn days(out: &PlanOutput, id: BucketId) -> f32 {
    out.buckets
        .iter()
        .find(|b| b.id == id)
        .and_then(|b| match b.target {
            Target::Days { value, .. } => Some(value),
            _ => None,
        })
        .unwrap_or(0.0)
}

fn covered(out: &PlanOutput) -> Vec<(BucketId, f32)> {
    out.buckets
        .iter()
        .filter(|b| b.id.kind() == BucketKind::Duration)
        .map(|b| match b.covered {
            Target::Days { value, .. } => (b.id, value),
            _ => (b.id, 0.0),
        })
        .collect()
}

fn line_qty(out: &PlanOutput, id: &str) -> f32 {
    out.requirements
        .iter()
        .find(|l| l.id == id)
        .map_or(0.0, |l| l.quantity)
}

/// Money: by the end of every month, purchases never exceed the one-off budget plus the monthly
/// budget for months 1..=m. (Savings deposits sit in one sinking fund whose money can move to the
/// next top item, so a purchase may draw on deposits listed under another item; purchases alone
/// are the part that must fit.) Every dollar deposited is accounted for in `Plan.envelopes`.
fn money_problems(input: &PlanInput, out: &PlanOutput) -> Vec<String> {
    let mut problems = Vec::new();
    let one_off = f64::from(input.finances.one_off_budget_usd);
    let monthly = f64::from(input.finances.monthly_budget_usd);
    let mut bought = 0.0_f64;
    let mut deposited = 0.0_f64;
    for m in &out.plan.months {
        for it in &m.items {
            if it.done {
                continue;
            }
            let cost = f64::from(it.est_cost_usd);
            match it.kind {
                PlanItemKind::Reserve => deposited += cost,
                PlanItemKind::Purchase => bought += cost,
                PlanItemKind::FreeAction => {
                    if cost > 0.0 {
                        problems.push(format!(
                            "month {}: free step {} costs ${cost}",
                            m.index, it.item_id
                        ));
                    }
                }
            }
        }
        let available = one_off + monthly * f64::from(m.index);
        if bought > available + 0.01 + 1e-6 * available {
            problems.push(format!(
                "month {}: ${bought:.2} of purchases, ${available:.2} available",
                m.index
            ));
            break;
        }
    }
    let saved: f64 = out
        .plan
        .envelopes
        .iter()
        .map(|e| f64::from(e.saved_usd))
        .sum();
    if (saved - deposited).abs() > 0.05 + 1e-4 * deposited {
        problems.push(format!(
            "${deposited:.2} deposited in savings, envelopes account for ${saved:.2}"
        ));
    }
    problems
}

/// Every plan step's catalogue item cites; every line, hazard and target cites, and every id
/// the output cites is in `provenance`.
fn citation_problems(out: &PlanOutput) -> Vec<String> {
    let content = rr_content::content();
    let mut problems = Vec::new();
    let prov: std::collections::BTreeSet<&str> =
        out.provenance.iter().map(|c| c.id.as_str()).collect();
    let resolves = |id: &str| content.citation(id).is_some();
    for m in &out.plan.months {
        for it in &m.items {
            match content.item(it.item_id.as_str()) {
                None => problems.push(format!("plan step {} is not in the catalogue", it.item_id)),
                Some(item) => {
                    if item.citations.is_empty()
                        || !item.citations.iter().all(|c| resolves(c.as_str()))
                    {
                        problems.push(format!(
                            "plan step {} has no resolvable citation",
                            it.item_id
                        ));
                    }
                }
            }
        }
    }
    for l in &out.requirements {
        if l.citations.is_empty() {
            problems.push(format!("requirement {} cites nothing", l.id));
        }
        for c in &l.citations {
            if !prov.contains(c.as_str()) {
                problems.push(format!("requirement {} cites {c}, not in provenance", l.id));
            }
        }
    }
    for h in &out.register {
        if h.sources.is_empty() {
            problems.push(format!("hazard {} cites nothing", h.id));
        }
        for c in &h.sources {
            if !prov.contains(c.as_str()) {
                problems.push(format!("hazard {} cites {c}, not in provenance", h.id));
            }
        }
    }
    for b in &out.buckets {
        let has_target = match b.target {
            Target::Days { value, .. } | Target::Months { value, .. } => value > 0.0,
            Target::Evacuate { p_need_10yr, .. } | Target::Readiness { p_need_10yr, .. } => {
                p_need_10yr > 0.0
            }
        };
        if has_target && b.sources.is_empty() {
            problems.push(format!("bucket {} has a target but no source", b.id));
        }
        for c in &b.sources {
            if !prov.contains(c.as_str()) {
                problems.push(format!("bucket {} cites {c}, not in provenance", b.id));
            }
        }
    }
    for c in &out.provenance {
        if !resolves(c.id.as_str()) {
            problems.push(format!("provenance {} does not resolve", c.id));
        }
    }
    problems
}

fn shape_problems(out: &PlanOutput) -> Vec<String> {
    let mut problems = Vec::new();
    for b in &out.buckets {
        match (b.target, b.covered) {
            (Target::Days { value, low, high }, Target::Days { value: c, .. }) => {
                if !(low <= value && value <= high) {
                    problems.push(format!(
                        "{}: target {value} outside its range {low}–{high}",
                        b.id
                    ));
                }
                if value > 0.0 && !rr_types::TARGET_LADDER_DAYS.contains(&value) {
                    problems.push(format!("{}: target {value} is not on the day ladder", b.id));
                }
                if c > value + 1e-4 {
                    problems.push(format!("{}: covered {c} above target {value}", b.id));
                }
            }
            (Target::Months { value, low, high }, _) if !(low <= value && value <= high) => {
                problems.push(format!(
                    "{}: target {value} outside its range {low}–{high}",
                    b.id
                ));
            }
            _ => {}
        }
    }
    for bad in [
        "about 1 times",
        "about fewer than",
        "NaN",
        "inf days",
        "{frequency}",
        "{if:",
        "{/if}",
        "  .",
    ] {
        if out.packet_markdown.contains(bad) {
            problems.push(format!("packet contains {bad:?}"));
        }
    }
    problems
}

#[test]
fn randomised_households_keep_the_invariants() {
    let n: usize = std::env::var("RR_VERIFY_HOUSEHOLDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let seed: u64 = std::env::var("RR_VERIFY_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5eed_7e51);
    let e = engine();
    let mut counties: Vec<String> = e.store().counties().map(|c| c.fips.clone()).collect();
    counties.sort();
    let item_ids: Vec<String> = rr_content::content()
        .items
        .iter()
        .map(|i| i.id.as_str().to_owned())
        .collect();
    let release = !cfg!(debug_assertions);

    let mut report = String::new();
    let mut failures: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut slowest = (0.0_f64, String::new());
    let mut soft = String::new();
    // `RR_VERIFY_ONLY=<k>` reruns one household (its number from a report) and prints its JSON.
    let only: Option<usize> = std::env::var("RR_VERIFY_ONLY")
        .ok()
        .and_then(|s| s.parse().ok());
    for k in 0..n.max(only.map_or(0, |o| o + 1)) {
        if only.is_some_and(|o| o != k) {
            continue;
        }
        let mut r = SplitMix64::new(seed ^ (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let fips = pick(&mut r, &counties).clone();
        let json = household(&mut r, &fips, &item_ids);
        let input = parse(&json);
        if only.is_some() {
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        let tag = format!("household {k} (county {fips})");
        let mut fail = |what: &'static str, detail: String, report: &mut String| {
            *failures.entry(what).or_insert(0) += 1;
            let _ = writeln!(report, "{tag}: {what}: {detail}");
        };

        // 1. No panic, and it plans.
        let t = Instant::now();
        let result = std::panic::catch_unwind(|| e.assess(&input));
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let out = match result {
            Err(_) => {
                fail("panic", serde_json::to_string(&json).unwrap(), &mut report);
                continue;
            }
            Ok(Err(err)) => {
                fail("error", err.to_string(), &mut report);
                continue;
            }
            Ok(Ok(out)) => out,
        };
        if ms > slowest.0 {
            slowest = (ms, tag.clone());
        }
        // 2. Fast (release builds only).
        if release && ms >= 100.0 {
            fail("slow", format!("{ms:.1} ms"), &mut report);
        }
        // 3. Money.
        for p in money_problems(&input, &out) {
            fail("money", p, &mut report);
        }
        // 4. Citations.
        for p in citation_problems(&out) {
            fail("citation", p, &mut report);
        }
        for p in shape_problems(&out) {
            fail("shape", p, &mut report);
        }
        // 10. Determinism.
        let again = e.assess(&input).expect("second run");
        if rr_plan::to_json(&out) != rr_plan::to_json(&again) {
            fail("determinism", "two runs differ".to_owned(), &mut report);
        }
        // 5. Targets never fall as the dial gets more cautious.
        let mut previous: Option<(ReturnPeriod, Vec<(BucketId, f32)>)> = None;
        for rp in ReturnPeriod::ALL {
            let mut i = input.clone();
            i.dials.return_period = *rp;
            let t = target_values(&e.assess(&i).expect("dial variant"));
            if let Some((prp, prev)) = &previous {
                for ((b, before), (_, now)) in prev.iter().zip(&t) {
                    if now < before {
                        fail(
                            "dial",
                            format!("{b}: {before} at {prp}, {now} at {rp}"),
                            &mut report,
                        );
                    }
                }
            }
            previous = Some((*rp, t));
        }
        // 6. One more person never needs less water or food.
        {
            let mut more = json.clone();
            more["people"].as_array_mut().unwrap().push(json!({
                "age_band": "adult", "pregnant_or_nursing": false,
                "medical": { "daily_rx": false, "refrigerated_rx": false, "powered_device": "none",
                             "mobility": "none", "dietary": [], "epinephrine": false },
                "earner": false
            }));
            let bigger = e.assess(&parse(&more)).expect("more people");
            for id in ["water_out.water_gallons", "supplies.food_kcal"] {
                let (a, b) = (line_qty(&out, id), line_qty(&bigger, id));
                if b + 1e-3 < a {
                    fail(
                        "people",
                        format!(
                            "{id}: {a} for {} people, {b} with one more",
                            input.people.len()
                        ),
                        &mut report,
                    );
                }
            }
        }
        // 7. A well makes the no-water target at least the power target.
        {
            let mut w = json.clone();
            w["housing"]["water"] = json!("well");
            let o = e.assess(&parse(&w)).expect("well");
            let (p, wo) = (days(&o, BucketId::Power), days(&o, BucketId::WaterOut));
            if wo + 1e-4 < p {
                fail("well", format!("water_out {wo} < power {p}"), &mut report);
            }
        }
        // 8. The confidence and stage answers never move a target.
        {
            let mut c = json.clone();
            c["confidence_1to5"] = json!(5);
            c["stage"] = json!("maintaining");
            let mut d = json.clone();
            d["confidence_1to5"] = json!(1);
            d["stage"] = json!("not_thought_about");
            let (tc, td) = (
                target_values(&e.assess(&parse(&c)).expect("confident")),
                target_values(&e.assess(&parse(&d)).expect("unsure")),
            );
            if tc != td {
                fail(
                    "confidence",
                    format!("targets {tc:?} vs {td:?}"),
                    &mut report,
                );
            }
        }
        // 9. Assuming everyday basics never credits less coverage: with no money (only what the
        //    household has and the free steps), and with its own budget.
        {
            let mut on = json.clone();
            on["assume_basics"] = json!(true);
            let mut off = json.clone();
            off["assume_basics"] = json!(false);
            for (label, zero) in [("no money", true), ("its budget", false)] {
                let (mut a, mut b) = (on.clone(), off.clone());
                if zero {
                    for v in [&mut a, &mut b] {
                        v["finances"]["monthly_budget_usd"] = json!(0.0);
                        v["finances"]["one_off_budget_usd"] = json!(0.0);
                    }
                }
                let (ca, cb) = (
                    covered(&e.assess(&parse(&a)).expect("basics on")),
                    covered(&e.assess(&parse(&b)).expect("basics off")),
                );
                for ((bucket, with), (_, without)) in ca.iter().zip(&cb) {
                    if with + 1e-4 < *without {
                        let line = format!(
                            "{label}, {bucket}: {with} days with basics assumed, {without} without"
                        );
                        if zero {
                            fail("basics", line, &mut report);
                        } else {
                            let _ = writeln!(soft, "{tag}: basics ({line})");
                        }
                    }
                }
            }
        }
        // 11. More money never ends the plan with less coverage.
        {
            let mut rich = json.clone();
            let m = input.finances.monthly_budget_usd;
            rich["finances"]["monthly_budget_usd"] = json!(if m > 0.0 { m * 2.0 } else { 50.0 });
            let (c0, c1) = (
                covered(&out),
                covered(&e.assess(&parse(&rich)).expect("richer")),
            );
            for ((bucket, before), (_, after)) in c0.iter().zip(&c1) {
                if after + 1e-4 < *before {
                    fail(
                        "budget",
                        format!("{bucket}: {before} days covered, {after} with twice the money"),
                        &mut report,
                    );
                }
            }
        }
    }
    println!(
        "{n} households; slowest assess {:.1} ms ({}); {}",
        slowest.0,
        slowest.1,
        if release {
            "release"
        } else {
            "debug: timing not checked"
        }
    );
    if !soft.is_empty() {
        println!("Soft findings (reported, not failed):\n{soft}");
    }
    assert!(
        failures.is_empty(),
        "{} invariant failures {failures:?}:\n{report}",
        failures.values().sum::<usize>()
    );
}
