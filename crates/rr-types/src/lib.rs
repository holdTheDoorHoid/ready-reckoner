//! Ready Reckoner — `rr-types`: the shared contract between every engine crate and the web app.
//!
//! It holds every type in `docs/DESIGN.md` §4 and `docs/ENGINE-API.md`, and nothing that
//! computes a plan. The one piece of arithmetic is [`DurationDist::cdf`] (with its complement
//! [`DurationDist::survival`]).
//!
//! # Conventions
//!
//! - **JSON is the contract.** Field names are snake_case; enums serialise as their snake_case
//!   string ids ([`HazardId::as_str`]); optional fields are omitted when absent; dates are ISO
//!   `YYYY-MM-DD` ([`Date`]). `web/src/engine/types.ts` mirrors every type by hand and a test in
//!   this crate compares the two.
//! - **Inputs fail loudly.** Every struct denies unknown fields, so a typo in the app, a fixture
//!   or a content file is an error, not a silently ignored value.
//! - **Units.** Money is `f32` US dollars and days are `f32` in outputs; probabilities, severity
//!   and coordinates are `f64`.
//! - **Determinism.** No clock, no OS entropy, no `rand`. Use [`math`] for transcendental
//!   functions so native and WebAssembly builds give the same bits, and [`rng::SplitMix64`] if
//!   you ever need pseudo-random numbers.
//!
//! # Modules
//!
//! - [`ids`]: hazards, buckets, tiers, citation and item ids.
//! - [`input`]: [`PlanInput`] and its parts; [`PlanInput::defaults`].
//! - [`validate`]: [`PlanInput::validate`] and [`Problem`].
//! - [`output`]: [`PlanOutput`] and its parts.
//! - [`content`]: catalogue [`Item`], [`Citation`], [`GuidanceMeta`].
//! - [`effect`]: [`Effect`], [`DurationDist`] and [`HouseholdEventRate`].
//! - [`data`]: data-pack records ([`CountyRecord`], [`BaseRate`]); engine-internal.
//! - [`api`]: [`Envelope`], [`EngineError`], and the other function arguments and results.
//! - [`date`], [`math`], [`rng`], [`fixtures`].
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(missing_docs))]

#[macro_use]
mod macros;

pub mod api;
pub mod content;
pub mod data;
pub mod date;
pub mod effect;
pub mod fixtures;
pub mod ids;
pub mod input;
pub mod math;
pub mod output;
pub mod rng;
pub mod validate;

pub use api::*;
pub use content::*;
pub use data::*;
pub use date::*;
pub use effect::*;
pub use ids::*;
pub use input::*;
pub use output::*;
pub use validate::*;

/// Version of the engine contract in `docs/ENGINE-API.md`. Bump it whenever a type, id or function
/// changes shape, and update `docs/ENGINE-API.md` and `web/src/engine/types.ts` in the same
/// commit.
pub const ENGINE_API_VERSION: u32 = 1;

/// Crate name, used by the CLI's `--version` and by the about screen.
pub const CRATE: &str = "rr-types";
