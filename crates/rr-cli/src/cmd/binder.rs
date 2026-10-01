//! `rr binder`: the household's binder alone (DESIGN-DELTA-v3 §4), as Markdown (the renderer the
//! goldens use) or as the Binder JSON (`PlanOutput.binder`, the tree the web app renders). The
//! checklists the binder still lacks, if any, are named on standard error.

use rr_plan::Engine;

use crate::Output;
use crate::args::BinderArgs;
use crate::error::CliError;
use crate::household;
use crate::source::Source;

/// Runs `rr binder`.
///
/// # Errors
///
/// Household problems or an unknown location (exit 2); an engine error (exit 1).
pub fn run(engine: &Engine<Source>, args: &BinderArgs) -> Result<Output, CliError> {
    let h = household::load(&args.household)?;
    let output = engine
        .assess(&h.input)
        .map_err(|e| CliError::engine(&e, engine.store().location_hint().as_deref()))?;
    let mut out = Output {
        notes: household::scenario_notes(&h, &output.scenarios, &super::place(&output.location)),
        ..Output::default()
    };
    if let Ok(a) = engine.run(&h.input) {
        for p in rr_plan::binder::problems(&a, engine.content()) {
            out.notes.push(format!("note: {p}"));
        }
    }
    out.stdout = if args.json {
        rr_plan::to_json(&output.binder)
    } else {
        rr_plan::binder::markdown::render(&output.binder)
    };
    Ok(out)
}
