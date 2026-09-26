//! Property tests over seeded random households, catalogues and curves (`common::generate`,
//! driven by `rr_types::rng::SplitMix64`). Each property names the seed that breaks it.

mod common;

use std::collections::BTreeMap;

use common::generate::{Case, case};
use common::{audit, days_at, readiness_at};
use rr_budget::{
    BucketCurve, BudgetInput, BudgetOptions, BudgetResult, Contributes, GuardrailContext, ItemMeta,
    Risks, Schedule, allocate,
};
use rr_types::{BucketId, PlanItemKind, TierId, fixtures};

const CASES: u64 = 250;

fn run(c: &Case, options: BudgetOptions) -> BudgetResult {
    let input = BudgetInput {
        household: &c.household,
        catalogue: &c.items,
        meta: &c.meta,
        requirements: &[],
        risks: &c.risks,
        context: &c.context,
        options,
    };
    allocate(&input).unwrap_or_else(|e| panic!("seed {}: {e}", c.seed))
}

/// The default schedule (split, half of each month's money reserved for an expensive item).
const SPLIT: Schedule = Schedule::Split { reserve_share: 0.5 };

/// All three schedules.
const SCHEDULES: [Schedule; 3] = [SPLIT, Schedule::FixedOrder, Schedule::ResearchShortcuts];

fn is_rare(c: &Case, id: &str) -> bool {
    c.items.iter().any(|i| i.id == id && i.rare_catastrophic)
}

fn options(schedule: Schedule, rare: bool) -> BudgetOptions {
    BudgetOptions {
        max_months: 120,
        rare_catastrophic_opt_in: rare,
        schedule,
    }
}

/// Spend never exceeds the money that has come in, in any month, under either schedule.
#[test]
fn spend_never_exceeds_budget() {
    for seed in 0..CASES {
        let c = case(seed);
        for schedule in SCHEDULES {
            let r = run(&c, options(schedule, seed % 3 == 0));
            let (left, _) = audit(&r, &|id| is_rare(&c, id));
            for (m, x) in left.iter().enumerate() {
                assert!(
                    *x > -0.05,
                    "seed {seed} {schedule:?}: month {m} ends at {x}"
                );
            }
            for p in &r.sequence {
                assert!(p.cost_usd >= 0.0 && p.quantity > 0.0, "seed {seed}: {p:?}");
            }
            // Every month after month 0 shows a step (a purchase or a deposit), and months are
            // numbered without gaps.
            for (m, month) in r.plan.months.iter().enumerate() {
                assert_eq!(usize::from(month.index), m, "seed {seed}");
                assert!(
                    m == 0 || !month.items.is_empty(),
                    "seed {seed} {schedule:?}: month {m} is empty"
                );
            }
        }
    }
}

/// Fixed-order schedule: more monthly budget never lowers coverage in any bucket in any month.
/// Holds for any two positive budgets; a household with no monthly budget spends leftover one-off
/// money on the best item that fits, so it is compared from the smallest positive budget up. (The
/// split schedule does not promise this month by month, only at the end of the plan: see
/// `split_final_coverage_never_lower_with_more_budget`.)
#[test]
fn fixed_order_more_budget_never_lowers_coverage_in_any_month() {
    for seed in 0..CASES {
        let mut c = case(seed);
        let rare = seed % 2 == 0;
        if c.household.finances.monthly_budget_usd <= 0.0 {
            c.household.finances.monthly_budget_usd = 1.0;
        }
        let small = run(&c, options(Schedule::FixedOrder, rare));
        let mut c2 = c.clone();
        let extra = [0.5_f32, 7.0, 25.0, 60.0, 400.0][(seed % 5) as usize];
        c2.household.finances.monthly_budget_usd += extra;
        if seed % 4 == 0 {
            c2.household.finances.one_off_budget_usd += 75.0;
        }
        let big = run(&c2, options(Schedule::FixedOrder, rare));
        let months = small
            .coverage_by_month
            .len()
            .max(big.coverage_by_month.len());
        for m in 0..months {
            for (b, lo) in days_at(&small, m) {
                let hi = days_at(&big, m)[b];
                assert!(
                    hi + 1e-9 >= *lo,
                    "seed {seed}: month {m} {b}: {hi} < {lo} with more money"
                );
            }
            for (b, lo) in readiness_at(&small, m) {
                let hi = readiness_at(&big, m)[b];
                assert!(
                    hi >= *lo,
                    "seed {seed}: month {m} {b}: {hi} < {lo} readiness with more money"
                );
            }
        }
        if let (Some(a), Some(b)) = (small.plan.done_month, big.plan.done_month) {
            assert!(
                b <= a,
                "seed {seed}: done in month {b} with more money, {a} with less"
            );
        }
    }
}

/// A higher exceedance curve for a bucket never moves that bucket's items later in the buying
/// order, with one exception the rules create on purpose: a later-tier item that also serves the
/// bucket can cross the five-times promotion line and be bought earlier, covering the bucket
/// sooner and making a single-bucket item unnecessary (or pushing it to a later tier). So: an item
/// that serves only that bucket keeps its place or moves up, unless an item serving the same
/// bucket was newly promoted ahead of it. Items for other buckets never overtake it.
#[test]
fn higher_curve_never_delays_its_items() {
    let (mut checked, mut substituted) = (0, 0);
    for seed in 0..CASES {
        let c = case(seed);
        let buckets: Vec<BucketId> = c.risks.curves.keys().copied().collect();
        for (n, &b) in buckets
            .iter()
            .enumerate()
            .filter(|(n, _)| (*n + seed as usize) % 2 == 0)
        {
            let k = 1.2 + ((seed as usize + n) % 7) as f64 * 0.3;
            let mut c2 = c.clone();
            let curve = c2.risks.curves.get_mut(&b).unwrap();
            curve.lambda.iter_mut().for_each(|l| *l *= k);
            let before = run(&c, options(Schedule::FixedOrder, false));
            let after = run(&c2, options(Schedule::FixedOrder, false));
            let serves_b = |id: &str| {
                c.meta
                    .iter()
                    .any(|m| m.item_id == id && m.contributes.iter().any(|x| x.bucket == b))
            };
            // Position = order of first purchase among distinct items, so an item bought in more,
            // smaller chunks does not push later items down the list.
            let order = |r: &BudgetResult| -> Vec<(String, bool)> {
                let mut seen: Vec<(String, bool)> = Vec::new();
                for p in &r.sequence {
                    if !seen.iter().any(|(id, _)| id == p.item_id.as_str()) {
                        seen.push((p.item_id.as_str().to_owned(), p.promoted));
                    }
                }
                seen
            };
            let (ob, oa) = (order(&before), order(&after));
            let first = |o: &[(String, bool)], id: &str| o.iter().position(|(x, _)| x == id);
            // Items serving b that are newly promoted after the rise (not promoted before, or
            // placed later before), with their new positions.
            let newly_promoted: Vec<usize> = oa
                .iter()
                .enumerate()
                .filter(|(_, (id, promoted))| *promoted && serves_b(id))
                .filter(|(i, (id, _))| {
                    ob.iter()
                        .position(|(x, promoted)| x == id && *promoted)
                        .is_none_or(|j| j > *i)
                })
                .map(|(i, _)| i)
                .collect();
            for (it, m) in c.items.iter().zip(&c.meta) {
                let only_b = !m.contributes.is_empty()
                    && m.contributes.iter().all(|x| x.bucket == b)
                    && m.readiness.is_empty()
                    && !it.free
                    && !it.rare_catastrophic;
                let Some(p0) = first(&ob, it.id.as_str()).filter(|_| only_b) else {
                    continue;
                };
                checked += 1;
                let p1 = first(&oa, it.id.as_str());
                if p1.is_some_and(|p1| p1 <= p0) {
                    continue;
                }
                let limit = p1.unwrap_or(usize::MAX).min(oa.len());
                assert!(
                    newly_promoted.iter().any(|&q| q < limit),
                    "seed {seed}: {} moved from {p0} to {p1:?} when {b} rose x{k}, and no item \
                     serving {b} was promoted ahead of it",
                    it.id
                );
                substituted += 1;
            }
        }
    }
    println!(
        "higher curve: {checked} items checked, {substituted} overtaken by a newly promoted item for the same bucket"
    );
    assert!(checked > 150, "only {checked} items checked");
    assert!(
        substituted * 10 < checked,
        "{substituted} of {checked} needed the exception"
    );
}

/// Free actions come first in month 0 and never appear later.
#[test]
fn free_items_always_precede_purchases() {
    for seed in 0..CASES {
        let c = case(seed);
        for schedule in SCHEDULES {
            let r = run(&c, options(schedule, seed % 2 == 1));
            let m0 = &r.plan.months[0].items;
            let first_other = m0
                .iter()
                .position(|i| i.kind != PlanItemKind::FreeAction)
                .unwrap_or(m0.len());
            assert!(
                m0[first_other..]
                    .iter()
                    .all(|i| i.kind != PlanItemKind::FreeAction),
                "seed {seed}: a free action after a purchase in month 0"
            );
            let free_items = c.items.iter().filter(|i| i.free).count();
            assert_eq!(
                first_other, free_items,
                "seed {seed}: every free item appears in month 0"
            );
            for m in &r.plan.months[1..] {
                assert!(
                    m.items.iter().all(|i| i.kind != PlanItemKind::FreeAction),
                    "seed {seed}"
                );
                assert!(
                    m.items.iter().all(|i| !i.done),
                    "seed {seed}: done items only in month 0"
                );
            }
        }
    }
}

/// Same input, same output: the result and its JSON are identical on a second run, under every
/// schedule.
#[test]
fn same_input_same_output() {
    for seed in 0..CASES {
        let c = case(seed);
        for schedule in SCHEDULES {
            let a = run(&c, options(schedule, seed % 2 == 0));
            let b = run(&c, options(schedule, seed % 2 == 0));
            assert_eq!(a, b, "seed {seed} {schedule:?}");
            let ja = serde_json::to_string(&a.plan).unwrap();
            let jb = serde_json::to_string(&b.plan).unwrap();
            assert_eq!(ja, jb, "seed {seed} {schedule:?}");
            assert!(
                !ja.contains("-0.0") && !ja.contains("NaN"),
                "seed {seed}: {ja}"
            );
        }
    }
}

/// A sinking fund's item is bought within a bounded number of months of its first deposit:
/// ceil(cost / monthly money) under the fixed-order and research schedules, and
/// ceil(cost / (reserve share x monthly money)) under the split schedule, where "monthly money" is
/// what reaches that track (90 % for the main plan and 10 % for the allowance with the
/// rare-catastrophe opt-in). A split fund whose item stops being worth buying pays for the next
/// purchase instead; that is not a late arrival and is counted separately.
#[test]
fn envelopes_resolve_within_ceil_cost_over_budget() {
    let (mut seen, mut split_seen, mut moved) = (0, 0, 0);
    for seed in 0..CASES {
        let c = case(seed);
        let monthly = f64::from(c.household.finances.monthly_budget_usd);
        let opt_in = seed % 2 == 0;
        for schedule in SCHEDULES {
            let r = run(&c, options(schedule, opt_in));
            let (_, lives) = audit(&r, &|id| is_rare(&c, id));
            let last = r.plan.months.last().unwrap().index;
            for life in lives {
                let rare = is_rare(&c, life.item_id.as_str());
                let per_month = match (rare, opt_in) {
                    (true, _) => 0.1 * monthly,
                    (false, true) => 0.9 * monthly,
                    (false, false) => monthly,
                };
                // Split deposits come at the start of a month, the others at the end. A split fund
                // opened in month 0 starts from the one-off amount, so its monthly deposits are
                // counted from month 1.
                let (share, first_month_counts) = match schedule {
                    Schedule::Split { reserve_share } if !rare => {
                        (reserve_share, u16::from(life.opened > 0))
                    }
                    _ => (1.0, 0),
                };
                // Cost: the largest the plan announced for the fund's item while saving (a
                // divisible item's chunk can shrink when another purchase covers part of it).
                let announced = if rare {
                    0.0
                } else {
                    r.money_by_month
                        .iter()
                        .filter(|mm| {
                            mm.month >= life.opened && life.bought.is_none_or(|b| mm.month <= b)
                        })
                        .filter_map(|mm| mm.saving_for.as_ref().map(|(_, cost)| *cost))
                        .fold(0.0, f64::max)
                };
                let needed = life.needed.max(announced);
                let limit = (needed / (share * per_month) - 1e-9).ceil() as u16;
                match (life.bought, &life.bought_item) {
                    (Some(b), Some(item)) if *item == life.item_id => {
                        assert!(
                            b - life.opened + first_month_counts <= limit,
                            "seed {seed} {schedule:?}: {} opened {} bought {b}, limit {limit}",
                            life.item_id,
                            life.opened
                        );
                        seen += 1;
                        if share < 1.0 {
                            split_seen += 1;
                        }
                    }
                    (Some(_), _) => {
                        assert!(
                            matches!(schedule, Schedule::Split { .. }),
                            "seed {seed} {schedule:?}: {} paid for something else",
                            life.item_id
                        );
                        moved += 1;
                    }
                    (None, _) => assert!(
                        last == 120 && last - life.opened + first_month_counts <= limit.max(1),
                        "seed {seed} {schedule:?}: {} never bought",
                        life.item_id
                    ),
                }
            }
        }
    }
    println!(
        "envelopes: {seen} bought in time ({split_seen} split), {moved} moved to another item"
    );
    assert!(
        seen > 20 && split_seen > 10,
        "only {seen} ({split_seen} split) envelopes exercised"
    );
}

/// Split schedule: every month in which the free money covers the cheapest item worth buying
/// (other than the one being saved for) buys at least one thing, so the household sees progress
/// every month.
#[test]
fn split_buys_something_every_month_it_can() {
    let mut months = 0;
    for seed in 0..CASES {
        let c = case(seed);
        let r = run(&c, options(SPLIT, seed % 2 == 0));
        for mm in &r.money_by_month {
            let Some(cheapest) = mm.cheapest_usd else {
                continue;
            };
            if mm.available_usd + 1e-6 >= cheapest {
                months += 1;
                assert!(
                    r.sequence
                        .iter()
                        .any(|p| p.month == mm.month && !p.rare_catastrophic),
                    "seed {seed}: month {} had ${:.2} free and a ${cheapest:.2} item, but bought \
                     nothing",
                    mm.month,
                    mm.available_usd
                );
            }
        }
    }
    assert!(months > 500, "only {months} months checked");
}

/// Split schedule: when the plan with the bigger budget finishes (nothing left worth buying), it
/// covers every bucket at least as far as the plan with the smaller budget does at its end, and
/// when both finish they cover the same. Coverage counts up to each target, since days beyond it
/// earn nothing (a bigger budget may buy one larger item where a smaller one bought two, and
/// overshoot a target by less). Plans run for 50 years here so that small budgets finish; when
/// neither plan finishes, the split schedule makes no promise, because its buying order depends
/// on the money (the fixed-order schedule is monotone even then).
#[test]
fn split_final_coverage_never_lower_with_more_budget() {
    let (mut compared, mut unfinished) = (0, 0);
    for seed in 0..CASES {
        let mut c = case(seed);
        let rare = seed % 2 == 0;
        if c.household.finances.monthly_budget_usd <= 0.0 {
            c.household.finances.monthly_budget_usd = 1.0;
        }
        let long = BudgetOptions {
            max_months: 600,
            ..options(SPLIT, rare)
        };
        let small = run(&c, long);
        let mut c2 = c.clone();
        c2.household.finances.monthly_budget_usd +=
            [0.5_f32, 7.0, 25.0, 60.0, 400.0][(seed % 5) as usize];
        let big = run(&c2, long);
        if big.stopped_month.is_none() {
            unfinished += 1;
            continue;
        }
        let both = small.stopped_month.is_some();
        let (lo, hi) = (days_at(&small, usize::MAX), days_at(&big, usize::MAX));
        for (b, curve) in &c.risks.curves {
            let cap = curve.target_days;
            assert!(
                hi[b].min(cap) + 1e-9 >= lo[b].min(cap),
                "seed {seed}: {b} ends at {} with more money, {} with less (target {cap})",
                hi[b],
                lo[b]
            );
            if both {
                assert!(
                    (hi[b].min(cap) - lo[b].min(cap)).abs() < 1e-6,
                    "seed {seed}: both finished but {b} differs: {} vs {}",
                    hi[b],
                    lo[b]
                );
            }
            compared += 1;
        }
        let (rlo, rhi) = (
            readiness_at(&small, usize::MAX),
            readiness_at(&big, usize::MAX),
        );
        for (b, n) in rlo {
            assert!(
                rhi[b] >= *n,
                "seed {seed}: {b} readiness {} < {n} with more money",
                rhi[b]
            );
            if both {
                assert_eq!(
                    rhi[b], *n,
                    "seed {seed}: both finished but {b} readiness differs"
                );
            }
        }
    }
    println!("final coverage: {compared} bucket comparisons, {unfinished} pairs unfinished");
    assert!(compared > 1000 && unfinished * 10 < CASES as usize);
}

/// Everything bought has positive value, and a finished plan covers every target.
#[test]
fn buys_only_what_is_worth_buying_and_stops_when_covered() {
    for seed in 0..CASES {
        let c = case(seed);
        for schedule in SCHEDULES {
            let r = run(&c, options(schedule, false));
            for p in &r.sequence {
                assert!(p.value > 0.0, "seed {seed}: {p:?}");
                assert!(
                    !p.rare_catastrophic,
                    "seed {seed}: rare items need the opt-in"
                );
            }
            if r.plan.done_month.is_some() {
                let end = days_at(&r, usize::MAX);
                for (b, curve) in &c.risks.curves {
                    assert!(
                        end[b] + 1e-6 >= curve.target_days,
                        "seed {seed}: done but {b} short"
                    );
                }
            }
            for w in &r.warnings {
                assert!(
                    !w.message.is_empty() && !w.why.is_empty(),
                    "seed {seed}: {w:?}"
                );
            }
        }
    }
}

// ---- Hand-built cases for the two schedules. ----

/// Two items in two buckets: a $100 item with the best value per dollar (power) and a $20 item
/// (comms) worth 60 % as much per dollar. Flat curves make the values easy to check by hand.
fn two_item_case(monthly: f32) -> Case {
    let mut household = fixtures::get("philadelphia-renters-4").unwrap();
    household.finances.monthly_budget_usd = monthly;
    household.finances.one_off_budget_usd = 0.0;
    household.existing.clear();
    let items = vec![
        common::item(
            "big",
            "Big item",
            "each",
            &[BucketId::Power],
            TierId::H72,
            false,
            false,
            (100.0, 100.0),
        ),
        common::item(
            "small",
            "Small item",
            "each",
            &[BucketId::Comms],
            TierId::H72,
            false,
            false,
            (20.0, 20.0),
        ),
    ];
    // Values: big 10·1·0.1·3 = 3.0 ($100: 0.030/$); small 10·1·0.036·1 = 0.36 ($20: 0.018/$).
    let mut big = ItemMeta::new("big");
    big.contributes = vec![Contributes::per_household(BucketId::Power, 3.0)];
    let mut small = ItemMeta::new("small");
    small.contributes = vec![Contributes::per_household(BucketId::Comms, 1.0)];
    let mut curves = BTreeMap::new();
    curves.insert(BucketId::Power, BucketCurve::new(vec![1.0], vec![0.1], 3.0));
    curves.insert(
        BucketId::Comms,
        BucketCurve::new(vec![1.0], vec![0.036], 1.0),
    );
    Case {
        seed: 0,
        household,
        items,
        meta: vec![big, small],
        risks: Risks {
            curves,
            assessments: BTreeMap::new(),
        },
        context: GuardrailContext::default(),
    }
}

fn month_bought(r: &BudgetResult, id: &str) -> Option<u16> {
    r.sequence.iter().find(|p| p.item_id == id).map(|p| p.month)
}

/// The research shortcuts do what research §4.2 says: with $60 a month the $100 item costs no more
/// than two months of budget but the $20 item is worth more than a quarter as much per dollar, so
/// the $20 item is bought now. The strict schedule saves for the $100 item instead.
#[test]
fn research_shortcuts_buy_the_best_affordable_item() {
    let c = two_item_case(60.0);
    let research = run(&c, options(Schedule::ResearchShortcuts, false));
    assert_eq!(month_bought(&research, "small"), Some(1));
    assert_eq!(month_bought(&research, "big"), Some(2));
    let strict = run(&c, options(Schedule::FixedOrder, false));
    assert_eq!(month_bought(&strict, "big"), Some(2));
    assert_eq!(month_bought(&strict, "small"), Some(2));
    // The strict schedule's month 1 is a $60 deposit toward the $100 item.
    let m1 = &strict.plan.months[1].items;
    assert_eq!(m1.len(), 1);
    assert_eq!(m1[0].kind, PlanItemKind::Reserve);
    assert_eq!(m1[0].est_cost_usd, 60.0);
    assert_eq!(strict.plan.envelopes.len(), 1);
    assert_eq!(
        (
            strict.plan.envelopes[0].saved_usd,
            strict.plan.envelopes[0].needed_usd
        ),
        (60.0, 100.0)
    );
}

/// Why the strict schedule is the default: under the research shortcuts, raising the budget from
/// $60 to $100 buys the $100 item in month 1 instead of the $20 one, so communications are *less*
/// covered in month 1 with more money. The strict schedule never does that.
#[test]
fn research_shortcuts_can_lower_coverage_when_the_budget_rises() {
    let research_60 = run(
        &two_item_case(60.0),
        options(Schedule::ResearchShortcuts, false),
    );
    let research_100 = run(
        &two_item_case(100.0),
        options(Schedule::ResearchShortcuts, false),
    );
    assert!(
        days_at(&research_100, 1)[&BucketId::Comms] < days_at(&research_60, 1)[&BucketId::Comms]
    );
    let strict_60 = run(&two_item_case(60.0), options(Schedule::FixedOrder, false));
    let strict_100 = run(&two_item_case(100.0), options(Schedule::FixedOrder, false));
    for m in 0..4 {
        for (b, lo) in days_at(&strict_60, m) {
            assert!(days_at(&strict_100, m)[b] >= *lo, "month {m} {b}");
        }
    }
}

/// The research sinking-fund rule: when the best item costs at most two months of budget and the
/// affordable alternative is worth less than a quarter as much per dollar, save instead.
#[test]
fn research_shortcuts_keep_a_sinking_fund_for_a_much_better_item() {
    let mut c = two_item_case(60.0);
    // Make the small item worth a tenth as much per dollar: 10·1·0.006·1 = 0.06 for $20.
    c.risks.curves.insert(
        BucketId::Comms,
        BucketCurve::new(vec![1.0], vec![0.006], 1.0),
    );
    let r = run(&c, options(Schedule::ResearchShortcuts, false));
    assert_eq!(month_bought(&r, "big"), Some(2));
    assert_eq!(r.plan.months[1].items[0].kind, PlanItemKind::Reserve);
    // With $40 a month the big item costs more than two months of budget, so the small one is
    // bought first even though it is worth much less.
    c.household.finances.monthly_budget_usd = 40.0;
    let r = run(&c, options(Schedule::ResearchShortcuts, false));
    assert_eq!(month_bought(&r, "small"), Some(1));
}
