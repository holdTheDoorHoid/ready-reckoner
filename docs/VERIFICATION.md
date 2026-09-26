# Verification (Phase 3, issue #13)

An adversarial pass over the numbers, invariants and claims, run on branch `agent/verify` from
main at `94c6a60` (data pack `e8b8cd6861e6`, content `2026.09.25+8f372052`). Each finding gives the
command that shows it, the evidence, a severity and either the fix committed on this branch or
the proposed fix.

Severity: **High** — a user sees a wrong or implausible number, or a promise (privacy, content
policy) is broken; **Medium** — a number or sentence is weaker than it should be, or a check the
design promises is missing; **Low** — cosmetic, or a tidy-up with no user-visible effect.

Instruments: the release CLI (`cargo build --release -p rr-cli`, `target/release/rr`), and two
harnesses added on this branch:

- `cargo run --release -p rr-plan --example verify_sweep -- <fixture> <out.csv> [dial]` plans one
  household in all 3,232 counties of the data pack and writes one row per county (targets,
  drivers, scenarios, cliffs, timing).
- `crates/rr-plan/tests/verify_invariants.rs`: randomised households over every county
  (see §2).

## Summary

(table at the end of the document, kept in step with the sections)

## 5. Real-pack outputs across the country

`target/release/examples/verify_sweep philadelphia-renters-4 sweep_phl.csv` — the Philadelphia
household (renters, rowhouse, gas heat, window AC, 4 people and a dog, 1-in-100) in every county.
All 3,232 counties plan without an error; `assess` takes 13.8 ms median, 17.8 ms at the 99th
percentile and 24 ms at worst (release, native).

Distribution of the power target (days → counties): ½ → 10, 1 → 29, 2 → 574, 3 → 747, 5 → 833,
7 → 264, 10 → 236, 14 → 261, 21 → 114, 30 → 35, 45 → 20, 60 → 4, 90 → 6, 180 → 11, **365 → 88**.
The recommended tier is "one year" in 88 counties and "six months" in 11.

### V-01 (High): a year without power in 88 counties, from noisy outage curves

**Evidence.** Every Puerto Rico municipio (78) and ten mainland counties get a 365-day power
target (and a "one year" tier); 11 more get 180 days and 91 counties a thermal target of 90 days
or more. All are driven 90–100 % by `strong_wind`, the hazard that carries the county's recorded
EAGLE-I outages. Examples (`rr targets --household philadelphia-renters-4 --county <fips>`,
`rr county show <fips>`):

| County | Outages / customer-year | ≥1 d / ≥3 d / ≥7 d / ≥14 d | Median / p90 | Longest event | Power target |
| --- | --- | --- | --- | --- | --- |
| Eddy ND 38027 (1,077 customers, 21 events) | 1.9 | 13.8 % / 0 / 0 / 0 | 1 h / 62 h | 62 h | **365 d** |
| Foard TX 48155 | 2.6 | 9.6 % / 0 / 0 / 0 | 1.8 h / 21 h | — | **365 d** |
| Calhoun GA 13037 (1,709 customers) | 2.6 | 7.7 % / 6.2 % / 0 / 0 | 2 h / 15 h | 218 h | **365 d** |
| Lafourche LA 22057 | 3.0 | 9.3 % / 4.0 % / 3.6 % / 2.3 % | 1.8 h / 20.5 h | 1,833 h | **365 d** |
| San Juan PR 72127 (island series) | 5.6 | 7.6 % / 2.9 % / 1.9 % / 0 | 3.5 h / 18 h | 766 h | **365 d** |
| Philadelphia PA 42101 | 0.037 | 9.8 % / 0.18 % / 0 / 0 | 5 h / 24 h | 78 h | 5 d |

Eddy County never recorded an outage longer than 62 hours, yet the plan tells a household there
to be ready for a year without power.

**Cause.** `rr_consequence::survival::EmpiricalCurve::from_points` drops a share of 0 ("not seen,
not impossible") and extends the last segment beyond the data with a slope floor
`MIN_SLOPE = 0.2` standard deviations per unit of ln d — the tail of a log-normal with σ = 5.
With a county rate of 2–3 outages a customer-year the 1-in-100 target sits where the survival
curve reaches about 0.004, far past the record, so the target is set entirely by that
extrapolation. Eddy: points (1 h, ½), (1 d, 0.14), (2.6 d, 0.1); the zeros at 3, 7 and 14 days are
dropped; the last slope (0.2) reaches S = 0.0055 at about 1,300 days. The data workstream already
warned that small counties are noisy and should be pooled with `core/outages_state.csv`
(`docs/DATA_SOURCES.md` §10), but `rr-data` loads that file "for completeness" and nothing reads
it; the pack's `events`, `customers` and `longest_event_hours` columns are not carried into
`OutageStats` either.

**Proposed fix** (model change; see the status in the summary table). In `outage_curve`:
(1) a share of 0 after the last positive share is an upper bound at the record's resolution
(one customer outage: `1 / (events_per_customer_year × customers × years_of_data)`), not a
missing point; (2) beyond the last informative point the tail decays at least as fast as a
log-normal with σ = 1.5 (slope ≥ 0.67), since the rare long outages are carried by the explicit
prior rows (big windstorms, ice storms of record, grid failure, hurricanes); (3) pool counties
with fewer than ~50 events with the state series (`outages_state.csv`), weighting by events.
Needs `events`, `customers`, `years_of_data` (and `longest_event_hours`) in `OutageStats`
(an engine-internal type; no API change).

### V-02 (High): the major-hurricane scenario is counted two to four times over

**Evidence.** `rr targets --household philadelphia-renters-4 --county 09110` (Capitol region,
Hartford CT): power 10 days, tap water 14, food 14, medicine 21, driven 98 % by hurricanes; the
scenario card says "Hurricanes reach Capitol County about once every 7 years, and about 1 in 3 of
them is a major storm". All nine Connecticut planning regions get power targets of 10–21 days.
HURDAT2 in the pack records 3 hurricane-strength and 1 major passage within 50 nautical miles of
Hartford County in 76 years (0.013 majors a year), but the engine plans a direct hit by a major
hurricane at 0.04 a year (1 in 25 years).

**Cause.** `rr_hazards::natural::hurricane` takes the major share as HURDAT2 major passages ÷
hurricane-strength passages and applies it to the NRI hurricane frequency. The NRI frequency is
not a count of hurricane-strength passages: across the 1,836 counties with both, NRI's
frequency ÷ HURDAT2 tropical-storm passages has a median of 0.81 (10th–90th percentile
0.47–1.49), while NRI ÷ hurricane-strength passages has a median of 3.8 (1.8–8.1). So the major
rate (NRI × share × 0.8) is inflated by the ratio of all tropical-storm passages to hurricane
passages, about 3.8 times at the median. Second, a county with hurricane passages but no major
passage row (Philadelphia: 1 passage, no major in 76 years) is given the national one-third
share, although `docs/DATA_SOURCES.md` says a missing row means none were recorded
(`RISK_MODEL.md` records the one-third fallback as a deliberate "err toward preparing"
decision). Miami-Dade: 4 majors in 76 years (0.053 a year, 0.042 with the 0.8 direct-hit
footprint), but the scenario runs at 0.305 × 4/11 × 0.8 = 0.089 a year.

**Proposed fix** (small code change in `rr-hazards`; changes goldens): the share of NRI events
that are major = major passages ÷ tropical-storm passages (the denominator that matches NRI's
event definition), and a missing major row with passage rows present counts as 0 majors, shrunk
toward the national share for small samples (for example (majors + 1)/(passages + 3)). The
scenario rate then equals the HURDAT2 major-passage rate × 0.8 where the pack has passages.
