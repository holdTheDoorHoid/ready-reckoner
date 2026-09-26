//! Ready Reckoner — `rr-consequence`: from hazard rates to consequence buckets, exceedance curves,
//! targets, ranges and plain-language frequency sentences (docs/DESIGN.md §4.3–§4.5,
//! docs/research/risk-model.md §2, §3, §7).
//!
//! # The model
//!
//! Each hazard `h` reaches the household `r_h` times a year ([`rr_types::HouseholdEventRate`],
//! from `rr-hazards`). The effects table ([`effects`], `effects.toml`) says what an event does:
//! with chance `q` it causes bucket `b`'s consequence, which lasts a random time with survival
//! curve `S(d) = P(D > d)`. For every duration bucket the central object is
//!
//! ```text
//! Λ_b(d) = Σ_h r_h · q_{h,b} · S_{h,b}(d)        disruptions per year lasting longer than d days
//! ```
//!
//! and everything follows from it ([`curve`]):
//!
//! - the **target** at return period N: the smallest ladder value `d` with Λ_b(d) ≤ 1/N
//!   ([`rr_types::TARGET_LADDER_DAYS`]);
//! - **natural frequencies**: of 100 households like yours, `100·(1 − e^(−T·Λ_b(d)))` face a
//!   disruption longer than `d` in the next `T` years;
//! - the **value of supplies**: covering day `x` is worth Λ_b(x), so
//!   [`ConsequenceAssessment::value_between`] = ∫ Λ is what the budget buys;
//! - **consumption**: Σ r·q·E[D] days per year.
//!
//! Household facts change the model through explainable **coupling rules** (a well pump makes
//! every power cut a water cut; a furnace that needs power turns winter power cuts into dangerous
//! cold; refrigerated medicine inherits power cuts longer than a day; and so on). County records
//! **override** default durations (EAGLE-I outage statistics for county-wide storm outages; county
//! event records). **Ranges** come from a seeded Latin-hypercube Monte Carlo over every uncertain
//! input ([`ranges`]), and the one or two inputs that move a target most are named. The **cliff
//! rule** warns when one event dominates a target and sits near the dial, and each **named
//! scenario** (Cascadia and others) is reported with what it changes.
//!
//! Readiness buckets (`evacuate`, `get_home`, `medical_emergency`, `fire`, `security`) report the
//! ten-year chance of needing the capability; `evacuate` adds the warning band and the typical
//! days away. Money buckets: `income` is months of income gap after unemployment insurance
//! ([`income`], research §3.5), `home_loss` the ten-year displacement chance.
//!
//! # Entry point
//!
//! [`assess`] takes the household, the hazard rates, the county's records and the scenario
//! candidates and returns a [`ConsequenceAssessment`]: the 14 [`rr_types::BucketAssessment`]s,
//! [`rr_types::ScenarioInfo`]s, cliff [`rr_types::Warning`]s, the self-sufficiency statement, and
//! the curves and numbers behind every target.
//!
//! Deterministic: no clock, no OS entropy; transcendental maths through [`rr_types::math`].
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(missing_docs))]

pub mod assess;
pub mod curve;
pub mod effects;
pub mod income;
pub mod model;
pub mod ranges;
pub mod survival;
pub mod words;

pub use assess::{
    BucketDetail, CommuteWalk, ConsequenceAssessment, DURATION_BUCKETS, EvacuateDetail,
    GAS_STOVE_ITEM_IDS, GetHomeDetail, HomeLossDetail, IncomeDetail, TermSummary, assess,
    assess_with_draws,
};
pub use curve::{CurveTerm, DialPoint, ExceedanceCurve, dial_rate, round_up_to_ladder};
pub use effects::{EffectRow, EffectsTable, IncomeRow, table};
pub use income::{GapRule, IncomeCurve, MONTHS_LADDER, round_up_months};
pub use model::{CountyData, CouplingApplied, OverrideApplied, ScenarioCandidate};
pub use ranges::DRAWS;
pub use survival::{Survival, probit};

/// Crate name, used by the CLI's `--version` and by the about screen.
pub const CRATE: &str = "rr-consequence";
