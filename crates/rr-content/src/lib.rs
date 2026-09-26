//! Ready Reckoner — `rr-content`: the item catalogue, citations, guidance blocks and glossary.
//!
//! Everything under `content/` is embedded at build time (see `build.rs`) and parsed into the
//! shared types from `rr-types` ([`Item`](rr_types::Item), [`Citation`](rr_types::Citation),
//! [`GuidanceMeta`](rr_types::GuidanceMeta)) the first time it is needed. The engine never reads a
//! file at run time, so this crate works the same in the browser (wasm32) and in the CLI.
//!
//! [`validate()`] enforces the mechanical parts of `docs/CONTENT_STANDARDS.md` §3–§5 (every item
//! cites, every citation has a URL and licence, quantity rules exist, no brands, no dosing, firearm
//! words only in the one permitted free action, guidance under 300 words, reading level) and runs in
//! `cargo test`, so content that breaks the policy never reaches a build that passes CI.
//!
//! # Where things are
//!
//! - [`parse`]: the [`Content`] container, file parsing, lookups and [`Catalogue`](rr_types::Catalogue).
//! - [`validate`](mod@validate): the content validator and its [`Report`].
//! - [`policy`]: the word lists the validator enforces (brands, firearm words, dosing units) and
//!   the conditional spans of guidance blocks ([`policy::Condition`], [`policy::HouseholdFacts`]).
//! - [`ids`]: the guidance kinds ([`GuidanceKind`]) and the ids content may name before engine
//!   contract v2 reaches `rr-types`.
//! - [`tables`]: the state table of zone lookups, registries, alert sign-ups and refill rules.
//! - [`readability`]: word, sentence and syllable counts and the Flesch-Kincaid grade.
//! - [`rules`]: the parser for the quantity-rule table in `docs/QUANTITY_RULES.md`.
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(missing_docs))]

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

pub mod ids;
pub mod parse;
pub mod policy;
pub mod readability;
pub mod rules;
pub mod tables;
pub mod validate;

pub use ids::GuidanceKind;
pub use parse::{Content, GlossaryEntry, Guidance, LoadError};
pub use tables::{StateLine, StateRow, StateTable};
pub use validate::{Finding, Report, Severity, validate};

use std::sync::OnceLock;

/// Crate name, used by the CLI's `--version` and by the about screen.
pub const CRATE: &str = "rr-content";

/// Version of the embedded content: `content/VERSION` plus the first eight hex digits of
/// [`CONTENT_HASH`]. It changes whenever any content file changes, and never otherwise.
pub const CONTENT_VERSION: &str = embedded::CONTENT_VERSION;

/// FNV-1a hash (hex) of every embedded content file, paths included.
pub const CONTENT_HASH: &str = embedded::CONTENT_HASH;

/// Every embedded content file as `(path relative to content/, text)`, sorted by path.
pub fn embedded_files() -> &'static [(&'static str, &'static str)] {
    embedded::FILES
}

/// The text of `docs/QUANTITY_RULES.md` as it was when this crate was built.
pub fn quantity_rules_markdown() -> &'static str {
    embedded::QUANTITY_RULES_MD
}

static CONTENT: OnceLock<Result<Content, LoadError>> = OnceLock::new();

/// The embedded content, parsed once. Returns the parse error if a content file is malformed
/// (the test suite guarantees it is not, so a passing build never returns an error here).
pub fn try_content() -> Result<&'static Content, &'static LoadError> {
    CONTENT
        .get_or_init(|| Content::from_files(embedded::FILES))
        .as_ref()
}

/// The embedded content, parsed once.
///
/// # Panics
///
/// If an embedded content file does not parse. `cargo test -p rr-content` fails in that case, so
/// this cannot happen in a build that passed CI; engine entry points that must never panic can call
/// [`try_content`] instead.
pub fn content() -> &'static Content {
    match try_content() {
        Ok(c) => c,
        Err(e) => panic!("embedded content failed to parse: {e}"),
    }
}

/// What the engine's `catalogue()` function returns: every item, citation and guidance block's
/// metadata, plus the plain names of every hazard, bucket and tier.
pub fn catalogue() -> rr_types::Catalogue {
    content().catalogue()
}

/// Validates the embedded content against the embedded quantity-rule table.
pub fn validate_embedded() -> Report {
    match try_content() {
        Ok(c) => {
            let rules = rules::parse_rule_table(quantity_rules_markdown());
            validate(c, &rules)
        }
        Err(e) => Report::from_load_error(e),
    }
}
