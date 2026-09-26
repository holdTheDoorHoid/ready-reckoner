//! `rr golden [--update]`: the golden packets in `fixtures/golden/`.
//!
//! By default the goldens are rendered by rr-plan's own helper (`rr_plan::golden::render_all`,
//! the same one `cargo test -p rr-plan --test goldens` uses), whatever data source that helper
//! uses, and compared with the files; `--update` rewrites them with `rr_plan::golden::write_all`.
//! Nothing here knows what a golden contains. With `--data <dir>` or `--fixtures` the fixture
//! households are rendered on that source instead and compared with the same files (read-only):
//! a preview of what would change if the goldens moved to that source.

use std::path::{Path, PathBuf};

use rr_plan::Engine;
use rr_plan::golden::{self, Golden};

use crate::Output;
use crate::args::{DataArgs, GoldenArgs};
use crate::error::{CliError, Exit};
use crate::source::{self, Source};

/// Runs `rr golden`.
///
/// # Errors
///
/// A fixture that fails to plan, or a file that cannot be written (exit 1); `--update` together
/// with `--data` or `--fixtures` (exit 2). Files that differ are a result (exit 1).
pub fn run(data: &DataArgs, args: &GoldenArgs) -> Result<Output, CliError> {
    let dir = golden::golden_dir();
    let shown = display_path(&dir);
    if args.update {
        if data.explicit() {
            return Err(CliError::input(
                "golden --update writes rr-plan's own rendering (the one its tests check), so it \
                 cannot be combined with --data or --fixtures. To see how another source \
                 differs, run `rr --data <dir> golden` without --update.",
            ));
        }
        return update(&dir, &shown);
    }
    let (goldens, rendered_by) = if data.explicit() {
        let opened = source::open(data)?;
        for n in &opened.notes {
            eprintln!("{n}");
        }
        let label = format!(
            "{} (read-only comparison; the goldens themselves come from rr-plan's helper)",
            opened.engine.store().describe()
        );
        (render_with(&opened.engine)?, label)
    } else {
        (
            golden::render_all().map_err(CliError::failure)?,
            "rr-plan's golden helper (rr_plan::golden::render_all)".to_owned(),
        )
    };
    Ok(compare(
        &dir,
        &shown,
        &goldens,
        &rendered_by,
        data.explicit(),
    ))
}

/// Renders every fixture household on `engine`, exactly as `rr_plan::golden::render_all` does
/// on the repository's data packs.
fn render_with(engine: &Engine<Source>) -> Result<Vec<Golden>, CliError> {
    let mut out = Vec::new();
    for (name, input) in rr_types::fixtures::all() {
        let output = engine
            .assess(&input)
            .map_err(|e| CliError::failure(format!("{name}: {e}")))?;
        out.push(Golden {
            name: name.to_owned(),
            markdown: output.packet_markdown.clone(),
            json: rr_plan::to_json(&output),
        });
    }
    Ok(out)
}

/// Every golden file a rendering produces: `(file name, contents)`.
fn files(goldens: &[Golden]) -> Vec<(String, &str)> {
    goldens
        .iter()
        .flat_map(|g| {
            [
                (format!("{}.md", g.name), g.markdown.as_str()),
                (format!("{}.json", g.name), g.json.as_str()),
            ]
        })
        .collect()
}

fn compare(
    dir: &Path,
    shown: &str,
    goldens: &[Golden],
    rendered_by: &str,
    read_only: bool,
) -> Output {
    let expected = files(goldens);
    let mut s = format!(
        "Golden files in {shown}: {} fixtures, {} files\nRendered by {rendered_by}\n\n",
        goldens.len(),
        expected.len()
    );
    let mut diffs = String::new();
    let mut matching = 0usize;
    for (name, body) in &expected {
        let path = dir.join(name);
        let status = match std::fs::read_to_string(&path) {
            Ok(on_disk) if on_disk == *body => {
                matching += 1;
                "ok".to_owned()
            }
            Ok(on_disk) => {
                diffs.push_str(&format!("\n--- {shown}/{name} differs:\n"));
                diffs.push_str(&golden::diff(&on_disk, body));
                "DIFFERS".to_owned()
            }
            Err(e) => format!("MISSING ({e})"),
        };
        s.push_str(&format!("  {status:<8} {name}\n"));
    }
    s.push_str(&diffs);
    let mut out = Output::default();
    let stale = stale_files(dir, &expected);
    if !stale.is_empty() {
        out.notes.push(format!(
            "note: {} in {shown} {} not produced by any fixture: {}",
            if stale.len() == 1 {
                "this file"
            } else {
                "these files"
            },
            if stale.len() == 1 { "is" } else { "are" },
            stale.join(", ")
        ));
    }
    s.push_str(&format!(
        "\n{matching} of {} golden files match.\n",
        expected.len()
    ));
    if matching < expected.len() {
        out.exit = Exit::Failure;
        s.push_str(if read_only {
            "This was a read-only comparison: the goldens stay as rr-plan's helper renders them \
             until that helper changes source.\n"
        } else {
            "If the change is intended, run `cargo run -p rr-cli -- golden --update` and explain \
             the change in the commit message.\n"
        });
    }
    out.stdout = s;
    out
}

fn update(dir: &Path, shown: &str) -> Result<Output, CliError> {
    let goldens = golden::render_all().map_err(CliError::failure)?;
    let changed: Vec<String> = files(&goldens)
        .into_iter()
        .filter(|(name, body)| {
            std::fs::read_to_string(dir.join(name)).map_or(true, |on_disk| on_disk != *body)
        })
        .map(|(name, _)| name)
        .collect();
    let written = golden::write_all().map_err(CliError::failure)?;
    let mut s = format!(
        "Rewrote {} golden files in {shown} with rr-plan's helper (rr_plan::golden::write_all).\n",
        written.len()
    );
    if changed.is_empty() {
        s.push_str("None changed.\n");
    } else {
        s.push_str(&format!("{} changed:\n", changed.len()));
        for name in &changed {
            s.push_str(&format!("  {name}\n"));
        }
        s.push_str("Explain the change in the commit message (CLAUDE.md).\n");
    }
    Ok(Output::text(s))
}

/// Golden-looking files no fixture produces (a renamed or removed fixture).
fn stale_files(dir: &Path, expected: &[(String, &str)]) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".md") || n.ends_with(".json"))
        .filter(|n| !expected.iter().any(|(x, _)| x == n))
        .collect();
    v.sort();
    v
}

/// The golden directory as the reader knows it: relative to the current directory when inside
/// it, otherwise the canonical path.
fn display_path(dir: &Path) -> String {
    let canonical: PathBuf = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    std::env::current_dir()
        .ok()
        .and_then(|cwd| cwd.canonicalize().ok())
        .and_then(|cwd| canonical.strip_prefix(&cwd).ok().map(Path::to_path_buf))
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(canonical)
        .display()
        .to_string()
}
