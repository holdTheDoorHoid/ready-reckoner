//! `rr catalogue`: every catalogue item by category, with its tier, price band and flags.
//! `--json` prints exactly what the engine's `catalogue()` returns (items filtered by
//! `--category`), so the output can be compared with the web app's mock.

use std::collections::BTreeMap;

use rr_types::{Catalogue, Item};

use crate::Output;
use crate::args::CatalogueArgs;
use crate::error::CliError;
use crate::format::{self, Table};

/// Runs `rr catalogue`. Needs no county data.
///
/// # Errors
///
/// An unknown category (exit 2), listing the real ones.
pub fn run(args: &CatalogueArgs) -> Result<Output, CliError> {
    let content = rr_content::try_content()
        .map_err(|e| CliError::failure(format!("The built-in content could not be read: {e}")))?;
    let mut catalogue: Catalogue = content.catalogue();
    let all = categories(&catalogue.items);
    if let Some(cat) = &args.category {
        if !all.contains_key(cat.as_str()) {
            let names: Vec<&str> = all.keys().copied().collect();
            return Err(CliError::input(format!(
                "There is no category \"{cat}\". The categories are: {}.",
                names.join(", ")
            )));
        }
        catalogue.items.retain(|i| &i.category == cat);
    }
    if args.json {
        return Ok(Output::text(rr_plan::to_json(&catalogue)));
    }
    let shown = categories(&catalogue.items);
    let mut s = format!(
        "Catalogue: {} items in {} {} (content {})\n",
        catalogue.items.len(),
        shown.len(),
        if shown.len() == 1 {
            "category"
        } else {
            "categories"
        },
        rr_content::CONTENT_VERSION
    );
    for (cat, items) in &shown {
        s.push_str(&format!(
            "\n{cat} ({} item{})\n\n",
            items.len(),
            if items.len() == 1 { "" } else { "s" }
        ));
        let mut t = Table::new(["Id", "Name", "Step", "Typical price", "Flags", "Needs"]);
        for it in items {
            t.row([
                it.id.as_str().to_owned(),
                it.name.clone(),
                format::lower_first(it.tier.name()),
                price(it),
                flags(it),
                it.buckets
                    .iter()
                    .map(|b| b.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            ]);
        }
        s.push_str(&t.render(1));
    }
    Ok(Output::text(s))
}

/// Items grouped by category, categories sorted, items in catalogue order.
fn categories(items: &[Item]) -> BTreeMap<&str, Vec<&Item>> {
    let mut m: BTreeMap<&str, Vec<&Item>> = BTreeMap::new();
    for it in items {
        m.entry(it.category.as_str()).or_default().push(it);
    }
    m
}

/// "$20–40 per kit"; "free" for free actions.
fn price(it: &Item) -> String {
    let b = &it.price_band_usd;
    let band = format::band(f64::from(b.low), f64::from(b.high));
    if band == "free" {
        band
    } else {
        format!("{band} per {}", b.per)
    }
}

fn flags(it: &Item) -> String {
    let mut f: Vec<&str> = Vec::new();
    if it.free {
        f.push("free action");
    }
    if it.life_safety {
        f.push("life safety");
    }
    if it.rare_catastrophic {
        f.push("rare catastrophe");
    }
    if it.assumed_basic {
        f.push("assumed basic");
    }
    f.join(", ")
}
