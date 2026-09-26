//! The command line, as `clap` types. The help text is the user guide's short form; `docs/CLI.md`
//! has the long form with examples.

use std::path::PathBuf;
use std::str::FromStr;

use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rr_types::{ClimateHorizon, ExplainKind, ReturnPeriod, ScenarioToggle, WaterLevel};

/// Ready Reckoner on the command line: the plan, the risks and the targets for a household, and
/// the checks CI runs on the engine.
#[derive(Debug, Parser)]
#[command(name = "rr", version, propagate_version = true)]
pub struct Cli {
    /// Where county data comes from.
    #[command(flatten)]
    pub data: DataArgs,
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// Where county data comes from (global options).
#[derive(Debug, Clone, Default, Args)]
pub struct DataArgs {
    /// Directory holding the data packs (`manifest.json`, `core/`, `geo/`). Default: `data` in the
    /// current directory when it has a manifest, otherwise the seven fixture counties.
    #[arg(long, global = true, value_name = "DIR")]
    pub data: Option<PathBuf>,
    /// Use the seven hand-built fixture counties instead of a data pack.
    #[arg(long, global = true, conflicts_with = "data")]
    pub fixtures: bool,
}

impl DataArgs {
    /// True when the user chose a source (`--data` or `--fixtures`) rather than taking the default.
    pub fn explicit(&self) -> bool {
        self.data.is_some() || self.fixtures
    }
}

/// The commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print the packet (Markdown) and/or the whole plan (PlanOutput JSON) for a household.
    Plan(PlanArgs),
    /// Print the risk register: ranked hazards and the rare-but-severe box, with sources.
    Risks(HouseholdArgs),
    /// Print how long to be ready for each need, with ranges and when help arrives.
    Targets(TargetsArgs),
    /// Explain why a number is what it is: plain words, the arithmetic, the sources.
    Explain(ExplainArgs),
    /// List the item catalogue.
    Catalogue(CatalogueArgs),
    /// List the source registry, or the citation ids referenced but not defined.
    Citations(CitationsArgs),
    /// Find a county, or show everything the data knows about one.
    County {
        /// Search or show.
        #[command(subcommand)]
        command: CountyCommand,
    },
    /// Check the data packs, or print their versions and attributions.
    Data {
        /// Verify or info.
        #[command(subcommand)]
        command: DataCommand,
    },
    /// Compare the golden packets in fixtures/golden with the engine (--update rewrites them).
    Golden(GoldenArgs),
    /// Run every fixture household: timing, warnings, uncited items and determinism.
    Doctor(DoctorArgs),
}

/// The household and the dial overrides shared by every command that plans.
#[derive(Debug, Clone, Args)]
pub struct HouseholdArgs {
    /// The household: a PlanInput JSON file, `-` for standard input, or a fixture name such as
    /// `philadelphia-renters-4`.
    #[arg(long, value_name = "FILE")]
    pub household: String,
    /// Plan for this ZIP code instead of the file's location.
    #[arg(long, value_name = "ZIP")]
    pub zip: Option<String>,
    /// Plan for this county (five-digit FIPS code) instead of the file's location. With --zip it
    /// picks the county for a ZIP code that spans several.
    #[arg(long, value_name = "FIPS")]
    pub county: Option<String>,
    /// How rare an event to be ready for.
    #[arg(long, value_name = "DIAL", value_parser = id_parser::<ReturnPeriod>(ReturnPeriod::STRS))]
    pub return_period: Option<ReturnPeriod>,
    /// Today's climate or the 2050 projection.
    #[arg(long, value_name = "HORIZON", value_parser = id_parser::<ClimateHorizon>(ClimateHorizon::STRS))]
    pub climate: Option<ClimateHorizon>,
    /// How much water per person per day to plan for.
    #[arg(long, value_name = "LEVEL", value_parser = id_parser::<WaterLevel>(WaterLevel::STRS))]
    pub water_level: Option<WaterLevel>,
    /// Turn a named scenario on or off, for example `cascadia_m9=off`. Repeat for several.
    #[arg(long, value_name = "ID=on|off", value_parser = parse_scenario)]
    pub scenario: Vec<ScenarioToggle>,
}

/// `rr plan`.
#[derive(Debug, Clone, Args)]
pub struct PlanArgs {
    /// The household and dials.
    #[command(flatten)]
    pub household: HouseholdArgs,
    /// What to print: the packet, the PlanOutput JSON, or both (both needs --out).
    #[arg(long, value_enum, default_value_t = Format::Md)]
    pub format: Format,
    /// Write `<name>.md` and/or `<name>.json` into this directory instead of printing.
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,
}

/// What `rr plan` prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// The printable packet (Markdown).
    Md,
    /// The whole PlanOutput (JSON, the bytes the goldens hold).
    Json,
    /// Both files (needs --out).
    Both,
}

/// `rr targets`.
#[derive(Debug, Clone, Args)]
pub struct TargetsArgs {
    /// The household and dials.
    #[command(flatten)]
    pub household: HouseholdArgs,
    /// Show the targets at every dial setting (1 in 10 to 1 in 500), so a jump is visible.
    #[arg(long)]
    pub sweep: bool,
}

/// `rr explain`.
#[derive(Debug, Clone, Args)]
pub struct ExplainArgs {
    /// What kind of thing to explain.
    #[arg(value_parser = id_parser::<ExplainKind>(ExplainKind::STRS))]
    pub kind: ExplainKind,
    /// Its id, for example `power`, `heat_wave`, `water_out.water_gallons`.
    pub id: String,
    /// The household and dials.
    #[command(flatten)]
    pub household: HouseholdArgs,
    /// Print the Explanation as JSON.
    #[arg(long)]
    pub json: bool,
}

/// `rr catalogue`.
#[derive(Debug, Clone, Args)]
pub struct CatalogueArgs {
    /// Only this category, for example `water`.
    #[arg(long, value_name = "NAME")]
    pub category: Option<String>,
    /// Print the engine's catalogue() JSON (items filtered by --category).
    #[arg(long)]
    pub json: bool,
}

/// `rr citations`.
#[derive(Debug, Clone, Args)]
pub struct CitationsArgs {
    /// List the citation ids the engine and content refer to that content/citations.toml does
    /// not define. Fails unless every one is formally awaited.
    #[arg(long)]
    pub missing: bool,
}

/// `rr county ...`.
#[derive(Debug, Clone, Subcommand)]
pub enum CountyCommand {
    /// Find counties by name, "Name, ST" or FIPS code (up to ten).
    Search {
        /// What to look for, for example `phila`, `Cook, IL` or `42`.
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
    },
    /// Show a county's data record.
    Show {
        /// Five-digit county FIPS code, for example `42101`.
        fips: String,
        /// Print the CountyRecord as JSON.
        #[arg(long)]
        json: bool,
    },
}

/// `rr data ...`.
#[derive(Debug, Clone, Copy, Subcommand)]
pub enum DataCommand {
    /// Check every pack file against the manifest and that counties join across packs.
    Verify,
    /// Print the pack version, dates, sources and the attributions the app must show.
    Info,
}

/// `rr golden`.
#[derive(Debug, Clone, Args)]
pub struct GoldenArgs {
    /// Rewrite the golden files from rr-plan's helper instead of comparing. Explain the change
    /// in the commit message.
    #[arg(long)]
    pub update: bool,
}

/// `rr doctor`.
#[derive(Debug, Clone, Args)]
pub struct DoctorArgs {
    /// How many timed runs per fixture (the median is reported).
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub runs: u32,
}

/// A parser that offers exactly the stable id strings of an `rr-types` enum (so `--help` lists
/// them and a typo gets a suggestion) and returns the typed value.
fn id_parser<T>(strs: &'static [&'static str]) -> impl TypedValueParser<Value = T>
where
    T: FromStr + Clone + Send + Sync + 'static,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    PossibleValuesParser::new(strs).try_map(|s| s.parse::<T>())
}

/// `cascadia_m9=on` (also `off`, `true`/`false`, `yes`/`no`, `1`/`0`).
fn parse_scenario(s: &str) -> Result<ScenarioToggle, String> {
    let usage = || format!("`{s}`: write ID=on or ID=off, for example cascadia_m9=off");
    let (id, value) = s.split_once('=').ok_or_else(usage)?;
    let on = match value.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => true,
        "off" | "false" | "no" | "0" => false,
        _ => return Err(usage()),
    };
    let id = id.trim();
    if id.is_empty() {
        return Err(usage());
    }
    Ok(ScenarioToggle {
        id: id.to_owned(),
        on,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn scenario_toggles_parse() {
        let t = parse_scenario("cascadia_m9=off").unwrap();
        assert_eq!((t.id.as_str(), t.on), ("cascadia_m9", false));
        assert!(parse_scenario("cascadia_m9=ON").unwrap().on);
        assert!(parse_scenario("cascadia_m9").is_err());
        assert!(parse_scenario("cascadia_m9=maybe").is_err());
        assert!(parse_scenario("=on").is_err());
    }

    #[test]
    fn dials_parse_to_their_ids() {
        let cli = Cli::try_parse_from([
            "rr",
            "targets",
            "--household",
            "x.json",
            "--return-period",
            "one_in_500",
            "--climate",
            "y2050",
            "--water-level",
            "survival",
            "--scenario",
            "cascadia_m9=off",
            "--fixtures",
        ])
        .unwrap();
        let Command::Targets(t) = cli.command else {
            panic!("targets")
        };
        assert_eq!(t.household.return_period, Some(ReturnPeriod::OneIn500));
        assert_eq!(t.household.climate, Some(ClimateHorizon::Y2050));
        assert_eq!(t.household.water_level, Some(WaterLevel::Survival));
        assert_eq!(t.household.scenario.len(), 1);
        assert!(cli.data.fixtures && cli.data.explicit());
        assert!(
            Cli::try_parse_from([
                "rr",
                "plan",
                "--household",
                "x",
                "--return-period",
                "1in100"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from(["rr", "--data", "d", "--fixtures", "data", "info"]).is_err(),
            "--data and --fixtures conflict"
        );
    }
}
