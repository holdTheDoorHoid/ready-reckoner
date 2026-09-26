//! Ready Reckoner — `rr-budget`: the risk function, the allocator, the savings track and the
//! guardrails (DESIGN §4.7–§4.8, research risk-model §3.2, §4, §6.3).
//!
//! Given a household, its budget, the items on offer and each bucket's exceedance curve, it
//! produces the month-by-month [`rr_types::Plan`]: free actions in month 0, then purchases in the
//! order that buys the most risk reduction per dollar, a sinking fund for anything that costs more
//! than a month's money, a stop when nothing more is worth buying, and the income savings goal on
//! its own track.
//!
//! # Inputs
//!
//! This crate depends only on `rr-types`. What it needs from the other engine crates arrives as
//! plain data in [`BudgetInput`] (see [`input`] for which crate fills which field): item metadata
//! ([`coverage::ItemMeta`], "one gallon is one person-day of water"), requirement lines, Λ curves as
//! tables ([`curve::BucketCurve`]) and bucket assessments. [`coverage::CoverageRule`] turns items
//! into days of cover; [`coverage::ContributionTable`] is the default rule.
//!
//! # Value
//!
//! [`value`]: `V = 10 · w_b · ∫ₓ^min(x+Δ, target) Λ_b(t) dt` for duration buckets and
//! `V = 10 · w · r_need · harm` for readiness items, with the harm weights in [`weights`].
//!
//! # Timing, and why the default never skips ahead
//!
//! The allocator decides *what* to buy next without looking at the money (tiers, caps,
//! life-safety first, value per dollar, promotion), then buys it as soon as the money is there,
//! saving toward it when it is not ([`Schedule::Strict`]). Because the order never depends on the
//! budget, more money can only move a purchase earlier, so a bigger budget covers every bucket at
//! least as well in every month. Buying in order of value per dollar is also the order that
//! maximises protection over time when money arrives steadily (it is the classic
//! weighted-completion-time rule).
//!
//! The research prototype's shortcut ("buy the best affordable item instead, unless the best
//! costs at most two months of budget and the alternative is worth less than a quarter as much")
//! is available as [`Schedule::ResearchShortcuts`]. It is not the default because it breaks that
//! guarantee: with $60 a month a plan buys a $20 item in month 1 while saving for a $100 one, but
//! with $100 a month it buys the $100 item first, so the $20 item's bucket is *less* covered in
//! month 1 with more money. The one exception in the strict schedule is a household with no
//! monthly budget: there is no later month to save for, so leftover one-off money buys the best
//! remaining item that fits.
//!
//! Divisible items (water, food, medicine) are bought in chunks that reach the next step of the
//! day ladder (½, 1, 2, 3, 5, 7, 10, 14 ... days), so the first days come first and each chunk has
//! its own value per dollar; same-month chunks of one item are merged into one plan line.
//!
//! # What this crate does not do
//!
//! Quantities per person (`rr-supply`), prices and specs (`rr-content`), hazard rates and curves
//! (`rr-hazards`, `rr-consequence`), and the packet text (`rr-plan`).
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(missing_docs))]

mod allocate;
pub mod coverage;
pub mod curve;
mod explain;
mod guardrails;
pub mod input;
mod savings;
pub mod value;
pub mod weights;

pub use allocate::{allocate, allocate_with_rule};
pub use coverage::{
    Contributes, ContributionTable, CoverageRule, ItemMeta, ItemRole, MetaError, ReadinessCredit,
    apply_requirements,
};
pub use curve::{BucketCurve, CurveError};
pub use guardrails::{DEVICE_PLAN_BY_MONTH, EVACUATION_HEAVY_P10, GO_BAG_BY_MONTH};
pub use input::{
    BudgetError, BudgetInput, BudgetOptions, BudgetResult, Cliff, GuardrailContext, MonthCoverage,
    Purchase, Risks, Schedule,
};

/// Crate name, used by the CLI's `--version` and by the about screen.
pub const CRATE: &str = "rr-budget";
