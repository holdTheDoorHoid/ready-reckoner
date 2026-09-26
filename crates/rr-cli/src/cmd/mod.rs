//! The commands, and what they share: the engine on the chosen source, the header naming the
//! household, the place and the dial, and the sources legend.

pub mod catalogue;
pub mod citations;
pub mod county;
pub mod data;
pub mod doctor;
pub mod explain;
pub mod golden;
pub mod plan;
pub mod risks;
pub mod targets;

use rr_content::Content;
use rr_plan::Engine;
use rr_types::{CitationId, ClimateHorizon, LocationResolved, PlanInput, WaterLevel};

use crate::Output;
use crate::args::{Cli, Command, DataArgs};
use crate::error::CliError;
use crate::format::{self, wrap};
use crate::household::Household;
use crate::source::{self, Source};

/// Runs the command the arguments name.
///
/// # Errors
///
/// Whatever the command reports; see [`CliError`].
pub fn run(cli: &Cli) -> Result<Output, CliError> {
    match &cli.command {
        Command::Plan(a) => with_engine(&cli.data, |e| plan::run(e, a)),
        Command::Risks(a) => with_engine(&cli.data, |e| risks::run(e, a)),
        Command::Targets(a) => with_engine(&cli.data, |e| targets::run(e, a)),
        Command::Explain(a) => with_engine(&cli.data, |e| explain::run(e, a)),
        Command::Catalogue(a) => catalogue::run(a),
        Command::Citations(a) => citations::run(&cli.data, a),
        Command::County { command } => with_engine(&cli.data, |e| county::run(e, command)),
        Command::Data { command } => data::run(&cli.data, *command),
        Command::Golden(a) => golden::run(&cli.data, a),
        Command::Doctor(a) => with_engine(&cli.data, |e| doctor::run(e, a)),
    }
}

/// Opens the source, says where the data comes from if that needs saying, and runs `f`.
fn with_engine(
    data: &DataArgs,
    f: impl FnOnce(&Engine<Source>) -> Result<Output, CliError>,
) -> Result<Output, CliError> {
    let opened = source::open(data)?;
    for n in &opened.notes {
        eprintln!("{n}");
    }
    f(&opened.engine)
}

/// "Philadelphia, Pennsylvania (county 42101, ZIP 19147)".
pub fn place(loc: &LocationResolved) -> String {
    let zip = loc
        .zip
        .as_deref()
        .map(|z| format!(", ZIP {z}"))
        .unwrap_or_default();
    format!(
        "{}, {} (county {}{zip})",
        loc.county_name, loc.state_name, loc.county_fips
    )
}

/// The lines every household report starts with: who, where, the dials, the data.
pub fn header(title: &str, h: &Household, loc: &LocationResolved, src: &Source) -> String {
    let d = &h.input.dials;
    let climate = match d.climate {
        ClimateHorizon::Today => "today's climate",
        ClimateHorizon::Y2050 => "the 2050 climate projection",
    };
    let water = match d.water_level {
        WaterLevel::Survival => "survival water (drinking only)",
        WaterLevel::Basic => "basic water (about 1 gallon a person a day)",
        WaterLevel::Comfortable => "comfortable water (drinking, cooking and washing)",
    };
    let mut s = format!("{title}\n\n");
    s.push_str(&format!(
        "Household  {} ({})\n",
        format::household(&h.input),
        h.name
    ));
    s.push_str(&format!("Where      {}\n", place(loc)));
    let setting = format!(
        "{}, {climate}, the next {} years, {water}",
        format::dial_phrase(d.return_period),
        d.horizon_years
    );
    s.push_str(&format!("Setting    {}\n", wrap(&setting, 88, 11)));
    s.push_str(&format!("Data       {}\n", src.describe()));
    if let Some(note) = &loc.data_note {
        s.push_str(&format!("           {}\n", wrap(note, 88, 11)));
    }
    s
}

/// The "Sources" block: each id with its title, publisher, year and URL, marking expert
/// estimates. Ids the registry does not define are listed as such (never silently dropped).
pub fn sources_legend(ids: &[CitationId], content: &Content) -> String {
    let mut seen: Vec<&CitationId> = Vec::new();
    for id in ids {
        if !seen.contains(&id) {
            seen.push(id);
        }
    }
    seen.sort();
    let mut s = String::from("Sources\n");
    for id in seen {
        match content.citation(id.as_str()) {
            Some(c) => {
                let year = c.year.map(|y| format!(", {y}")).unwrap_or_default();
                let prior = if c.prior { " [expert estimate]" } else { "" };
                s.push_str(&format!(
                    "  {id}  {}. {}{year}.{prior} {}\n",
                    c.title, c.publisher, c.url
                ));
            }
            None => s.push_str(&format!(
                "  {id}  (not in content/citations.toml yet; see `rr citations --missing`)\n"
            )),
        }
    }
    s
}

/// The ids of a list, as one "Sources: a, b" line.
pub fn source_line(ids: &[CitationId]) -> String {
    let v: Vec<&str> = ids.iter().map(CitationId::as_str).collect();
    format!("Sources: {}", v.join(", "))
}

/// The input with a different return period (for the dial sweep).
pub fn with_dial(input: &PlanInput, rp: rr_types::ReturnPeriod) -> PlanInput {
    let mut i = input.clone();
    i.dials.return_period = rp;
    i
}
