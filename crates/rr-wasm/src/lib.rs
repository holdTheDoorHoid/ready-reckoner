//! Ready Reckoner — `rr-wasm`: the engine as WebAssembly, for the web app.
//!
//! The eight exports below are the `RawEngine` surface of `web/src/engine/index.ts` and
//! `docs/ENGINE-API.md`: strings (and, for `load_pack`, bytes) in, the JSON of an envelope out.
//! `web/src/engine/wasm.ts` wraps them as the app's `Engine`, fetches the data packs from the
//! site's own origin and hands them to [`load_pack`]; the engine itself never touches the network,
//! the clock or any randomness.
//!
//! ```text
//! JavaScript ── JSON string ──► export ─► api::* ─► rr_plan::Engine<WasmSource> ─► envelope JSON
//!                                                        │
//!                              data packs (rr-data) ◄────┴────► built-in sample counties
//!                              once loaded                      before any pack is loaded
//! ```
//!
//! Build for the site with `bash crates/rr-wasm/build-web.sh` (wasm-pack, `--target web`, output
//! in `web/public/pkg/`, data packs copied to `web/public/data/`). Loading order and error
//! behaviour: `docs/ENGINE-API.md`, "Loading".
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(missing_docs))]

pub mod api;
pub mod source;

use wasm_bindgen::prelude::wasm_bindgen;

pub use source::{Mode, WasmSource};

/// Crate name, used by the CLI's `--version` and by the about screen.
pub const CRATE: &str = "rr-wasm";

/// `engine_info()`: `EngineInfo` with versions, loaded packs and attributions.
#[wasm_bindgen]
pub fn engine_info() -> String {
    api::engine_info()
}

/// `load_pack(name, bytes)`: loads one data file; `name` is its path in `data/manifest.json`
/// (`manifest.json` first, then `core/…`). Returns `PackInfo`.
#[wasm_bindgen]
pub fn load_pack(name: &str, bytes: &[u8]) -> String {
    api::load_pack(name, bytes)
}

/// `county_search(query)`: up to ten `LocationResolved`, best first.
#[wasm_bindgen]
pub fn county_search(query: &str) -> String {
    api::county_search(query)
}

/// `resolve_location(json)`: `LocationInput` JSON to `LocationResolved`.
#[wasm_bindgen]
pub fn resolve_location(input_json: &str) -> String {
    api::resolve_location(input_json)
}

/// `assess(json)`: `PlanInput` JSON to `PlanOutput`.
#[wasm_bindgen]
pub fn assess(input_json: &str) -> String {
    api::assess(input_json)
}

/// `explain(json)`: `ExplainRequest` JSON to `Explanation`.
#[wasm_bindgen]
pub fn explain(request_json: &str) -> String {
    api::explain(request_json)
}

/// `catalogue()`: the `Catalogue`.
#[wasm_bindgen]
pub fn catalogue() -> String {
    api::catalogue()
}

/// `defaults()`: a valid `PlanInput` skeleton.
#[wasm_bindgen]
pub fn defaults() -> String {
    api::defaults()
}
