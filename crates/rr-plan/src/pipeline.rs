//! The engine pipeline (DESIGN §5): location → hazards → consequences → supplies → budget, with
//! every intermediate result kept in an [`Assessment`] so the packet and `explain` can show the
//! numbers behind each output.

use std::collections::BTreeMap;

use rr_budget::{
    BucketCurve, BudgetInput, BudgetOptions, BudgetResult, Cliff, GuardrailContext, Risks, Schedule,
};
use rr_consequence::{ConsequenceAssessment, CountyData};
use rr_content::Content;
use rr_hazards::HazardAssessment;
use rr_supply::{ItemSizer, SizedLine, SupplyContext};
use rr_types::{
    Attribution, BucketAssessment, BucketId, BucketKind, ClimateHorizon, CountyRecord, EngineError,
    ErrorCode, HazardId, ItemId, LocationResolved, PlanInput, PlanItemKind, TARGET_LADDER_DAYS,
    Target, Warning, WarningSeverity,
};

use crate::coverage::{self, Offers};
use crate::source::CountySource;

/// Share of each month's new money a split schedule sets aside toward an item that costs more than
/// a month's budget (`rr_budget::Schedule::Split`, the planner's default).
pub const RESERVE_SHARE: f64 = 0.5;

/// The longest plan, in months after month 0 (the ten-year horizon).
pub const MAX_PLAN_MONTHS: u16 = 120;

/// How much a powered medical device multiplies the harm weight of `power` (the allocator's
/// `power_device` harm-weight row, DESIGN §4.7).
pub const DEVICE_MULTIPLIER: f64 = 3.0;

/// A household flood or earthquake rate at or above this counts as "prone" for the insurance
/// guardrails: the one-percent-a-year yardstick behind FEMA flood maps (`fema_flood_zones`).
pub const PRONE_RATE_PER_YEAR: f64 = 0.01;

/// Every result of one `assess` call before the packet is written.
#[derive(Debug, Clone)]
pub struct Assessment {
    /// The household, as validated.
    pub input: PlanInput,
    /// Where it lives.
    pub location: LocationResolved,
    /// The county's core-pack record.
    pub county: CountyRecord,
    /// Hazard rates, register cards, named scenarios and notes (`rr-hazards`).
    pub hazards: HazardAssessment,
    /// Bucket targets, curves, scenarios, cliff warnings and the statement (`rr-consequence`).
    pub consequence: ConsequenceAssessment,
    /// Location facts for the quantity rules.
    pub supply_context: SupplyContext,
    /// Requirement lines with their rates and formulas (`rr-supply`).
    pub lines: Vec<SizedLine>,
    /// The catalogue as offered to this household.
    pub offers: Offers,
    /// The month-by-month plan (`rr-budget`).
    pub budget: BudgetResult,
    /// The final bucket assessments: consequence targets, the plan's coverage, and `tier_enough`
    /// from `rr-supply`.
    pub buckets: Vec<BucketAssessment>,
    /// Guardrail and cliff warnings (the consequence crate's cliff warnings are kept; the budget
    /// crate's duplicates are dropped).
    pub warnings: Vec<Warning>,
    /// Credit lines and disclaimers from the data source.
    pub attributions: Vec<Attribution>,
    /// Existing inventory entries whose item id is not in the catalogue.
    pub unknown_existing: Vec<ItemId>,
    /// Everyday basics credited as owned because `assume_basics` is on: (item, quantity in the
    /// item's unit). The budget ran with these added to `existing`.
    pub assumed: Vec<(ItemId, f64)>,
}

impl Assessment {
    /// The final assessment of one bucket.
    pub fn bucket(&self, id: BucketId) -> &BucketAssessment {
        let i = BucketId::ALL.iter().position(|b| *b == id).unwrap_or(0);
        &self.buckets[i]
    }

    /// A duration bucket's target in days (0 for other buckets).
    pub fn target_days(&self, id: BucketId) -> f64 {
        coverage::target_days(&self.bucket(id).target)
    }

    /// The register rate of a hazard (0 when it is not in the register).
    pub fn hazard_rate(&self, id: HazardId) -> f64 {
        self.hazards
            .profiles
            .iter()
            .find(|p| p.id == id)
            .map_or(0.0, |p| p.rate_per_year)
    }

    /// Everything the household will have at the end of the plan: what it owns, the free actions
    /// and every purchase, by item id.
    pub fn final_inventory(&self) -> BTreeMap<ItemId, f64> {
        let mut out: BTreeMap<ItemId, f64> = BTreeMap::new();
        for m in &self.budget.plan.months {
            for it in &m.items {
                if it.kind != PlanItemKind::Reserve {
                    *out.entry(it.item_id.clone()).or_insert(0.0) += f64::from(it.quantity);
                }
            }
        }
        out
    }
}

/// The supply context from the county record: days at or above 95 °F under the chosen climate
/// dial (the county count, or today's count times the pack's ratio), the centroid's latitude,
/// and a nuclear plant within 16 km (the location's flag, or the county's nearest plant).
pub fn supply_context(
    county: &CountyRecord,
    location: &LocationResolved,
    climate: ClimateHorizon,
) -> SupplyContext {
    let get = |k: &str| county.climate.get(k).map(|v| f64::from(*v));
    let hist = get("days_over_95f_hist");
    let days = match climate {
        ClimateHorizon::Today => hist,
        ClimateHorizon::Y2050 => get("days_over_95f_2050")
            .or_else(|| hist.zip(get("hot_days_95f")).map(|(h, r)| h * r))
            .or(hist),
    };
    let near_by_county = county
        .facilities
        .as_ref()
        .and_then(|f| f.nearest_nuclear_km)
        .is_some_and(|km| km <= 16.093_44);
    SupplyContext {
        days_at_or_above_95f: days,
        latitude: Some(county.centroid.lat),
        nuclear_plant_within_16km: Some(
            location.facility_flags.nuclear_plant_within_16km || near_by_county,
        ),
    }
}

/// Tabulates a consequence curve for the allocator: 64 log-spaced durations from an hour to two
/// years (or four times the target) plus every ladder value, with Λ never rising.
fn budget_curve(curve: &rr_consequence::ExceedanceCurve, target: f64) -> BucketCurve {
    let max_days = (4.0 * target).max(730.0);
    let (mut days, _) = curve.sample(64, max_days);
    days.extend(TARGET_LADDER_DAYS.iter().map(|&d| f64::from(d)));
    days.sort_by(f64::total_cmp);
    days.dedup_by(|a, b| (*a - *b).abs() <= 1e-9 * b.abs().max(1.0));
    let mut lambda: Vec<f64> = days.iter().map(|&d| curve.lambda(d).max(0.0)).collect();
    for i in 1..lambda.len() {
        if lambda[i] > lambda[i - 1] {
            lambda[i] = lambda[i - 1];
        }
    }
    BucketCurve::new(days, lambda, target)
}

/// The event a cliff warning names, from its message ("... depends mostly on one event: X.").
fn cliff_driver(w: &Warning) -> String {
    w.message
        .split_once(": ")
        .map(|(_, d)| d.trim_end_matches('.').to_owned())
        .unwrap_or_else(|| w.message.clone())
}

fn internal(what: &str, e: impl std::fmt::Display) -> EngineError {
    EngineError::new(
        ErrorCode::Internal,
        format!("The engine could not {what}: {e}"),
    )
}

/// Runs every stage for a validated household.
pub fn run<S: CountySource + ?Sized>(
    source: &S,
    content: &Content,
    input: &PlanInput,
) -> Result<Assessment, EngineError> {
    let problems = input.validate();
    if !problems.is_empty() {
        return Err(EngineError::bad_input(problems));
    }
    let location = source.resolve(&input.location)?;
    let county = source
        .county(&location.county_fips)
        .ok_or_else(|| {
            EngineError::new(
                ErrorCode::PackMissing,
                format!("The data for {} is not loaded yet.", location.county_name),
            )
        })?
        .clone();

    // Hazards and consequences.
    let hazards = rr_hazards::assess(input, &county, source.base_rates(), &location);
    let consequence = rr_consequence::assess(
        input,
        &hazards.rates,
        CountyData::from_record(&county),
        &hazards.scenarios,
    );
    // tier_enough has one source: rr-supply's rule (DESIGN §4.5).
    let mut buckets: Vec<BucketAssessment> = consequence.buckets.clone();
    for b in &mut buckets {
        b.tier_enough = rr_supply::tier_enough(b);
    }

    // Supplies.
    let supply_context = supply_context(&county, &location, input.dials.climate);
    let sizer = ItemSizer::new(input, &buckets, &supply_context);
    let lines: Vec<SizedLine> = sizer.lines().to_vec();
    let targets: BTreeMap<BucketId, f64> = buckets
        .iter()
        .filter(|b| b.id.kind() == BucketKind::Duration)
        .map(|b| (b.id, coverage::target_days(&b.target)))
        .collect();
    let register: BTreeMap<HazardId, f64> = hazards
        .profiles
        .iter()
        .map(|p| (p.id, p.rate_per_year))
        .collect();
    let offers = coverage::build(content, &sizer, &targets, &register);

    // Everyday basics most homes have (blankets, a pot, a bag per person, three days of ordinary
    // food): credited as owned when `assume_basics` is on, unless the household listed the item
    // itself (any quantity, 0 included, is its own answer).
    let assumed = assumed_basics(input, &offers, &lines);
    let mut household = input.clone();
    household
        .existing
        .extend(assumed.iter().map(|(id, qty)| rr_types::Owned {
            item_id: id.clone(),
            qty: *qty as f32,
            paid_usd: None,
        }));

    // Budget.
    // A curve for every duration bucket the catalogue can cover. A bucket with no part to buy
    // (nobody takes a prescription, so no medicine to stock) gets no curve: there is nothing to
    // value, and the plan counts it as covered.
    let mut curves = BTreeMap::new();
    for (&bucket, &target) in &targets {
        if target <= 0.0 || offers.rule.parts_of(bucket).is_empty() {
            continue;
        }
        if let Some(c) = consequence.curve(bucket) {
            let mut curve = budget_curve(c, target);
            if bucket == BucketId::Power
                && offers
                    .rule
                    .parts_of(bucket)
                    .iter()
                    .any(|p| p.name == coverage::DEVICE_PART)
            {
                // A powered medical device triples the harm weight of a power cut (DESIGN §4.7):
                // two thirds of the bucket's harm is the device's, so its part gets that share.
                curve.part_shares.insert(
                    coverage::DEVICE_PART.to_owned(),
                    1.0 - 1.0 / DEVICE_MULTIPLIER,
                );
            }
            curves.insert(bucket, curve);
        }
    }
    let risks = Risks {
        curves,
        assessments: buckets.iter().map(|b| (b.id, b.clone())).collect(),
    };
    let prone = |ids: &[HazardId]| {
        ids.iter()
            .map(|h| register.get(h).copied().unwrap_or(0.0))
            .sum::<f64>()
    };
    let context = GuardrailContext {
        flood_zone: prone(&[HazardId::RiverineFlooding, HazardId::CoastalFlooding])
            >= PRONE_RATE_PER_YEAR,
        quake_zone: prone(&[HazardId::Earthquake]) >= PRONE_RATE_PER_YEAR,
        cliffs: consequence
            .warnings
            .iter()
            .filter_map(|w| {
                let bucket: BucketId = w.id.strip_prefix("cliff_")?.parse().ok()?;
                Some(Cliff {
                    bucket,
                    driver: cliff_driver(w),
                })
            })
            .collect(),
    };
    let items = offers.items();
    let budget_input = BudgetInput {
        household: &household,
        catalogue: &items,
        meta: &offers.meta,
        // The offers already fold in every requirement line (see `coverage`).
        requirements: &[],
        risks: &risks,
        context: &context,
        options: BudgetOptions {
            max_months: MAX_PLAN_MONTHS,
            rare_catastrophic_opt_in: input.dials.rare_catastrophic_opt_in,
            schedule: Schedule::Split {
                reserve_share: RESERVE_SHARE,
            },
        },
    };
    let mut budget = rr_budget::allocate_with_rule(&budget_input, &offers.rule)
        .map_err(|e| internal("plan the purchases", e))?;
    // The allocator lists what the household has as "You already have this"; say what was
    // assumed instead.
    for m in &mut budget.plan.months {
        for it in &mut m.items {
            if it.done && assumed.iter().any(|(id, _)| *id == it.item_id) {
                it.why = ASSUMED_WHY.to_owned();
            }
        }
    }

    // Final buckets: the plan's coverage (every step done) and today's (before the plan buys
    // anything). The allocator lists what the household owns, listed or assumed, and the free
    // steps it has done as done steps in month 0.
    let inventory = {
        let mut out: BTreeMap<ItemId, f64> = BTreeMap::new();
        for m in &budget.plan.months {
            for it in &m.items {
                if it.kind != PlanItemKind::Reserve {
                    *out.entry(it.item_id.clone()).or_insert(0.0) += f64::from(it.quantity);
                }
            }
        }
        out
    };
    let today = {
        let mut out: BTreeMap<ItemId, f64> = BTreeMap::new();
        for m in &budget.plan.months {
            for it in m.items.iter().filter(|it| it.done) {
                *out.entry(it.item_id.clone()).or_insert(0.0) += f64::from(it.quantity);
            }
        }
        out
    };
    // A go-bag itself, not a staging step packed into one.
    let has_bag = |inv: &BTreeMap<ItemId, f64>| {
        offers.offered.iter().any(|o| {
            o.joins
                .iter()
                .any(|j| j.line_id == "evacuate.go_bag" && !j.via_alternative)
                && inv.get(&o.item.id).copied().unwrap_or(0.0) > 0.0
        })
    };
    let (bag_at_end, bag_today) = (has_bag(&inventory), has_bag(&today));
    for b in &mut buckets {
        let nothing_to_buy =
            b.id.kind() == BucketKind::Duration && offers.rule.parts_of(b.id).is_empty();
        let end = coverage_of(
            b.target,
            budget.covered.get(&b.id).copied(),
            nothing_to_buy,
            bag_at_end,
        )
        .unwrap_or(b.covered);
        let now = coverage_of(
            b.target,
            budget.covered_today.get(&b.id).copied(),
            nothing_to_buy,
            bag_today,
        )
        .unwrap_or(b.covered_today);
        // A checklist's size is the plan's: the target counts the steps the plan lists.
        if let (Target::Readiness { p_need_10yr, .. }, Target::Readiness { done, of, .. }) =
            (b.target, end)
        {
            b.target = Target::Readiness {
                p_need_10yr,
                done,
                of,
            };
        }
        b.covered = end;
        b.covered_today = now;
    }

    // Warnings: the consequence crate's cliff warnings are canonical; drop the budget's duplicate
    // ids. The budget's "no stored water" check reads the whole no-water bucket (its weakest
    // part, often the toilet); drop it when the stored-water part itself has water by month 1.
    let stored_by_month_1 = {
        let mut early: Vec<(ItemId, f64)> = Vec::new();
        for m in budget.plan.months.iter().take_while(|m| m.index <= 1) {
            for it in &m.items {
                if it.kind != PlanItemKind::Reserve {
                    early.push((it.item_id.clone(), f64::from(it.quantity)));
                }
            }
        }
        offers
            .rule
            .part_days(BucketId::WaterOut, coverage::STORED_WATER_PART, &early)
    };
    let mut warnings: Vec<Warning> = consequence.warnings.clone();
    for w in &budget.warnings {
        if w.id == "no_water_after_month_1" && stored_by_month_1 > 0.0 {
            continue;
        }
        if !warnings.iter().any(|x| x.id == w.id) {
            warnings.push(w.clone());
        }
    }

    let unknown_existing: Vec<ItemId> = {
        let mut v: Vec<ItemId> = input
            .existing
            .iter()
            .filter(|o| content.item(o.item_id.as_str()).is_none())
            .map(|o| o.item_id.clone())
            .collect();
        v.sort();
        v.dedup();
        v
    };
    if !assumed.is_empty() {
        let names: Vec<String> = assumed
            .iter()
            .filter_map(|(id, _)| content.item(id.as_str()))
            .map(|i| crate::packet::text::lower_first(&i.name))
            .collect();
        warnings.push(Warning {
            id: "assumed_basics".to_owned(),
            severity: WarningSeverity::Note,
            message: format!(
                "The plan counts everyday basics most homes already have: {}.",
                crate::packet::text::join_and(&names)
            ),
            why: ASSUMED_HOW_TO_UNTICK.to_owned(),
            related: assumed
                .iter()
                .map(|(id, _)| id.as_str().to_owned())
                .collect(),
        });
    }
    if !unknown_existing.is_empty() {
        let names: Vec<&str> = unknown_existing.iter().map(|i| i.as_str()).collect();
        warnings.push(Warning {
            id: "unknown_existing_items".to_owned(),
            severity: WarningSeverity::Note,
            message: format!(
                "Some things on your list of what you already have were not counted: {}.",
                names.join(", ")
            ),
            why: "The plan only counts items it knows. Pick them again from the list of what you \
                  already have, so the plan does not ask you to buy them twice."
                .to_owned(),
            related: names.iter().map(|s| (*s).to_owned()).collect(),
        });
    }

    Ok(Assessment {
        input: input.clone(),
        location,
        county,
        hazards,
        consequence,
        supply_context,
        lines,
        offers,
        budget,
        buckets,
        warnings,
        attributions: source.attributions(),
        unknown_existing,
        assumed,
    })
}

/// A bucket's coverage in its target's kind, from the allocator's coverage for one state of the
/// household (the plan's end, or today): duration days capped at the target (the whole target
/// when there is nothing to buy for the bucket), the go-bag's days away when the household has a
/// go-bag in that state, and a checklist's steps done. `None` when the allocator reports nothing
/// usable, so the consequence crate's value stands.
fn coverage_of(
    target: Target,
    covered: Option<Target>,
    nothing_to_buy: bool,
    bag: bool,
) -> Option<Target> {
    match (target, covered) {
        (Target::Days { value, .. }, _) if nothing_to_buy => Some(Target::Days {
            value,
            low: value,
            high: value,
        }),
        (Target::Days { value, .. }, Some(Target::Days { value: c, .. })) => {
            let v = c.min(value).max(0.0);
            Some(Target::Days {
                value: v,
                low: v,
                high: v,
            })
        }
        (
            Target::Evacuate {
                p_need_10yr,
                notice_hours_low,
                notice_hours_high,
                days_away,
            },
            _,
        ) => Some(Target::Evacuate {
            p_need_10yr,
            notice_hours_low,
            notice_hours_high,
            days_away: if bag { days_away } else { 0.0 },
        }),
        (Target::Readiness { p_need_10yr, .. }, Some(Target::Readiness { done, of, .. })) => {
            Some(Target::Readiness {
                p_need_10yr,
                done,
                of,
            })
        }
        (_, Some(c)) if c.kind() == target.kind() => Some(c),
        _ => None,
    }
}

/// The "why" of a basic the plan assumed the household has.
pub const ASSUMED_WHY: &str = "Assumed: most homes already have this, so the plan counts it \
    instead of buying it. If yours does not, untick \"Assume everyday basics\" on the Have screen.";

/// How to undo the assumption, for the note and the packet.
pub const ASSUMED_HOW_TO_UNTICK: &str = "Most homes have these, so the plan does not ask you to \
    buy them. If any is missing, untick \"Assume everyday basics\" on the Have screen, or list \
    the item as not owned, and the plan will add it.";

/// Days of a divisible basic (ordinary food) assumed on hand: the three-day step.
pub const ASSUMED_DAYS: f64 = 3.0;

/// The basics to credit: offered items the catalogue flags `assumed_basic` (free ones too, such
/// as a charged phone, which then count as done), not listed by the household, at the quantity
/// the household needs (a divisible basic at three days' worth, or its line's days if fewer).
fn assumed_basics(input: &PlanInput, offers: &Offers, lines: &[SizedLine]) -> Vec<(ItemId, f64)> {
    if !input.assume_basics {
        return Vec::new();
    }
    offers
        .offered
        .iter()
        .filter(|o| o.item.assumed_basic)
        .filter(|o| !input.existing.iter().any(|e| e.item_id == o.item.id))
        .filter_map(|o| {
            let qty = if o.divisible {
                // Three days of the line it meets, in the item's unit.
                o.joins
                    .iter()
                    .filter_map(|j| {
                        let l = lines.iter().find(|l| l.line.id == j.line_id)?;
                        let per_day = l.per_day?;
                        let days = l.days.unwrap_or(ASSUMED_DAYS).min(ASSUMED_DAYS);
                        Some(per_day * days / j.units_per_item)
                    })
                    .fold(0.0_f64, f64::max)
                    .ceil()
            } else {
                o.quantity
            };
            (qty > 0.0).then(|| (o.item.id.clone(), qty))
        })
        .collect()
}
