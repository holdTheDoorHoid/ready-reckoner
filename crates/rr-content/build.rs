//! Embeds `content/` (and the quantity-rule registry in `docs/QUANTITY_RULES.md`) into the crate.
//!
//! Generates `$OUT_DIR/embedded.rs` with one `include_str!` per file, sorted by path so the
//! output is identical on every machine, plus a content hash (FNV-1a over paths and bytes) that
//! becomes part of `CONTENT_VERSION`. No clock is read, so the same content always produces the
//! same version string.

use std::fs;
use std::path::{Path, PathBuf};

fn collect(dir: &Path, root: &Path, out: &mut Vec<(String, PathBuf)>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|e| e.expect("directory entry").path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            println!("cargo:rerun-if-changed={}", p.display());
            collect(&p, root, out);
        } else if matches!(
            p.extension().and_then(|e| e.to_str()),
            Some("toml") | Some("md")
        ) || p.file_name().and_then(|n| n.to_str()) == Some("VERSION")
        {
            let rel = p
                .strip_prefix(root)
                .expect("path under content root")
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, p));
        }
    }
}

fn fnv1a(hash: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *hash ^= u64::from(*b);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let repo = manifest.join("../..").canonicalize().expect("repo root");
    let content = repo.join("content");
    let rules = repo.join("docs/QUANTITY_RULES.md");
    println!("cargo:rerun-if-changed={}", content.display());
    println!("cargo:rerun-if-changed={}", rules.display());

    let mut files = Vec::new();
    collect(&content, &content, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut body = String::from(
        "/// Every embedded content file as `(path relative to content/, text)`, sorted by path.\n\
         pub static FILES: &[(&str, &str)] = &[\n",
    );
    for (rel, path) in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = fs::read(path).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"));
        fnv1a(&mut hash, rel.as_bytes());
        fnv1a(&mut hash, &[0]);
        fnv1a(&mut hash, &bytes);
        body.push_str(&format!(
            "    ({rel:?}, include_str!({:?})),\n",
            path.display().to_string()
        ));
    }
    body.push_str("];\n\n");
    body.push_str(&format!(
        "/// `docs/QUANTITY_RULES.md`, the shared registry of quantity-rule names.\n\
         pub static QUANTITY_RULES_MD: &str = include_str!({:?});\n\n",
        rules.display().to_string()
    ));
    body.push_str(&format!(
        "/// FNV-1a hash of every embedded content file (paths and bytes).\n\
         pub const CONTENT_HASH: &str = \"{hash:016x}\";\n\n"
    ));
    let version_file = content.join("VERSION");
    let version = fs::read_to_string(&version_file)
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|_| "unversioned".to_owned());
    body.push_str(&format!(
        "/// `content/VERSION` plus the first eight hex digits of [`CONTENT_HASH`].\n\
         pub const CONTENT_VERSION: &str = \"{version}+{}\";\n",
        &format!("{hash:016x}")[..8]
    ));
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("out dir")).join("embedded.rs");
    fs::write(out, body).expect("write embedded.rs");
}
