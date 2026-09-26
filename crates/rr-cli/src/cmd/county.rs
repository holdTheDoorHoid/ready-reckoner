//! `rr county search <query>` and `rr county show <fips>`: the engine's county search, and
//! everything the data holds about one county (the `CountyRecord` the hazard model reads).

use rr_plan::{CountySource, Engine};
use rr_types::{AfreqKind, CountyRecord, HazardId, NriHazard};

use crate::Output;
use crate::args::CountyCommand;
use crate::error::{CliError, Exit};
use crate::format::{self, Table, sig2, thousands, wrap};
use crate::source::Source;

/// Runs `rr county ...`.
///
/// # Errors
///
/// An unknown county (exit 2). A search with no match is a result (exit 1, like `grep`).
pub fn run(engine: &Engine<Source>, command: &CountyCommand) -> Result<Output, CliError> {
    match command {
        CountyCommand::Search { query } => Ok(search(engine, &query.join(" "))),
        CountyCommand::Show { fips, json } => show(engine, fips.trim(), *json),
    }
}

fn search(engine: &Engine<Source>, query: &str) -> Output {
    let found = engine.county_search(query);
    if found.is_empty() {
        let mut out = Output::text(String::new());
        out.exit = Exit::Failure;
        out.notes.push(format!(
            "No county matches \"{query}\" in {}.",
            engine.store().describe()
        ));
        if let Some(h) = engine.store().location_hint() {
            out.notes.push(h);
        }
        return out;
    }
    let mut t = Table::new(["FIPS", "County", "State", "Population"]).right(3);
    for l in &found {
        let pop = engine
            .store()
            .county(&l.county_fips)
            .and_then(|c| c.population)
            .map(|p| thousands(u64::from(p)))
            .unwrap_or_default();
        t.row([
            l.county_fips.clone(),
            l.county_name.clone(),
            format!("{} ({})", l.state_name, l.state_abbr),
            pop,
        ]);
    }
    let mut s = format!(
        "{} match{} for \"{query}\" in {}\n\n",
        found.len(),
        if found.len() == 1 { "" } else { "es" },
        engine.store().describe()
    );
    s.push_str(&t.render(1));
    Output::text(s)
}

fn show(engine: &Engine<Source>, fips: &str, json: bool) -> Result<Output, CliError> {
    let store = engine.store();
    let Some(c) = store.county(fips) else {
        let state: String = fips.chars().take(2).collect();
        let nearby: Vec<String> = if state.len() == 2 && state.chars().all(|c| c.is_ascii_digit()) {
            store
                .search(&state)
                .iter()
                .take(5)
                .map(|l| format!("{} {}, {}", l.county_fips, l.county_name, l.state_abbr))
                .collect()
        } else {
            Vec::new()
        };
        let mut message = format!(
            "There is no county with FIPS code {fips} in {}. A county FIPS code is five digits \
             (two for the state, three for the county); find one with `rr county search <name>`.",
            store.describe()
        );
        if !nearby.is_empty() {
            message.push_str(&format!("\n  In the same state: {}", nearby.join("; ")));
        }
        if let Some(h) = store.location_hint() {
            message.push_str(&format!("\n  {h}"));
        }
        return Err(CliError::input(message));
    };
    if json {
        return Ok(Output::text(rr_plan::to_json(c)));
    }
    Ok(Output::text(render(c, store.location(fips, None).as_ref())))
}

fn opt<T>(v: Option<T>, f: impl Fn(T) -> String) -> String {
    v.map(f).unwrap_or_else(|| "-".to_owned())
}

fn pct(x: f32) -> String {
    format!("{:.1}%", 100.0 * f64::from(x))
}

fn money(x: f64) -> String {
    format::usd(x)
}

/// The record as text.
pub fn render(c: &CountyRecord, loc: Option<&rr_types::LocationResolved>) -> String {
    let mut s = format!("{}, {} (FIPS {})\n\n", c.name, c.state_name, c.fips);
    let mut t = Table::new(["Field", "Value"]);
    t.row([
        "Centre".to_owned(),
        format!("{:.4}, {:.4}", c.centroid.lat, c.centroid.lon),
    ]);
    t.row(["Climate region (NCA5)".to_owned(), c.nca_region.clone()]);
    t.row(["On the coast".to_owned(), yes_no(c.coastal)]);
    t.row(["Tsunami zone".to_owned(), yes_no(c.tsunami_zone)]);
    t.row([
        "Population".to_owned(),
        opt(c.population, |p| thousands(u64::from(p))),
    ]);
    t.row([
        "Households".to_owned(),
        opt(c.households, |p| thousands(u64::from(p))),
    ]);
    t.row([
        "Building value".to_owned(),
        opt(c.building_value_usd, money),
    ]);
    t.row(["National Risk Index".to_owned(), c.nri_version.clone()]);
    if let Some(l) = loc {
        let f = &l.facility_flags;
        t.row([
            "Nuclear plant within 16 km / 80 km".to_owned(),
            format!(
                "{} / {}",
                yes_no(f.nuclear_plant_within_16km),
                yes_no(f.nuclear_plant_within_80km)
            ),
        ]);
    }
    s.push_str(&t.render(1));

    if !c.nri.is_empty() {
        s.push_str("\nNational Risk Index, per hazard\n\n");
        let mut t = Table::new([
            "Hazard",
            "Frequency (AFREQ)",
            "Expected loss a year",
            "Loss ratio (bldg)",
            "Risk score",
        ])
        .right(2)
        .right(3)
        .right(4);
        let mut rows: Vec<(&HazardId, &NriHazard)> = c.nri.iter().collect();
        rows.sort_by_key(|(h, _)| h.name());
        for (h, n) in rows {
            let freq = opt(n.afreq, |a| match n.afreq_kind {
                AfreqKind::EventsPerYear => format!("{} events a year", sig2(f64::from(a))),
                AfreqKind::AnnualProbability => format!("{} chance a year", sig2(f64::from(a))),
            });
            t.row([
                h.name().to_owned(),
                freq,
                opt(n.ealt, |x| money(f64::from(x))),
                opt(n.hlrb, |x| sig2(f64::from(x))),
                opt(n.risk_score, |x| format!("{x:.1}")),
            ]);
        }
        s.push_str(&t.render(1));
    }

    if let Some(o) = &c.outages {
        s.push_str("\nPower outages\n\n");
        let mut t = Table::new(["Field", "Value"]);
        t.row([
            "Outages per customer a year".to_owned(),
            sig2(f64::from(o.events_per_customer_year)),
        ]);
        t.row([
            "Lasting 1 / 3 / 7 / 14 days or more".to_owned(),
            format!(
                "{} / {} / {} / {}",
                pct(o.p_ge_1d),
                pct(o.p_ge_3d),
                pct(o.p_ge_7d),
                pct(o.p_ge_14d)
            ),
        ]);
        t.row([
            "Median / 90th percentile".to_owned(),
            format!(
                "{} / {} hours",
                sig2(f64::from(o.median_hours)),
                sig2(f64::from(o.p90_hours))
            ),
        ]);
        t.row(["Years covered".to_owned(), o.years_covered.clone()]);
        s.push_str(&t.render(1));
        s.push_str(&format!(
            "  {}\n",
            wrap(
                &format!("What counts as an outage: {}", o.event_definition),
                96,
                2
            )
        ));
    }

    if !c.events.is_empty() {
        s.push_str("\nEvent rates\n\n");
        let mut t = Table::new([
            "Event",
            "Per year",
            "Damaging",
            "Median days",
            "90th pct days",
        ])
        .right(1)
        .right(2)
        .right(3)
        .right(4);
        for (k, e) in &c.events {
            t.row([
                k.clone(),
                sig2(f64::from(e.rate_per_year)),
                opt(e.share_damaging, pct),
                opt(e.median_days, |d| sig2(f64::from(d))),
                opt(e.p90_days, |d| sig2(f64::from(d))),
            ]);
        }
        s.push_str(&t.render(1));
    }

    if let Some(q) = &c.seismic {
        s.push_str("\nEarthquake shaking (USGS)\n\n");
        let mut t = Table::new(["Field", "Value"]);
        t.row([
            "Yearly chance of 0.1 g or more".to_owned(),
            sig2(f64::from(q.p_pga_ge_0_1g_per_year)),
        ]);
        t.row([
            "Yearly chance of 0.2 g or more".to_owned(),
            sig2(f64::from(q.p_pga_ge_0_2g_per_year)),
        ]);
        t.row([
            "Chance of damaging shaking in 100 years".to_owned(),
            opt(q.mmi6_100yr, pct),
        ]);
        s.push_str(&t.render(1));
    }

    if !c.climate.is_empty() {
        s.push_str("\nClimate (ratios to today, and day counts)\n\n");
        let mut t = Table::new(["Variable", "Value"]).right(1);
        for (k, v) in &c.climate {
            t.row([k.clone(), sig2(f64::from(*v))]);
        }
        s.push_str(&t.render(1));
    }

    if let Some(f) = &c.flood {
        s.push_str("\nFlood\n\n");
        let mut t = Table::new(["Field", "Value"]);
        let basis = match f.sfha_basis.as_deref() {
            Some("structures") => " (counted from structures)",
            Some("policies_lower_bound") => " (from insured homes only, so a lower bound)",
            _ => "",
        };
        t.row([
            "Homes in the 1%-a-year flood zone".to_owned(),
            format!("{}{basis}", pct(f.sfha_home_share)),
        ]);
        t.row([
            "Claims per 1,000 policies a year".to_owned(),
            opt(f.claims_per_1000_policies_year, |x| sig2(f64::from(x))),
        ]);
        t.row([
            "Mean paid claim".to_owned(),
            opt(f.mean_paid_usd, |x| money(f64::from(x))),
        ]);
        s.push_str(&t.render(1));
    }

    if let Some(f) = &c.facilities {
        s.push_str("\nFacilities\n\n");
        let mut t = Table::new(["Field", "Value"]);
        t.row([
            "Nearest nuclear plant".to_owned(),
            opt(f.nearest_nuclear_km, |k| format!("{k:.0} km")),
        ]);
        t.row([
            "Toxics Release Inventory facilities".to_owned(),
            f.tri_facilities.to_string(),
        ]);
        t.row([
            "High-hazard dams".to_owned(),
            f.high_hazard_dams.to_string(),
        ]);
        s.push_str(&t.render(1));
    }

    if let Some(v) = &c.vulnerability {
        s.push_str("\nSocial vulnerability\n\n");
        let mut t = Table::new(["Field", "Value"]);
        t.row([
            "Residents with 3+ risk factors (CRE)".to_owned(),
            opt(v.cre_share_3plus_risk_factors, pct),
        ]);
        t.row([
            "Social Vulnerability Index percentile".to_owned(),
            opt(v.svi_percentile, |x| format!("{:.0}", 100.0 * f64::from(x))),
        ]);
        s.push_str(&t.render(1));
    }
    s
}

fn yes_no(b: bool) -> String {
    if b { "yes" } else { "no" }.to_owned()
}
