//! Eviction in every county of the national pack (hazards3, v0.3.0). Run with
//! `RR_PRINT_EVICTION=1 cargo test -p rr-hazards --test eviction -- --nocapture` to print the
//! national distribution of the ten-year chance and the six renting fixture households.
//! Skipped, with a message, when `data/manifest.json` is absent.

mod common;

use common::*;
use rr_data::DataStore;
use rr_types::{HazardId, IncomeStability, PlanInput};

/// One county's eviction row for one household.
#[derive(Debug, Clone)]
struct Row {
    fips: String,
    label: String,
    filing_rate: Option<f64>,
    rate: f64,
    per_100: f64,
    sentence: String,
}

fn rows(s: &DataStore, input: &PlanInput) -> Vec<Row> {
    let mut out = Vec::new();
    for county in s.counties() {
        let Some(location) = s.location(&county.fips, None) else {
            continue;
        };
        let a = rr_hazards::assess(input, county, s.base_rates(), &location);
        let p = profile(&a, HazardId::Eviction);
        out.push(Row {
            fips: county.fips.clone(),
            label: format!("{}, {}", county.name, county.state_abbr),
            filing_rate: county.exposure.eviction_filing_rate.map(f64::from),
            rate: p.rate_per_year,
            per_100: per_100(p.rate_per_year, 10.0),
            sentence: p.frequency_sentence.clone(),
        });
    }
    out
}

/// Nearest-rank percentiles of `values` (sorted ascending): minimum, 10th … 90th, maximum.
fn deciles(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    (0..=10)
        .map(|i| values[((i as f64 / 10.0) * (n - 1) as f64).round() as usize])
        .collect()
}

fn print_distribution(title: &str, rows: &[Row]) {
    let mut sorted: Vec<&Row> = rows.iter().collect();
    // Most likely first; ties (the capped counties) by filings, most first.
    sorted.sort_by(|a, b| {
        b.per_100
            .total_cmp(&a.per_100)
            .then_with(|| {
                b.filing_rate
                    .unwrap_or(0.0)
                    .total_cmp(&a.filing_rate.unwrap_or(0.0))
            })
            .then_with(|| a.fips.cmp(&b.fips))
    });
    let all: Vec<f64> = sorted.iter().rev().map(|r| r.per_100).collect();
    let with: Vec<f64> = sorted
        .iter()
        .rev()
        .filter(|r| r.filing_rate.is_some())
        .map(|r| r.per_100)
        .collect();
    let fmt = |v: &[f64]| {
        deciles(v)
            .iter()
            .map(|x| format!("{x:.1}"))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    println!("\n### {title}\n");
    println!("| counties | min | 10 | 20 | 30 | 40 | 50 | 60 | 70 | 80 | 90 | max |");
    println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
    println!("| all {} | {} |", all.len(), fmt(&all));
    println!("| with a figure {} | {} |", with.len(), fmt(&with));
    println!("\nTop twenty:\n");
    println!(
        "| # | county | fips | filings per renter household | rate a year | per 100 in ten years | \
         county figure before any cap, per 100 in ten years |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- |");
    for (i, r) in sorted.iter().take(20).enumerate() {
        // What the county's own households-ordered-to-leave figure says before the cap, × the
        // household's modifier (the ratio of its rate to the county's).
        let before_cap = r.filing_rate.filter(|f| *f > 0.0).map_or(r.per_100, |f| {
            let k = if r.rate > 0.0 {
                r.rate / uncapped(f).min(0.07)
            } else {
                1.0
            };
            per_100(uncapped(f) * k, 10.0)
        });
        println!(
            "| {} | {} | {} | {} | {:.4} | {:.1} | {:.1} |",
            i + 1,
            r.label,
            r.fips,
            r.filing_rate
                .map_or("none".to_owned(), |f| format!("{f:.3}")),
            r.rate,
            r.per_100,
            before_cap
        );
    }
    println!("\nBottom ten with a figure:\n");
    println!(
        "| county | fips | filings per renter household | rate a year | per 100 in ten years |"
    );
    println!("| --- | --- | --- | --- | --- |");
    let bottom: Vec<&&Row> = sorted
        .iter()
        .rev()
        .filter(|r| r.filing_rate.is_some())
        .take(10)
        .collect();
    for r in bottom {
        println!(
            "| {} | {} | {:.5} | {:.5} | {:.2} |",
            r.label,
            r.fips,
            r.filing_rate.unwrap_or(0.0),
            r.rate,
            r.per_100
        );
    }
    let over_one = rows
        .iter()
        .filter(|r| r.filing_rate.is_some_and(|f| f > 1.0))
        .map(|r| format!("{} ({}) {:.1}", r.label, r.fips, r.per_100))
        .collect::<Vec<_>>();
    println!(
        "\nCounties without a figure: {}. Filing rate above 1: {}.",
        rows.iter().filter(|r| r.filing_rate.is_none()).count(),
        over_one.join("; ")
    );
    let capped: Vec<&Row> = rows
        .iter()
        .filter(|r| r.sentence.contains(CAPPED))
        .collect();
    let mut by_state: std::collections::BTreeMap<&str, usize> = Default::default();
    for r in &capped {
        *by_state.entry(&r.label[r.label.len() - 2..]).or_default() += 1;
    }
    println!(
        "Capped: {} counties ({}).",
        capped.len(),
        by_state
            .iter()
            .map(|(s, n)| format!("{s} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// The words the card adds when the county's own figure was capped.
const CAPPED: &str = "often take the same renters to court again and again, so we cap its figure \
                      at 7 in 100 households a year.";

/// Households ordered to leave per renter household a year, from the pack's filing rate, before
/// the cap: filings ÷ (1 + 1.7 × filings) households taken to court, × 0.32 ordered to leave.
fn uncapped(filings: f64) -> f64 {
    filings / (1.0 + 1.7 * filings) * 0.32
}

/// The six renting fixture households in their real counties, through the pack.
const RENTERS: &[&str] = &[
    "chicago-student-zero-budget-1",
    "detroit-snap-3",
    "galveston-highrise-1",
    "missoula-smoke-2",
    "philadelphia-renters-4",
    "phoenix-apartment-cpap-1",
];

fn renter_rows(s: &DataStore) -> Vec<(String, Row)> {
    RENTERS
        .iter()
        .map(|name| {
            let input = household(name);
            let location = s
                .resolve(&input.location)
                .unwrap_or_else(|e| panic!("{name}: {e:?}"));
            let county = s.county(&location.county_fips).unwrap();
            let a = rr_hazards::assess(&input, county, s.base_rates(), &location);
            let p = profile(&a, HazardId::Eviction);
            (
                (*name).to_owned(),
                Row {
                    fips: county.fips.clone(),
                    label: format!("{}, {}", county.name, county.state_abbr),
                    filing_rate: county.exposure.eviction_filing_rate.map(f64::from),
                    rate: p.rate_per_year,
                    per_100: per_100(p.rate_per_year, 10.0),
                    sentence: p.frequency_sentence.clone(),
                },
            )
        })
        .collect()
}

#[test]
fn eviction_in_every_county() {
    let Some(s) = store() else {
        eprintln!("data/manifest.json not found: skipped");
        return;
    };
    // The reference renter: stable income and under three months of savings, so the county's own
    // figure (× 1). The riskiest: gig income and no savings (× 1.75).
    let reference = household("philadelphia-renters-4");
    assert_eq!(reference.finances.income.stability, IncomeStability::Stable);
    assert!(reference.finances.emergency_fund_months < 3.0);
    let mut gig = reference.clone();
    gig.finances.income.stability = IncomeStability::Gig;
    gig.finances.emergency_fund_months = 0.0;
    let reference_rows = rows(&s, &reference);
    let gig_rows = rows(&s, &gig);
    assert!(reference_rows.len() > 3000, "{}", reference_rows.len());
    for (r, g) in reference_rows.iter().zip(&gig_rows) {
        match r.filing_rate.filter(|f| *f > 0.0) {
            // A county with a figure: its own households, ordered to leave, capped at 7 in 100.
            Some(f) => {
                let expected = uncapped(f).min(0.07);
                assert!(
                    (r.rate - expected).abs() < 1e-9,
                    "{} {}: {} against {expected}",
                    r.fips,
                    r.label,
                    r.rate
                );
                assert_eq!(
                    r.sentence.contains(CAPPED),
                    uncapped(f) > 0.07,
                    "{} {}: {}",
                    r.fips,
                    r.label,
                    r.sentence
                );
            }
            // No figure (the territories) or a Lab estimate of zero: the national judgment rate.
            None => assert!((r.rate - 0.0192).abs() < 1e-12, "{} {}", r.fips, r.label),
        }
        // Nothing reads "about 100 of 100": at most about half for the county's own figure, and
        // under three in four for a household on gig income with no savings.
        assert!(r.per_100 < 51.0, "{} {}: {}", r.fips, r.label, r.sentence);
        assert!(g.per_100 < 72.0, "{} {}: {}", g.fips, g.label, g.sentence);
        assert!((g.rate - 1.75 * r.rate).abs() < 1e-9, "{}", r.fips);
    }
    // More filings never mean a lower chance.
    let mut by_filings: Vec<&Row> = reference_rows
        .iter()
        .filter(|r| r.filing_rate.is_some_and(|f| f > 0.0))
        .collect();
    by_filings.sort_by(|a, b| a.filing_rate.unwrap().total_cmp(&b.filing_rate.unwrap()));
    for w in by_filings.windows(2) {
        assert!(
            w[0].rate <= w[1].rate + 1e-12,
            "{} {}",
            w[0].fips,
            w[1].fips
        );
    }
    // The three counties with more than one filing per renter household (Baltimore County,
    // Prince George's County and Baltimore city, MD) keep their own capped figure.
    let over_one: Vec<&Row> = reference_rows
        .iter()
        .filter(|r| r.filing_rate.is_some_and(|f| f > 1.0))
        .collect();
    assert_eq!(over_one.len(), 3);
    assert!(over_one.iter().all(|r| (r.rate - 0.07).abs() < 1e-9));
    // The card names the place as the Census does: Baltimore city is not Baltimore County.
    for (fips, place) in [
        ("24005", "Baltimore County"),
        ("24510", "Baltimore city"),
        ("24033", "Prince George's County"),
    ] {
        let r = reference_rows.iter().find(|r| r.fips == fips).unwrap();
        assert!(
            r.sentence
                .contains(&format!("Landlords in {place} {CAPPED}")),
            "{fips}: {}",
            r.sentence
        );
    }
    if std::env::var_os("RR_PRINT_EVICTION").is_some() {
        print_distribution(
            "A renting household with stable income and under three months of savings",
            &reference_rows,
        );
        print_distribution(
            "A renting household on gig income with no savings",
            &gig_rows,
        );
        println!("\n### The six renting fixture households\n");
        println!(
            "| household | county | filings per renter household | rate a year | per 100 in ten years | sentence |"
        );
        println!("| --- | --- | --- | --- | --- | --- |");
        for (name, r) in renter_rows(&s) {
            println!(
                "| {name} | {} | {} | {:.5} | {:.1} | {} |",
                r.label,
                r.filing_rate
                    .map_or("none".to_owned(), |f| format!("{f:.3}")),
                r.rate,
                r.per_100,
                r.sentence
            );
        }
    }
}
