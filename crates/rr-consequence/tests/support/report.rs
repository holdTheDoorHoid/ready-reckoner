//! The calibration report: this crate's targets for the research worked examples, next to the
//! research numbers (docs/research/risk-model.md §3.8, §8.4, §9.4), by dial setting.

#![allow(dead_code)]

use std::fmt::Write as _;

use rr_consequence::{
    ConsequenceAssessment, CountyData, DURATION_BUCKETS, ExceedanceCurve, assess, dial_rate,
};
use rr_types::{BucketId, PlanInput, ReturnPeriod, Target};

use super::research::{self, RESEARCH_DEFAULT_RATE, RESEARCH_RATES, ResearchRow};

/// One number checked against the research.
#[derive(Debug, Clone)]
pub struct Check {
    pub label: String,
    pub research: f64,
    pub ours: f64,
    pub tolerance: f64,
    pub ok: bool,
}

fn check(label: &str, research: f64, ours: f64, rel: f64, abs: f64) -> Check {
    let tolerance = (rel * research).max(abs);
    Check {
        label: label.to_owned(),
        research,
        ours,
        tolerance,
        ok: (ours - research).abs() <= tolerance,
    }
}

/// A household run at every dial.
pub struct Runs {
    pub name: &'static str,
    pub by_dial: Vec<ConsequenceAssessment>,
}

fn run_all(
    name: &'static str,
    input: &PlanInput,
    rates: &[rr_types::HouseholdEventRate],
    county: CountyData<'_>,
    scenarios: &[rr_consequence::ScenarioCandidate],
) -> Runs {
    let by_dial = ReturnPeriod::ALL
        .iter()
        .map(|rp| {
            let mut p = input.clone();
            p.dials.return_period = *rp;
            assess(&p, rates, county, scenarios)
        })
        .collect();
    Runs { name, by_dial }
}

pub fn philadelphia() -> Runs {
    let stats = research::philadelphia_outages();
    run_all(
        "Philadelphia",
        &research::philadelphia_household(),
        &research::philadelphia_rates(),
        CountyData {
            outages: Some(&stats),
            ..CountyData::default()
        },
        &[],
    )
}

pub fn coos(cascadia_on: bool, cascadia_rate: Option<f64>) -> Runs {
    let stats = research::coos_outages();
    let mut scenarios = research::coos_scenarios(cascadia_on);
    if let Some(r) = cascadia_rate {
        for s in &mut scenarios {
            s.rate_per_year = r;
            s.low = s.low.min(r);
            s.high = r;
        }
    }
    run_all(
        "Coos Bay",
        &research::coos_household(),
        &research::coos_rates(),
        CountyData {
            outages: Some(&stats),
            coastal: true,
            tsunami_zone: true,
            ..CountyData::default()
        },
        &scenarios,
    )
}

/// Continuous target of a bucket at any yearly rate (days; months for income).
fn at_rate(a: &ConsequenceAssessment, bucket: BucketId, rate: f64) -> f64 {
    if bucket == BucketId::Income {
        a.income.curve.target_at(rate)
    } else {
        a.curve(bucket).map_or(0.0, |c| c.target_at(rate))
    }
}

fn curve(a: &ConsequenceAssessment, bucket: BucketId) -> &ExceedanceCurve {
    a.curve(bucket).expect("duration bucket")
}

fn fmt_days(d: f64) -> String {
    if d <= 0.0 {
        "—".to_owned()
    } else if d < 1.0 {
        format!("{:.0} h", d * 24.0)
    } else if d < 10.0 {
        format!("{d:.1} d")
    } else {
        format!("{d:.0} d")
    }
}

fn fmt_value(bucket: BucketId, v: f64) -> String {
    if bucket == BucketId::Income {
        if v <= 0.0 {
            "—".to_owned()
        } else {
            format!("{v:.1} mo")
        }
    } else {
        fmt_days(v)
    }
}

fn fmt_target(t: &Target) -> String {
    match *t {
        Target::Days { value, low, high } => {
            if value == 0.0 && high == 0.0 {
                "—".to_owned()
            } else {
                format!("{value} d ({low}–{high})")
            }
        }
        Target::Months { value, low, high } => format!("{value} mo ({low}–{high})"),
        _ => String::new(),
    }
}

fn label(b: BucketId) -> &'static str {
    match b {
        BucketId::Power => "Power",
        BucketId::WaterBoil => "Boil-water notice",
        BucketId::WaterOut => "No tap water",
        BucketId::Supplies => "Food and supplies",
        BucketId::Thermal => "Heat or cold",
        BucketId::Medication => "Medicine",
        BucketId::Comms => "Communications",
        BucketId::Income => "Income gap",
        _ => "",
    }
}

fn dial_table(out: &mut String, runs: &Runs) {
    let _ = writeln!(
        out,
        "| Bucket | 1 in 10 | 1 in 50 | **1 in 100 (default)** | 1 in 500 |\n|---|---|---|---|---|"
    );
    for b in DURATION_BUCKETS.iter().chain([BucketId::Income].iter()) {
        let cells: Vec<String> = runs
            .by_dial
            .iter()
            .map(|a| fmt_target(&a.bucket(*b).target))
            .collect();
        let _ = writeln!(
            out,
            "| {} | {} | {} | **{}** | {} |",
            label(*b),
            cells[0],
            cells[1],
            cells[2],
            cells[3]
        );
    }
}

fn research_table(out: &mut String, runs: &Runs, rows: &[ResearchRow]) -> Vec<Check> {
    let a = &runs.by_dial[2];
    let mut checks = Vec::new();
    let _ = writeln!(
        out,
        "| Bucket | 1 in 10: research / ours | 1 in 50: research / ours | ≈1 in 95: research / ours | 1 in 500: research / ours |\n|---|---|---|---|---|"
    );
    for row in rows {
        let mut cells = Vec::new();
        for (k, rate) in RESEARCH_RATES.iter().enumerate() {
            let ours = at_rate(a, row.bucket, *rate);
            match row.values[k] {
                Some(r) => {
                    cells.push(format!(
                        "{} / {}",
                        fmt_value(row.bucket, r),
                        fmt_value(row.bucket, ours)
                    ));
                    let name = format!(
                        "{} {} at {}",
                        runs.name,
                        label(row.bucket),
                        ["1 in 10", "1 in 50", "1 in 95", "1 in 500"][k]
                    );
                    checks.push(check(
                        &name,
                        r,
                        ours,
                        0.25,
                        if r < 1.0 { 0.25 } else { 0.0 },
                    ));
                }
                None => cells.push(format!("— / {}", fmt_value(row.bucket, ours))),
            }
        }
        let _ = writeln!(out, "| {} | {} |", label(row.bucket), cells.join(" | "));
    }
    checks
}

fn natural_frequency_rows(out: &mut String, runs: &Runs, rows: &[(BucketId, f64, f64)]) {
    let a = &runs.by_dial[2];
    let _ = writeln!(
        out,
        "| Disruption | Research (per 100 over 10 years) | Ours |\n|---|---|---|"
    );
    for (b, d, research) in rows {
        let ours = curve(a, *b).natural_frequency(*d, 10.0);
        let _ = writeln!(
            out,
            "| {} ≥ {} | {:.0} | {:.0} |",
            label(*b),
            fmt_days(*d),
            research,
            ours
        );
    }
}

/// The brief's headline checks, at the product's dial (`one_in_100` is the research's default,
/// −ln(0.9)/10) and ladder rule (round up; within 3 % of a step counts as the step).
fn headline(out: &mut String, checks: &mut Vec<Check>, phl: &Runs, coos: &Runs) {
    let _ = writeln!(
        out,
        "| Check | Research | Ours, raw design duration | Ours on the ladder (what the app shows) |\n|---|---|---|---|"
    );
    let cases: [(&Runs, BucketId, usize, f64, &str); 8] = [
        (phl, BucketId::Power, 2, 2.8, "Philadelphia power ≈ 3 d"),
        (
            phl,
            BucketId::WaterOut,
            2,
            2.8,
            "Philadelphia no tap water ≈ 3 d",
        ),
        (phl, BucketId::Supplies, 2, 9.6, "Philadelphia food ≈ 10 d"),
        (
            phl,
            BucketId::Medication,
            2,
            12.0,
            "Philadelphia medicine ≈ 14 d",
        ),
        (
            phl,
            BucketId::Income,
            2,
            3.8,
            "Philadelphia income ≈ 4 months",
        ),
        (coos, BucketId::Power, 2, 13.0, "Coos Bay power ≈ 13 d"),
        (
            coos,
            BucketId::WaterOut,
            2,
            50.0,
            "Coos Bay water ≈ 50 d (Cascadia on)",
        ),
        (
            coos,
            BucketId::WaterOut,
            1,
            14.0,
            "Coos Bay water 14 d at one_in_50",
        ),
    ];
    for (runs, b, dial, research, name) in cases {
        let a = &runs.by_dial[dial];
        let rate = dial_rate(ReturnPeriod::ALL[dial]);
        let raw = at_rate(a, b, rate);
        let ladder = match a.bucket(b).target {
            Target::Days { value, .. } | Target::Months { value, .. } => value,
            _ => 0.0,
        };
        let unit = if b == BucketId::Income { " mo" } else { " d" };
        // One decimal, so the 3 % rule is visible (14.5 days is past 14 × 1.03).
        let _ = writeln!(
            out,
            "| {name} | {research}{unit} | {raw:.1}{unit} | **{ladder}{unit}** |",
        );
        checks.push(check(name, research, raw, 0.25, 0.0));
    }
}

/// Builds the report and the list of checks.
pub fn calibration_report() -> (String, Vec<Check>) {
    let phl = philadelphia();
    let coos_off = coos(false, None);
    let coos_recurrence = coos(true, Some(0.004));
    let coos = coos(true, None);
    let mut out = String::new();
    let mut checks = Vec::new();

    let _ = writeln!(
        out,
        "# Calibration: consequence targets against the risk-model research\n\n\
         Generated by `cargo test -p rr-consequence --test calibration` (and printed by \
         `cargo run -p rr-consequence --example calibrate --release`). Inputs: the research \
         households (§8.1, §9.1) with household event rates that reproduce the research \
         prototype's event classes (`tests/support/research.rs`), the EAGLE-I outage fits from \
         §8.2/§9.2, and for Coos Bay the Cascadia and local-tsunami scenarios (1.0 %/yr).\n\n\
         Dial rates (planner decision): `one_in_10` 0.1, `one_in_50` 0.02, `one_in_100` \
         −ln(0.9)/10 = 0.010536 (the research's default, \"90 % sure nothing in the next ten \
         years is worse\"), `one_in_500` 0.002. Targets are rounded up to the day ladder, a raw \
         value within 3 % above a step counting as that step. Research numbers are compared \
         with our raw design durations at the same rates.\n"
    );

    let _ = writeln!(out, "## Headline checks\n");
    headline(&mut out, &mut checks, &phl, &coos);

    let _ = writeln!(
        out,
        "\n## Philadelphia (renting family of four, gas furnace, window air conditioning)\n\n\
         ### Targets by dial (what the app shows: ladder value, 10th–90th percentile)\n"
    );
    dial_table(&mut out, &phl);
    let _ = writeln!(
        out,
        "\n### Against research §8.4 (continuous design durations at the research's dial rates)\n"
    );
    checks.extend(research_table(
        &mut out,
        &phl,
        &research::philadelphia_research(),
    ));
    let _ = writeln!(
        out,
        "\n### Natural frequencies against the research register (§8.3)\n"
    );
    natural_frequency_rows(
        &mut out,
        &phl,
        &[
            (BucketId::Power, 1.0, 26.0),
            (BucketId::Power, 3.0, 9.0),
            (BucketId::Power, 7.0, 3.0),
            (BucketId::WaterOut, 1.0, 24.0),
            (BucketId::WaterOut, 3.0, 10.0),
            (BucketId::WaterOut, 7.0, 4.0),
            (BucketId::WaterBoil, 1.0, 33.0),
            (BucketId::Supplies, 3.0, 59.0),
            (BucketId::Medication, 3.0, 38.0),
            (BucketId::Thermal, 1.0, 16.0),
        ],
    );
    let phl_a = &phl.by_dial[2];
    let ratio = |a: &ConsequenceAssessment, b: BucketId| {
        a.value_between(b, 0.0, 3.0) / a.value_between(b, 30.0, 33.0).max(1e-300)
    };
    let _ = writeln!(
        out,
        "\n### Diminishing returns (research §3.2): disruption-days covered by days 0–3 over days 30–33\n\n\
         | Bucket | Research | Ours |\n|---|---|---|\n\
         | Philadelphia water | ≈ 60× | {:.0}× |\n| Philadelphia power | ≈ 1,600× | {:.0}× |\n\
         | Philadelphia food | ≈ 190× | {:.0}× |\n| Coos Bay well water | ≈ 19× | {:.0}× |",
        ratio(phl_a, BucketId::WaterOut),
        ratio(phl_a, BucketId::Power),
        ratio(phl_a, BucketId::Supplies),
        ratio(&coos.by_dial[2], BucketId::WaterOut),
    );
    let _ = writeln!(
        out,
        "\n### Readiness\n\n- Leaving home quickly: {:.0} in 100 over 10 years (research ≈ 5; ours adds hurricane evacuation orders and chemical releases), warning from {:.2} to {:.0} hours, about {} days away.\n- Stranded away from home: {:.0} in 100; the 19 km commute is {:.1} hours on foot.\n- Home fire: {:.1} in 100 (research ≈ 5 with the attached neighbour).\n",
        100.0 * phl_a.evacuate.p_need_10yr,
        phl_a.evacuate.notice_hours[0],
        phl_a.evacuate.notice_hours[1],
        phl_a.evacuate.days_away,
        100.0 * phl_a.get_home.p_need_10yr,
        phl_a.get_home.commuters[0].walk_hours,
        match phl_a.bucket(BucketId::Fire).target {
            Target::Readiness { p_need_10yr, .. } => 100.0 * p_need_10yr,
            _ => 0.0,
        },
    );

    let _ = writeln!(
        out,
        "## Coos Bay (well, wood stove, Cascadia on)\n\n### Targets by dial\n"
    );
    dial_table(&mut out, &coos);
    let _ = writeln!(out, "\n### Against research §9.4\n");
    checks.extend(research_table(&mut out, &coos, &research::coos_research()));
    let _ = writeln!(
        out,
        "\n### Natural frequencies against the research register (§9.3)\n"
    );
    natural_frequency_rows(
        &mut out,
        &coos,
        &[
            (BucketId::WaterOut, 1.0, 70.0),
            (BucketId::WaterOut, 7.0, 25.0),
            (BucketId::WaterOut, 30.0, 13.0),
            (BucketId::Power, 1.0, 44.0),
            (BucketId::Supplies, 3.0, 62.0),
            (BucketId::Medication, 7.0, 22.0),
        ],
    );
    let _ = writeln!(
        out,
        "\n### Without Cascadia (research §9.4: about 3 days of power, 9 days of food and medicine, 2 weeks of water)\n"
    );
    dial_table(&mut out, &coos_off);
    let off = &coos_off.by_dial[2];
    let _ = writeln!(
        out,
        "\nAt the research's dial: power {}, food {}, medicine {}, water {}.",
        fmt_days(at_rate(off, BucketId::Power, RESEARCH_DEFAULT_RATE)),
        fmt_days(at_rate(off, BucketId::Supplies, RESEARCH_DEFAULT_RATE)),
        fmt_days(at_rate(off, BucketId::Medication, RESEARCH_DEFAULT_RATE)),
        fmt_days(at_rate(off, BucketId::WaterOut, RESEARCH_DEFAULT_RATE)),
    );
    let rec = &coos_recurrence.by_dial[2];
    let _ = writeln!(
        out,
        "\n### The Cascadia rate\n\nWith the time-independent recurrence reading (0.4 %/yr instead of 1.0 %/yr) the research's default water target drops from 50 to 23 days. Ours at the research's dial: {} (with 1.0 %/yr: {}).",
        fmt_days(at_rate(rec, BucketId::WaterOut, RESEARCH_DEFAULT_RATE)),
        fmt_days(at_rate(
            &coos.by_dial[2],
            BucketId::WaterOut,
            RESEARCH_DEFAULT_RATE
        )),
    );
    let _ = writeln!(out, "\n### Scenario and cliff output at the default dial\n");
    for s in &coos.by_dial[2].scenarios {
        let _ = writeln!(
            out,
            "- **{}** ({}): {}",
            s.name,
            if s.on { "on" } else { "off" },
            s.effect_summary
        );
    }
    for w in &coos.by_dial[2].warnings {
        let _ = writeln!(out, "- Warning: {} {}", w.message, w.why);
    }
    for (label, a) in [("Philadelphia", phl_a), ("Coos Bay", &coos.by_dial[2])] {
        let _ = writeln!(out, "\n### Self-sufficiency statement, {label}\n");
        for s in &a.statement {
            let _ = writeln!(out, "> {s}");
        }
    }

    let _ = writeln!(
        out,
        "\n## All checks (tolerance ±25 %, or ±6 hours below a day)\n"
    );
    let _ = writeln!(out, "| Check | Research | Ours | Pass |\n|---|---|---|---|");
    for c in &checks {
        let _ = writeln!(
            out,
            "| {} | {:.2} | {:.2} | {} |",
            c.label,
            c.research,
            c.ours,
            if c.ok { "yes" } else { "**no**" }
        );
    }
    (out, checks)
}
