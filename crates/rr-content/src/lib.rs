//! Ready Reckoner — `rr-content`.
//!
//! See `docs/DESIGN.md` for where this crate sits in the pipeline. This is a scaffold; the
//! Phase 1 workstream that owns this crate replaces it.
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(missing_docs))]

/// Crate name, used by the CLI's `--version` and by the about screen.
pub const CRATE: &str = "rr-content";
