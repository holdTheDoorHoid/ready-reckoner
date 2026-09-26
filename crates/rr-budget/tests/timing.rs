//! Allocator timing, ignored by default. `assess` must stay under 50 ms in the browser for any
//! fixture (docs/ENGINE-API.md), and the allocator shares that budget. Run with:
//!
//!     cargo test --release -p rr-budget --test timing -- --ignored --nocapture

mod common;
use common::generate::case;
use rr_budget::{BudgetInput, BudgetOptions, allocate};
use std::time::Instant;

#[test]
#[ignore]
fn timing() {
    // Philadelphia example.
    let household = rr_types::fixtures::get("philadelphia-renters-4").unwrap();
    let (items, meta) = common::philadelphia::catalogue();
    let risks = common::philadelphia::risks();
    let context = rr_budget::GuardrailContext::default();
    let input = BudgetInput {
        household: &household,
        catalogue: &items,
        meta: &meta,
        requirements: &[],
        risks: &risks,
        context: &context,
        options: BudgetOptions::default(),
    };
    let t = Instant::now();
    for _ in 0..100 {
        let _ = allocate(&input).unwrap();
    }
    println!(
        "philadelphia: {:.3} ms per run",
        t.elapsed().as_secs_f64() * 1000.0 / 100.0
    );
    // Big synthetic catalogues: concatenate generated items up to ~140.
    let mut worst: f64 = 0.0;
    for seed in 0..20u64 {
        let mut c = case(seed);
        let mut k = 1000;
        while c.items.len() < 140 {
            let extra = case(seed * 7919 + k);
            for (mut it, mut m) in extra.items.into_iter().zip(extra.meta) {
                let id = format!("x{k}_{}", it.id);
                it.id = rr_types::ItemId::from(id.as_str());
                m.item_id = it.id.clone();
                c.items.push(it);
                c.meta.push(m);
            }
            k += 1;
        }
        c.household.finances.monthly_budget_usd = 60.0;
        let input = BudgetInput {
            household: &c.household,
            catalogue: &c.items,
            meta: &c.meta,
            requirements: &[],
            risks: &c.risks,
            context: &c.context,
            options: BudgetOptions::default(),
        };
        let t = Instant::now();
        let r = allocate(&input).unwrap();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        worst = worst.max(ms);
        println!(
            "seed {seed}: {} items, {} purchases, {} months, {:.2} ms",
            c.items.len(),
            r.sequence.len(),
            r.plan.months.len(),
            ms
        );
    }
    println!("worst {worst:.2} ms");
}
