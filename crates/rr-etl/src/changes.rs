//! `data/CHANGES.md`: a dated, human-readable summary appended by every refresh, so a reviewer
//! of the quarterly data pull request can see which packs moved and by how much.

use crate::Result;
use crate::jobs::RefreshSummary;
use crate::manifest::Manifest;
use std::io::Write;
use std::path::Path;

/// Append one dated section for this refresh.
pub fn append(data_dir: &Path, manifest: &Manifest, summary: &RefreshSummary) -> Result<()> {
    let path = data_dir.join("CHANGES.md");
    let mut text = String::new();
    if !path.exists() {
        text.push_str("# Data pack changes\n\nEach `rr-etl refresh` appends a section: which files changed and how many rows were added, removed or changed (by key). Provenance for every file is in `manifest.json`.\n");
    }
    text.push_str(&format!(
        "\n## {} — pack version {}\n\n",
        crate::timefmt::now_utc(),
        manifest.pack_version
    ));
    let ok = if summary.ok.is_empty() {
        "none".to_string()
    } else {
        summary.ok.join(", ")
    };
    text.push_str(&format!("Jobs run: {ok}.\n"));
    for (id, err) in &summary.failed {
        text.push_str(&format!("Job **{id} failed**: {err}\n"));
    }
    if !summary.changes.is_empty() {
        text.push_str("\n| File | Rows before | Rows after | Added | Removed | Changed | Status |\n|---|---:|---:|---:|---:|---:|---|\n");
        let mut changes = summary.changes.clone();
        changes.sort_by(|a, b| a.path.cmp(&b.path));
        for w in &changes {
            let d = &w.diff;
            let status = if d.new_file {
                "new"
            } else if d.identical {
                "unchanged"
            } else {
                "changed"
            };
            text.push_str(&format!(
                "| `{}` | {} | {} | {} | {} | {} | {} |\n",
                w.path, d.previous_rows, d.rows, d.added, d.removed, d.changed, status
            ));
        }
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    f.write_all(text.as_bytes())?;
    Ok(())
}
