//! `rr plan`: the printable packet (Markdown) and/or the whole PlanOutput (JSON). The JSON is
//! written with `rr_plan::to_json`, so for a fixture household on the fixture counties it is the
//! same bytes as `fixtures/golden/<name>.json`.

use rr_plan::Engine;

use crate::Output;
use crate::args::{Format, PlanArgs};
use crate::error::CliError;
use crate::household;
use crate::source::Source;

/// Runs `rr plan`.
///
/// # Errors
///
/// Household problems or an unknown location (exit 2); an engine or file error (exit 1).
pub fn run(engine: &Engine<Source>, args: &PlanArgs) -> Result<Output, CliError> {
    if args.format == Format::Both && args.out.is_none() {
        return Err(CliError::input(
            "--format both writes two files, the packet and the JSON: add --out <dir>.",
        ));
    }
    let h = household::load(&args.household)?;
    let output = engine
        .assess(&h.input)
        .map_err(|e| CliError::engine(&e, engine.store().location_hint().as_deref()))?;
    let mut out = Output {
        notes: household::scenario_notes(&h, &output.scenarios, &super::place(&output.location)),
        ..Output::default()
    };
    let md = || output.packet_markdown.clone();
    let json = || rr_plan::to_json(&output);
    match &args.out {
        None => {
            out.stdout = match args.format {
                Format::Md => md(),
                Format::Json | Format::Both => json(),
            };
        }
        Some(dir) => {
            std::fs::create_dir_all(dir).map_err(|e| {
                CliError::failure(format!("Could not create {}: {e}", dir.display()))
            })?;
            let mut files: Vec<(&str, String)> = Vec::new();
            if matches!(args.format, Format::Md | Format::Both) {
                files.push(("md", md()));
            }
            if matches!(args.format, Format::Json | Format::Both) {
                files.push(("json", json()));
            }
            for (ext, body) in files {
                let path = dir.join(format!("{}.{ext}", h.name));
                std::fs::write(&path, body).map_err(|e| {
                    CliError::failure(format!("Could not write {}: {e}", path.display()))
                })?;
                out.notes.push(format!("wrote {}", path.display()));
            }
        }
    }
    Ok(out)
}
