//! Reading a household: a PlanInput JSON file, standard input, or a fixture by name; then the
//! command-line overrides (location, dials, scenario toggles) and validation.
//!
//! Parsing and validation are the same two steps as `rr_types::PlanInput::from_json` (the single
//! entry point the web engine uses), with the overrides applied in between so a flag can fix what
//! it replaces.

use std::io::Read;
use std::path::Path;

use rr_types::{EngineError, PlanInput, ScenarioToggle};

use crate::args::HouseholdArgs;
use crate::error::CliError;

/// A household ready to plan.
#[derive(Debug, Clone)]
pub struct Household {
    /// The validated input, overrides applied.
    pub input: PlanInput,
    /// A short name for files and headers: the file stem, the fixture name, or `plan` for stdin.
    pub name: String,
    /// The scenario toggles given on the command line (to warn about ones that do not apply).
    pub cli_toggles: Vec<ScenarioToggle>,
}

/// Reads, overrides and validates the household.
///
/// # Errors
///
/// A file that cannot be found or read, JSON that is not a PlanInput, or validation problems:
/// all exit 2 with the plain-language list.
pub fn load(args: &HouseholdArgs) -> Result<Household, CliError> {
    let (name, json) = read(&args.household)?;
    let mut input: PlanInput =
        rr_types::parse_json(&json).map_err(|e| CliError::engine(&e, None))?;
    apply(&mut input, args);
    let problems = input.validate();
    if !problems.is_empty() {
        return Err(CliError::engine(&EngineError::bad_input(problems), None));
    }
    Ok(Household {
        input,
        name,
        cli_toggles: args.scenario.clone(),
    })
}

/// The household's name and JSON text.
fn read(spec: &str) -> Result<(String, String), CliError> {
    if spec == "-" {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).map_err(|e| {
            CliError::input(format!(
                "Could not read the household from standard input: {e}"
            ))
        })?;
        return Ok(("plan".to_owned(), s));
    }
    let path = Path::new(spec);
    if path.is_file() {
        let text = std::fs::read_to_string(path)
            .map_err(|e| CliError::input(format!("Could not read {spec}: {e}")))?;
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "plan".to_owned());
        return Ok((name, text));
    }
    let stem = spec.strip_suffix(".json").unwrap_or(spec);
    if let Some((name, json)) = rr_types::fixtures::RAW.iter().find(|(n, _)| *n == stem) {
        return Ok(((*name).to_owned(), (*json).to_owned()));
    }
    let names: Vec<&str> = rr_types::fixtures::RAW.iter().map(|(n, _)| *n).collect();
    Err(CliError::input(format!(
        "There is no household file {spec}. Give a PlanInput JSON file, `-` for standard input, \
         or a fixture name: {}.",
        names.join(", ")
    )))
}

/// Applies the command-line overrides. `--zip` and `--county` replace the file's location (both
/// together pick the county for a ZIP code that spans several, as the app records it); a
/// scenario toggle replaces any toggle for the same scenario.
pub fn apply(input: &mut PlanInput, args: &HouseholdArgs) {
    if args.zip.is_some() || args.county.is_some() {
        input.location.zip = args.zip.clone();
        input.location.county_fips = args.county.clone();
    }
    if let Some(rp) = args.return_period {
        input.dials.return_period = rp;
    }
    if let Some(c) = args.climate {
        input.dials.climate = c;
    }
    if let Some(w) = args.water_level {
        input.dials.water_level = w;
    }
    for t in &args.scenario {
        input.dials.scenario_overrides.retain(|x| x.id != t.id);
        input.dials.scenario_overrides.push(t.clone());
    }
}

/// Notes for scenario toggles that name no scenario the engine considered for this place.
pub fn scenario_notes(
    h: &Household,
    scenarios: &[rr_types::ScenarioInfo],
    place: &str,
) -> Vec<String> {
    let here: Vec<&str> = scenarios.iter().map(|s| s.id.as_str()).collect();
    h.cli_toggles
        .iter()
        .filter(|t| !here.contains(&t.id.as_str()))
        .map(|t| {
            let list = if here.is_empty() {
                "none apply here".to_owned()
            } else {
                format!("the ones here are {}", here.join(", "))
            };
            format!(
                "note: --scenario {}={} changes nothing: no scenario with that id applies to \
                 {place} ({list}).",
                t.id,
                if t.on { "on" } else { "off" }
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::{ClimateHorizon, ReturnPeriod};

    fn args(household: &str) -> HouseholdArgs {
        HouseholdArgs {
            household: household.to_owned(),
            zip: None,
            county: None,
            return_period: None,
            climate: None,
            water_level: None,
            scenario: Vec::new(),
        }
    }

    #[test]
    fn fixtures_load_by_name_with_or_without_json() {
        let h = load(&args("coos-bay-well-owner-2")).unwrap();
        assert_eq!(h.name, "coos-bay-well-owner-2");
        assert_eq!(h.input.location.zip.as_deref(), Some("97420"));
        assert!(load(&args("coos-bay-well-owner-2.json")).is_ok());
        let e = load(&args("no-such-household")).unwrap_err();
        assert_eq!(e.exit, crate::Exit::Input);
        assert!(
            e.message.contains("philadelphia-renters-4"),
            "{}",
            e.message
        );
    }

    #[test]
    fn overrides_replace_what_they_name() {
        let mut a = args("philadelphia-renters-4");
        a.county = Some("17031".into());
        a.return_period = Some(ReturnPeriod::OneIn500);
        a.climate = Some(ClimateHorizon::Y2050);
        a.scenario = vec![
            ScenarioToggle {
                id: "cascadia_m9".into(),
                on: true,
            },
            ScenarioToggle {
                id: "cascadia_m9".into(),
                on: false,
            },
        ];
        let h = load(&a).unwrap();
        assert_eq!(h.input.location.county_fips.as_deref(), Some("17031"));
        assert_eq!(
            h.input.location.zip, None,
            "--county alone replaces the ZIP code"
        );
        assert_eq!(h.input.dials.return_period, ReturnPeriod::OneIn500);
        assert_eq!(h.input.dials.climate, ClimateHorizon::Y2050);
        assert_eq!(
            h.input.dials.scenario_overrides,
            [ScenarioToggle {
                id: "cascadia_m9".into(),
                on: false
            }],
            "the last toggle for a scenario wins"
        );
    }

    #[test]
    fn a_bad_override_is_a_validation_problem() {
        let mut a = args("philadelphia-renters-4");
        a.zip = Some("123".into());
        let e = load(&a).unwrap_err();
        assert_eq!(e.exit, crate::Exit::Input);
        assert!(e.message.contains("location.zip"), "{}", e.message);
        assert!(e.message.contains("[zip_format]"), "{}", e.message);
    }
}
