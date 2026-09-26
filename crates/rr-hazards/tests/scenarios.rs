//! Named-scenario detection for places the fixtures do not cover, using a fixture record moved
//! to another county (only the FIPS code, state and flags matter for detection).

mod common;

use common::*;
use rr_hazards::ScenarioZone;
use rr_types::{HazardId as H, ScenarioToggle};

/// Philadelphia's record, relabelled as another county.
fn moved(fips: &str, name: &str, state_abbr: &str, state_name: &str, tsunami: bool) -> Fixture {
    let mut f = county("42101");
    f.county.fips = fips.into();
    f.county.name = name.into();
    f.county.state_abbr = state_abbr.into();
    f.county.state_name = state_name.into();
    f.county.tsunami_zone = tsunami;
    f.location.county_fips = fips.into();
    f.location.county_name = name.into();
    f.location.state_abbr = state_abbr.into();
    f.location.state_name = state_name.into();
    f.location.tsunami_zone = tsunami;
    f
}

fn ids(a: &rr_hazards::HazardAssessment) -> Vec<&str> {
    a.scenarios.iter().map(|s| s.id.as_str()).collect()
}

#[test]
fn cascadia_inland_counties_get_the_valley_zone_and_the_full_margin_rate() {
    // Multnomah County (Portland): full-margin ruptures only, 19 in 10,000 years (research S31);
    // on by default because Oregon asks for two weeks.
    let a = run(
        &household("philadelphia-renters-4"),
        &moved("41051", "Multnomah", "OR", "Oregon", false),
    );
    assert_eq!(ids(&a), ["cascadia_m9"]);
    let c = &a.scenarios[0];
    assert_eq!(c.zone, Some(ScenarioZone::Valley));
    assert!(close(c.rate.rate_per_year, 0.0019, 1e-12));
    assert!(c.on && c.default_on && c.alternatives.is_empty());
    assert!(c.applies_because.contains("inland from the Cascadia fault"));
}

#[test]
fn washington_and_california_cascadia_counties() {
    let a = run(
        &household("philadelphia-renters-4"),
        &moved("53033", "King", "WA", "Washington", false),
    );
    let c = &a.scenarios[0];
    assert!(c.on && c.applies_because.contains("Washington asks"));
    assert!(c.sources.iter().any(|s| s == "wa_emd_2_weeks_ready"));
    // Humboldt County, California: southern margin, 1 %/yr, on because it reaches the
    // one-in-100 yardstick.
    let a = run(
        &household("philadelphia-renters-4"),
        &moved("06023", "Humboldt", "CA", "California", true),
    );
    assert_eq!(ids(&a), ["cascadia_m9", "local_tsunami"]);
    assert!(a.scenarios[0].on && a.scenarios[0].applies_because.contains("yardstick"));
    assert!(a.scenarios[1].applies_because.contains("15 to 20 minutes"));
}

#[test]
fn hayward_is_on_and_new_madrid_is_off_by_default() {
    // USGS UCERF3: 33 % chance of M6.7+ on the Hayward–Rodgers Creek fault in 30 years ->
    // 1.33 %/yr, above the default dial.
    let a = run(
        &household("philadelphia-renters-4"),
        &moved("06001", "Alameda", "CA", "California", false),
    );
    assert_eq!(ids(&a), ["hayward_m7"]);
    let h = &a.scenarios[0];
    assert!(h.on && h.default_on);
    assert!(close(
        h.rate.rate_per_year,
        -rr_types::math::ln_1p(-0.33) / 30.0,
        1e-12
    ));
    // The earthquake rate handed on excludes the scenario but never drops below a quarter.
    let quake = rate(&a, H::Earthquake).rate_per_year;
    let nri = -rr_types::math::ln_1p(-f64::from(0.001618f32));
    assert!(close(quake, 0.25 * nri, 1e-9), "{quake}");
    // USGS: 7–10 % chance of a repeat of 1811–12 in 50 years -> about 0.18 %/yr: off by
    // default, rarer than the one-in-100 yardstick.
    let a = run(
        &household("philadelphia-renters-4"),
        &moved("47157", "Shelby", "TN", "Tennessee", false),
    );
    assert_eq!(ids(&a), ["new_madrid_m7"]);
    let n = &a.scenarios[0];
    assert!(!n.on && !n.default_on);
    assert!(close(
        n.rate.rate_per_year,
        -rr_types::math::ln_1p(-0.085) / 50.0,
        1e-12
    ));
    assert!(n.applies_because.contains("off unless you turn it on"));
    // The user can turn it on.
    let mut input = household("philadelphia-renters-4");
    input.dials.scenario_overrides = vec![ScenarioToggle {
        id: "new_madrid_m7".into(),
        on: true,
    }];
    let a = run(&input, &moved("47157", "Shelby", "TN", "Tennessee", false));
    assert!(a.scenarios[0].on && a.scenarios[0].overridden);
}

#[test]
fn a_tsunami_zone_outside_cascadia_uses_the_local_source_prior() {
    let a = run(
        &household("philadelphia-renters-4"),
        &moved("15003", "Honolulu", "HI", "Hawaii", true),
    );
    assert_eq!(ids(&a), ["local_tsunami"]);
    let t = &a.scenarios[0];
    assert!(t.applies_because.contains("within minutes"));
    // No NRI tsunami exposure in the moved record: 1 in 1,000 a year × the 10 % fallback share.
    assert!(close(t.rate.rate_per_year, 0.001 * 0.1, 1e-12));
}

#[test]
fn a_setting_for_a_scenario_that_does_not_apply_is_ignored_with_a_note() {
    let mut input = household("philadelphia-renters-4");
    input.dials.scenario_overrides = vec![ScenarioToggle {
        id: "cascadia_m9".into(),
        on: true,
    }];
    let a = run(&input, &county("42101"));
    assert!(a.scenarios.is_empty());
    assert!(
        a.notes
            .iter()
            .any(|n| n.contains("\"cascadia_m9\" does not apply to Philadelphia County"))
    );
}
