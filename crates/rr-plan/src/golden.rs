//! Golden files: `fixtures/golden/<fixture>.md` (the packet) and `.json` (the whole
//! [`rr_types::PlanOutput`], keys sorted) for every fixture household. [`compare_all`] is what the
//! tests (and `rr golden`) run; [`write_all`] regenerates them when `RR_UPDATE_GOLDENS=1`. A change
//! to a golden must be explained in the commit message (CLAUDE.md).

use std::path::{Path, PathBuf};

use crate::{Engine, to_json};

/// Set to `1` to rewrite the goldens instead of comparing.
pub const UPDATE_ENV: &str = "RR_UPDATE_GOLDENS";

/// One fixture's rendered goldens.
#[derive(Debug, Clone, PartialEq)]
pub struct Golden {
    /// The fixture's name (file stem).
    pub name: String,
    /// The packet.
    pub markdown: String,
    /// The PlanOutput as sorted JSON.
    pub json: String,
}

/// `fixtures/golden` in the repository.
pub fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden")
}

/// Renders every fixture household with the fixture counties.
///
/// # Errors
///
/// The first fixture that fails to assess, with its name.
pub fn render_all() -> Result<Vec<Golden>, String> {
    let engine = Engine::with_fixtures().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for (name, input) in rr_types::fixtures::all() {
        let output = engine.assess(&input).map_err(|e| format!("{name}: {e}"))?;
        out.push(Golden {
            name: name.to_owned(),
            markdown: output.packet_markdown.clone(),
            json: to_json(&output),
        });
    }
    Ok(out)
}

/// Writes every golden file.
///
/// # Errors
///
/// A rendering or file-system error, as text.
pub fn write_all() -> Result<Vec<PathBuf>, String> {
    let dir = golden_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut written = Vec::new();
    for g in render_all()? {
        for (ext, body) in [("md", &g.markdown), ("json", &g.json)] {
            let path = dir.join(format!("{}.{ext}", g.name));
            std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
            written.push(path);
        }
    }
    Ok(written)
}

/// Compares every golden file with a fresh rendering.
///
/// # Errors
///
/// A readable report of every file that differs (or is missing), with the changed lines.
pub fn compare_all() -> Result<(), String> {
    let dir = golden_dir();
    let mut report = String::new();
    for g in render_all()? {
        for (ext, body) in [("md", &g.markdown), ("json", &g.json)] {
            let path = dir.join(format!("{}.{ext}", g.name));
            match std::fs::read_to_string(&path) {
                Ok(expected) if expected == *body => {}
                Ok(expected) => {
                    report.push_str(&format!("\n--- {} differs:\n", path.display()));
                    report.push_str(&diff(&expected, body));
                }
                Err(e) => report.push_str(&format!("\n--- {} is missing ({e})\n", path.display())),
            }
        }
    }
    if report.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "golden files differ from the engine's output. If the change is intended, run \
             `{UPDATE_ENV}=1 cargo test -p rr-plan --test goldens` and explain the change in the \
             commit message.{report}"
        ))
    }
}

/// A short line diff: the region between the common head and tail, with a little context.
pub fn diff(expected: &str, actual: &str) -> String {
    const CONTEXT: usize = 3;
    const MAX_LINES: usize = 60;
    let a: Vec<&str> = expected.lines().collect();
    let b: Vec<&str> = actual.lines().collect();
    let head = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let tail = a[head..]
        .iter()
        .rev()
        .zip(b[head..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (a_end, b_end) = (a.len() - tail, b.len() - tail);
    let start = head.saturating_sub(CONTEXT);
    let mut out = String::new();
    out.push_str(&format!(
        "@@ expected lines {}-{}, actual lines {}-{} @@\n",
        head + 1,
        a_end,
        head + 1,
        b_end
    ));
    for line in &a[start..head] {
        out.push_str(&format!("  {line}\n"));
    }
    for line in a[head..a_end].iter().take(MAX_LINES) {
        out.push_str(&format!("- {line}\n"));
    }
    if a_end - head > MAX_LINES {
        out.push_str(&format!("- ... {} more\n", a_end - head - MAX_LINES));
    }
    for line in b[head..b_end].iter().take(MAX_LINES) {
        out.push_str(&format!("+ {line}\n"));
    }
    if b_end - head > MAX_LINES {
        out.push_str(&format!("+ ... {} more\n", b_end - head - MAX_LINES));
    }
    for line in a[a_end..].iter().take(CONTEXT) {
        out.push_str(&format!("  {line}\n"));
    }
    out
}
