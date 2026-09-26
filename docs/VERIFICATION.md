# Verification (Phase 3, issue #13)

An adversarial pass over the numbers, invariants and claims, run on branch `agent/verify` from
main at `94c6a60` (data pack `e8b8cd6861e6`). Each finding gives the command that shows it, the
evidence, a severity, and either the fix committed on this branch or the proposed fix.

**Severity.** *High*: a user sees a wrong or implausible number, a screen fails, or a promise
(privacy, content policy) is broken. *Medium*: a number or sentence is weaker than it should be, or
a check the design promises is missing. *Low*: cosmetic, or a tidy-up with no user-visible effect.

**Instruments.** The release CLI (`cargo build --release -p rr-cli`; `target/release/rr`) and two
harnesses added on this branch:

- `cargo run --release -p rr-plan --example verify_sweep -- <fixture> <out.csv> [dial]` plans one
  household in all 3,232 counties of the data pack, one CSV row per county (targets, ranges,
  drivers, scenarios, cliffs, spend, timing).
- `crates/rr-plan/tests/verify_invariants.rs`: seeded random households over every county (3 by
  default so `cargo test` stays quick; `RR_VERIFY_HOUSEHOLDS=500 cargo test --release -p rr-plan
  --test verify_invariants -- --nocapture` for the full run; `RR_VERIFY_ONLY=<k>` replays one).

## Summary

| # | Finding | Severity | Status |
| --- | --- | --- | --- |
| V-01 | A year without power in 88 counties (every Puerto Rico municipio and ten mainland counties), from outage curves extrapolated far past their records | High | Fixed `2c4e45b` |
| V-02 | The major-hurricane scenario counted two to four times over (share taken against hurricane-strength passages, applied to NRI's tropical-storm-strength count; missing rows read as one-third) | High | Fixed `c7b2f3b` |
| V-10 | The plan screen crashes (`each_key_duplicate`) on the real engine's output whenever a sinking fund fills | High | Open: web-engine (one-line key fix); test `b448a68` |
| V-03 | Home-loss sentence "1 in 3 displaced households are back within a week" is not in the cited source | Medium | Fixed `688318b` |
| V-04 | "Many states" allow 30-day emergency refills after a declaration; the source says 10 of 51 | Medium | Fixed `688318b` |
| V-05 | The nuclear card calls our annualised range "experts' estimates" | Medium | Fixed `688318b` |
| V-09 | A scenario cliff recommends water treatment and warmth for every bucket, phone and medicine included | Medium | Fixed `688318b` |
| V-11 | Family guidance shows avalanche, tsunami, volcano, landslide, drought and tornado-only advice where those hazards do not apply | Medium | Fixed `86d550c` |
| V-12 | Heat and cold waves read "Minor" even for households with a senior, a baby or no cooling/heating | Medium | Fixed `26d7004` |
| V-13 | A second card of the same family (winter storm after cold wave, tsunami after earthquake) shows a threat with no "What helps" | Medium | Fixed `7ce98e0` |
| V-15 | The 79 counties without EAGLE-I records (37 in Nebraska, 21 in Alaska) get lower power and thermal targets than their neighbours (Juneau: power ½ day, thermal 0 for a gas-heated home) | Medium | Fixed `8be5619` (agent/followups) |
| V-06 | SSA's "1 in 4" disability chance paraphrased as "more than 1 in 4" | Low | Fixed `688318b` |
| V-07 | "about 1 times a year" | Low | Fixed `688318b`, `2c11e41` |
| V-08 | "about fewer than 1 (0–2) of 100" | Low | Fixed `688318b` |
| V-14 | Engine text "leaving home in a hurry" uses a pressure word the content policy bans | Low | Fixed `7ce98e0` |
| V-16 | The `no_water_after_month_1` guardrail cannot fire: the free reused-bottles step always gives water in month 0 | Low | Retargeted `08c44ce` (agent/followups) |
| V-17 | "Do not wait …" appears three times in guidance; the pressure-phrase check only catches "don't wait" | Low | Fixed `ff049b1` (agent/followups) |
| V-18 | The packet's "Spend" column adds a sinking-fund deposit and the full price in the purchase month ($130 in a $60 month) | Low | Fixed `42a085a` (agent/followups) |
| V-19 | localStorage also holds `rr.prefs.v1` (display preferences); the docs name only `rr.plan.v1` | Low | Proposed (docs) |
| V-20 | The CSP is a `<meta>` tag (no `frame-ancestors`), with `'unsafe-inline'` styles | Low | Noted |
| V-21 | Citation hygiene: JOLTS cited by its home page, the FEMA survey deck by a third-party mirror, the Hazus manual by a university copy, two price bands by one search-results URL, bottled-water prices from a wholesale case listing | Low | Proposed |
| V-22 | Constants note: the highest Thrifty Food Plan cost per person is $10.76 (a boy 14–19), not $10.51 | Low | Fixed `688318b` |
| V-23 | rr-cli built reqwest, rustls and aws-lc only for `rr_etl::verify` | Low | Fixed `26cc4a2` |
| V-24 | Attributions sorted by name in rr-data and re-sorted NRI-first in three callers | Low | Fixed `26cc4a2` |

Every other check below passed: 500 random households across the country (no panic, no error,
money never overspent, every step and number cites, targets monotone in the dial, more people
never need less, a well puts no-water at or above power, confidence answers move nothing,
assumed basics never credit less, more money never covers less, byte-identical reruns, `assess`
under 100 ms); every guardrail fires when it should and not otherwise; every named scenario appears
where documented with the documented default; of 43 sampled groups of source figures (46
citations and five price listings), 38 match the source, one partly and four do not (V-03–V-06);
packets read at grade 5.7–6.8, with no brands, no dosing, firearms only in the one free action, the
NRI statement with version and date; and the site makes no request to any other origin.

## 1. Numbers against their sources

46 citations and five price listings were opened at the cited URL (two sub-checks run in parallel;
WebFetch, with at most eight searches in all): 43 groups of figures (about 80 numbers) across
water, food, medicine, sanitation, power, outages, base rates, scenarios and prices. 38 groups
match, one partly, four do not:

| Area | Citation | Figure the app uses | Result |
| --- | --- | --- | --- |
| Water | `ready_gov_water` | 1 gal a person a day; needs can double in heat | match |
| Water | `cdc_water_storage` | replace stored water every 6 months | match |
| Water | `epa_emergency_disinfection` | boil 1 min (3 above 5,000 ft); 8 drops 6 % or 6 drops 8.25 % bleach a gallon, 30 min | match |
| Water | `cdc_water_disinfection` | boil 1 min, 3 above 6,500 ft; ½ mL bleach a gallon | match (CDC 6,500 ft vs EPA 5,000 ft is recorded in `constants.toml`; the app uses EPA's stricter figure) |
| Water | `doe_water_heaters` (archived) | tank holds 20–80 gallons | match |
| Water | `sphere_2018` | 15 L a person a day; 250 g + 200 g soap a month; 2.5–3 L drinking | match (URL is the handbook landing page) |
| Water | `petmd_dog_water` | 1 oz per lb a day | match |
| Food | `usda_tfp_aug2026` | $8.44 a person-day | match ($236.30 ÷ 4 ÷ 7); size adjustments match; see V-22 |
| Food | `usda_dga_2020_2025` | kcal by band 1,067 / 1,667 / 2,280 / 2,262 / 2,009 | match, all within 1 % |
| Food | `fsis_shelf_stable` | below 85 °F; no bulging, leaking, rusted or deeply dented cans | match |
| Food | `ready_gov_food`, `ready_gov_power_outages` | fridge 4 h, full freezer 48 h; discard refrigerated medicine after a day | match |
| Medicine | `redcross_survival_kit` | 7-day medication supply; 3-day evacuation water and food | match |
| Medicine | `florida_dem_medication` | at least two weeks | match |
| Medicine | `cdc_diabetes_emergencies` | at least 1 to 2 weeks | match |
| Medicine | `healthcare_ready_refill_laws` | up to 30 days after a declaration in "many states" | **mismatch**: 10 of 51 (V-04) |
| Medicine | `dailymed_epinephrine_autoinjector` | carry two auto-injectors | match |
| Sanitation | `rdpo_emergency_toilet` | two-bucket toilet; bag and cover rates tagged as estimates | match (RDPO gives no rates, as the constants say) |
| Sanitation | `cdc_period_factsheet` | products for 2 cycles | match |
| Power | `sil_cpap_power` | CPAP 170 Wh a night, 96 without the humidifier | partial: one of several anecdotes on a hobbyist page (already tagged an estimate) |
| Power | `cdc_co_basics` | never indoors or in a garage; 20 ft from openings | match |
| Power | `honda_eu2200i_spec` (archived) | 2.8 gal a day at ¼ load, 7.1 at full load | match (0.95 gal ÷ 8.1 h and ÷ 3.2 h) |
| Power | `lehi_fuel_storage` | 25 gal at home, 10 in an attached garage | match |
| Outages | `do_2023_outages` | 62.1 % of long outages coincide with extreme weather | match (secondary source; journal page gated) |
| Outages | `ornl_repowrd_2022` | Irma 1.5 d / 5 d; Michael 5 d / 16 d | match (from the report's restoration rates) |
| Base rates | `bls_work_experience_2024` | 8.3 % unemployed at some point in 2024 | match |
| Base rates | `bls_jolts_layoffs` | 2025 layoffs 1.1167 % a month | match, but the URL is the JOLTS home page (V-21) |
| Base rates | `usfa_residential_fires` | 344,600 fires, 2,890 deaths, 10,400 injuries, $11.27 billion (2023) | match |
| Base rates | `census_households_cps` | 131,434 thousand households (2023) | match |
| Base rates | `cdc_nchs_ed_visits` | 155.4 million visits, 47.3 per 100, 11.5 % admitted | match |
| Base rates | `nchs_accidental_injury_2024` | 197,449 deaths, 58.1 per 100,000 | match |
| Base rates | `nhtsa_crashes_2023` | 6.14 million crashes, 2.44 million injured | match |
| Base rates | `ssa_disability_facts` | "more than 1 in 4" (RISK_MODEL, why-text) | **mismatch**: 1 in 4 (V-06) |
| Base rates | `bjs_criminal_victimization_2023` | 1.01 % of households | match |
| Base rates | `cdc_pandemic_history`, `marani_2021_pandemics` | 5 onsets in 108 years; 675,000 US deaths in 1918 | match for 1918; the other onsets are history the one page cannot show |
| Scenarios | `fri_nuclear_risk_2024` | "experts' estimates" 1 in 2,000 to 1 in 400 a year | **mismatch in framing**: 1–5 % before 2045 (V-05) |
| Scenarios | `usgs_bay_area_outlook_2016` | 33 % Hayward M6.7+ in 30 years | match |
| Scenarios | `usgs_new_madrid` | 7–10 % in 50 years; 25–40 % for M6+ | match |
| Scenarios | `usgs_pp1661f_cascadia`, `osu_cascadia_2012` | 40 % in 50 years (south); 19 full-margin, 41 southern ruptures in 10,000 years | match |
| Behaviour | `fema_nhs_2024` | 60 % vs 43 %; 26 % cost; 71 % expect family help; 18 % know how to help neighbours | match (on a third-party mirror, V-21) |
| Behaviour | `census_pulse_displacement` | a third back within a week; 12 % out over six months | **mismatch**: not in the source (V-03) |
| Comms | `nws_weather_radio` | more than 1,000 transmitters, all 50 states | match |
| Insurance | `fema_nfip_flood_insurance`, `floodsmart_buy_policy` | 30-day wait; $250,000 / $100,000; contents-only for renters | match |
| Prices | PRICE_OBSERVATIONS rows | N95, bleach, bottled water listings | match; CO alarm and NOAA radio rows unverifiable (store blocks automated requests; both rows of each share one search URL, V-21) |

*Method note.* Some federal and non-profit pages (fsis.usda.gov, dietaryguidelines.gov,
redcross.org, spherestandards.org) answer automated requests with HTTP 403; one sub-check fetched
them with an ordinary browser user agent (no login or CAPTCHA was involved, and none was worked
around; a PubMed reCAPTCHA page and a publisher 403 were left alone). The Aung and Sehgal 2025 paper
behind the 35 % "back within a week" parameter could not be read, so that figure is unverified.

**V-03 (Medium, fixed).** `rr plan --household philadelphia-renters-4 --format json`: the home-loss
bucket said "After a disaster, about 1 in 3 displaced households are back within a week". The cited
NLIHC write-up of the Household Pulse data says only that 56 % of displaced renters and 71 % of
owners were back in less than a month, and that nearly 1 in 4 renters and 1 in 10 owners never
returned. The sentence now reads "most displaced households are back within a month, but about 1
in 4 renters never return" (`crates/rr-consequence/src/assess.rs`), and `aung_2025_displacement`
(unreadable here) no longer backs a sentence.

**V-04 (Medium, fixed).** The medicine line said "After a disaster declaration many states let
pharmacies give an emergency refill of up to 30 days". Healthcare Ready: 10 of 51 jurisdictions
allow 30 days after a governor's declaration; 15 allow 72 hours and 16 have no rule. Now "some
states … and many allow only a few days" (`crates/rr-supply/src/rules/medication.rs`).

**V-05 (Medium, fixed).** The nuclear card said "Experts' estimates of a worldwide nuclear
catastrophe range from about 1 in 2,000 to about 1 in 400 a year". The Forecasting Research
Institute reports median forecasts of 1 % (superforecasters) and 5 % (experts) for a catastrophe
killing 10 million or more before 2045; the per-year range is the research report's own
annualisation. The card now says both (`crates/rr-hazards/src/societal.rs`).

**V-06, V-22 (Low, fixed).** SSA: "a 20-year-old worker has a 1-in-4 chance of developing a
disability before reaching full retirement age" (why-text, comments and RISK_MODEL said "more
than"). TFP note corrected to $10.76.

**V-21 (Low, proposed).** Point `bls_jolts_layoffs` at the series page
(`data.bls.gov/timeseries/JTS000000000000000LDR`, already named in `base_rates.toml`); find an
official FEMA URL for the 2024 National Household Survey deck (now on survivingcascadia.com); cite
FEMA's own Hazus technical manual rather than a university copy; give the CO alarm and NOAA radio
bands two distinct product listings each; note that the $3.59 bottled-water row is a food-service
case price with an 84-case minimum.

## 2. Invariants under randomised households

`RR_VERIFY_HOUSEHOLDS=500 cargo test --release -p rr-plan --test verify_invariants -- --nocapture`.
The generator (seeded `SplitMix64`) draws a county from all 3,232, 1–7 people of every age band
with medical needs, earners and commutes, every housing kind, water source, heating, cooling and
backup power, pets and livestock, vehicles, budgets from $0 to $1,000 a month and one-off amounts,
insurance, random existing inventory, assume-basics on or off, every dial, climate horizon, water
level, horizons of 1–50 years, random scenario overrides and the rare-catastrophe opt-in. For each
household it checks: no panic or error; `assess` under 100 ms; purchases never exceed the money
available by any month, and every deposit is accounted for in the envelopes; every plan step's
item cites a source that resolves, every requirement line, hazard and target cites and sits in
`provenance`; target ranges bracket the value, values sit on the ladder, coverage never exceeds the
target; no stray markers in the packet ("about 1 times", "about fewer than", `{if:`, `NaN` …);
every hazard card has "What helps"; two runs give identical bytes; targets never fall as the dial
gets more cautious; one more adult never needs less water or food; a well puts the no-water target
at or above the power target; the confidence and stage answers change no target; assuming basics
never credits less coverage with no money (with the household's own budget this is reported, not
failed); twice the money never ends with less coverage.

**Result on the final code (`2c11e41`):** all 500 households pass every hard invariant; the slowest
`assess` took 25 ms (release). Four soft findings (with the household's own budget, assuming
basics ended one bucket slightly lower): households 264, 397 and 480 by 0.1–1.2 days, and
household 350 (well, 4 people, $10 a month) with no-tap-water coverage 14 days with basics assumed
and 30 without, because with basics assumed the plan starts saving for the gravity filter in month
117 instead of 111 and the ten-year horizon ends first. It is the greedy allocator's path
dependence on a budget that never finishes; noted, not fixed.

Along the way the harness found V-07 (29 of 500 packets before the first fix; 13 after it, from a
second sentence fixed in `2c11e41`). An apparent overspend (household 367) was the harness's own
accounting (a sinking fund's money can move to the next top item); it now checks purchases alone
against the money, as the allocator promises. The "What helps" check was added after the packet
review of §6 found V-13.

## 3. Guardrails

Each case is `fixtures/households/philadelphia-renters-4.json` with the change in the table,
run as `rr plan --household case.json --format json` and read from `.warnings[].id` (for example
the CPAP case sets `people[0].medical.powered_device` to `"cpap"` and
`finances.monthly_budget_usd` to 10; the flood case sets `housing.tenure` to `"own"` and
`location` to `{"country": "US", "county_fips": "22109", "setting": "suburban"}`):

| Guardrail | Should fire | Fires | Control (should not fire) | Fires |
| --- | --- | --- | --- | --- |
| `zero_budget` | $0 monthly, $0 one-off | yes | $60 monthly; $0 monthly with a $200 one-off | no, no |
| `device_power_plan` | CPAP, no backup power, $10 a month | yes | with a generator; with $1,000 a month; with a $300 one-off | no, no, no |
| `cold_chain_plan` | refrigerated medicine, $0 | yes | $150 a month | no |
| `evacuation_no_go_bag` | leave-home chance 16 %, basics not assumed, $0 | yes | basics assumed (a bag counts) | no |
| `insurance_flood` | owner in Terrebonne LA without flood cover | yes | insured; renter | no, no |
| `insurance_quake` | owner in Alameda CA without quake cover | yes | insured; owner in Philadelphia | no, no |
| `cliff_<bucket>` | Coos Bay at 1 in 100 (power, water) | yes | Philadelphia | no |
| `no_water_after_month_1` | household lists reused bottles as 0, $0 | **no** | — | — |

**V-16 (Low, noted).** `no_water_after_month_1` cannot fire with the current catalogue: the free
"tap water in clean reused bottles" step is always offered in month 0 (a household that lists it
at 0 is still asked to do it), so there is always a day of stored water by month 1. Harmless; the
guardrail protects a future catalogue.

## 4. Named scenarios and the cliff

`rr risks --household philadelphia-renters-4 --county <fips>` and `rr targets … --sweep`:

| Place | Scenario offered | Default | As documented | Toggle effect |
| --- | --- | --- | --- | --- |
| Coos Bay, Coos OR 41011 | `cascadia_m9` (coast), `local_tsunami` | on, on | yes | Cascadia off: power 14 → 3 days, water 45 → 3 |
| Seattle, King WA 53033 | `cascadia_m9` (valley), `local_tsunami` | on, on | yes | power 2 → 3, water 3 → 5 with it on |
| Portland, Multnomah OR 41051 | `cascadia_m9` (valley) | on | yes | water 3 → 5, phone 2 → 3 |
| Memphis, Shelby TN 47157 | `new_madrid_m7` | off | yes ("rarer than the yardstick") | turned on: water 3 → 5 |
| Oakland, Alameda CA 06001 | `hayward_m7`, `local_tsunami` | on, on | yes | power 2 → 5, water 3 → 10, medicine 14 → 21 |
| Miami-Dade FL 12086 | `major_hurricane_direct_hit` | on | yes | power 7 → 14, water 3 → 10 (after V-02) |
| Galveston TX 48167 | `major_hurricane_direct_hit` | on | yes | water 3 → 5, medicine 14 → 21 (after V-02) |

A toggle for a scenario that does not apply is ignored with a note ("The scenario setting
"hayward_m7" does not apply to Philadelphia County"). Coos Bay's cliff warnings at 1 in 100 name
the Cascadia earthquake and give 3, 14 and 180 days (power) and 3, 45 and 365 days (water) at the
neighbouring settings.

**V-09 (Medium, fixed).** Every scenario cliff ended "long outages are better met with ways to make
water safe and stay warm than with bigger stockpiles", whatever the bucket. It is now
bucket-specific (`capability_advice` in `crates/rr-consequence/src/assess.rs`): water — a filter
and a source; power — ways to stay warm or cool, keep medicine cold and charge phones; phone — a
battery radio, a way to charge phones and meeting places; medicine — the prescriber and early
refills; unit-tested so no non-water bucket mentions water.

## 5. Real-pack outputs across the country

`target/release/examples/verify_sweep philadelphia-renters-4 sweep.csv`: the Philadelphia
household in every county. All 3,232 plan; `assess` 14 ms median, 23 ms at the 99th percentile,
26 ms at worst (release, native).

| Power target (days) | ½ | 1 | 2 | 3 | 5 | 7 | 10 | 14 | 21 | 30 | 45 | 60 | 90 | 180 | 365 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Counties at main | 10 | 29 | 574 | 747 | 833 | 264 | 236 | 261 | 114 | 35 | 20 | 4 | 6 | 11 | 88 |
| Counties on this branch | 10 | 29 | 604 | 792 | 1,002 | 399 | 203 | 75 | 25 | 85 | 5 | 2 | 1 | 0 | 0 |

No-tap-water targets of 14 days or more: 348 counties at main, 7 now (the Cascadia coast and
Monroe County FL). Recommended tier at main: two weeks 2,596, one month 497, three months 40, six
months 11, one year 88; now two weeks 3,000, one month 219, three months 13.

### V-01 (High, fixed): a year without power from noisy outage curves

Every Puerto Rico municipio (78) and ten mainland counties had a 365-day power target and the "one
year" tier; 11 more had 180 days; 91 had a thermal target of 90 days or more. All were driven
90–100 % by `strong_wind`, the hazard that carries the county's EAGLE-I outage records.

| County (`rr county show <fips>`) | Outages / customer-year | ≥1 d / ≥3 d / ≥7 d / ≥14 d | Median / p90 | Longest event | Power at main | Now |
| --- | --- | --- | --- | --- | --- | --- |
| Eddy ND 38027 (1,077 customers, 21 events) | 1.9 | 13.8 % / 0 / 0 / 0 | 1 h / 62 h | 62 h | 365 d | 5 d |
| Foard TX 48155 | 2.6 | 9.6 % / 0 / 0 / 0 | 1.8 h / 21 h | — | 365 d | 5 d |
| Calhoun GA 13037 (1,709 customers) | 2.6 | 7.7 % / 6.2 % / 0 / 0 | 2 h / 15 h | 218 h | 365 d | 10 d |
| Lafourche LA 22057 | 3.0 | 9.3 % / 4.0 % / 3.6 % / 2.3 % | 1.8 h / 20.5 h | 1,833 h | 365 d | 60 d |
| San Juan PR 72127 (island series) | 5.6 | 7.6 % / 2.9 % / 1.9 % / 0 | 3.5 h / 18 h | 766 h | 365 d | 30 d |
| Philadelphia PA 42101 | 0.037 | 9.8 % / 0.18 % / 0 / 0 | 5 h / 24 h | 78 h | 5 d | 3 d (V-02) |

*Cause.* `EmpiricalCurve::from_points` dropped every share of 0 ("not seen") and extended the last
segment with a slope floor of 0.2 standard deviations per unit of ln d, the tail of a log-normal
with σ = 5. At 2–3 county outages a customer-year the 1-in-100 target sits where the curve reaches
about 0.004, far past the record, so the target was the extrapolation. The data workstream had
warned that small counties are noisy (`docs/DATA_SOURCES.md` §10) and shipped
`core/outages_state.csv` for pooling, which nothing reads.

*Fix.* (`crates/rr-consequence/src/model.rs`, `survival.rs`) The first zero share past the last
positive one is an upper bound of 0.5 % at that length, applied only when the curve would sit above
it (Philadelphia's and Coos Bay's curves are unchanged); beyond the last point the curve decays at
least like a log-normal with σ = 2, heavier than every duration in the effects table and the 2 h /
20 h no-records fit. The long, rare outages the records cannot show stay with the table's own rows.
Only Chicago's golden moves (power range 2–5 days, heat or cold 3 → 2 days).

*Still proposed (V-15 below).* Pool counties with few events with the state series, weighting by
events, and carry `events`, `customers`, `years_of_data` and `longest_event_hours` into
`OutageStats`, so the zero bound can be the record's own resolution rather than a constant.

### V-02 (High, fixed): the major-hurricane scenario counted two to four times over

`rr risks --household philadelphia-renters-4 --county 09110` at main: "Hurricanes reach Capitol
County about once every 7 years, and about 1 in 3 of them is a major storm", a direct hit by a
major hurricane on Hartford at 0.04 a year (1 in 25 years); all nine Connecticut regions had power
targets of 10–21 days and leave-home chances of 35–46 in 100. HURDAT2 in the pack: 3
hurricane-strength and 1 major passage within 50 nautical miles of Hartford in 76 years. Sagadahoc
County, Maine: no hurricane-strength passage since 1950, yet "about once every 10 years, about 1 in
3 major" and 79 % of its leave-home chance from hurricanes.

*Cause.* `rr_hazards::natural::hurricane` applied HURDAT2's major ÷ hurricane-strength passages to
NRI's hurricane frequency. Across the 1,836 counties with both, NRI's frequency ÷ HURDAT2
tropical-storm passages has a median of 0.81 (10th–90th percentile 0.47–1.49), while NRI ÷
hurricane-strength passages has a median of 3.8 (1.8–8.1): NRI counts tropical-storm-strength
events (the crate's own footprint comment says so). And a county with passages but no major row
got the national one-third, although HURDAT2 covers every county.

*Fix.* Share = (major passages + 5 × 5.4 %) ÷ (tropical-storm passages + 5), where 5.4 % is the
pooled share (601 majors in 11,155 tropical-storm passages over the 556 counties the scenario can
apply to); 5.4 % (3–10 %) without passages. The scenario rate is now about HURDAT2's own
major-passage rate × the 0.8 direct-hit footprint (Miami-Dade 0.037 a year against 4 majors in 76
years × 0.8 = 0.042). The card gives the county's own share ("about 1 in 7 of them"). Hartford:
power 10 → 5 days, water 14 → 5, medicine 21 → 14; Philadelphia's power target becomes the
research's 3 days (it was 5, which RISK_MODEL expected the pack's passage counts to fix).

### V-15 (Medium, proposed): counties without outage records

79 counties have no EAGLE-I record (37 in Nebraska, 21 in Alaska, 5 in Tennessee, the Pacific
territories). Their power targets are mostly 2 days (56 of 79) against 3–5 elsewhere, and 12
Alaska boroughs get ½ to 1 day of power and **no thermal target at all** for a gas-heated home (Juneau,
Sitka, North Slope: `verify_sweep` rows 02110, 02220, 02185), because the fallback is a third of
short storm outages at 2 h / 20 h and NRI's cold-wave counts in Alaska are low. Four Alaska
boroughs also get no communications target. *Proposed:* when a county has no record, use its
state's row in `core/outages_state.csv` (Alaska: 1.5 outages a customer-year, 1.9 % a day or more),
as the data workstream recommended; say so in the notes.

### Places the brief named

- **Puerto Rico**: every municipio carries the island-wide EAGLE-I series (as documented); after
  V-01, 30 days of power (21–30), heat or cold 21, medicine 21 (San Juan). The 2017 Maria outage
  predates the record (2021–2025).
- **Alaska**: see V-15; Anchorage 3 days of power, Fairbanks 5, Mat-Su 5, all "two weeks" enough.
- **Connecticut**: all nine planning regions plan; before V-02 their hurricane-driven targets were
  the highest in the Northeast; now Capitol region power 5, water 5, medicine 14.
- **District of Columbia**: power 3, water 3, food 10, medicine 14; two weeks enough.
- No county has a 365-day water target outside Cascadia (Del Norte, Humboldt, Mendocino, Coos,
  Curry have 45 days at 1 in 100, with a cliff warning), and no heat target in Alaska.

## 6. Packet and UI claims

Checked on the seven goldens and on 20 counties' packets for the Philadelphia household
(`rr plan --household philadelphia-renters-4 --county <fips>` for 02185, 02020, 72127, 15003,
06037, 06023, 12087, 48201, 08097, 30031, 36061, 11001, 09110, 47157, 53033, 04013, 22071, 38027,
26119, 01003), with the content validator's own word lists and grade formula:

- **Reading level**: Flesch-Kincaid grade of the packet prose 5.7–6.8 (limit 9); every guidance
  block passes the validator (`rr doctor`: 0 errors, 0 warnings).
- **No brands, no dosing**: none of the validator's 150+ brand tokens and no drug-plus-dose pattern
  in any packet.
- **Firearms**: firearm words appear only in "If you own firearms: safe storage and training".
- **Threat with action**: every hazard card now has "What helps" (V-13); the rare-catastrophe box
  has its own. **No countdown or scarcity**: after V-14, only "Do not wait" remains (V-17).
- **Natural frequencies**: every packet leads with "Of 100 households like yours"; no probability
  is given as a bare percentage (percentages are bleach strengths and shares).
- **NRI disclaimer**: every packet's data credits open with "FEMA National Risk Index, version
  1.20.0 (December 2025), accessed September 26, 2026" and "is not endorsed by FEMA"; the About
  screen lists it first (V-24 keeps it first at the source).

**V-11 (Medium, fixed; coordinator finding).** Hazard-family blocks were shown whole for every
hazard in the family: Philadelphia's cold-wave card told a rowhouse to carry an avalanche beacon.
Families affected: winter + ice + cold + avalanche (avalanche sentence), earthquake + tsunami +
volcano (tsunami and volcano steps), floods + landslides (burn-scar and slope steps), tornado +
wind + hail + lightning (two tornado-only steps), heat + drought (drought sentence). Wildfire and
smoke, hurricane, hazmat, and blackout/cyber/local-outage blocks are about one hazard or are
generic, so they have none. A span about one hazard is now `{if:<hazard_id>}…{/if}`; the packet
keeps it only when that hazard's ten-year chance for the household is at least 1 in 100; the
validator checks the markup (`docs/PACKET.md`, `docs/CONTENT_STANDARDS.md` §4).

**V-12 (Medium, fixed; coordinator finding).** Severity is not building loss only: NRI's expected
loss includes deaths and injuries valued per statistical life (Philadelphia heat: 18.7
fatality-equivalents a year, $256 million). The fixed scale divides it by every household and every
county episode (4.5 a year), about $85 an episode, which reads "Minor". Heat and cold waves now show
at least "Serious" (an emergency visit on the same scale) for a household with someone 65 or older,
a baby, someone on a powered medical device, someone pregnant (heat), or no air conditioning (heat)
or heating (cold), with a note saying why. Display only; no target changes.

**V-13 (Medium, fixed).** A second card of the same family showed its threat sentence and "What it
can do" but no "What helps" (Gallatin MT and Eddy ND winter-storm cards; Coos Bay's tsunami card),
because the family block is shown once. It now points to the earlier card ("The steps under "Cold
wave" above apply here too").

**V-14 (Low, fixed).** rr-budget's evacuation "why" said "leaving home in a hurry"; "hurry" is on
the pressure-word list the content validator enforces (it never sees engine strings). Now
"quickly".

**V-17 (Low, proposed, content).** "Do not wait to file" (job loss), "Do not wait for a disaster to
meet the people next door" (neighbours), "Do not wait until things feel unbearable to reach out"
(mental health). The validator bans "don't wait" but not "do not wait". The uses are supportive
rather than pressure; suggest rephrasing ("Contact your state's unemployment program as soon as you
can", "Meet the people next door before you need them", "Reach out before things feel
unbearable") and adding "do not wait" to `BANNED_PHRASES`.

**V-18 (Low, proposed).** Philadelphia's "After the first year" table shows "14 (December 2027) |
save toward cash in small bills; Cash in small bills: $100 | $130" in a $60-a-month plan: the
month's last $30 deposit and the $100 purchase it completes are both counted. Show the purchase
month as "$100 (from $90 saved)" or count only new money.

## 7. Privacy

Built with `bash crates/rr-wasm/build-web.sh && cd web && npm ci && npm run build`, served from
`web/dist` on 127.0.0.1, and driven in a browser through the interview (ZIP 19147, $60 a month),
the risks screen, a dial change, the packet and the About screen.

- **Requests**: 26 per page load, every one same-origin: the page, `theme-init.js`, two scripts and
  the stylesheet, `pkg/rr_wasm.js` and `rr_wasm_bg.wasm`, `data/manifest.json`, and the 18
  `data/core/*` files with `?v=e8b8cd6861e6`. No request to any other origin; nothing after load
  but the packs (route changes are hash-only). The bundle contains no `sendBeacon`, `WebSocket`,
  `EventSource` or `XMLHttpRequest`, and no absolute URL it fetches; `<meta name="referrer"
  content="no-referrer">` keeps source links from leaking the page.
- **Storage**: after the interview, `localStorage` holds only `rr.plan.v1` (1,194 bytes: the
  household, dials, progress); no cookies, no sessionStorage, no IndexedDB. **V-19 (Low):** the code
  also writes `rr.prefs.v1` when a display preference (theme, expert view) is changed; UI.md and
  DESIGN §8 name only `rr.plan.v1`. Document it (it holds no household data).
- **CSP** (a `<meta>` added at build): `default-src 'self'; script-src 'self' 'wasm-unsafe-eval';
  style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; connect-src
  'self'; worker-src 'self'; manifest-src 'self'; object-src 'none'; base-uri 'self'; form-action
  'none'`. No inline scripts. **V-20 (Low, noted):** a meta CSP cannot carry `frame-ancestors`
  (Pages sends no headers), so framing is not blocked; styles allow `'unsafe-inline'` (Svelte style
  attributes).
- The service worker did not register in the test browser ("unknown error when fetching the
  script" although `sw.js` is served with `text/javascript`), so offline use was not verified here.
- The shared-origin question (DESIGN §10) stands: the saved household would be readable by any
  other page on `holdthedoorhoid.github.io`.

**V-10 (High, open: web-engine).** After the interview the plan screen never renders: Svelte
throws `each_key_duplicate` (console: `https://svelte.dev/e/each_key_duplicate`) and the router
stays on "Your risks". When a sinking fund fills, the engine lists that month's last deposit
(`reserve`) and the purchase (`purchase`) for the same item and tier; `PlanScreen.svelte` keys its
month lists by `item.item_id + item.tier`. It happens whenever a sinking fund fills (three of the
seven goldens, Philadelphia, Hays and Phoenix, have 4–6 such months; so did the one-adult household
of the browser run), never with the mock engine, so the screen tests pass.
*Fix (web):* key by `item.item_id + item.tier + item.kind` (PlanScreen.svelte lines 113, 129, 134,
143, 189, 213; ReadinessCard.svelte 34, 42; HazardCard.svelte 61). `web/src/screens/verify.real-output.test.ts`
reproduces it with the real Philadelphia golden (recorded as `it.fails`; flip to `it` with the fix).

## 8. Follow-ups from FOLLOWUPS.md

- Text nits: "about 1 times a year" (V-07), "about fewer than 1 (0–2) of 100" (V-08), the generic
  cliff advice (V-09): fixed.
- `rr_etl::verify` moved to `rr_data::verify` (native only); `rr-etl verify` calls it; rr-cli no
  longer depends on rr-etl, so it no longer builds reqwest, rustls or aws-lc (V-23).
- `DataStore::attributions` returns the NRI statement first; the reordering in rr-plan, rr-cli and
  rr-wasm is gone (V-24).

## 9. Release follow-ups (agent/followups, 2026-09-26)

Fixed after this pass, each as its own commit on `agent/followups`:

- **V-15** (`8be5619`): a county with no outage record takes its state's pooled series from
  `core/outages_state.csv` (72 counties in 9 states; American Samoa, Guam and the Northern Mariana
  Islands have no state row). The Philadelphia household in those counties: power ½–3 days
  (mostly 2) became 2–5 (mostly 5, Nebraska), heat or cold 0–2 became 2–5 (mostly 3), phones 0–2
  became 2–5; Juneau power ½ → 3 days, heat or cold 0 → 3, phones ½ → 5. The packet's notes and
  the power override name the state.
- **V-16** (`08c44ce`): the guardrail is now `no_stored_water_by_month_3`: it warns when refilled
  bottles and what the household has leave the stored-water need short and no stored water is
  owned or bought by month 3. Of the fixtures, only the $0 Chicago student is warned (Phoenix's
  refilled bottles cover its whole 3-day need).
- **V-17** (`ff049b1`): "do not wait" joins "don't wait" on the pressure list; the seven guidance
  sentences that used it were rewritten (three of them reach the fixture packets).
- **V-18** (`42a085a`): the "Spend" column counts a deposit once and a purchase paid from savings
  only for the rest (Philadelphia month 14: $130 → $40), and says "$90 of it from savings".

## Golden changes on this branch

| Commit | What moved |
| --- | --- |
| `688318b` | Text only: nuclear sentence (every packet), "about once a year" (Philadelphia), "some states" refills, the home-loss sentence (every JSON), source count |
| `86d550c` | Sub-hazard sentences dropped where the hazard does not apply (avalanche, drought, tornado-only); citation renumbering |
| `26d7004` | Heat/cold "Serious" with a note, for Philadelphia, Miami, Sugar Land (heat and cold), Hays, Phoenix, Coos Bay (heat) |
| `c7b2f3b` | Philadelphia power 5 → 3 days; Miami power 3 → 2 weeks, water 1 month → 3 weeks, the other buckets a step lower, done month 11 → 10; Sugar Land a step lower, done month 3 → 2; Chicago leave-home 15 → 10 in 100 |
| `2c4e45b` | Chicago power range and heat-or-cold 3 → 2 days |
| `7ce98e0` | Coos Bay tsunami card points to the earthquake card; "in a hurry" → "quickly" |
| `2c11e41` | Miami and Sugar Land outage notes: "about 0.47 times a year" → "about once every 2 years" |
