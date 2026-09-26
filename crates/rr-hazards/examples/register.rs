//! Prints the hazard register for a fixture household in a fixture county, for eyeballing.
//!
//! ```text
//! cargo run -p rr-hazards --example register -- philadelphia-renters-4 42101
//! cargo run -p rr-hazards --example register -- coos-bay-well-owner-2 41011 one_in_100 today
//! ```
//!
//! Optional third and fourth arguments override the return period and climate dials.

use rr_types::{BaseRate, ClimateHorizon, CountyRecord, LocationResolved, ReturnPeriod};

#[derive(serde::Deserialize)]
struct Fixture {
    county: CountyRecord,
    location: LocationResolved,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (household, fips) = match (args.first(), args.get(1)) {
        (Some(h), Some(f)) => (h.as_str(), f.as_str()),
        _ => {
            eprintln!(
                "usage: register <fixture-household> <county-fips> [return_period] [climate]"
            );
            std::process::exit(2);
        }
    };
    let mut input = rr_types::fixtures::get(household).expect("no such fixture household");
    if let Some(rp) = args.get(2) {
        input.dials.return_period = rp.parse::<ReturnPeriod>().expect("return period");
    }
    if let Some(c) = args.get(3) {
        input.dials.climate = c.parse::<ClimateHorizon>().expect("climate");
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");
    let fixture: Fixture = serde_json::from_str(
        &std::fs::read_to_string(format!("{dir}/counties/{fips}.json")).expect("county file"),
    )
    .expect("county json");
    let base_rates: Vec<BaseRate> = serde_json::from_str(
        &std::fs::read_to_string(format!("{dir}/base_rates.json")).expect("base rates"),
    )
    .expect("base rates json");
    let a = rr_hazards::assess(&input, &fixture.county, &base_rates, &fixture.location);
    println!(
        "{household} in {} ({fips}), climate {:?}",
        fixture.county.name, input.dials.climate
    );
    println!(
        "{:>3} {:<34} {:>10} {:>21} {:>6} {:>5} {:>7}  sentence",
        "#", "hazard", "rate/yr", "range", "sev", "clim", "conf"
    );
    for (i, p) in a.profiles.iter().enumerate() {
        println!(
            "{:>3} {:<34} {:>10.5} [{:>9.5}, {:>9.5}] {:>6.2} {:>5.2} {:>7}  {}",
            i + 1,
            format!("{} ({})", p.id, p.display),
            p.rate_per_year,
            p.rate_range[0],
            p.rate_range[1],
            p.severity,
            p.climate_multiplier,
            p.confidence.as_str(),
            p.frequency_sentence
        );
    }
    println!("\nrates to rr-consequence:");
    for r in &a.rates {
        println!(
            "  {:<28} {:>10.5} [{:>9.5}, {:>9.5}] {:<9} {}",
            r.hazard.as_str(),
            r.rate_per_year,
            r.low,
            r.high,
            r.evidence.as_str(),
            r.sources
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    println!("\nscenarios:");
    for s in &a.scenarios {
        println!(
            "  {} ({}) on={} default={} rate={:.5} [{:.5}, {:.5}] variant={:?} alt={:?}\n    {}",
            s.id,
            s.hazard,
            s.on,
            s.default_on,
            s.rate_per_year,
            s.low,
            s.high,
            s.variant,
            s.alternatives
                .iter()
                .map(|a| format!("{:.5}", a.rate_per_year))
                .collect::<Vec<_>>(),
            s.applies_because
        );
    }
    println!("\nrare box (range only):");
    for p in a
        .profiles
        .iter()
        .filter(|p| p.display == rr_types::HazardDisplay::RareCatastrophic)
    {
        println!(
            "  {} [{:.2e}, {:.2e}] family={:?}",
            p.id,
            p.rate_range[0],
            p.rate_range[1],
            p.family.as_deref().unwrap_or("")
        );
        if let Some(a) = &p.anchor_sentence {
            println!("    anchor: {a}");
        }
        if let Some(lf) = &p.location_factor {
            println!(
                "    why here [{} x{:.3} ({:.3}-{:.3})]: {}",
                lf.class, lf.multiplier[1], lf.multiplier[0], lf.multiplier[2], lf.label
            );
        }
        if let Some(i) = &p.if_it_reaches_you {
            println!("    if it reaches you: {i}");
        }
        if let Some(w) = &p.what_it_changes {
            println!("    what it changes: {w}");
        }
        for s in &p.sub_causes {
            println!("    - {} {:?}", s.name, s.rate_range);
        }
    }
    println!("\nsub-causes on ranked cards:");
    for p in a
        .profiles
        .iter()
        .filter(|p| p.display == rr_types::HazardDisplay::Ranked && !p.sub_causes.is_empty())
    {
        let names: Vec<&str> = p.sub_causes.iter().map(|s| s.name.as_str()).collect();
        println!("  {}: {}", p.id, names.join("; "));
        if let Some(lf) = &p.location_factor {
            println!("    why here [{}]: {}", lf.class, lf.label);
        }
    }
    println!("\nalso checked:");
    for c in &a.also_checked {
        println!(
            "  {:<28} {:.2e} [{:.2e}, {:.2e}] {}",
            c.id, c.rate_per_year, c.rate_range[0], c.rate_range[1], c.name
        );
    }
    println!("\nnotes:");
    for n in &a.notes {
        println!("  - {n}");
    }
}
