//! Verification sweep (docs/VERIFICATION.md §5): plans one household profile in every county of
//! the data pack and writes one CSV row per county, so the targets can be tabulated and the
//! implausible ones found.
//!
//! ```text
//! cargo run --release -p rr-plan --example verify_sweep -- [fixture] [out.csv] [dial]
//! ```
//!
//! `fixture` defaults to `philadelphia-renters-4` (its ZIP code is dropped and the county set
//! instead); `out.csv` defaults to standard output; `dial` is a return period id (default: the
//! fixture's own). Timing per county is in the `ms` column.

use std::fmt::Write as _;
use std::io::Write as _;
use std::time::Instant;

use rr_plan::Engine;
use rr_types::{BucketId, HazardId, PlanItemKind, PlanOutput, ReturnPeriod, Target};

fn days(out: &PlanOutput, id: BucketId) -> (f32, f32, f32) {
    out.buckets
        .iter()
        .find(|b| b.id == id)
        .map_or((f32::NAN, f32::NAN, f32::NAN), |b| match b.target {
            Target::Days { value, low, high } | Target::Months { value, low, high } => {
                (value, low, high)
            }
            Target::Evacuate { p_need_10yr, .. } | Target::Readiness { p_need_10yr, .. } => {
                (p_need_10yr as f32, f32::NAN, f32::NAN)
            }
        })
}

fn top_driver(out: &PlanOutput, id: BucketId) -> String {
    out.buckets
        .iter()
        .find(|b| b.id == id)
        .and_then(|b| {
            b.contributions
                .iter()
                .max_by(|a, b| a.share.total_cmp(&b.share))
                .map(|c| format!("{}:{:.2}", c.hazard, c.share))
        })
        .unwrap_or_default()
}

fn rate(out: &PlanOutput, id: HazardId) -> f64 {
    out.register
        .iter()
        .find(|p| p.id == id)
        .map_or(0.0, |p| p.rate_per_year)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let fixture = args
        .first()
        .map_or("philadelphia-renters-4", String::as_str);
    let out_path = args.get(1).cloned();
    let dial: Option<ReturnPeriod> = args.get(2).map(|s| s.parse().expect("a return period id"));

    let t0 = Instant::now();
    let engine = Engine::with_data_dir(rr_plan::golden::data_dir()).expect("data pack loads");
    eprintln!("data pack loaded in {} ms", t0.elapsed().as_millis());
    let base = rr_types::fixtures::get(fixture).expect("a fixture household name");

    let mut fips: Vec<(String, String, String)> = engine
        .store()
        .counties()
        .map(|c| (c.fips.clone(), c.name.clone(), c.state_abbr.clone()))
        .collect();
    fips.sort();

    let heads = [
        "fips",
        "county",
        "state",
        "ms",
        "power",
        "power_lo",
        "power_hi",
        "water_boil",
        "water_out",
        "water_out_hi",
        "supplies",
        "thermal",
        "medication",
        "comms",
        "income_months",
        "evac_p10",
        "tier_recommended",
        "done_month",
        "spend_usd",
        "warnings",
        "cliffs",
        "scenarios_on",
        "scenarios_off",
        "power_driver",
        "water_out_driver",
        "thermal_driver",
        "heat_rate",
        "cold_rate",
        "wind_rate",
        "hurricane_rate",
        "winter_rate",
        "earthquake_rate",
        "wildfire_rate",
        "flood_rate",
        "has_outages",
        "has_climate",
        "has_seismic",
        "has_flood",
        "error",
    ];
    let mut csv = String::new();
    csv.push_str(&heads.join(","));
    csv.push('\n');
    let mut times: Vec<f64> = Vec::new();
    let mut errors = 0usize;
    for (f, name, state) in &fips {
        let mut input = base.clone();
        input.location.zip = None;
        input.location.county_fips = Some(f.clone());
        if let Some(d) = dial {
            input.dials.return_period = d;
        }
        let county = engine.store().county(f).expect("listed county");
        let flags = format!(
            "{},{},{},{}",
            county.outages.is_some(),
            !county.climate.is_empty(),
            county.seismic.is_some(),
            county.flood.is_some()
        );
        let t = Instant::now();
        let result = engine.assess(&input);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        times.push(ms);
        let name = name.replace(',', " ");
        match result {
            Err(e) => {
                errors += 1;
                let _ = writeln!(
                    csv,
                    "{f},{name},{state},{ms:.1}{},{flags},\"{}\"",
                    ",".repeat(heads.len() - 9),
                    e.to_string().replace('"', "'")
                );
            }
            Ok(out) => {
                let p = days(&out, BucketId::Power);
                let wb = days(&out, BucketId::WaterBoil);
                let wo = days(&out, BucketId::WaterOut);
                let s = days(&out, BucketId::Supplies);
                let th = days(&out, BucketId::Thermal);
                let m = days(&out, BucketId::Medication);
                let c = days(&out, BucketId::Comms);
                let inc = days(&out, BucketId::Income);
                let ev = days(&out, BucketId::Evacuate);
                let spend: f32 = out
                    .plan
                    .months
                    .iter()
                    .flat_map(|m| &m.items)
                    .filter(|i| i.kind == PlanItemKind::Purchase && !i.done)
                    .map(|i| i.est_cost_usd)
                    .sum();
                let cliffs: Vec<&str> = out
                    .warnings
                    .iter()
                    .filter(|w| w.id.starts_with("cliff_"))
                    .map(|w| w.id.as_str())
                    .collect();
                let on: Vec<&str> = out
                    .scenarios
                    .iter()
                    .filter(|s| s.on)
                    .map(|s| s.id.as_str())
                    .collect();
                let off: Vec<&str> = out
                    .scenarios
                    .iter()
                    .filter(|s| !s.on)
                    .map(|s| s.id.as_str())
                    .collect();
                let _ = writeln!(
                    csv,
                    "{f},{name},{state},{ms:.1},{},{},{},{},{},{},{},{},{},{},{},{:.4},{},{},{:.0},{},{},{},{},{},{},{},{:.5},{:.5},{:.5},{:.5},{:.5},{:.5},{:.5},{:.5},{flags},",
                    p.0,
                    p.1,
                    p.2,
                    wb.0,
                    wo.0,
                    wo.2,
                    s.0,
                    th.0,
                    m.0,
                    c.0,
                    inc.0,
                    ev.0,
                    out.tier_recommended,
                    out.plan
                        .done_month
                        .map_or("none".to_owned(), |d| d.to_string()),
                    spend,
                    out.warnings.len(),
                    cliffs.join(" "),
                    on.join(" "),
                    off.join(" "),
                    top_driver(&out, BucketId::Power),
                    top_driver(&out, BucketId::WaterOut),
                    top_driver(&out, BucketId::Thermal),
                    rate(&out, HazardId::HeatWave),
                    rate(&out, HazardId::ColdWave),
                    rate(&out, HazardId::StrongWind),
                    rate(&out, HazardId::Hurricane),
                    rate(&out, HazardId::WinterWeather),
                    rate(&out, HazardId::Earthquake),
                    rate(&out, HazardId::Wildfire),
                    rate(&out, HazardId::RiverineFlooding),
                );
            }
        }
    }
    times.sort_by(f64::total_cmp);
    let n = times.len();
    eprintln!(
        "{n} counties, {errors} errors; assess ms: median {:.1}, p99 {:.1}, max {:.1}; total {:.1} s",
        times[n / 2],
        times[(n * 99) / 100],
        times[n - 1],
        t0.elapsed().as_secs_f64()
    );
    match out_path {
        Some(p) => std::fs::write(&p, csv).expect("write the CSV"),
        None => {
            let _ = std::io::stdout().write_all(csv.as_bytes());
        }
    }
}
