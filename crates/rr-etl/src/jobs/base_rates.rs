//! Job 10 — national base rates for societal and personal hazards (`base_rates.toml`).
//!
//! Every value is computed here from published figures, and every entry records the figure, the
//! year, the publisher, the URL and the citation id the content layer should use. Two inputs are
//! fetched live (BLS JOLTS layoffs through the public API, and the Census population estimate);
//! the rest are transcribed from pages that refuse scripted clients (read 2026-09-25 for
//! `docs/research/data-sources.md` §8). A refresh re-computes the arithmetic, so a revised
//! figure is changed in one place.

use super::{Ctx, JobOutput};
use crate::csvout::{col, parse_delimited};
use crate::manifest::SourceRecord;
use crate::num::sig4;
use crate::{Result, data_err};
use rr_types::math::{exp, pow};

/// Base rates pack file.
pub const BASE_RATES: &str = "core/base_rates.toml";

const POPEST: &str = "https://www2.census.gov/programs-surveys/popest/datasets/2020-2025/state/totals/NST-EST2025-ALLDATA.csv";
const BLS: &str = "https://api.bls.gov/publicAPI/v1/timeseries/data/JTS000000000000000LDR";
/// When the transcribed figures were read from their sources.
pub const TRANSCRIBED: &str = "2026-09-25";

/// A publication the rates cite.
struct Pub {
    id: &'static str,
    title: &'static str,
    publisher: &'static str,
    url: &'static str,
}

const PUBS: &[Pub] = &[
    Pub {
        id: "census_households_cps",
        title: "Table HH-1. Households by Type: 1940 to Present",
        publisher: "U.S. Census Bureau, Current Population Survey",
        url: "https://www2.census.gov/programs-surveys/demo/tables/families/time-series/households/hh1.xls",
    },
    Pub {
        id: "usfa_residential_fires",
        title: "Residential Building Fire Estimates",
        publisher: "U.S. Fire Administration (FEMA)",
        url: "https://www.usfa.fema.gov/statistics/residential-fires/",
    },
    Pub {
        id: "bls_jolts_layoffs",
        title: "Job Openings and Labor Turnover Survey: layoffs and discharges rate, total nonfarm (JTS000000000000000LDR)",
        publisher: "U.S. Bureau of Labor Statistics",
        url: "https://data.bls.gov/timeseries/JTS000000000000000LDR",
    },
    Pub {
        id: "bls_work_experience_2024",
        title: "Work Experience of the Population, 2024",
        publisher: "U.S. Bureau of Labor Statistics",
        url: "https://www.bls.gov/news.release/work.nr0.htm",
    },
    Pub {
        id: "cdc_nchs_ed_visits",
        title: "FastStats: Emergency Department Visits (NHAMCS 2022)",
        publisher: "CDC National Center for Health Statistics",
        url: "https://www.cdc.gov/nchs/fastats/emergency-department.htm",
    },
    Pub {
        id: "nchs_accidental_injury_2024",
        title: "FastStats: Accidents or Unintentional Injuries",
        publisher: "CDC National Center for Health Statistics",
        url: "https://www.cdc.gov/nchs/fastats/accidental-injury.htm",
    },
    Pub {
        id: "nhtsa_early_estimate_2025",
        title: "Early Estimate of Motor Vehicle Traffic Fatalities in 2025 (DOT HS 813 800)",
        publisher: "National Highway Traffic Safety Administration",
        url: "https://crashstats.nhtsa.dot.gov/Api/Public/ViewPublication/813800",
    },
    Pub {
        id: "nhtsa_crashes_2023",
        title: "Overview of Motor Vehicle Traffic Crashes in 2023 (DOT HS 813 705)",
        publisher: "National Highway Traffic Safety Administration",
        url: "https://crashstats.nhtsa.dot.gov/Api/Public/ViewPublication/813705",
    },
    Pub {
        id: "census_popest_vintage_2025",
        title: "Annual Estimates of the Resident Population, Vintage 2025 (NST-EST2025-ALLDATA)",
        publisher: "U.S. Census Bureau",
        url: POPEST,
    },
    Pub {
        id: "cdc_pandemic_history",
        title: "1918 Pandemic (H1N1 virus) and past pandemics",
        publisher: "Centers for Disease Control and Prevention",
        url: "https://archive.cdc.gov/www_cdc_gov/flu/pandemic-resources/1918-pandemic-h1n1.html",
    },
    Pub {
        id: "eia_outage_hours_2024",
        title: "Today in Energy: U.S. electricity customers averaged more hours of interruptions in 2024",
        publisher: "U.S. Energy Information Administration",
        url: "https://www.eia.gov/todayinenergy/detail.php?id=66744",
    },
];

/// One output entry.
struct Rate {
    id: &'static str,
    value: f64,
    unit: &'static str,
    low: Option<f64>,
    high: Option<f64>,
    source: &'static str,
    year: u16,
    figure: String,
    derivation: String,
    note: &'static str,
}

fn toml_str(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

/// Chi-square quantiles used for the exact Poisson 90% interval on 5 events:
/// chi2(0.05; 2*5) / 2 and chi2(0.95; 2*(5+1)) / 2.
const POISSON_5_LOW: f64 = 3.940_299 / 2.0;
const POISSON_5_HIGH: f64 = 21.026_07 / 2.0;

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();

    // Live: population 2023 (Vintage 2025).
    let pop = ctx
        .http
        .get(POPEST, Some("base_rates/NST-EST2025-ALLDATA.csv"))?;
    out.source(super::source_from(
        "Census population estimates, Vintage 2025 (national and state totals)",
        &pop,
        "NST-EST2025-ALLDATA",
        super::PUBLIC_DOMAIN,
        "",
    ));
    let (h, rows) = parse_delimited(&pop.text(), b',')?;
    let (i_name, i_2023) = (col(&h, "NAME")?, col(&h, "POPESTIMATE2023")?);
    let pop2023: f64 = rows
        .iter()
        .find(|r| r[i_name] == "United States")
        .and_then(|r| r[i_2023].parse().ok())
        .ok_or_else(|| data_err("Census popest: no United States POPESTIMATE2023"))?;

    // Live: JOLTS layoffs and discharges rate, monthly average for the last complete year.
    let bls = ctx.http.get(BLS, None)?;
    out.source(super::source_from(
        "BLS Public Data API v1: JTS000000000000000LDR",
        &bls,
        "JOLTS, latest release",
        super::PUBLIC_DOMAIN,
        "",
    ));
    let v: serde_json::Value = serde_json::from_slice(&bls.bytes)?;
    let last_year = (crate::timefmt::today_utc()[..4]
        .parse::<i32>()
        .unwrap_or(2026)
        - 1)
    .to_string();
    let monthly: Vec<f64> = v["Results"]["series"][0]["data"]
        .as_array()
        .ok_or_else(|| data_err("BLS: no data array"))?
        .iter()
        .filter(|x| {
            x["year"].as_str() == Some(last_year.as_str())
                && x["period"]
                    .as_str()
                    .is_some_and(|p| p.starts_with('M') && p != "M13")
        })
        .filter_map(|x| x["value"].as_str()?.parse::<f64>().ok())
        .collect();
    if monthly.len() != 12 {
        return Err(data_err(format!(
            "BLS JOLTS: expected 12 months for {last_year}, got {}",
            monthly.len()
        )));
    }
    let layoff_pct = monthly.iter().sum::<f64>() / 12.0;
    let layoff = layoff_pct / 100.0;
    let year_u16: u16 = last_year.parse().unwrap_or(2025);

    let households_2023 = 131_434_000.0;
    let lambda_pandemic = 5.0 / 108.0;
    let rates = vec![
        Rate { id: "us_households_2023", value: households_2023, unit: "households", low: None, high: None, source: "census_households_cps", year: 2023, figure: "131,434 thousand households in 2023".into(), derivation: "Table HH-1, 2023 row".into(), note: "Denominator for household rates." },
        Rate { id: "house_fire_per_household_year", value: 344_600.0 / households_2023, unit: "residential building fires per household per year", low: None, high: None, source: "usfa_residential_fires", year: 2023, figure: "344,600 residential building fires in 2023".into(), derivation: "344,600 fires / 131,434,000 households".into(), note: "About 1 in 381 households a year. Counts fires reported to fire departments." },
        Rate { id: "house_fire_death_per_household_year", value: 2_890.0 / households_2023, unit: "residential fire deaths per household per year", low: None, high: None, source: "usfa_residential_fires", year: 2023, figure: "2,890 residential fire deaths in 2023".into(), derivation: "2,890 deaths / 131,434,000 households".into(), note: "" },
        Rate { id: "house_fire_injury_per_household_year", value: 10_400.0 / households_2023, unit: "residential fire injuries per household per year", low: None, high: None, source: "usfa_residential_fires", year: 2023, figure: "10,400 residential fire injuries in 2023".into(), derivation: "10,400 injuries / 131,434,000 households".into(), note: "" },
        Rate { id: "house_fire_mean_loss_usd", value: 11_270_000_000.0 / 344_600.0, unit: "US dollars of direct loss per residential building fire", low: None, high: None, source: "usfa_residential_fires", year: 2023, figure: "$11.27 billion direct loss from 344,600 residential building fires in 2023".into(), derivation: "$11,270,000,000 / 344,600 fires".into(), note: "A mean; most fires cost far less and a few cost far more." },
        Rate { id: "layoff_per_worker_month", value: layoff, unit: "layoffs and discharges per worker per month", low: None, high: None, source: "bls_jolts_layoffs", year: year_u16, figure: format!("{last_year} average of the 12 monthly rates: {:.4}%", layoff_pct), derivation: "mean of monthly JTS000000000000000LDR values / 100".into(), note: "Counts events, not people: some workers are laid off twice, so per-person risk is lower." },
        Rate { id: "layoff_upper_bound_per_worker_year", value: 1.0 - pow(1.0 - layoff, 12.0), unit: "chance of at least one layoff or discharge per worker per year (upper bound)", low: None, high: None, source: "bls_jolts_layoffs", year: year_u16, figure: format!("{:.4}% a month", layoff_pct), derivation: "1 - (1 - monthly rate)^12".into(), note: "An upper bound: layoffs cluster in high-turnover jobs, so a typical worker's chance is lower." },
        Rate { id: "unemployment_spell_per_worker_year", value: 0.083, unit: "chance a labour-force participant is unemployed at some point in a year", low: None, high: None, source: "bls_work_experience_2024", year: 2024, figure: "work-experience unemployment rate 8.3% in 2024 (14.7 million people unemployed at some time during the year)".into(), derivation: "8.3 / 100".into(), note: "People unemployed at some point in the year / people who worked or looked for work; includes people who quit or were entering the labour force, so it overstates involuntary job loss." },
        Rate { id: "ed_visits_per_person_year", value: 0.473, unit: "emergency department visits per person per year", low: None, high: None, source: "cdc_nchs_ed_visits", year: 2022, figure: "155.4 million visits; 47.3 visits per 100 persons (2022)".into(), derivation: "47.3 / 100".into(), note: "" },
        Rate { id: "ed_visit_admission_share", value: 0.115, unit: "share of emergency department visits that lead to hospital admission", low: None, high: None, source: "cdc_nchs_ed_visits", year: 2022, figure: "11.5% of visits resulted in hospital admission (2022)".into(), derivation: "11.5 / 100".into(), note: "" },
        Rate { id: "injury_ed_visits_per_person_year", value: 0.473 * 43.5 / 155.4, unit: "injury-related emergency department visits per person per year", low: None, high: None, source: "cdc_nchs_ed_visits", year: 2022, figure: "43.5 million injury-related visits of 155.4 million (2022)".into(), derivation: "0.473 x 43.5 / 155.4".into(), note: "" },
        Rate { id: "unintentional_injury_death_per_person_year", value: 58.1 / 100_000.0, unit: "unintentional-injury deaths per person per year", low: None, high: None, source: "nchs_accidental_injury_2024", year: 2024, figure: "197,449 deaths; 58.1 per 100,000 population (2024)".into(), derivation: "58.1 / 100,000".into(), note: "" },
        Rate { id: "traffic_deaths_per_100m_vmt", value: 1.10, unit: "traffic deaths per 100 million vehicle miles traveled", low: None, high: None, source: "nhtsa_early_estimate_2025", year: 2025, figure: "36,640 deaths (early estimate); 1.10 fatalities per 100 million VMT (2025)".into(), derivation: "as published".into(), note: "All road deaths per mile driven, not only vehicle occupants." },
        Rate { id: "traffic_injuries_per_person_year", value: 2_440_000.0 / pop2023, unit: "people injured in traffic crashes per person per year", low: None, high: None, source: "nhtsa_crashes_2023", year: 2023, figure: format!("2.44 million people injured (2023); U.S. population {pop2023:.0} (Census Vintage 2025, July 1, 2023)"), derivation: "2,440,000 / population 2023".into(), note: "" },
        Rate { id: "pandemic_onset_per_year", value: 1.0 - exp(-lambda_pandemic), unit: "chance a new pandemic begins in a given year", low: Some(1.0 - exp(-POISSON_5_LOW / 108.0)), high: Some(1.0 - exp(-POISSON_5_HIGH / 108.0)), source: "cdc_pandemic_history", year: 2025, figure: "5 pandemic onsets in 108 years (1918, 1957, 1968, 2009, 2020; 1918-2025)".into(), derivation: "1 - exp(-5/108); range is the exact Poisson 90% interval for 5 events".into(), note: "A crude historical rate with wide uncertainty; treat as a planning figure, not a forecast." },
        Rate { id: "power_interruption_hours_per_customer_year", value: 11.0, unit: "hours without power per electricity customer per year (national average)", low: Some(6.0), high: None, source: "eia_outage_hours_2024", year: 2024, figure: "about 9 hours from major events plus about 2 hours from other events in 2024; major events averaged about 4 hours a year in 2014-2023".into(), derivation: "9 + 2; low = 4 + 2 (2014-2023 typical year)".into(), note: "National average; use outages.csv for county detail." },
    ];

    let mut s = String::new();
    s.push_str("# National base rates for societal and personal hazards. Written by rr-etl (job `base_rates`).\n");
    s.push_str("# Every value is computed from the figure quoted with it. `source` is the citation id for\n");
    s.push_str("# content/citations.toml; the [[publication]] entries give the title, publisher and URL.\n");
    s.push_str(&format!("# Figures transcribed from publications were read on {TRANSCRIBED}; the population and JOLTS\n# figures are fetched on each refresh.\n\n"));
    for r in &rates {
        s.push_str("[[rate]]\n");
        s.push_str(&format!("id = {}\n", toml_str(r.id)));
        s.push_str(&format!("value = {}\n", sig4(r.value)));
        s.push_str(&format!("unit = {}\n", toml_str(r.unit)));
        if let Some(l) = r.low {
            s.push_str(&format!("low = {}\n", sig4(l)));
        }
        if let Some(h) = r.high {
            s.push_str(&format!("high = {}\n", sig4(h)));
        }
        s.push_str(&format!("source = {}\n", toml_str(r.source)));
        s.push_str(&format!("year = {}\n", r.year));
        s.push_str(&format!("figure = {}\n", toml_str(&r.figure)));
        s.push_str(&format!("derivation = {}\n", toml_str(&r.derivation)));
        s.push_str(&format!("note = {}\n\n", toml_str(r.note)));
    }
    for p in PUBS {
        s.push_str("[[publication]]\n");
        s.push_str(&format!("id = {}\n", toml_str(p.id)));
        s.push_str(&format!("title = {}\n", toml_str(p.title)));
        s.push_str(&format!("publisher = {}\n", toml_str(p.publisher)));
        s.push_str(&format!("url = {}\n", toml_str(p.url)));
        let how = if p.url == POPEST || p.id == "bls_jolts_layoffs" {
            "fetched on each refresh"
        } else {
            "transcribed"
        };
        s.push_str(&format!(
            "retrieved = {}\n",
            toml_str(if how == "transcribed" {
                TRANSCRIBED
            } else {
                "each refresh"
            })
        ));
        s.push_str(&format!("how = {}\n\n", toml_str(how)));
    }
    // The TOML must parse and every rate must cite a listed publication.
    let parsed: toml::Table = s
        .parse()
        .map_err(|e| data_err(format!("base_rates.toml does not parse: {e}")))?;
    let n = parsed["rate"].as_array().map(|a| a.len()).unwrap_or(0)
        + parsed["publication"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
    for r in &rates {
        if !PUBS.iter().any(|p| p.id == r.source) {
            return Err(data_err(format!(
                "rate {} cites unknown publication {}",
                r.id, r.source
            )));
        }
    }
    out.text(ctx, BASE_RATES, &s, n as u64)?;
    for p in PUBS
        .iter()
        .filter(|p| p.url != POPEST && p.id != "bls_jolts_layoffs")
    {
        out.source(SourceRecord {
            name: format!("{} ({})", p.title, p.publisher),
            url: p.url.to_string(),
            version:
                "figures transcribed (the site refuses scripted clients or is not machine-readable)"
                    .into(),
            retrieved: format!("{TRANSCRIBED}T00:00:00Z"),
            sha256: String::new(),
            bytes: 0,
            license: super::PUBLIC_DOMAIN.into(),
            obligations: String::new(),
        });
    }
    out.rows_in = rates.len() as u64;
    out.notes.push(format!("{} rates. Values are computed from the quoted figures and rounded to 4 significant figures. Residential burglary and household cardiac-arrest rates are not included yet (no verified source in the research).", rates.len()));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pandemic_interval_brackets_the_estimate() {
        let lo = 1.0 - exp(-POISSON_5_LOW / 108.0);
        let mid = 1.0 - exp(-5.0 / 108.0);
        let hi = 1.0 - exp(-POISSON_5_HIGH / 108.0);
        assert!(lo < mid && mid < hi);
        assert!((mid - 0.04525).abs() < 1e-4, "{mid}");
    }

    #[test]
    fn house_fire_rate_matches_research() {
        // docs/research/data-sources.md §8: 0.262% per household-year, about 1 in 381.
        let r: f64 = 344_600.0 / 131_434_000.0;
        assert!((r - 0.002622).abs() < 1e-6);
        assert!(((1.0 / r) - 381.4).abs() < 0.5);
    }
}
