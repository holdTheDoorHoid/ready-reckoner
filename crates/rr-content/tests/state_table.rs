//! The state table (`content/tables/state_registries.toml`): one row per state, DC and Puerto
//! Rico; confirmed tools named with their address; everything else sent to the local office; the
//! refill rule sourced, with both sources where they disagree.

use rr_content::tables::{JURISDICTIONS, LOCAL_OFFICE_SOURCES};

fn table() -> &'static rr_content::StateTable {
    &rr_content::content().states
}

fn row(code: &str) -> &'static rr_content::StateRow {
    rr_content::content()
        .state_row(code)
        .unwrap_or_else(|| panic!("no row for {code}"))
}

fn text(code: &str) -> String {
    row(code)
        .lines()
        .iter()
        .map(|l| format!("{}: {}", l.topic, l.text))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_state_dc_and_puerto_rico_has_one_row() {
    assert_eq!(table().state.len(), 52);
    for code in JURISDICTIONS {
        assert!(table().row(code).is_some(), "{code}");
    }
    for code in ["FL", "KS", "PR", "DC"] {
        println!("--- {code}\n{}", text(code));
    }
}

#[test]
fn confirmed_tools_are_named_with_their_address() {
    let fl = text("FL");
    assert!(fl.contains(
        "Know Your Zone, Know Your Home (https://www.floridadisaster.org/knowyourzone/)"
    ));
    assert!(fl.contains("Florida Special Needs Registry"));
    assert!(fl.contains("does not guarantee"));
    assert!(text("TX").contains("STEAR"));
    assert!(text("NJ").contains("Register Ready"));
    assert!(text("NY").contains("NY-Alert"));
    assert!(text("DC").contains("AlertDC"));
    // Zone lookups confirmed at state level (review S2): coastal states and Hawaii's tsunami map.
    for code in ["FL", "GA", "NC", "SC", "VA", "MD", "HI"] {
        assert!(row(code).zone.is_some(), "{code} has a zone lookup");
    }
}

#[test]
fn unconfirmed_rows_send_the_household_to_its_local_office() {
    let ks = row("KS").lines();
    assert!(
        ks[0]
            .text
            .starts_with("Your county emergency management office handles")
    );
    assert!(ks[0].text.contains("https://www.kansastag.gov/101/KDEM"));
    let local: Vec<&str> = ks[0].sources.iter().map(|c| c.as_str()).collect();
    assert_eq!(local, LOCAL_OFFICE_SOURCES);
    assert!(text("LA").contains("parish office"));
    assert!(text("MA").contains("city or town emergency management office"));
}

#[test]
fn puerto_rico_says_plainly_what_was_not_found() {
    let pr = text("PR");
    assert!(pr.contains("No evacuation-zone lookup or registry"));
    assert!(pr.contains("No alert sign-up was found for all of Puerto Rico"));
    assert!(pr.contains("No emergency refill rule was confirmed for Puerto Rico"));
    assert!(row("PR").none_found);
}

#[test]
fn refill_rules_cite_both_sources_where_they_disagree() {
    for code in ["VA", "NC"] {
        assert!(row(code).refill.starts_with("Rules differ"), "{code}");
    }
    for code in ["VA", "NC", "SC", "MD", "DC", "FL", "TX"] {
        assert_eq!(row(code).refill_sources.len(), 2, "{code}");
    }
    // Florida's two sources agree once both are read: 72 hours day to day, 30 days in a
    // declared emergency (Board of Pharmacy page, 2024; Healthcare Ready, 2022).
    assert!(row("FL").refill.contains("30-day") && row("FL").refill.contains("72-hour"));
}

#[test]
fn no_row_points_at_a_vendor_or_a_lapsed_domain() {
    // A registry domain found in an old directory now hosts a gambling site; alert systems run
    // on vendor platforms are linked through the state's own page instead.
    for r in &table().state {
        for url in r.urls() {
            for bad in ["specialneedsutah", "genasys", "everbridge", "nixle"] {
                assert!(!url.contains(bad), "{}: {url}", r.code);
            }
        }
        for line in r.lines() {
            for bad in ["Genasys", "Everbridge", "Nixle", "Smart911"] {
                assert!(!line.text.contains(bad), "{}: {}", r.code, line.text);
            }
        }
    }
}
