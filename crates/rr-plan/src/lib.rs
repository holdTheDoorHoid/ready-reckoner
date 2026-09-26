//! Ready Reckoner — `rr-plan`: the engine pipeline, [`PlanOutput`], `explain`, and the printable
//! packet (DESIGN §4.8, §5, §9; `docs/ENGINE-API.md`).
//!
//! ```text
//! PlanInput ─► location ─► rr-hazards ─► rr-consequence ─► rr-supply ─► rr-budget ─► PlanOutput
//!                (source)   (rates)       (targets)          (lines)       (plan)       + packet
//! ```
//!
//! [`Engine`] owns a county data source ([`CountySource`]; [`FixtureSource`] until the national
//! data pack lands) and the embedded content, and answers every engine function in
//! `docs/ENGINE-API.md`: [`Engine::assess`], [`Engine::explain`], [`Engine::county_search`],
//! [`Engine::resolve_location`], [`Engine::catalogue`], [`Engine::defaults`] and
//! [`Engine::engine_info`]. `rr-wasm` and `rr-cli` wrap it.
//!
//! What this crate adds to the other engine crates:
//!
//! - the join between the item catalogue and `rr-supply`'s requirement lines that the allocator
//!   needs ([`coverage`]);
//! - the final bucket assessments (the plan's coverage, and `tier_enough` from `rr-supply`);
//! - one list of warnings (the consequence crate's cliff warnings are kept, the budget crate's
//!   duplicates dropped);
//! - the provenance list and the packet ([`packet`], `docs/PACKET.md`).
//!
//! Deterministic: no clock (the planning date is an input), no randomness, no hash-map iteration;
//! the same input gives byte-identical output on every target ([`to_json`] writes it with every
//! object's keys sorted).
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(missing_docs))]

pub mod coverage;
pub mod explain;
pub mod golden;
pub mod location;
pub mod packet;
pub mod pipeline;
pub mod provenance;
pub mod source;

use rr_content::Content;
use rr_types::{
    Catalogue, EngineError, EngineInfo, ErrorCode, ExplainKind, ExplainRequest, Explanation,
    LocationInput, LocationResolved, PlanInput, PlanOutput,
};

pub use pipeline::Assessment;
pub use source::{CountySource, FixtureSource};

/// Crate name, used by the CLI's `--version` and by the about screen.
pub const CRATE: &str = "rr-plan";

/// Engine version (semver), reported in [`PlanOutput::engine_version`] and [`EngineInfo`].
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Reported as the data pack version when the source has loaded nothing.
pub const NO_DATA_PACK: &str = "none";

/// The engine: a county data source plus the embedded content.
#[derive(Debug, Clone)]
pub struct Engine<S: CountySource = FixtureSource> {
    store: S,
    content: &'static Content,
}

impl Engine<FixtureSource> {
    /// An engine on the seven fixture counties (see [`FixtureSource`]).
    ///
    /// # Errors
    ///
    /// If the embedded fixtures or content do not parse (the test suite guarantees they do).
    pub fn with_fixtures() -> Result<Self, EngineError> {
        Engine::new(FixtureSource::new()?)
    }
}

impl<S: CountySource> Engine<S> {
    /// An engine over a data source.
    ///
    /// # Errors
    ///
    /// `internal` if the embedded content does not parse (`cargo test -p rr-content` prevents
    /// that).
    pub fn new(store: S) -> Result<Self, EngineError> {
        let content = rr_content::try_content().map_err(|e| {
            EngineError::new(
                ErrorCode::Internal,
                format!("The engine's built-in content could not be read: {e}"),
            )
        })?;
        Ok(Self { store, content })
    }

    /// The data source.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// The data source, for loading packs.
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    /// The embedded content.
    pub fn content(&self) -> &'static Content {
        self.content
    }

    /// `engine_info()`: versions, loaded packs and the attributions the app must show.
    pub fn engine_info(&self) -> EngineInfo {
        EngineInfo {
            engine_version: ENGINE_VERSION.to_owned(),
            api_version: rr_types::ENGINE_API_VERSION,
            data_pack_version: self.store.pack_version(),
            content_version: rr_content::CONTENT_VERSION.to_owned(),
            packs_loaded: self.store.packs_loaded(),
            attributions: self.store.attributions(),
        }
    }

    /// `catalogue()`: every item, citation and guidance block, and the plain names of every id.
    pub fn catalogue(&self) -> Catalogue {
        self.content.catalogue()
    }

    /// `defaults()`: a valid input skeleton the interview starts from.
    pub fn defaults(&self) -> PlanInput {
        PlanInput::defaults()
    }

    /// `county_search(query)`: up to ten counties, best first.
    pub fn county_search(&self, query: &str) -> Vec<LocationResolved> {
        self.store.search(query)
    }

    /// `resolve_location(input)`.
    ///
    /// # Errors
    ///
    /// `unknown_zip`, `unknown_county` or `ambiguous_zip` with suggestions; `pack_missing` before
    /// county data is loaded; `bad_input` without a ZIP code or county.
    pub fn resolve_location(&self, input: &LocationInput) -> Result<LocationResolved, EngineError> {
        self.store.resolve(input)
    }

    /// Runs the pipeline and keeps every intermediate result (for the CLI, `explain` and tests).
    ///
    /// # Errors
    ///
    /// As [`Engine::assess`].
    pub fn run(&self, input: &PlanInput) -> Result<Assessment, EngineError> {
        pipeline::run(&self.store, self.content, input)
    }

    /// `assess(input)`: the whole plan.
    ///
    /// # Errors
    ///
    /// `bad_input` with every problem; the location errors of [`Engine::resolve_location`];
    /// `internal` for a bug in an engine crate.
    pub fn assess(&self, input: &PlanInput) -> Result<PlanOutput, EngineError> {
        let a = self.run(input)?;
        Ok(self.output(&a))
    }

    /// The [`PlanOutput`] for an assessment: provenance, warnings and the packet.
    pub fn output(&self, a: &Assessment) -> PlanOutput {
        let marked = packet::render(a, self.content);
        let first = packet::cited_ids(&marked);
        let rest = provenance::output_ids(a, self.content);
        let order = provenance::order(&first, &rest);
        let (citations, unknown) = provenance::resolve(&order, self.content);
        let mut warnings = a.warnings.clone();
        warnings.extend(provenance::missing_warning(&unknown));
        let packet_markdown = packet::finish(&marked, a, &citations);
        PlanOutput {
            engine_version: ENGINE_VERSION.to_owned(),
            api_version: rr_types::ENGINE_API_VERSION,
            data_pack_version: self
                .store
                .pack_version()
                .unwrap_or_else(|| NO_DATA_PACK.to_owned()),
            content_version: rr_content::CONTENT_VERSION.to_owned(),
            location: a.location.clone(),
            register: a.hazards.profiles.clone(),
            buckets: a.buckets.clone(),
            scenarios: a.consequence.scenarios.clone(),
            tier_reached: a.budget.tier_reached,
            tier_recommended: rr_supply::tier_recommended(&a.buckets),
            plan: a.budget.plan.clone(),
            requirements: a.lines.iter().map(|l| l.line.clone()).collect(),
            warnings,
            packet_markdown,
            provenance: citations,
        }
    }

    /// `explain(kind, id, input)`.
    ///
    /// # Errors
    ///
    /// As [`Engine::assess`], and `bad_input` when the id names nothing in the plan.
    pub fn explain(
        &self,
        kind: ExplainKind,
        id: &str,
        input: &PlanInput,
    ) -> Result<Explanation, EngineError> {
        let a = self.run(input)?;
        explain::explain(&a, self.content, kind, id)
    }

    /// `explain(request)`, the contract's shape.
    ///
    /// # Errors
    ///
    /// As [`Engine::explain`].
    pub fn explain_request(&self, req: &ExplainRequest) -> Result<Explanation, EngineError> {
        self.explain(req.kind, &req.id, &req.input)
    }
}

/// The output as pretty JSON with every object's keys sorted, so the same input always gives the
/// same bytes (the goldens use it).
pub fn to_json<T: serde::Serialize>(value: &T) -> String {
    let v = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
    let mut out = String::new();
    write_sorted(&v, 0, &mut out);
    out.push('\n');
    out
}

fn write_sorted(v: &serde_json::Value, indent: usize, out: &mut String) {
    use serde_json::Value;
    let pad = |n: usize| "  ".repeat(n);
    match v {
        Value::Object(map) => {
            if map.is_empty() {
                out.push_str("{}");
                return;
            }
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push_str("{\n");
            for (i, k) in keys.iter().enumerate() {
                out.push_str(&pad(indent + 1));
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push_str(": ");
                write_sorted(&map[k.as_str()], indent + 1, out);
                if i + 1 < keys.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&pad(indent));
            out.push('}');
        }
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                out.push_str(&pad(indent + 1));
                write_sorted(item, indent + 1, out);
                if i + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&pad(indent));
            out.push(']');
        }
        other => out.push_str(&serde_json::to_string(other).unwrap_or_default()),
    }
}
