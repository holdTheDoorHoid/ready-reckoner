//! Ready Reckoner — `rr-budget`: the risk function, the allocator, the savings track and the
//! guardrails (DESIGN §4.7–§4.8, research risk-model §3.2, §4, §6.3).
//!
//! Given a household, its budget, the items on offer and each bucket's exceedance curve, it
//! produces the month-by-month [`rr_types::Plan`]: free actions first (at most eight to do in any
//! month, all within the first three months), then purchases in the order that buys the most risk
//! reduction per dollar, a sinking fund for anything that costs more than a month's money, a stop
//! when nothing more is worth buying, and the income savings goal on its own track.
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
//! # Timing
//!
//! The allocator decides *what* to buy next without looking at the money (tiers, caps,
//! life-safety first, value per dollar, promotion) and then times purchases by one of three
//! [`Schedule`]s:
//!
//! - **Split** (the default, `reserve_share` 0.5). When the top-priority item costs more than the
//!   month's money, half of each month's money goes into a sinking fund for it and the rest buys
//!   the best affordable items in priority order; the item is bought as soon as the fund (with any
//!   free money) covers it. Otherwise items are bought in priority order. The household sees
//!   progress every month (every month whose free money covers the cheapest useful item buys
//!   something), and the expensive item still arrives within ceil(cost / (share × monthly money))
//!   months of its first deposit. A bigger budget never ends the plan with less coverage in any
//!   bucket, but month by month it can reach some bucket later, because the purchase order depends
//!   on the money. The one-off money goes to life-safety items first: the top life-safety item a
//!   month's money cannot buy (a CPAP battery) is bought in month 0 when the one-off covers it;
//!   otherwise half the one-off opens its fund, the rest buys the cheaper life-safety items, and
//!   what they leave joins the fund.
//! - **FixedOrder.** Buy strictly in priority order and save everything for the next item when it
//!   costs more than the money on hand. The order never depends on the budget, so more money only
//!   ever moves purchases earlier and every bucket is covered at least as well in every month. The
//!   cost: a small budget facing an expensive top item sees months of saving with nothing bought.
//! - **ResearchShortcuts.** The research prototype's rule (buy the best affordable item unless
//!   the best costs at most two months of money and the alternative is worth less than a quarter
//!   as much per dollar). Its expensive items can wait until nothing cheaper is affordable, and
//!   more money can lower a bucket's coverage in some month.
//!
//! With no monthly budget there is no later month to save for, so every schedule spends leftover
//! one-off money on the best item that fits.
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

pub use allocate::{
    FREE_ACTIONS_BY_MONTH, FREE_ACTIONS_MONTH_0, FREE_ACTIONS_PER_MONTH, allocate,
    allocate_with_rule,
};
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
