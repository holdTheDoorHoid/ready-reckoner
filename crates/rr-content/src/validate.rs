//! The content validator: the mechanical parts of `docs/CONTENT_STANDARDS.md` §2–§5 and
//! `docs/PRINCIPLES.md` §8–§9.
//!
//! Errors fail `cargo test`; warnings are printed and listed in the content agent's report
//! (today only the reading-level check, spec length and exclamation marks warn).
//!
//! | Rule | Severity |
//! | --- | --- |
//! | ids well formed and unique; guidance id equals its file name; item category equals its file | error |
//! | every item and guidance block cites; every cited id resolves; every citation has a URL and a licence, and any quote is under 50 words | error |
//! | `quantity_rule` is `once` or a row in `docs/QUANTITY_RULES.md` | error |
//! | `look_for` has 2–5 entries (up to 8 for a free action's steps or a kit's contents); `avoid` 2–5 | error |
//! | free items cost $0 and sit in tier `now` (rare-catastrophic items excepted); priced items have a note, a `retrieved` date and cite the price-observation log | error |
//! | `rare_catastrophic` items sit in tier `y1` | error |
//! | no brand names (a citation's URL may name a maker; its title, publisher and quote may not); no pressure phrases; no drug name with a dose anywhere | error |
//! | medical items: no dosing pattern; aquarium, fish or veterinary antibiotics only in `avoid` | error |
//! | firearm and weapon words only in the one permitted free action, which is free, unpriced, in `security`, tier `now` | error |
//! | potassium iodide only alongside "official" instructions | error |
//! | guidance: `kind` matches the id's prefix and at least one `applies_to` target; `applies_to` resolves; footnotes match citations; under 300 words; bucket, hazard and family blocks open with `{frequency}` | error |
//! | guidance: conditional spans close, do not nest and stay in one paragraph; a hazard condition names one of the block's hazards (any hazard in plan, after and topic blocks); `need:`, `benefit:` and `has:` name known needs, benefits and catalogue items | error |
//! | state table: every state, DC and Puerto Rico once; web addresses; a refill rule with sources that resolve; printed lines pass the text checks | error |
//! | guidance reading level above grade 9 (Flesch-Kincaid) | warning |
//! | checklists (contract v3, DESIGN-DELTA-v3 §5.4): id is the file name and starts `check_`; `pages` 1, or 2 for the two named blocks; `applies_to` names only `hazard:` and `event:` ids, each hazard or event in one block; the eight sections in order and no others; at most 6 / 10 / 4 / 3 / 5 / 5 steps, branches, lines and bullets; steps open with bold words (after a span marker); every step, "Do not" and "When it is over" bullet cites (a bullet that only points with `{ref:}` excepted); only §5.4's placeholders and `{ref:}` pages; spans as in guidance, any hazard; at most 330 words (620 for two pages); footnotes and the text rules as in guidance | error |
//! | checklists: a retired hazard in `applies_to`; "Use this when" over two sentences; reading level above grade 8 | warning |
//! | every active hazard and everyday emergency has a checklist: a warning while `content/checklists/` has ten files or fewer, an error after | warning, then error |

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use rr_types::{HazardId, Item, TierId, is_well_formed_id};

use crate::checklist::{
    self, Checklist, MAX_DO_FIRST, MAX_DO_NOT, MAX_LEAVE_OR_STAY, MAX_THEN, MAX_WHEN_OVER,
    MAX_WHERE_WHO, REF_TARGETS, SECTION_HEADINGS, TWO_PAGE_CHECKLISTS,
};
use crate::ids::{self, GuidanceKind, KindRules};
use crate::parse::{Content, LoadError};
use crate::policy::{self, ConditionScope, find_dosing, find_drug_dose, find_phrases, tokens};
use crate::readability;
use crate::tables::{self, STATE_REGISTRIES_FILE};

/// Most words allowed in a guidance block's prose (the Sources section is not counted).
pub const MAX_GUIDANCE_WORDS: usize = 300;
/// Most words allowed in a citation's quote.
pub const MAX_QUOTE_WORDS: usize = 50;
/// Guidance above this Flesch-Kincaid grade gets a warning.
pub const WARN_GRADE_ABOVE: f64 = 9.0;
/// Every priced item cites a citation whose id starts with this (the price-observation log).
pub const PRICE_CITATION_PREFIX: &str = "rr_price_observations";
/// A checklist above this Flesch-Kincaid grade gets a warning (DESIGN-DELTA-v3 §5.4: grade 8 or
/// below).
pub const WARN_CHECKLIST_GRADE_ABOVE: f64 = 8.0;
/// A missing checklist for a hazard or an everyday emergency is a warning while
/// `content/checklists/` holds this many files or fewer, and an error once it holds more: the
/// content workstreams fill the directory in v0.3.0.
pub const CHECKLIST_COVERAGE_ERROR_ABOVE: usize = 10;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Printed and reported; does not fail the build.
    Warning,
    /// Fails `cargo test`.
    Error,
}

/// One problem found in the content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Error or warning.
    pub severity: Severity,
    /// Where: a file and, for items and citations, the id.
    pub location: String,
    /// What is wrong and what to do, in plain words.
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{s}: {}: {}", self.location, self.message)
    }
}

/// Everything the validator found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Findings in the order they were found (content order, which is deterministic).
    pub findings: Vec<Finding>,
}

impl Report {
    /// A report holding one error for content that did not parse.
    pub fn from_load_error(e: &LoadError) -> Report {
        let mut r = Report::default();
        r.error(e.file.clone(), e.message.clone());
        r
    }

    /// The errors.
    pub fn errors(&self) -> impl Iterator<Item = &Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
    }

    /// The warnings.
    pub fn warnings(&self) -> impl Iterator<Item = &Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
    }

    /// True when there are no errors (warnings are allowed).
    pub fn is_ok(&self) -> bool {
        self.errors().next().is_none()
    }

    fn error(&mut self, location: impl Into<String>, message: impl Into<String>) {
        self.findings.push(Finding {
            severity: Severity::Error,
            location: location.into(),
            message: message.into(),
        });
    }

    fn warn(&mut self, location: impl Into<String>, message: impl Into<String>) {
        self.findings.push(Finding {
            severity: Severity::Warning,
            location: location.into(),
            message: message.into(),
        });
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for finding in &self.findings {
            writeln!(f, "{finding}")?;
        }
        Ok(())
    }
}

/// Counts for reports and the about screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// Catalogue items.
    pub items: usize,
    /// Of which free actions.
    pub free_items: usize,
    /// Citations.
    pub citations: usize,
    /// Of which expert estimates (`prior = true`).
    pub prior_citations: usize,
    /// Guidance blocks.
    pub guidance: usize,
    /// Incident checklists (contract v3).
    pub checklists: usize,
    /// Glossary entries.
    pub glossary: usize,
}

/// Counts of everything in `content`.
pub fn summary(content: &Content) -> Summary {
    Summary {
        items: content.items.len(),
        free_items: content.items.iter().filter(|i| i.free).count(),
        citations: content.citations.len(),
        prior_citations: content.citations.iter().filter(|c| c.prior).count(),
        guidance: content.guidance.len(),
        checklists: content.checklists.len(),
        glossary: content.glossary.len(),
    }
}

/// Validates `content`. `rules` is the set of known quantity-rule names
/// ([`crate::rules::parse_rule_table`]).
pub fn validate(content: &Content, rules: &BTreeSet<String>) -> Report {
    let mut r = Report::default();
    check_citations(content, &mut r);
    check_items(content, rules, &mut r);
    check_guidance(content, &mut r);
    check_checklists(content, &mut r);
    check_glossary(content, &mut r);
    check_states(content, &mut r);
    r
}

/// Brand, pressure-phrase, firearm and drug-dose checks shared by every kind of text. For a
/// citation this is the displayed part (title, publisher, quote); its URL may name a maker.
fn check_text(r: &mut Report, loc: &str, text: &str, allow_firearms: bool) {
    let toks = tokens(text);
    for b in find_phrases(&toks, policy::BRANDS) {
        r.error(
            loc,
            format!("brand name `{b}`: use the generic word instead"),
        );
    }
    for p in find_phrases(&toks, policy::BANNED_PHRASES) {
        r.error(loc, format!("`{p}` breaks the calm, no-pressure voice"));
    }
    if !allow_firearms {
        for f in find_phrases(&toks, policy::FIREARM_TOKENS) {
            r.error(
                loc,
                format!(
                    "firearm or weapon word `{f}`: these appear only in the free action `{}`",
                    policy::PERMITTED_FIREARM_ITEM
                ),
            );
        }
    }
    if let Some(d) = find_drug_dose(&toks) {
        r.error(
            loc,
            format!("drug name with a dose (`{d}`): the app never gives doses"),
        );
    }
    if text.contains('!') {
        r.warn(loc, "exclamation mark: keep the voice calm");
    }
}

/// Potassium iodide may be mentioned only together with official instructions
/// (`docs/CONTENT_STANDARDS.md` §5, `docs/PRINCIPLES.md` §9).
fn check_potassium_iodide(r: &mut Report, loc: &str, text: &str) {
    let toks = tokens(text);
    let mentions =
        !find_phrases(&toks, &["potassium iodide"]).is_empty() || toks.iter().any(|t| t == "ki");
    let official = toks.iter().any(|t| t == "official" || t == "officials");
    if mentions && !official {
        r.error(
            loc,
            "potassium iodide is mentioned without saying it is taken only on official instruction",
        );
    }
}

fn who_cites(content: &Content) -> BTreeMap<String, BTreeSet<String>> {
    let mut users: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for item in &content.items {
        for c in &item.citations {
            users
                .entry(c.as_str().to_owned())
                .or_default()
                .insert(format!("item:{}", item.id));
        }
    }
    for g in &content.guidance {
        for c in &g.meta.citations {
            users
                .entry(c.as_str().to_owned())
                .or_default()
                .insert(format!("guidance:{}", g.meta.id));
        }
    }
    for k in &content.checklists {
        for c in &k.meta.citations {
            users
                .entry(c.as_str().to_owned())
                .or_default()
                .insert(format!("checklist:{}", k.meta.id));
        }
    }
    for t in &content.glossary {
        for c in &t.citations {
            users
                .entry(c.as_str().to_owned())
                .or_default()
                .insert(format!("glossary:{}", t.term));
        }
    }
    for row in &content.states.state {
        for line in row.lines() {
            for c in &line.sources {
                users
                    .entry(c.as_str().to_owned())
                    .or_default()
                    .insert(format!("state:{}", row.code));
            }
        }
    }
    users
}

fn check_citations(content: &Content, r: &mut Report) {
    let users = who_cites(content);
    let permitted = format!("item:{}", policy::PERMITTED_FIREARM_ITEM);
    let mut seen = BTreeSet::new();
    for c in &content.citations {
        let loc = format!("citations.toml `{}`", c.id);
        if !seen.insert(c.id.as_str()) {
            r.error(&loc, "duplicate citation id");
        }
        if !is_well_formed_id(c.id.as_str()) {
            r.error(
                &loc,
                "id must be snake_case (lowercase letters, digits, underscores)",
            );
        }
        if c.title.trim().is_empty() {
            r.error(&loc, "missing title");
        }
        if c.publisher.trim().is_empty() {
            r.error(&loc, "missing publisher");
        }
        if c.license.trim().is_empty() {
            r.error(&loc, "missing licence");
        }
        let url_ok = (c.url.starts_with("https://") || c.url.starts_with("http://"))
            && !c.url.contains(char::is_whitespace)
            && c.url.len() > 12;
        if !url_ok {
            r.error(&loc, format!("url `{}` is not a web address", c.url));
        }
        if let Some(y) = c.year
            && !(1800..=2100).contains(&y)
        {
            r.error(&loc, format!("year {y} is implausible"));
        }
        if let Some(q) = &c.quote {
            let n = q.split_whitespace().count();
            if n == 0 {
                r.error(&loc, "empty quote: remove the field instead");
            } else if n > MAX_QUOTE_WORDS {
                r.error(
                    &loc,
                    format!("quote has {n} words; the limit is {MAX_QUOTE_WORDS}"),
                );
            }
        }
        let text = format!(
            "{}\n{}\n{}",
            c.title,
            c.publisher,
            c.quote.as_deref().unwrap_or("")
        );
        let only_permitted = users
            .get(c.id.as_str())
            .is_some_and(|u| !u.is_empty() && u.iter().all(|x| *x == permitted));
        check_text(r, &loc, &text, only_permitted);
    }
}

fn item_text(item: &Item) -> String {
    let mut parts = vec![item.name.as_str(), item.spec.as_str()];
    parts.extend(item.look_for.iter().map(String::as_str));
    parts.extend(item.avoid.iter().map(String::as_str));
    if let Some(n) = &item.price_band_usd.note {
        parts.push(n);
    }
    parts.join("\n")
}

fn check_items(content: &Content, rules: &BTreeSet<String>, r: &mut Report) {
    let mut seen = BTreeSet::new();
    for (item, file) in content.items.iter().zip(&content.item_files) {
        let loc = format!("{file} item `{}`", item.id);
        let id = item.id.as_str();
        if !seen.insert(id) {
            r.error(&loc, "duplicate item id");
        }
        if !is_well_formed_id(id) {
            r.error(&loc, "id must be snake_case");
        }
        let stem = file
            .strip_prefix("items/")
            .and_then(|f| f.strip_suffix(".toml"))
            .unwrap_or(file);
        if !policy::CATEGORIES.contains(&item.category.as_str()) {
            r.error(
                &loc,
                format!(
                    "unknown category `{}` (see policy::CATEGORIES)",
                    item.category
                ),
            );
        }
        if item.category != stem {
            r.error(
                &loc,
                format!("category `{}` must match its file `{file}`", item.category),
            );
        }
        if item.name.trim().is_empty() {
            r.error(&loc, "missing name");
        } else if item.name.chars().count() > 80 {
            r.warn(&loc, "name is longer than 80 characters");
        }
        if item.unit.trim().is_empty() {
            r.error(&loc, "missing unit");
        }
        if item.buckets.is_empty() {
            r.error(&loc, "an item must help with at least one bucket");
        }
        if item.buckets.iter().collect::<BTreeSet<_>>().len() != item.buckets.len() {
            r.error(&loc, "a bucket is listed twice");
        }
        if item.hazard_extras.iter().collect::<BTreeSet<_>>().len() != item.hazard_extras.len() {
            r.error(&loc, "a hazard extra is listed twice");
        }
        if item.spec.trim().is_empty() {
            r.error(&loc, "missing spec");
        } else if readability::sentence_count(&item.spec) > 3 {
            r.warn(
                &loc,
                "spec is longer than three sentences (the standard is one or two)",
            );
        }
        for (field, list, max) in [
            ("look_for", &item.look_for, max_look_for(item)),
            ("avoid", &item.avoid, 5),
        ] {
            if !(2..=max).contains(&list.len()) {
                r.error(
                    &loc,
                    format!("`{field}` needs 2–{max} entries (has {})", list.len()),
                );
            }
            if list.iter().any(|s| s.trim().is_empty()) {
                r.error(&loc, format!("`{field}` has an empty entry"));
            }
        }
        check_item_citations(content, item, &loc, r);
        check_price(item, &loc, r);
        check_tier(item, &loc, r);
        if !rules.contains(&item.quantity_rule) {
            r.error(
                &loc,
                format!(
                    "quantity_rule `{}` is not `once` and not in docs/QUANTITY_RULES.md: add a row marked \"requested by content\"",
                    item.quantity_rule
                ),
            );
        }
        if let Some(m) = &item.maintenance {
            if m.rotate_months.is_none() && m.check_months.is_none() {
                r.error(&loc, "maintenance is empty: remove it or give an interval");
            }
            for (name, v) in [
                ("rotate_months", m.rotate_months),
                ("check_months", m.check_months),
            ] {
                if let Some(v) = v
                    && !(1..=240).contains(&v)
                {
                    r.error(&loc, format!("{name} = {v} is outside 1–240"));
                }
            }
        }
        for (name, v) in [
            ("energy_kcal_per_unit", item.energy_kcal_per_unit),
            ("volume_l_per_unit", item.volume_l_per_unit),
        ] {
            if let Some(v) = v
                && !(v.is_finite() && v > 0.0)
            {
                r.error(&loc, format!("{name} must be a positive number"));
            }
        }

        let permitted = id == policy::PERMITTED_FIREARM_ITEM;
        let text = item_text(item);
        check_text(r, &loc, &text, permitted);
        check_potassium_iodide(r, &loc, &text);
        if item.category == "medical" {
            check_medical(item, &loc, r);
        }
        if permitted {
            check_permitted_firearm_item(item, &loc, r);
        }
    }
}

fn check_item_citations(content: &Content, item: &Item, loc: &str, r: &mut Report) {
    if item.citations.is_empty() {
        r.error(loc, "every item cites at least one source");
    }
    let mut seen = BTreeSet::new();
    for c in &item.citations {
        if content.citation(c.as_str()).is_none() {
            r.error(loc, format!("citation `{c}` is not in citations.toml"));
        }
        if !seen.insert(c.as_str()) {
            r.warn(loc, format!("citation `{c}` is listed twice"));
        }
    }
}

fn check_price(item: &Item, loc: &str, r: &mut Report) {
    let band = &item.price_band_usd;
    if !(band.low.is_finite() && band.high.is_finite()) || band.low < 0.0 || band.high < band.low {
        r.error(
            loc,
            format!(
                "price band {}–{} must satisfy 0 ≤ low ≤ high",
                band.low, band.high
            ),
        );
    }
    if band.per.trim().is_empty() {
        r.error(loc, "price band needs `per` (what the price is for)");
    }
    if item.free {
        if band.low != 0.0 || band.high != 0.0 {
            r.error(loc, "a free action must have a $0 price band");
        }
    } else if is_money_reserve(item) {
        // Money set aside (cash): one dollar buys one dollar, so there is nothing to observe.
        if band.note.as_deref().is_none_or(|n| n.trim().is_empty()) {
            r.error(
                loc,
                "a money reserve needs a note saying how the amount is chosen",
            );
        }
    } else {
        if band.high <= 0.0 {
            r.error(
                loc,
                "a purchase needs a price band above $0 (or mark it free)",
            );
        }
        if band.note.as_deref().is_none_or(|n| n.trim().is_empty()) {
            r.error(loc, "a price band needs a note saying what it reflects");
        }
        if item.retrieved.is_none() {
            r.error(
                loc,
                "a priced item needs the `retrieved` date of its price observations",
            );
        }
        if !item
            .citations
            .iter()
            .any(|c| c.as_str().starts_with(PRICE_CITATION_PREFIX))
        {
            r.error(
                loc,
                format!(
                    "a priced item cites the price-observation log (`{PRICE_CITATION_PREFIX}_…`)"
                ),
            );
        }
    }
}

/// Most `look_for` entries an item may have: 5 checks for a purchase, but up to 8 for a free
/// action (its steps) or a kit you assemble (its contents), `docs/CONTENT_STANDARDS.md` §3.
pub fn max_look_for(item: &Item) -> usize {
    if item.free || matches!(item.unit.as_str(), "bag" | "kit") {
        8
    } else {
        5
    }
}

/// An item that is money set aside rather than bought: unit `dollar`, priced at exactly $1 per
/// dollar. It needs no price observation (`docs/CONTENT_STANDARDS.md` §3).
pub fn is_money_reserve(item: &Item) -> bool {
    item.unit == "dollar"
        && item.price_band_usd.per == "dollar"
        && item.price_band_usd.low == 1.0
        && item.price_band_usd.high == 1.0
}

fn check_tier(item: &Item, loc: &str, r: &mut Report) {
    if item.free && item.tier != TierId::Now && !item.rare_catastrophic {
        r.error(loc, "free actions belong in tier `now`");
    }
    if !item.free && item.tier == TierId::Now {
        r.error(loc, "tier `now` is for free actions only");
    }
    if item.rare_catastrophic && item.tier != TierId::Y1 {
        r.error(
            loc,
            "rare-catastrophic items belong in tier `y1` (budget-capped, off by default)",
        );
    }
}

fn check_medical(item: &Item, loc: &str, r: &mut Report) {
    let mut fields: Vec<(&str, &str)> = vec![("spec", item.spec.as_str())];
    fields.extend(item.look_for.iter().map(|s| ("look_for", s.as_str())));
    for (field, text) in &fields {
        let toks = tokens(text);
        if let Some(d) = find_dosing(&toks) {
            r.error(
                loc,
                format!("dosing pattern `{d}` in `{field}`: medical items never give doses"),
            );
        }
        for s in find_phrases(&toks, policy::UNSAFE_ANTIBIOTIC_SOURCES) {
            r.error(
                loc,
                format!("`{s}` in `{field}`: aquarium, fish or veterinary antibiotics may appear only in `avoid`, as a warning"),
            );
        }
    }
    for text in &item.avoid {
        if let Some(d) = find_dosing(&tokens(text)) {
            r.error(
                loc,
                format!("dosing pattern `{d}` in `avoid`: medical items never give doses"),
            );
        }
    }
}

fn check_permitted_firearm_item(item: &Item, loc: &str, r: &mut Report) {
    if !item.free || item.price_band_usd.high != 0.0 {
        r.error(
            loc,
            "the firearms item is a free action with no price, never a purchase",
        );
    }
    if item.category != "security" {
        r.error(loc, "the firearms item belongs in category `security`");
    }
    if item.tier != TierId::Now {
        r.error(loc, "the firearms item sits in tier `now` (a free action)");
    }
    if item.quantity_rule != "once" {
        r.error(loc, "the firearms item uses quantity_rule `once`");
    }
}

/// Resolves one `applies_to` entry. `slugs` holds, for the topic, plan and after kinds, the
/// slugs their blocks define (`topic_renters` defines `topic:renters`).
fn resolve_target(
    target: &str,
    slugs: &BTreeMap<GuidanceKind, BTreeSet<String>>,
) -> Result<(), String> {
    let (kind, id) = target
        .split_once(':')
        .ok_or_else(|| format!("`{target}` should be kind:id"))?;
    let kind = kind.parse::<GuidanceKind>().map_err(|_| {
        format!(
            "`{kind}` is not a target kind (use bucket, hazard, tier, topic, plan, after or family)"
        )
    })?;
    match kind {
        GuidanceKind::Bucket if ids::is_bucket(id) => Ok(()),
        GuidanceKind::Bucket => Err(format!("`{id}` is not a bucket id")),
        GuidanceKind::Hazard if ids::is_hazard(id) => Ok(()),
        GuidanceKind::Hazard => Err(format!("`{id}` is not a hazard id")),
        GuidanceKind::Tier => TierId::from_str(id).map(|_| ()).map_err(|e| e.to_string()),
        GuidanceKind::Family if ids::is_rare_family(id) => Ok(()),
        GuidanceKind::Family => Err(format!(
            "`{id}` is not the lead hazard of a rare family (one of {:?})",
            ids::rare_families()
        )),
        GuidanceKind::Checklist => Err(format!(
            "`{target}`: a checklist is not a target; checklists apply to `hazard:` and `event:` \
             targets themselves"
        )),
        GuidanceKind::Topic | GuidanceKind::Plan | GuidanceKind::After => {
            if slugs.get(&kind).is_some_and(|s| s.contains(id)) {
                Ok(())
            } else {
                Err(format!(
                    "no guidance block `{}{id}` defines {kind} `{id}`",
                    kind.prefix()
                ))
            }
        }
    }
}

fn check_guidance(content: &Content, r: &mut Report) {
    let mut slugs: BTreeMap<GuidanceKind, BTreeSet<String>> = BTreeMap::new();
    for g in &content.guidance {
        if let Some(slug) = g.meta.id.strip_prefix(&g.kind().prefix()) {
            slugs.entry(g.kind()).or_default().insert(slug.to_owned());
        }
    }
    let items: BTreeSet<String> = content
        .items
        .iter()
        .map(|i| i.id.as_str().to_owned())
        .collect();
    let mut seen = BTreeSet::new();
    for g in &content.guidance {
        let loc = g.file.clone();
        let id = g.meta.id.as_str();
        if !seen.insert(id) {
            r.error(&loc, "duplicate guidance id");
        }
        if !is_well_formed_id(id) {
            r.error(&loc, "id must be snake_case");
        }
        let stem = g
            .file
            .strip_prefix("guidance/")
            .and_then(|f| f.strip_suffix(".md"))
            .unwrap_or(&g.file);
        if id != stem {
            r.error(&loc, format!("id `{id}` must equal the file name `{stem}`"));
        }
        let kind = g.kind();
        if kind == GuidanceKind::Checklist {
            r.error(
                &loc,
                "checklists live in content/checklists/, not guidance/ (DESIGN-DELTA-v3 §5.4)",
            );
        }
        let prefix = kind.prefix();
        if !id.starts_with(&prefix) {
            r.error(
                &loc,
                format!("a `{kind}` block's id starts with `{prefix}`"),
            );
        }
        let own_kind = format!("{kind}:");
        if !g.meta.applies_to.iter().any(|t| t.starts_with(&own_kind)) {
            r.error(
                &loc,
                format!("a `{kind}` block applies to at least one `{own_kind}` target"),
            );
        }
        if g.meta.title.trim().is_empty() {
            r.error(&loc, "missing title");
        }
        if g.meta.applies_to.is_empty() {
            r.error(&loc, "applies_to is empty");
        }
        for t in &g.meta.applies_to {
            if let Err(e) = resolve_target(t, &slugs) {
                r.error(&loc, format!("applies_to: {e}"));
            }
        }
        if g.meta.citations.is_empty() {
            r.error(&loc, "a guidance block cites at least one source");
        }
        let cited: BTreeSet<&str> = g.meta.citations.iter().map(|c| c.as_str()).collect();
        if cited.len() != g.meta.citations.len() {
            r.warn(&loc, "a citation is listed twice in the front matter");
        }
        for c in &cited {
            if content.citation(c).is_none() {
                r.error(&loc, format!("citation `{c}` is not in citations.toml"));
            }
        }
        check_footnotes(g, &cited, &loc, r);
        let hazards: Vec<&str> = g
            .meta
            .applies_to
            .iter()
            .filter_map(|t| {
                t.strip_prefix("hazard:")
                    .or_else(|| t.strip_prefix("family:"))
            })
            .collect();
        let scope = ConditionScope {
            hazards: if kind.any_hazard_condition() {
                None
            } else {
                Some(&hazards)
            },
            items: Some(&items),
        };
        for p in policy::condition_problems_in(&g.body, scope) {
            r.error(&loc, p);
        }

        let prose = g.prose();
        let plain = readability::plain_text(prose);
        let words = readability::words(&plain).len();
        if words > MAX_GUIDANCE_WORDS {
            r.error(
                &loc,
                format!("{words} words; the limit is {MAX_GUIDANCE_WORDS}"),
            );
        }
        if kind.opens_with_frequency()
            && !first_paragraph(prose).contains(policy::FREQUENCY_PLACEHOLDER)
        {
            r.error(
                &loc,
                "bucket, hazard and family blocks open with the household's own `{frequency}` sentence",
            );
        }
        if let Some(grade) = readability::flesch_kincaid_grade(&plain)
            && grade > WARN_GRADE_ABOVE
        {
            r.warn(
                &loc,
                format!("reading level is grade {grade:.1} (Flesch-Kincaid); aim for 8"),
            );
        }
        // Footnote references (`[^id]`) are ids, not text a reader sees: the packet turns them
        // into numbered markers. Only the words are checked.
        let all = format!("{}\n{}", g.meta.title, strip_footnote_references(prose));
        check_text(r, &loc, &all, false);
        check_potassium_iodide(r, &loc, &all);
        check_antibiotic_warnings(&plain, &loc, r);
    }
}

/// A web address a household can type: `https://`, no spaces.
fn is_web_address(url: &str) -> bool {
    url.starts_with("https://") && !url.contains(char::is_whitespace) && url.len() > 12
}

/// The state table: every jurisdiction once, web addresses, a sourced refill rule, and printed
/// lines that pass the same text checks as guidance.
fn check_states(content: &Content, r: &mut Report) {
    let rows = &content.states.state;
    if rows.is_empty() {
        return;
    }
    let mut seen = BTreeSet::new();
    for row in rows {
        let loc = format!("{STATE_REGISTRIES_FILE} `{}`", row.code);
        if !tables::JURISDICTIONS.contains(&row.code.as_str()) {
            r.error(
                &loc,
                "code is not a state, DC or PR two-letter postal code in capitals",
            );
        }
        if !seen.insert(row.code.as_str()) {
            r.error(&loc, "the state appears twice");
        }
        for (field, v) in [
            ("name", &row.name),
            ("em_agency", &row.em_agency),
            ("refill", &row.refill),
        ] {
            if v.trim().is_empty() {
                r.error(&loc, format!("missing `{field}`"));
            }
        }
        for url in row.urls() {
            if !is_web_address(url) {
                r.error(&loc, format!("`{url}` is not an https web address"));
            }
        }
        for e in [&row.zone, &row.registry, &row.alerts]
            .into_iter()
            .flatten()
        {
            if e.name.trim().is_empty() {
                r.error(&loc, "a listed tool needs its name");
            }
        }
        if row.refill_sources.is_empty() {
            r.error(&loc, "the refill rule cites at least one source");
        }
        for line in row.lines() {
            for c in &line.sources {
                if content.citation(c.as_str()).is_none() {
                    r.error(&loc, format!("citation `{c}` is not in citations.toml"));
                }
            }
            // No reading-level check here: official names and web addresses dominate these
            // short lines, and the fixed wording around them lives in `tables.rs`.
            check_text(r, &loc, &line.text, false);
        }
    }
    for code in tables::JURISDICTIONS {
        if !seen.contains(code) {
            r.error(STATE_REGISTRIES_FILE, format!("no row for `{code}`"));
        }
    }
}

/// `text` without its footnote references (`[^id]`).
fn strip_footnote_references(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("[^") {
        out.push_str(&rest[..start]);
        match rest[start..].find(']') {
            Some(end) => rest = &rest[start + end + 1..],
            None => {
                rest = &rest[start..];
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

fn first_paragraph(prose: &str) -> &str {
    let t = prose.trim_start();
    match t.find("\n\n") {
        Some(i) => &t[..i],
        None => t,
    }
}

fn check_footnotes(g: &crate::Guidance, cited: &BTreeSet<&str>, loc: &str, r: &mut Report) {
    check_footnotes_in(&g.body, g.prose(), cited, loc, r);
}

/// Footnotes and front-matter citations agree: every `[^id]` in `prose` is cited and defined under
/// `## Sources`, every cited id is defined, and (a warning) used.
fn check_footnotes_in(body: &str, prose: &str, cited: &BTreeSet<&str>, loc: &str, r: &mut Report) {
    let refs = crate::parse::footnote_references_in(prose);
    let defs: BTreeSet<String> = crate::parse::footnote_definitions_in(body)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    if !body
        .lines()
        .any(|l| l.trim().eq_ignore_ascii_case("## sources"))
    {
        r.error(loc, "missing the closing `## Sources` section");
    }
    for id in &refs {
        if !cited.contains(id.as_str()) {
            r.error(
                loc,
                format!("footnote `[^{id}]` is not in the front-matter citations"),
            );
        }
        if !defs.contains(id) {
            r.error(
                loc,
                format!("footnote `[^{id}]` has no definition under ## Sources"),
            );
        }
    }
    for c in cited {
        if !defs.contains(*c) {
            r.error(
                loc,
                format!("citation `{c}` has no footnote definition under ## Sources"),
            );
        }
        if !refs.iter().any(|x| x == c) {
            r.warn(
                loc,
                format!("citation `{c}` is never referenced in the text"),
            );
        }
    }
}

/// A step or bullet shortened for a message.
fn short(item: &str) -> String {
    let words: Vec<&str> = item.split_whitespace().collect();
    if words.len() <= 8 {
        words.join(" ")
    } else {
        format!("{} …", words[..8].join(" "))
    }
}

/// The checklists (DESIGN-DELTA-v3 §5.4): each file's form, caps, citations, placeholders and
/// words, the text rules every block follows, and then whether every active hazard and everyday
/// emergency has one.
fn check_checklists(content: &Content, r: &mut Report) {
    let items: BTreeSet<String> = content
        .items
        .iter()
        .map(|i| i.id.as_str().to_owned())
        .collect();
    let mut seen = BTreeSet::new();
    let mut owner: BTreeMap<String, String> = BTreeMap::new();
    for c in &content.checklists {
        check_checklist(content, c, &items, &mut seen, &mut owner, r);
    }

    let required: Vec<String> = HazardId::ACTIVE
        .iter()
        .map(|h| format!("hazard:{}", h.as_str()))
        .chain(ids::EVENTS.iter().map(|e| format!("event:{e}")))
        .collect();
    let missing: Vec<&str> = required
        .iter()
        .filter(|t| !owner.contains_key(t.as_str()))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        let message = format!(
            "{} of the {} active hazards and everyday emergencies have no checklist yet \
             (DESIGN-DELTA-v3 §5.1): {}",
            missing.len(),
            required.len(),
            missing.join(", ")
        );
        if content.checklists.len() > CHECKLIST_COVERAGE_ERROR_ABOVE {
            r.error("checklists/", message);
        } else {
            r.warn("checklists/", message);
        }
    }
}

fn check_checklist(
    content: &Content,
    c: &Checklist,
    items: &BTreeSet<String>,
    seen: &mut BTreeSet<String>,
    owner: &mut BTreeMap<String, String>,
    r: &mut Report,
) {
    let loc = c.file.clone();
    let id = c.meta.id.as_str();
    if !seen.insert(id.to_owned()) {
        r.error(&loc, "duplicate checklist id");
    }
    if !is_well_formed_id(id) {
        r.error(&loc, "id must be snake_case");
    }
    let stem = c
        .file
        .strip_prefix("checklists/")
        .and_then(|f| f.strip_suffix(".md"))
        .unwrap_or(&c.file);
    if id != stem {
        r.error(&loc, format!("id `{id}` must equal the file name `{stem}`"));
    }
    if !id.starts_with(ids::CHECKLIST_PREFIX) {
        r.error(&loc, "a checklist's id starts with `check_`");
    }
    if c.meta.title.trim().is_empty() {
        r.error(&loc, "missing title");
    }
    match c.pages {
        1 => {}
        2 if TWO_PAGE_CHECKLISTS.contains(&id) => {}
        n => r.error(
            &loc,
            format!(
                "`pages: {n}`: a checklist takes one page; only {TWO_PAGE_CHECKLISTS:?} may take two"
            ),
        ),
    }

    // What it applies to: hazards and everyday emergencies, each in one block.
    if c.meta.applies_to.is_empty() {
        r.error(&loc, "applies_to is empty");
    }
    let mut mine = BTreeSet::new();
    for t in &c.meta.applies_to {
        let known = match t.split_once(':') {
            Some(("hazard", h)) if ids::is_hazard(h) => {
                if h.parse::<HazardId>().is_ok_and(HazardId::is_retired) {
                    r.warn(
                        &loc,
                        format!(
                            "`{t}` is retired: no household's binder selects it; apply the page \
                             to the hazards that replaced it"
                        ),
                    );
                }
                true
            }
            Some(("hazard", h)) => {
                r.error(&loc, format!("applies_to: `{h}` is not a hazard id"));
                false
            }
            Some(("event", e)) if ids::is_event(e) => true,
            Some(("event", e)) => {
                r.error(
                    &loc,
                    format!(
                        "applies_to: `{e}` is not an everyday emergency (one of {:?})",
                        ids::EVENTS
                    ),
                );
                false
            }
            _ => {
                r.error(
                    &loc,
                    format!("applies_to: `{t}` is not `hazard:<id>` or `event:<id>`"),
                );
                false
            }
        };
        if !known {
            continue;
        }
        if !mine.insert(t.as_str()) {
            r.warn(&loc, format!("applies_to lists `{t}` twice"));
            continue;
        }
        if let Some(other) = owner.insert(t.clone(), id.to_owned()) {
            r.error(
                &loc,
                format!(
                    "`{t}` already has the checklist `{other}`; a hazard or event has one page"
                ),
            );
        }
    }

    // Citations and footnotes.
    if c.meta.citations.is_empty() {
        r.error(&loc, "a checklist cites at least one source");
    }
    let cited: BTreeSet<&str> = c.meta.citations.iter().map(|x| x.as_str()).collect();
    if cited.len() != c.meta.citations.len() {
        r.warn(&loc, "a citation is listed twice in the front matter");
    }
    for x in &cited {
        if content.citation(x).is_none() {
            r.error(&loc, format!("citation `{x}` is not in citations.toml"));
        }
    }
    check_footnotes_in(&c.body, c.prose(), &cited, &loc, r);

    // The eight sections, in order, and nothing before them.
    if !c.preamble.trim().is_empty() {
        r.error(
            &loc,
            "text before `## Use this when`: everything goes in a section",
        );
    }
    let headings: Vec<&str> = c.raw.iter().map(|s| s.heading.as_str()).collect();
    if headings != SECTION_HEADINGS {
        for h in &headings {
            if !SECTION_HEADINGS.contains(h) {
                r.error(&loc, format!("unknown section `## {h}`"));
            }
        }
        let mut first_seen: Vec<&str> = Vec::new();
        for want in SECTION_HEADINGS {
            match headings.iter().filter(|h| **h == want).count() {
                0 => r.error(&loc, format!("missing section `## {want}`")),
                1 => {}
                n => r.error(&loc, format!("`## {want}` appears {n} times")),
            }
        }
        for h in &headings {
            if SECTION_HEADINGS.contains(h) && !first_seen.contains(h) {
                first_seen.push(h);
            }
        }
        let in_order: Vec<&str> = SECTION_HEADINGS
            .iter()
            .copied()
            .filter(|h| first_seen.contains(h))
            .collect();
        if first_seen != in_order {
            r.error(
                &loc,
                format!(
                    "the sections are out of order: they go {}",
                    SECTION_HEADINGS.join(", ")
                ),
            );
        }
    }

    // "Use this when": one or two sentences.
    let use_when = &c.sections.use_when;
    if use_when.trim().is_empty() {
        r.error(&loc, "`Use this when` is empty: say when to use the page");
    } else if crate::readability::sentence_count(&strip_footnote_references(use_when)) > 2 {
        r.warn(&loc, "`Use this when` is longer than two sentences");
    }

    // The lists: kind, caps, bold leads, citations.
    let text_of = |h: &str| {
        c.raw
            .iter()
            .find(|s| s.heading == h)
            .map(|s| s.text.as_str())
            .unwrap_or("")
    };
    // (section, numbered, most, bold lead, must cite)
    let lists: [(&str, bool, usize, bool, bool); 6] = [
        ("Do first", true, MAX_DO_FIRST, true, true),
        ("Then", true, MAX_THEN, true, true),
        ("Leave or stay", false, MAX_LEAVE_OR_STAY, false, false),
        ("Where and who", false, MAX_WHERE_WHO, false, false),
        ("Do not", false, MAX_DO_NOT, false, true),
        ("When it is over", false, MAX_WHEN_OVER, false, true),
    ];
    for (section, numbered, most, bold, cite) in lists {
        let scan = checklist::scan_list(text_of(section), numbered, section);
        for p in scan.problems {
            r.error(&loc, p);
        }
        let what = if numbered { "steps" } else { "entries" };
        if scan.items.len() > most {
            r.error(
                &loc,
                format!(
                    "`{section}` has {} {what}; the most is {most}",
                    scan.items.len()
                ),
            );
        }
        if section == "Do first" && scan.items.is_empty() {
            r.error(&loc, "`Do first` needs at least one step");
        }
        for item in &scan.items {
            let opening = checklist::without_leading_condition(item);
            if bold && !(opening.starts_with("**") && opening[2..].contains("**")) {
                r.error(
                    &loc,
                    format!(
                        "`{section}` step `{}` opens with bold lead words (`**Get out.** …`)",
                        short(item)
                    ),
                );
            }
            let pointer = section == "When it is over" && item.contains(checklist::REF_OPEN);
            if cite && !pointer && !item.contains("[^") {
                r.error(
                    &loc,
                    format!(
                        "`{section}`: `{}` tells the reader what to do and cites no source",
                        short(item)
                    ),
                );
            }
        }
    }

    // Placeholders and cross-references (DESIGN-DELTA-v3 §5.4) only.
    let prose = c.prose();
    for inner in checklist::braces(prose) {
        let inner = inner.trim();
        if inner.starts_with("if:") || inner == "/if" {
            continue;
        }
        if let Some(page) = inner.strip_prefix("ref:") {
            if !REF_TARGETS.contains(&page) {
                r.error(
                    &loc,
                    format!("`{{ref:{page}}}` names no page (one of {REF_TARGETS:?})"),
                );
            }
        } else if checklist::placeholder_width(inner).is_none() {
            let known: Vec<&str> = checklist::PLACEHOLDERS.iter().map(|(n, _)| *n).collect();
            r.error(
                &loc,
                format!("`{{{inner}}}` is not a checklist placeholder (one of {known:?})"),
            );
        }
    }
    // Conditional spans: as in guidance, and a checklist may name any hazard.
    let scope = ConditionScope {
        hazards: None,
        items: Some(items),
    };
    for p in policy::condition_problems_in(prose, scope) {
        r.error(&loc, p);
    }

    // Words and reading level.
    let words = c.word_count();
    let budget = c.word_budget();
    if words > budget {
        r.error(&loc, format!("{words} words; the limit is {budget}"));
    }
    if let Some(grade) = c.reading_grade()
        && grade > WARN_CHECKLIST_GRADE_ABOVE
    {
        r.warn(
            &loc,
            format!("reading level is grade {grade:.1} (Flesch-Kincaid); aim for 8 or below"),
        );
    }

    // The text rules every block follows: no brands, no pressure, no firearms, no drug doses.
    let all = format!("{}\n{}", c.meta.title, strip_footnote_references(prose));
    check_text(r, &loc, &all, false);
    check_potassium_iodide(r, &loc, &all);
    check_antibiotic_warnings(&crate::readability::plain_text(prose), &loc, r);
}

/// In guidance, unsafe antibiotic sources may be named only in a sentence that says not to use
/// them.
fn check_antibiotic_warnings(plain: &str, loc: &str, r: &mut Report) {
    for sentence in plain.split(['.', '!', '?', '\n']) {
        let toks = tokens(sentence);
        let named = find_phrases(&toks, policy::UNSAFE_ANTIBIOTIC_SOURCES);
        if named.is_empty() {
            continue;
        }
        if find_phrases(&toks, policy::NEGATIONS).is_empty() {
            r.error(
                loc,
                format!(
                    "`{}` is named without a warning in the same sentence",
                    named.join("`, `")
                ),
            );
        }
    }
}

fn check_glossary(content: &Content, r: &mut Report) {
    let mut seen = BTreeSet::new();
    for t in &content.glossary {
        let loc = format!("glossary.toml `{}`", t.term);
        if !seen.insert(t.term.to_lowercase()) {
            r.error(&loc, "term defined twice");
        }
        for (field, v) in [
            ("plain", &t.plain),
            ("term", &t.term),
            ("definition", &t.definition),
        ] {
            if v.trim().is_empty() {
                r.error(&loc, format!("missing `{field}`"));
            }
        }
        if t.plain.trim().eq_ignore_ascii_case(t.term.trim()) {
            r.warn(
                &loc,
                "the plain phrase repeats the term; give the everyday words",
            );
        }
        for c in &t.citations {
            if content.citation(c.as_str()).is_none() {
                r.error(&loc, format!("citation `{c}` is not in citations.toml"));
            }
        }
        let text = format!("{}\n{}\n{}", t.plain, t.term, t.definition);
        check_text(r, &loc, &text, false);
        check_potassium_iodide(r, &loc, &text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::parse_rule_table;

    const CITES: &str = r#"
[[citation]]
id = "ready_gov_water"
title = "Water"
publisher = "FEMA / Ready.gov"
year = 2021
url = "https://www.ready.gov/water"
retrieved = "2026-09-25"
license = "US Government Work (public domain)"

[[citation]]
id = "rr_price_observations_2026_09"
title = "Price observations"
publisher = "Ready Reckoner contributors"
year = 2026
url = "https://github.com/holdTheDoorHoid/ready-reckoner/blob/main/docs/PRICE_OBSERVATIONS.md"
retrieved = "2026-09-25"
license = "CC BY-SA 4.0"
"#;

    fn item(id: &str, category: &str, extra: &str) -> String {
        format!(
            r#"
[[item]]
id = "{id}"
name = "Stored drinking water"
category = "{category}"
unit = "gallon"
buckets = ["water_out"]
tier = "h72"
free = false
spec = "Water in clean food-grade containers."
look_for = ["Food-grade plastic", "Tight lids"]
avoid = ["Milk jugs", "Containers that held chemicals"]
price_band_usd = {{ low = 0.5, high = 1.5, per = "gallon", note = "two retail listings" }}
retrieved = "2026-09-25"
quantity_rule = "water_gallons"
citations = ["ready_gov_water", "rr_price_observations_2026_09"]
hazard_extras = []
{extra}
"#
        )
    }

    fn run(files: &[(&str, &str)]) -> Report {
        let c = Content::from_files(files).expect("parses");
        let rules = parse_rule_table("| `water_gallons` | x |\n");
        validate(&c, &rules)
    }

    fn errors(r: &Report) -> Vec<String> {
        r.errors().map(|f| f.message.clone()).collect()
    }

    #[test]
    fn a_clean_item_passes() {
        let body = item("water_stored", "water", "");
        let r = run(&[("citations.toml", CITES), ("items/water.toml", &body)]);
        assert!(r.is_ok(), "{r}");
    }

    #[test]
    fn a_brand_name_is_rejected() {
        let body = item("water_stored", "water", "").replace(
            "Water in clean food-grade containers.",
            "A LifeStraw filter and water.",
        );
        let r = run(&[("citations.toml", CITES), ("items/water.toml", &body)]);
        assert!(errors(&r).iter().any(|e| e.contains("lifestraw")), "{r}");
    }

    #[test]
    fn a_citation_url_may_name_a_maker_but_its_title_may_not() {
        // A maker's specification sheet can be the source of a number. Its URL names the maker;
        // the title and publisher shown in the packet's source list stay neutral.
        let entry = |title: &str| {
            format!(
                "{CITES}\n[[citation]]\nid = \"maker_spec\"\ntitle = \"{title}\"\n\
                 publisher = \"Manufacturer specification sheet, archived\"\n\
                 url = \"https://web.archive.org/web/2026/https://powerequipment.honda.com/eu2200i\"\n\
                 retrieved = \"2026-09-25\"\nlicense = \"All rights reserved (paraphrased, not quoted)\"\n"
            )
        };
        let body = item("water_stored", "water", "");
        let neutral =
            entry("Portable inverter generator (2.2 kW class): manufacturer specification");
        let r = run(&[("citations.toml", &neutral), ("items/water.toml", &body)]);
        assert!(r.is_ok(), "{r}");
        let branded = entry("Honda EU2200i specifications");
        let r = run(&[("citations.toml", &branded), ("items/water.toml", &body)]);
        assert!(errors(&r).iter().any(|e| e.contains("honda")), "{r}");
    }

    #[test]
    fn unknown_rule_and_missing_citation_are_rejected() {
        let body = item("water_stored", "water", "")
            .replace("\"water_gallons\"", "\"magic_rule\"")
            .replace("\"ready_gov_water\", ", "\"nowhere\", ");
        let r = run(&[("citations.toml", CITES), ("items/water.toml", &body)]);
        let e = errors(&r);
        assert!(e.iter().any(|m| m.contains("magic_rule")), "{r}");
        assert!(e.iter().any(|m| m.contains("nowhere")), "{r}");
    }

    #[test]
    fn category_must_match_file() {
        let body = item("water_stored", "food", "");
        let r = run(&[("citations.toml", CITES), ("items/water.toml", &body)]);
        assert!(
            errors(&r).iter().any(|e| e.contains("must match its file")),
            "{r}"
        );
    }

    #[test]
    fn dosing_in_a_medical_item_is_rejected_but_warnings_in_avoid_are_allowed() {
        let body = item("med_x", "medical", "")
            .replace(
                "Water in clean food-grade containers.",
                "Pain reliever, 500 mg.",
            )
            .replace("\"Milk jugs\"", "\"Aquarium or veterinary antibiotics\"");
        let r = run(&[("citations.toml", CITES), ("items/medical.toml", &body)]);
        let e = errors(&r);
        assert!(e.iter().any(|m| m.contains("dosing pattern")), "{r}");
        assert!(!e.iter().any(|m| m.contains("aquarium")), "{r}");
        let bad = item("med_y", "medical", "").replace(
            "\"Food-grade plastic\"",
            "\"Fish antibiotics from a pet store\"",
        );
        let r = run(&[("citations.toml", CITES), ("items/medical.toml", &bad)]);
        assert!(
            errors(&r).iter().any(|m| m.contains("fish antibiotics")),
            "{r}"
        );
    }

    #[test]
    fn firearm_words_only_in_the_permitted_free_action() {
        let other = item("security_x", "security", "").replace(
            "Water in clean food-grade containers.",
            "Keep a firearm by the door.",
        );
        let r = run(&[("citations.toml", CITES), ("items/security.toml", &other)]);
        assert!(errors(&r).iter().any(|m| m.contains("firearm")), "{r}");

        let priced = item(policy::PERMITTED_FIREARM_ITEM, "security", "").replace(
            "Water in clean food-grade containers.",
            "If you own firearms, lock them.",
        );
        let r = run(&[("citations.toml", CITES), ("items/security.toml", &priced)]);
        assert!(
            errors(&r)
                .iter()
                .any(|m| m.contains("free action with no price")),
            "{r}"
        );
    }

    #[test]
    fn free_actions_cost_nothing_and_sit_in_tier_now() {
        let body = item("water_x", "water", "").replace("free = false", "free = true");
        let r = run(&[("citations.toml", CITES), ("items/water.toml", &body)]);
        let e = errors(&r);
        assert!(e.iter().any(|m| m.contains("$0 price band")), "{r}");
        assert!(e.iter().any(|m| m.contains("tier `now`")), "{r}");
    }

    #[test]
    fn priced_items_need_note_date_and_price_source() {
        let body = item("water_x", "water", "")
            .replace(", note = \"two retail listings\"", "")
            .replace("retrieved = \"2026-09-25\"\n", "")
            .replace(", \"rr_price_observations_2026_09\"", "");
        let r = run(&[("citations.toml", CITES), ("items/water.toml", &body)]);
        let e = errors(&r);
        assert!(e.iter().any(|m| m.contains("note")), "{r}");
        assert!(e.iter().any(|m| m.contains("retrieved")), "{r}");
        assert!(e.iter().any(|m| m.contains("price-observation log")), "{r}");
    }

    #[test]
    fn potassium_iodide_needs_official_instruction() {
        let body = item("rare_x", "water", "").replace(
            "Water in clean food-grade containers.",
            "Keep potassium iodide at home.",
        );
        let r = run(&[("citations.toml", CITES), ("items/water.toml", &body)]);
        assert!(
            errors(&r).iter().any(|m| m.contains("potassium iodide")),
            "{r}"
        );
    }

    fn guide(id: &str, applies: &str, body: &str) -> String {
        let kind = id.split('_').next().unwrap_or(id);
        format!(
            "---\nid: {id}\ntitle: A title\nkind: {kind}\napplies_to: [{applies}]\ncitations: [ready_gov_water]\n---\n{body}\n\n## Sources\n\n[^ready_gov_water]: FEMA, Water (2021).\n"
        )
    }

    #[test]
    fn guidance_rules() {
        let ok = guide(
            "bucket_water_out",
            "bucket:water_out",
            "{frequency} Store one gallon per person per day.[^ready_gov_water]",
        );
        let r = run(&[
            ("citations.toml", CITES),
            ("guidance/bucket_water_out.md", &ok),
        ]);
        assert!(r.is_ok(), "{r}");

        let no_freq = guide(
            "bucket_water_out",
            "bucket:water_out",
            "Store water.[^ready_gov_water]",
        );
        let r = run(&[
            ("citations.toml", CITES),
            ("guidance/bucket_water_out.md", &no_freq),
        ]);
        assert!(errors(&r).iter().any(|m| m.contains("{frequency}")), "{r}");

        let bad_target = guide(
            "topic_x",
            "bucket:weather, topic:nothing",
            "Text.[^ready_gov_water]",
        );
        let r = run(&[
            ("citations.toml", CITES),
            ("guidance/topic_x.md", &bad_target),
        ]);
        let e = errors(&r);
        assert!(e.iter().any(|m| m.contains("weather")), "{r}");
        assert!(e.iter().any(|m| m.contains("topic_nothing")), "{r}");

        let long = guide(
            "topic_x",
            "topic:x",
            &format!("{}[^ready_gov_water]", "Word ".repeat(301)),
        );
        let r = run(&[("citations.toml", CITES), ("guidance/topic_x.md", &long)]);
        assert!(errors(&r).iter().any(|m| m.contains("limit is 300")), "{r}");

        let unwarned = guide(
            "topic_x",
            "topic:x",
            "Fish antibiotics are cheap.[^ready_gov_water]",
        );
        let r = run(&[
            ("citations.toml", CITES),
            ("guidance/topic_x.md", &unwarned),
        ]);
        assert!(
            errors(&r).iter().any(|m| m.contains("without a warning")),
            "{r}"
        );

        let warned = guide(
            "topic_x",
            "topic:x",
            "Never use fish antibiotics.[^ready_gov_water]",
        );
        let r = run(&[("citations.toml", CITES), ("guidance/topic_x.md", &warned)]);
        assert!(r.is_ok(), "{r}");
    }

    #[test]
    fn kinds_match_ids_and_targets() {
        let ok = guide(
            "plan_shelter",
            "plan:shelter",
            "Pick a spot.[^ready_gov_water]",
        );
        let r = run(&[("citations.toml", CITES), ("guidance/plan_shelter.md", &ok)]);
        assert!(r.is_ok(), "{r}");

        // The id says hazard, the kind says topic.
        let mismatch = guide("hazard_x", "topic:x", "{frequency} Text.[^ready_gov_water]")
            .replace("kind: hazard", "kind: topic");
        let r = run(&[
            ("citations.toml", CITES),
            ("guidance/hazard_x.md", &mismatch),
        ]);
        let e = errors(&r);
        assert!(e.iter().any(|m| m.contains("starts with `topic_`")), "{r}");

        // A plan block that applies to no plan target.
        let stray = guide(
            "plan_shelter",
            "hazard:tornado",
            "Pick a spot.[^ready_gov_water]",
        );
        let r = run(&[
            ("citations.toml", CITES),
            ("guidance/plan_shelter.md", &stray),
        ]);
        assert!(
            errors(&r)
                .iter()
                .any(|m| m.contains("at least one `plan:` target")),
            "{r}"
        );

        // After and plan targets resolve only to blocks that exist.
        let orphan = guide(
            "after_first_30_days",
            "after:first_30_days, plan:nowhere",
            "Return safely.[^ready_gov_water]",
        );
        let r = run(&[
            ("citations.toml", CITES),
            ("guidance/after_first_30_days.md", &orphan),
        ]);
        assert!(errors(&r).iter().any(|m| m.contains("plan_nowhere")), "{r}");
    }

    #[test]
    fn contract_v2_ids_and_family_targets_resolve() {
        let smoke = guide(
            "hazard_wildfire_smoke",
            "hazard:wildfire_smoke",
            "{frequency} Smoke travels far.[^ready_gov_water]",
        );
        let air = guide(
            "bucket_clean_air",
            "bucket:clean_air",
            "{frequency} Clean air.[^ready_gov_water]",
        );
        let family = guide(
            "family_solar_storm",
            "family:geomagnetic_storm",
            "{frequency} Rare.[^ready_gov_water]",
        );
        let r = run(&[
            ("citations.toml", CITES),
            ("guidance/hazard_wildfire_smoke.md", &smoke),
            ("guidance/bucket_clean_air.md", &air),
            ("guidance/family_solar_storm.md", &family),
        ]);
        assert!(r.is_ok(), "{r}");

        let not_rare = guide(
            "family_x",
            "family:tornado",
            "{frequency} Rare.[^ready_gov_water]",
        );
        let r = run(&[
            ("citations.toml", CITES),
            ("guidance/family_x.md", &not_rare),
        ]);
        assert!(
            errors(&r)
                .iter()
                .any(|m| m.contains("lead hazard of a rare family")),
            "{r}"
        );
        let no_frequency = guide("family_x", "family:cbrn_attack", "Rare.[^ready_gov_water]");
        let r = run(&[
            ("citations.toml", CITES),
            ("guidance/family_x.md", &no_frequency),
        ]);
        assert!(errors(&r).iter().any(|m| m.contains("{frequency}")), "{r}");
    }

    #[test]
    fn conditions_follow_the_block_kind() {
        let body = "Pick a spot. {if:tornado}Use the basement.{/if} \
                    {if:has:water_stored}Keep water there.{/if}[^ready_gov_water]";
        let item = item("water_stored", "water", "");
        // A plan block may name any hazard, and a catalogue item.
        let plan = guide("plan_shelter", "plan:shelter", body);
        let r = run(&[
            ("citations.toml", CITES),
            ("items/water.toml", &item),
            ("guidance/plan_shelter.md", &plan),
        ]);
        assert!(r.is_ok(), "{r}");
        // A hazard block may name only its own hazards.
        let hazard = guide(
            "hazard_x",
            "hazard:earthquake",
            &format!("{{frequency}} {body}"),
        );
        let r = run(&[
            ("citations.toml", CITES),
            ("items/water.toml", &item),
            ("guidance/hazard_x.md", &hazard),
        ]);
        assert!(
            errors(&r).iter().any(|m| m.contains("does not apply to")),
            "{r}"
        );
        // An item that is not in the catalogue, and an unknown need.
        let bad = guide(
            "plan_shelter",
            "plan:shelter",
            "A {if:has:jetpack}b{/if} {if:need:telepathy}c{/if}.[^ready_gov_water]",
        );
        let r = run(&[
            ("citations.toml", CITES),
            ("items/water.toml", &item),
            ("guidance/plan_shelter.md", &bad),
        ]);
        let e = errors(&r);
        assert!(e.iter().any(|m| m.contains("jetpack")), "{r}");
        assert!(e.iter().any(|m| m.contains("telepathy")), "{r}");
    }

    /// A clean checklist (DESIGN-DELTA-v3 §5.4) for the tests below; `edit` changes its text.
    fn checklist(id: &str, applies: &str) -> String {
        format!(
            "---\nid: {id}\ntitle: A title\nkind: checklist\nonset: now\napplies_to: [{applies}]\n\
             citations: [ready_gov_water]\n---\n\
             ## Use this when\n\nThe water stops.\n\n\
             ## Do first\n\n1. **Stop.** Turn off the tap.[^ready_gov_water]\n\
             2. {{if:pets}}**Water the pets.** Use stored water.[^ready_gov_water]{{/if}}\n\n\
             ## Then\n\n1. **Listen.** Tune to {{alerts}}.[^ready_gov_water]\n\n\
             ## Leave or stay\n\n- **Leave if** told to. Go to {{where_go}}. {{ref:getting_out}}\n\n\
             ## Where and who\n\n- Meeting place: {{meeting_near}}\n\n\
             ## Do not\n\n- Do not drink from the tap.[^ready_gov_water]\n\n\
             ## When it is over\n\n- Run the tap for a minute.[^ready_gov_water]\n\
             - Use the After pages. {{ref:after}}\n\n\
             ## Sources\n\n[^ready_gov_water]: FEMA, Water (2021).\n"
        )
    }

    fn run_checklist(text: &str) -> Report {
        run(&[
            ("citations.toml", CITES),
            ("checklists/check_water.md", text),
        ])
    }

    fn has(r: &Report, needle: &str) -> bool {
        errors(r).iter().any(|m| m.contains(needle))
    }

    #[test]
    fn a_clean_checklist_passes() {
        let text = checklist(
            "check_water",
            "event:boil_water_notice, hazard:local_utility_outage",
        );
        let r = run_checklist(&text);
        assert!(r.is_ok(), "{r}");
        // With two files, missing coverage is only a warning.
        assert!(
            r.warnings()
                .any(|w| w.message.contains("have no checklist yet")),
            "{r}"
        );
    }

    #[test]
    fn checklist_sections_must_be_the_eight_in_order() {
        let base = checklist("check_water", "event:boil_water_notice");
        let missing = base.replace(
            "## Then\n\n1. **Listen.** Tune to {alerts}.[^ready_gov_water]\n\n",
            "",
        );
        let r = run_checklist(&missing);
        assert!(has(&r, "missing section `## Then`"), "{r}");
        let swapped = base
            .replace("## Do not", "## Do not_")
            .replace("## When it is over", "## Do not")
            .replace("## Do not_", "## When it is over");
        let r = run_checklist(&swapped);
        assert!(has(&r, "out of order"), "{r}");
        let extra = base.replace("## Sources", "## Why\n\nBecause.\n\n## Sources");
        let r = run_checklist(&extra);
        assert!(has(&r, "unknown section `## Why`"), "{r}");
        let early = base.replace("---\n## Use this when", "---\nIntro.\n\n## Use this when");
        let r = run_checklist(&early);
        assert!(has(&r, "text before `## Use this when`"), "{r}");
    }

    #[test]
    fn checklist_caps_bold_leads_and_citations() {
        let base = checklist("check_water", "event:boil_water_notice");
        let steps: String = (3..=7)
            .map(|n| format!("{n}. **Step {n}.** Do it.[^ready_gov_water]\n"))
            .collect();
        let seven = base.replace("{/if}\n\n## Then", &format!("{{/if}}\n{steps}\n## Then"));
        let r = run_checklist(&seven);
        assert!(has(&r, "`Do first` has 7 steps; the most is 6"), "{r}");
        let six = base.replace(
            "{/if}\n\n## Then",
            &format!("{{/if}}\n{}\n## Then", &steps[..steps.find("7.").unwrap()]),
        );
        assert!(run_checklist(&six).is_ok(), "six steps are fine");

        let uncited = base.replace("Turn off the tap.[^ready_gov_water]", "Turn off the tap.");
        let r = run_checklist(&uncited);
        assert!(has(&r, "cites no source"), "{r}");
        let uncited_bullet = base.replace(
            "Do not drink from the tap.[^ready_gov_water]",
            "Do not drink from the tap.",
        );
        assert!(has(&run_checklist(&uncited_bullet), "cites no source"));
        // A pointer to the After pages needs no source; a leave-or-stay branch needs none either.
        assert!(run_checklist(&base).is_ok());

        let plain = base.replace("1. **Stop.** Turn off", "1. Stop and turn off");
        assert!(has(&run_checklist(&plain), "opens with bold lead words"));
        // A span marker may come before the bold lead (the pets step above).

        let bullets_in_steps = base.replace("1. **Listen.**", "- **Listen.**");
        assert!(has(&run_checklist(&bullets_in_steps), "is the other kind"));
        let loose = base.replace("## Do not\n\n", "## Do not\n\nSome words.\n\n");
        assert!(has(&run_checklist(&loose), "text outside its bullets"));
        let five_branches: String = (0..5)
            .map(|n| format!("- **Stay if** case {n}.\n"))
            .collect();
        let many = base.replace(
            "- **Leave if** told to.",
            &format!("{five_branches}- **Leave if** told to."),
        );
        assert!(has(
            &run_checklist(&many),
            "`Leave or stay` has 6 entries; the most is 4"
        ));
        let empty_first = base.replace(
            "1. **Stop.** Turn off the tap.[^ready_gov_water]\n2. {if:pets}**Water the pets.** Use stored water.[^ready_gov_water]{/if}\n",
            "",
        );
        assert!(has(
            &run_checklist(&empty_first),
            "`Do first` needs at least one step"
        ));
    }

    #[test]
    fn checklist_words_stop_at_the_budget() {
        let base = checklist("check_water", "event:boil_water_notice");
        let with_trigger = |n: usize| {
            let words = vec!["word"; n].join(" ");
            base.replace("The water stops.", &format!("{words}."))
        };
        let c = Content::from_files(&[("checklists/check_water.md", &with_trigger(1))]).unwrap();
        let rest = c.checklists[0].word_count() - 1;
        let r = run_checklist(&with_trigger(331 - rest));
        assert!(has(&r, "331 words; the limit is 330"), "{r}");
        let r = run_checklist(&with_trigger(330 - rest));
        assert!(!has(&r, "words; the limit"), "{r}");
        // Two pages only for the two named blocks, with the larger budget.
        let two = base.replace("onset: now\n", "onset: now\npages: 2\n");
        assert!(has(
            &run_checklist(&two),
            "only [\"check_hurricane\", \"check_nuclear_attack\"] may take two"
        ));
        let hurricane = checklist("check_hurricane", "hazard:hurricane")
            .replace("onset: now\n", "onset: coming\npages: 2\n")
            .replace(
                "The water stops.",
                &format!("{}.", vec!["word"; 400].join(" ")),
            );
        let r = run(&[
            ("citations.toml", CITES),
            ("checklists/check_hurricane.md", &hurricane),
        ]);
        assert!(!has(&r, "words; the limit"), "{r}");
    }

    #[test]
    fn checklist_placeholders_references_and_spans() {
        let base = checklist("check_water", "event:boil_water_notice");
        let unknown = base.replace("{meeting_near}", "{frequency}");
        assert!(has(
            &run_checklist(&unknown),
            "`{frequency}` is not a checklist placeholder"
        ));
        let colour = base.replace("{meeting_near}", "{favourite_colour}");
        assert!(has(
            &run_checklist(&colour),
            "is not a checklist placeholder"
        ));
        let bad_ref = base.replace("{ref:after}", "{ref:kitchen}");
        assert!(has(
            &run_checklist(&bad_ref),
            "`{ref:kitchen}` names no page"
        ));
        let nested = base.replace(
            "{if:pets}**Water the pets.** Use stored water.",
            "{if:pets}**Water the pets.** {if:children}Use stored water.{/if}",
        );
        assert!(
            has(&run_checklist(&nested), "spans do not nest"),
            "{}",
            run_checklist(&nested)
        );
        // Every household condition, a home and a hazard condition are fine in a checklist.
        let spans = base.replace(
            "Turn off the tap.",
            "Turn off the tap. {if:vehicle}Or the car's water.{/if} {if:children}Watch the kids.{/if} \
             {if:powered_device}Charge devices.{/if} {if:home:mobile_home}Go out.{/if} {if:tornado}Go down.{/if}",
        );
        let r = run_checklist(&spans);
        assert!(r.is_ok(), "{r}");
        let unknown_need = base.replace("{if:pets}", "{if:need:telepathy}");
        assert!(has(&run_checklist(&unknown_need), "telepathy"));
    }

    #[test]
    fn checklist_targets_are_hazards_and_events_once_each() {
        let r = run_checklist(&checklist("check_water", "bucket:water_out"));
        assert!(has(&r, "is not `hazard:<id>` or `event:<id>`"), "{r}");
        let r = run_checklist(&checklist("check_water", "event:asteroid"));
        assert!(has(&r, "`asteroid` is not an everyday emergency"), "{r}");
        let r = run_checklist(&checklist("check_water", "hazard:zombies"));
        assert!(has(&r, "`zombies` is not a hazard id"), "{r}");
        let r = run_checklist(&checklist("check_water", "hazard:terrorism"));
        assert!(r.is_ok(), "{r}");
        assert!(r.warnings().any(|w| w.message.contains("retired")), "{r}");
        let r = run(&[
            ("citations.toml", CITES),
            (
                "checklists/check_water.md",
                &checklist("check_water", "event:power_outage"),
            ),
            (
                "checklists/check_power.md",
                &checklist("check_power", "event:power_outage"),
            ),
        ]);
        assert!(
            has(
                &r,
                "`event:power_outage` already has the checklist `check_power`"
            ),
            "{r}"
        );
        let r = run_checklist(&checklist("check_other", "event:power_outage"));
        assert!(has(&r, "must equal the file name"), "{r}");
        let r = run(&[
            ("citations.toml", CITES),
            (
                "checklists/water.md",
                &checklist("water", "event:power_outage"),
            ),
        ]);
        assert!(has(&r, "starts with `check_`"), "{r}");
        // A guidance block may not claim to be a checklist.
        let g = guide("topic_x", "topic:x", "Text.[^ready_gov_water]")
            .replace("kind: topic", "kind: checklist");
        let r = run(&[("citations.toml", CITES), ("guidance/topic_x.md", &g)]);
        assert!(has(&r, "checklists live in content/checklists/"), "{r}");
    }

    #[test]
    fn missing_coverage_turns_into_an_error_past_ten_files() {
        let hazards = [
            "tornado",
            "hail",
            "lightning",
            "earthquake",
            "tsunami",
            "wildfire",
            "drought",
            "heat_wave",
            "cold_wave",
            "hurricane",
            "avalanche",
        ];
        let files: Vec<(String, String)> = hazards
            .iter()
            .map(|h| {
                let id = format!("check_{h}");
                (
                    format!("checklists/{id}.md"),
                    checklist(&id, &format!("hazard:{h}")),
                )
            })
            .collect();
        let mut all: Vec<(&str, &str)> = vec![("citations.toml", CITES)];
        all.extend(files.iter().map(|(p, t)| (p.as_str(), t.as_str())));
        let r = run(&all);
        assert!(has(&r, "have no checklist yet"), "{r}");
        assert!(has(&r, "event:power_outage"), "{r}");
        assert!(!has(&r, "hazard:tornado,"), "tornado is covered: {r}");
        // Ten files: still a warning.
        let r = run(&all[..11]);
        assert!(!has(&r, "have no checklist yet"), "{r}");
        assert!(
            r.warnings()
                .any(|w| w.message.contains("have no checklist yet"))
        );
    }

    const STATE: &str = "[[state]]\ncode = \"KS\"\nname = \"Kansas\"\n\
        checked = \"2026-09-26\"\nem_agency = \"Kansas Division of Emergency Management\"\n\
        em_url = \"https://www.kansastag.gov/kdem\"\n\
        refill = \"A pharmacist may give an emergency supply.\"\n\
        refill_sources = [\"ready_gov_water\"]\n";

    fn with_alert_sources() -> String {
        format!(
            "{CITES}\n[[citation]]\nid = \"ready_gov_disability\"\ntitle = \"People with Disabilities\"\n\
             publisher = \"FEMA / Ready.gov\"\nurl = \"https://www.ready.gov/disability\"\n\
             retrieved = \"2026-09-26\"\nlicense = \"US Government Work (public domain)\"\n\
             \n[[citation]]\nid = \"ready_gov_evacuation\"\ntitle = \"Evacuation\"\n\
             publisher = \"FEMA / Ready.gov\"\nurl = \"https://www.ready.gov/evacuation\"\n\
             retrieved = \"2026-09-26\"\nlicense = \"US Government Work (public domain)\"\n\
             \n[[citation]]\nid = \"ready_gov_alerts\"\ntitle = \"Emergency Alerts\"\n\
             publisher = \"FEMA / Ready.gov\"\nurl = \"https://www.ready.gov/alerts\"\n\
             retrieved = \"2026-09-26\"\nlicense = \"US Government Work (public domain)\"\n"
        )
    }

    #[test]
    fn the_state_table_is_checked() {
        let cites = with_alert_sources();
        let r = run(&[("citations.toml", &cites), (STATE_REGISTRIES_FILE, STATE)]);
        let e = errors(&r);
        // One row is not the whole table.
        assert!(e.iter().any(|m| m.contains("no row for `AL`")), "{r}");
        assert!(!e.iter().any(|m| m.contains("`KS`")), "{r}");

        let bad = format!(
            "{}{}",
            STATE
                .replace("https://www.kansastag.gov/kdem", "www.kansastag.gov")
                .replace("[\"ready_gov_water\"]", "[\"nowhere\"]"),
            STATE.replace("A pharmacist", "Act now: a pharmacist")
        );
        let r = run(&[("citations.toml", &cites), (STATE_REGISTRIES_FILE, &bad)]);
        let e = errors(&r);
        assert!(
            e.iter().any(|m| m.contains("not an https web address")),
            "{r}"
        );
        assert!(e.iter().any(|m| m.contains("`nowhere`")), "{r}");
        assert!(e.iter().any(|m| m.contains("appears twice")), "{r}");
        assert!(e.iter().any(|m| m.contains("act now")), "{r}");
    }

    #[test]
    fn footnote_ids_are_not_checked_as_words() {
        // A source's id may contain a word the text may not (the talk cited for the free steps
        // has one in its id); the reader never sees the id.
        let cites = CITES.replace("ready_gov_water", "talk_about_guns");
        let body = "---\nid: topic_x\ntitle: A title\nkind: topic\napplies_to: [topic:x]\n\
                    citations: [talk_about_guns]\n---\nKeep a spare key with a friend.[^talk_about_guns]\n\n\
                    ## Sources\n\n[^talk_about_guns]: FEMA / Ready.gov, Water (2021).\n";
        let r = run(&[("citations.toml", &cites), ("guidance/topic_x.md", body)]);
        assert!(r.is_ok(), "{r}");
        assert_eq!(strip_footnote_references("a[^b] c[^d]"), "a c");
        assert_eq!(strip_footnote_references("a [^b"), "a [^b");
    }

    #[test]
    fn dense_guidance_gets_a_reading_level_warning_not_an_error() {
        let dense = guide(
            "topic_x",
            "topic:x",
            "Comprehensive organizational preparedness necessitates considerable administrative coordination between municipal authorities and residential communities.[^ready_gov_water]",
        );
        let r = run(&[("citations.toml", CITES), ("guidance/topic_x.md", &dense)]);
        assert!(r.is_ok(), "{r}");
        assert!(
            r.warnings().any(|w| w.message.contains("reading level")),
            "{r}"
        );
    }

    #[test]
    fn long_quotes_and_bad_urls_are_rejected() {
        let bad = CITES
            .replace("https://www.ready.gov/water", "www.ready.gov/water")
            .replace(
                "license = \"US Government Work (public domain)\"",
                &format!(
                    "license = \"US Government Work (public domain)\"\nquote = \"{}\"",
                    "word ".repeat(51).trim()
                ),
            );
        let r = run(&[("citations.toml", &bad)]);
        let e = errors(&r);
        assert!(e.iter().any(|m| m.contains("not a web address")), "{r}");
        assert!(e.iter().any(|m| m.contains("51 words")), "{r}");
    }
}
