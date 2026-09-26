//! Prints the calibration report (research worked examples against this crate's targets) and
//! times `assess` for every fixture household.
//!
//! ```text
//! cargo run -p rr-consequence --example calibrate --release
//! ```

#[path = "../tests/support/generic.rs"]
mod generic;
#[path = "../tests/support/report.rs"]
mod report;
#[path = "../tests/support/research.rs"]
mod research;

use std::time::Instant;

use rr_consequence::{CountyData, DRAWS, assess};

fn time_ms(runs: usize, mut f: impl FnMut()) -> (f64, f64) {
    f(); // warm the effects-table cache
    let mut best = f64::INFINITY;
    let start = Instant::now();
    for _ in 0..runs {
        let t = Instant::now();
        f();
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
    }
    (start.elapsed().as_secs_f64() * 1e3 / runs as f64, best)
}

fn main() {
    let (md, checks) = report::calibration_report();
    println!("{md}");
    let failed = checks.iter().filter(|c| !c.ok).count();
    println!(
        "{} checks, {} outside ±25 % (see the calibration test for the explained ones)\n",
        checks.len(),
        failed
    );

    println!("## Timing of assess() ({DRAWS} Monte Carlo draws, native, this machine)\n");
    println!("| Household | mean ms | best ms |\n|---|---|---|");
    for (name, input) in rr_types::fixtures::all() {
        let rates = generic::rates(&input);
        let (mean, best) = time_ms(30, || {
            std::hint::black_box(assess(&input, &rates, CountyData::default(), &[]));
        });
        println!("| {name} (generic rates, all hazards) | {mean:.1} | {best:.1} |");
    }
    let stats = research::philadelphia_outages();
    let (input, rates) = (
        research::philadelphia_household(),
        research::philadelphia_rates(),
    );
    let county = CountyData {
        outages: Some(&stats),
        ..CountyData::default()
    };
    let (mean, best) = time_ms(30, || {
        std::hint::black_box(assess(&input, &rates, county, &[]));
    });
    println!("| Philadelphia (research rates, county outage records) | {mean:.1} | {best:.1} |");
    let stats = research::coos_outages();
    let (input, rates, scen) = (
        research::coos_household(),
        research::coos_rates(),
        research::coos_scenarios(true),
    );
    let county = CountyData {
        outages: Some(&stats),
        coastal: true,
        tsunami_zone: true,
        ..CountyData::default()
    };
    let (mean, best) = time_ms(30, || {
        std::hint::black_box(assess(&input, &rates, county, &scen));
    });
    println!(
        "| Coos Bay (research rates, Cascadia and tsunami scenarios) | {mean:.1} | {best:.1} |"
    );
}
