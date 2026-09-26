//! The word lists behind the content policy (`docs/PRINCIPLES.md` §8–§9,
//! `docs/CONTENT_STANDARDS.md` §3–§5) and small token matchers that use them.
//!
//! Matching works on lowercase alphanumeric tokens, so punctuation and hyphens never hide a word:
//! "Band-Aid", "band aid" and "BAND AID" all become the tokens `band aid`. No regular-expression
//! engine is needed, which keeps the WebAssembly build small.

/// The placeholder the engine replaces with the household's own natural-frequency sentence.
pub const FREQUENCY_PLACEHOLDER: &str = "{frequency}";

/// Opens a conditional span of a guidance block. Six kinds of condition (see [`Condition`]):
///
/// - a hazard: `{if:avalanche}In avalanche country, get training ...{/if}`. The packet keeps the
///   span when that hazard is likely enough for the household (a ten-year chance of at least 1 in
///   100) and drops it otherwise. In a hazard, family, bucket or tier block the hazard must be one
///   the block applies to; plan, after-disaster and topic blocks may name any hazard.
/// - the kind of home: `{if:home:apartment_high_rise}…{/if}` keeps the span only for those homes,
///   `{if:not_home:apartment_low_rise|apartment_high_rise}…{/if}` only for the others. Kinds are
///   `HousingKind` strings, several joined with `|`.
/// - an access or functional need: `{if:need:hearing|vision}…{/if}` keeps the span when anyone in
///   the household has one of these needs ([`crate::ids::ACCESS_NEEDS`]).
/// - an item: `{if:has:power_generator}…{/if}` keeps the span when the household has one of
///   these catalogue items: it already owns it, or the plan includes it.
/// - a benefit: `{if:benefit:snap_wic}…{/if}` keeps the span when the household relies on one of
///   these benefits ([`crate::ids::BENEFITS`]).
///
/// Every household-free view keeps every span. Spans do not nest and stay within one paragraph.
pub const CONDITION_OPEN: &str = "{if:";

/// Closes a conditional span (see [`CONDITION_OPEN`]).
pub const CONDITION_CLOSE: &str = "{/if}";

/// Prefix of a condition on the kind of home: kept for these kinds of home.
pub const HOME_PREFIX: &str = "home:";

/// Prefix of a condition on the kind of home: kept for every other kind of home.
pub const NOT_HOME_PREFIX: &str = "not_home:";

/// Prefix of a condition on the household's access and functional needs.
pub const NEED_PREFIX: &str = "need:";

/// Prefix of a condition on a catalogue item the household has.
pub const HAS_PREFIX: &str = "has:";

/// Prefix of a condition on a benefit the household relies on.
pub const BENEFIT_PREFIX: &str = "benefit:";

/// What a conditional span depends on (the text between `{if:` and `}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// A hazard id (`avalanche`): kept when that hazard is likely enough for the household.
    Hazard(String),
    /// `home:<kind>|<kind>…`: kept when the home is one of these kinds.
    Home(Vec<rr_types::HousingKind>),
    /// `not_home:<kind>|<kind>…`: kept when the home is none of these kinds.
    NotHome(Vec<rr_types::HousingKind>),
    /// `need:<need>|<need>…`: kept when anyone in the household has one of these access or
    /// functional needs.
    Need(Vec<String>),
    /// `has:<item id>|<item id>…`: kept when the household has one of these catalogue items.
    Has(Vec<String>),
    /// `benefit:<id>|<id>…`: kept when the household relies on one of these benefits.
    Benefit(Vec<String>),
}

/// What a household looks like to the conditions in a guidance block. The packet and the web
/// implement it from the plan input and outputs, and [`Condition::holds`] and
/// [`apply_conditions_for`] use it, so every renderer applies the same rules.
pub trait HouseholdFacts {
    /// Whether this hazard is likely enough for the household to keep advice about it (the
    /// packet's cut is a ten-year chance of at least 1 in 100). Unknown ids should return true.
    fn hazard_relevant(&self, hazard: &str) -> bool;
    /// The household's kind of home.
    fn home(&self) -> rr_types::HousingKind;
    /// Whether anyone in the household has this access or functional need
    /// ([`crate::ids::ACCESS_NEEDS`]).
    fn has_access_need(&self, need: &str) -> bool;
    /// Whether the household has this catalogue item: it already owns it, or the plan includes
    /// it.
    fn has_item(&self, item: &str) -> bool;
    /// Whether the household relies on this benefit ([`crate::ids::BENEFITS`]).
    fn has_benefit(&self, benefit: &str) -> bool;
}

/// Splits `a|b|c` into its parts, rejecting an empty or repeated part.
fn split_values<'a>(id: &str, values: &'a str) -> Result<Vec<&'a str>, String> {
    let mut out: Vec<&str> = Vec::new();
    for v in values.split('|').map(str::trim) {
        if v.is_empty() {
            return Err(format!("`{{if:{id}}}` has an empty value"));
        }
        if out.contains(&v) {
            return Err(format!("`{{if:{id}}}` lists `{v}` twice"));
        }
        out.push(v);
    }
    Ok(out)
}

/// The values of a `need:` or `benefit:` condition, each one of `allowed`.
fn listed_values(
    id: &str,
    values: &str,
    allowed: &[&str],
    what: &str,
) -> Result<Vec<String>, String> {
    let parts = split_values(id, values)?;
    for v in &parts {
        if !allowed.contains(v) {
            return Err(format!(
                "`{{if:{id}}}` names `{v}`, which is not {what} (one of {allowed:?})"
            ));
        }
    }
    Ok(parts.into_iter().map(str::to_owned).collect())
}

impl Condition {
    /// Reads a condition. A `home:` or `not_home:` condition needs at least one known kind of
    /// home and no unknown one; `need:` and `benefit:` need known needs and benefits; `has:`
    /// needs well-formed item ids (the validator checks them against the catalogue). Anything
    /// else is taken as a hazard id (the validator checks it).
    ///
    /// # Errors
    ///
    /// A plain-language problem: an unknown kind of home, need or benefit, a malformed item id,
    /// an empty or repeated value.
    pub fn parse(id: &str) -> Result<Condition, String> {
        let id = id.trim();
        if let Some(v) = id.strip_prefix(NEED_PREFIX) {
            return listed_values(id, v, crate::ids::ACCESS_NEEDS, "an access need")
                .map(Condition::Need);
        }
        if let Some(v) = id.strip_prefix(BENEFIT_PREFIX) {
            return listed_values(id, v, crate::ids::BENEFITS, "a benefit").map(Condition::Benefit);
        }
        if let Some(v) = id.strip_prefix(HAS_PREFIX) {
            let parts = split_values(id, v)?;
            for p in &parts {
                if !rr_types::is_well_formed_id(p) {
                    return Err(format!(
                        "`{{if:{id}}}` names `{p}`, which is not an item id"
                    ));
                }
            }
            return Ok(Condition::Has(
                parts.into_iter().map(str::to_owned).collect(),
            ));
        }
        let (negated, kinds) = match (
            id.strip_prefix(HOME_PREFIX),
            id.strip_prefix(NOT_HOME_PREFIX),
        ) {
            (Some(k), _) => (false, k),
            (None, Some(k)) => (true, k),
            (None, None) => return Ok(Condition::Hazard(id.to_owned())),
        };
        let mut out: Vec<rr_types::HousingKind> = Vec::new();
        for k in kinds.split('|').map(str::trim) {
            match k.parse::<rr_types::HousingKind>() {
                Ok(kind) if !out.contains(&kind) => out.push(kind),
                Ok(_) => return Err(format!("`{{if:{id}}}` lists `{k}` twice")),
                Err(_) => {
                    return Err(format!(
                        "`{{if:{id}}}` names `{k}`, which is not a kind of home (one of {:?})",
                        rr_types::HousingKind::STRS
                    ));
                }
            }
        }
        Ok(if negated {
            Condition::NotHome(out)
        } else {
            Condition::Home(out)
        })
    }

    /// Whether a span with this condition belongs in a packet for this kind of home: `Some` for
    /// the home conditions, `None` for the others (which depend on more than the home).
    pub fn for_home(&self, kind: rr_types::HousingKind) -> Option<bool> {
        match self {
            Condition::Home(kinds) => Some(kinds.contains(&kind)),
            Condition::NotHome(kinds) => Some(!kinds.contains(&kind)),
            Condition::Hazard(_)
            | Condition::Need(_)
            | Condition::Has(_)
            | Condition::Benefit(_) => None,
        }
    }

    /// Whether a span with this condition belongs in this household's text.
    pub fn holds(&self, h: &impl HouseholdFacts) -> bool {
        match self {
            Condition::Hazard(id) => h.hazard_relevant(id),
            Condition::Home(kinds) => kinds.contains(&h.home()),
            Condition::NotHome(kinds) => !kinds.contains(&h.home()),
            Condition::Need(needs) => needs.iter().any(|n| h.has_access_need(n)),
            Condition::Has(items) => items.iter().any(|i| h.has_item(i)),
            Condition::Benefit(benefits) => benefits.iter().any(|b| h.has_benefit(b)),
        }
    }
}

/// `text` with the conditional spans whose hazard id `keep` rejects removed and the markers of
/// the others dropped. A span removed from between two words takes one of the two spaces around
/// it; one at the start of a line takes the space after it. Malformed markers are left as they
/// are (the validator reports them).
pub fn apply_conditions(text: &str, keep: impl Fn(&str) -> bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(CONDITION_OPEN) {
        let after_open = &rest[start + CONDITION_OPEN.len()..];
        let (Some(id_end), Some(close)) = (after_open.find('}'), after_open.find(CONDITION_CLOSE))
        else {
            break;
        };
        if close < id_end {
            break;
        }
        let id = after_open[..id_end].trim();
        let inner = &after_open[id_end + 1..close];
        out.push_str(&rest[..start]);
        let mut tail = &after_open[close + CONDITION_CLOSE.len()..];
        if keep(id) {
            out.push_str(inner);
        } else {
            let at_line_start = out.is_empty() || out.ends_with('\n');
            if (at_line_start || out.ends_with(' ')) && tail.starts_with(' ') {
                tail = &tail[1..];
            }
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// `text` with each conditional span kept only when its condition holds for this household
/// ([`Condition::holds`]). A malformed condition keeps its text; the validator rejects it.
pub fn apply_conditions_for(text: &str, household: &impl HouseholdFacts) -> String {
    apply_conditions(text, |id| {
        Condition::parse(id)
            .map(|c| c.holds(household))
            .unwrap_or(true)
    })
}

/// Which conditions a block may use (see [`condition_problems_in`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct ConditionScope<'a> {
    /// The hazards a hazard condition may name. `None` accepts any hazard id content may name
    /// ([`crate::ids::is_hazard`]).
    pub hazards: Option<&'a [&'a str]>,
    /// The catalogue item ids a `has:` condition may name. `None` accepts any well-formed id.
    pub items: Option<&'a std::collections::BTreeSet<String>>,
}

/// Problems with the conditional spans of a guidance body: an unclosed or nested span, a hazard
/// id that is not one of `allowed` (the block's own `hazard:` targets), or a malformed condition
/// (see [`Condition`]). Same as [`condition_problems_in`] with only the hazards restricted.
pub fn condition_problems(body: &str, allowed: &[&str]) -> Vec<String> {
    condition_problems_in(
        body,
        ConditionScope {
            hazards: Some(allowed),
            items: None,
        },
    )
}

/// Problems with the conditional spans of a guidance body: an unclosed or nested span, a span
/// that runs past its paragraph, a hazard or item outside `scope`, or a malformed condition
/// (an unknown kind of home, need or benefit; see [`Condition`]).
pub fn condition_problems_in(body: &str, scope: ConditionScope<'_>) -> Vec<String> {
    let mut problems = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find(CONDITION_OPEN) {
        let after_open = &rest[start + CONDITION_OPEN.len()..];
        let Some(id_end) = after_open.find('}') else {
            problems.push("a `{if:` marker has no closing brace".to_owned());
            break;
        };
        let id = after_open[..id_end].trim();
        match Condition::parse(id) {
            Ok(Condition::Hazard(h)) => match scope.hazards {
                Some(allowed) if !allowed.contains(&h.as_str()) => problems.push(format!(
                    "`{{if:{id}}}` names a hazard this block does not apply to (its applies_to lists {allowed:?})"
                )),
                None if !crate::ids::is_hazard(&h) => problems.push(format!(
                    "`{{if:{id}}}` names `{h}`, which is not a hazard id"
                )),
                _ => {}
            },
            Ok(Condition::Home(kinds) | Condition::NotHome(kinds)) if kinds.is_empty() => {
                problems.push(format!("`{{if:{id}}}` names no kind of home"));
            }
            Ok(Condition::Has(items)) => {
                if let Some(known) = scope.items {
                    for i in items.iter().filter(|i| !known.contains(i.as_str())) {
                        problems.push(format!(
                            "`{{if:{id}}}` names `{i}`, which is not a catalogue item"
                        ));
                    }
                }
            }
            Ok(_) => {}
            Err(p) => problems.push(p),
        }
        let inner_and_rest = &after_open[id_end + 1..];
        let Some(close) = inner_and_rest.find(CONDITION_CLOSE) else {
            problems.push(format!(
                "`{{if:{id}}}` is never closed with `{CONDITION_CLOSE}`"
            ));
            break;
        };
        let inner = &inner_and_rest[..close];
        if inner.contains(CONDITION_OPEN) {
            problems.push(format!(
                "`{{if:{id}}}` contains another `{{if:` (spans do not nest)"
            ));
        }
        if inner.contains("\n\n") {
            problems.push(format!("`{{if:{id}}}` runs past the end of its paragraph"));
        }
        rest = &inner_and_rest[close + CONDITION_CLOSE.len()..];
    }
    if rest.contains(CONDITION_CLOSE) {
        problems.push(format!("a `{CONDITION_CLOSE}` has no opening `{{if:...}}`"));
    }
    problems
}

/// The item categories, one `content/items/<category>.toml` file each.
pub const CATEGORIES: &[&str] = &[
    "water",
    "food",
    "medical",
    "sanitation",
    "thermal",
    "power",
    "comms",
    "documents_money",
    "evacuation",
    "get_home",
    "community",
    "security",
    "fire",
    "special_needs",
    "rare_catastrophic",
];

/// The one catalogue item allowed to mention firearms: a free action with no price, in category
/// `security` (`docs/CONTENT_STANDARDS.md` §5).
pub const PERMITTED_FIREARM_ITEM: &str = "security_firearms_safe_storage";

/// Brand and trademark names that must never appear in user-visible content. Generic words are
/// used instead (vacuum flask, adhesive bandage, zip-top bag, foil-lined bag, water storage jug).
/// Written as lowercase tokens separated by single spaces.
pub const BRANDS: &[&str] = &[
    // named in the content brief
    "thermos",
    "band aid",
    "bandaid",
    "lifestraw",
    "sawyer",
    "jackery",
    "goal zero",
    "goalzero",
    "honda",
    "midland",
    "mountain house",
    "readywise",
    "ready wise",
    "augason",
    "berkey",
    "yeti",
    "coleman",
    "garmin",
    "baofeng",
    "anker",
    "ecoflow",
    "eco flow",
    "bluetti",
    "zippo",
    "leatherman",
    "nalgene",
    "aquatainer",
    "aqua tainer",
    "waterbrick",
    "water brick",
    "sceptre",
    "mylar",
    // water, food and kits
    "reliance products",
    "reliance outdoors",
    "jumbo tainer",
    "luggable loo",
    "double doodie",
    "waterbob",
    "water bob",
    "aquamira",
    "katadyn",
    "grayl",
    "brita",
    "zerowater",
    "alexapure",
    "aquablox",
    "aqua blox",
    "datrex",
    "ready hour",
    "my patriot supply",
    "4patriots",
    "legacy food storage",
    "wise company",
    "peak refuel",
    "backpacker s pantry",
    "judy",
    "preppi",
    "uncharted supply",
    "ready america",
    "sustain supply",
    // power, light and radio
    "westinghouse",
    "generac",
    "dewalt",
    "ryobi",
    "duracell",
    "energizer",
    "biolite",
    "jetboil",
    "streamlight",
    "maglite",
    "petzl",
    "fenix",
    "olight",
    "nitecore",
    "eton",
    "kaito",
    "uniden",
    "motorola",
    "yaesu",
    "kenwood",
    "icom",
    "meshtastic",
    "starlink",
    "mr heater",
    "sterno",
    // medical and hygiene
    "mymedic",
    "my medic",
    "quikclot",
    "celox",
    "tylenol",
    "advil",
    "motrin",
    "aleve",
    "benadryl",
    "imodium",
    "pepto",
    "tums",
    "neosporin",
    "pedialyte",
    "vaseline",
    "purell",
    "clorox",
    "lysol",
    "kleenex",
    "charmin",
    "tampax",
    "kotex",
    "pampers",
    "huggies",
    "similac",
    "enfamil",
    "gatorade",
    "jase",
    "duration health",
    // household and outdoor
    "ziploc",
    "tupperware",
    "rubbermaid",
    "igloo",
    "hydro flask",
    "camelbak",
    "kelty",
    "velcro",
    "sharpie",
    "tyvek",
    "gore tex",
    "styrofoam",
    "plexiglas",
    "kevlar",
    "duck tape",
    "gorilla tape",
    "kidde",
    "sentrysafe",
    "sentry safe",
    "honeywell",
    "quakehold",
    "reflectix",
    "o2cool",
    "bic",
    "gerber",
    "victorinox",
    "swiss army",
    // retailers
    "walmart",
    "amazon",
    "costco",
    "home depot",
    "lowe s",
    "target com",
    "harbor freight",
    "cabela s",
    "bass pro",
    "rei",
    "walgreens",
    "cvs",
    "rite aid",
    "goodrx",
    "kroger",
    "webstaurant",
    "do it best",
    // vehicles and carriers
    "tesla",
    "rivian",
    "verizon",
    "t mobile",
];

/// Words that name firearms, ammunition or weapons. Allowed only in [`PERMITTED_FIREARM_ITEM`]
/// and in citations used by it alone.
pub const FIREARM_TOKENS: &[&str] = &[
    "firearm",
    "firearms",
    "gun",
    "guns",
    "handgun",
    "handguns",
    "rifle",
    "rifles",
    "shotgun",
    "shotguns",
    "pistol",
    "pistols",
    "revolver",
    "revolvers",
    "ammunition",
    "ammo",
    "weapon",
    "weapons",
    "holster",
    "holsters",
    "bullet",
    "bullets",
    "gunshot",
    "shooter",
    "shooting",
    "concealed carry",
    "taser",
    "stun gun",
    "pepper spray",
    "mace",
    "crossbow",
];

/// Sources of antibiotics that must never be suggested. In medical items they may appear only in
/// `avoid` (as the warning), and in guidance only in a sentence that also says no.
pub const UNSAFE_ANTIBIOTIC_SOURCES: &[&str] = &[
    "fish antibiotic",
    "fish antibiotics",
    "aquarium",
    "veterinary",
    "fish mox",
    "fishmox",
    "bird antibiotics",
    "livestock antibiotics",
];

/// Words that mark a sentence as a warning rather than a suggestion.
pub const NEGATIONS: &[&str] = &[
    "never",
    "not",
    "no",
    "don",
    "avoid",
    "unapproved",
    "unsafe",
    "instead",
    "without",
];

/// Units that turn a number into a dose.
pub const DOSE_UNITS: &[&str] = &[
    "mg",
    "mcg",
    "ug",
    "µg",
    "ml",
    "iu",
    "units",
    "tablet",
    "tablets",
    "tab",
    "tabs",
    "capsule",
    "capsules",
    "caplet",
    "caplets",
    "pill",
    "pills",
    "puff",
    "puffs",
    "milligram",
    "milligrams",
    "microgram",
    "micrograms",
    "milliliter",
    "milliliters",
    "millilitre",
    "millilitres",
];

/// Drug names (generic). Anywhere in content, a drug name followed closely by a dose is an error.
pub const DRUG_NAMES: &[&str] = &[
    "acetaminophen",
    "paracetamol",
    "ibuprofen",
    "naproxen",
    "aspirin",
    "diphenhydramine",
    "loratadine",
    "cetirizine",
    "fexofenadine",
    "loperamide",
    "bismuth subsalicylate",
    "famotidine",
    "omeprazole",
    "pseudoephedrine",
    "phenylephrine",
    "dextromethorphan",
    "guaifenesin",
    "hydrocortisone",
    "amoxicillin",
    "clavulanate",
    "azithromycin",
    "ciprofloxacin",
    "levofloxacin",
    "doxycycline",
    "cephalexin",
    "metronidazole",
    "nitrofurantoin",
    "trimethoprim",
    "sulfamethoxazole",
    "penicillin",
    "clindamycin",
    "ivermectin",
    "potassium iodide",
    "insulin",
    "epinephrine",
    "glucagon",
    "albuterol",
    "prednisone",
    "naloxone",
    "oseltamivir",
    "meclizine",
    "dimenhydrinate",
    "melatonin",
];

/// Phrases that break the calm, no-pressure voice (`docs/CONTENT_STANDARDS.md` §4, §6).
pub const BANNED_PHRASES: &[&str] = &[
    "you must",
    "shtf",
    "teotwawki",
    "doomsday",
    "before it s too late",
    "while supplies last",
    "limited time",
    "act now",
    "don t wait",
    "do not wait",
    "last chance",
    "hurry",
];

/// Splits text into lowercase alphanumeric tokens. Apostrophes, hyphens and all other
/// punctuation separate tokens, so "Band-Aid" is `band`, `aid` and "don't" is `don`, `t`.
pub fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == 'µ' {
            cur.extend(ch.to_lowercase());
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The phrases from `list` that occur in `toks` as whole-token sequences, in list order.
pub fn find_phrases<'a>(toks: &[String], list: &[&'a str]) -> Vec<&'a str> {
    let mut found = Vec::new();
    for phrase in list {
        let p: Vec<&str> = phrase.split(' ').collect();
        if p.is_empty() || p.len() > toks.len() {
            continue;
        }
        if toks
            .windows(p.len())
            .any(|w| w.iter().zip(&p).all(|(a, b)| a == b))
        {
            found.push(*phrase);
        }
    }
    found
}

fn is_number(tok: &str) -> bool {
    !tok.is_empty() && tok.chars().all(|c| c.is_ascii_digit())
}

fn is_count_word(tok: &str) -> bool {
    matches!(
        tok,
        "one" | "two" | "three" | "four" | "five" | "six" | "eight" | "ten" | "twelve" | "half"
    )
}

/// A dosing instruction in `toks`: a number followed by a dose unit, "every N hours",
/// "N times a day", or "once/twice a day". Returns the offending words.
pub fn find_dosing(toks: &[String]) -> Option<String> {
    for i in 0..toks.len() {
        let t = toks[i].as_str();
        let next = toks.get(i + 1).map(String::as_str).unwrap_or("");
        if (is_number(t) || is_count_word(t)) && DOSE_UNITS.contains(&next) {
            return Some(format!("{t} {next}"));
        }
        if t == "every" && is_number(next) {
            let n2 = toks.get(i + 2).map(String::as_str).unwrap_or("");
            let n4 = toks.get(i + 4).map(String::as_str).unwrap_or("");
            if matches!(n2, "hour" | "hours") || (n2 == "to" && matches!(n4, "hour" | "hours")) {
                return Some(toks[i..(i + 5).min(toks.len())].join(" "));
            }
        }
        if (is_number(t) || is_count_word(t))
            && next == "times"
            && matches!(
                toks.get(i + 2).map(String::as_str),
                Some("a" | "per" | "daily")
            )
        {
            return Some(toks[i..(i + 4).min(toks.len())].join(" "));
        }
        if matches!(t, "once" | "twice" | "thrice") {
            let n2 = toks.get(i + 2).map(String::as_str).unwrap_or("");
            if next == "daily" || (matches!(next, "a" | "per") && n2 == "day") {
                return Some(format!("{t} {next}"));
            }
        }
    }
    None
}

/// A drug name followed within six tokens by a number and a dose unit. Returns the words.
pub fn find_drug_dose(toks: &[String]) -> Option<String> {
    for drug in DRUG_NAMES {
        let p: Vec<&str> = drug.split(' ').collect();
        for i in 0..toks.len().saturating_sub(p.len() - 1) {
            if toks[i..i + p.len()].iter().zip(&p).all(|(a, b)| a == b) {
                let end = (i + p.len() + 6).min(toks.len());
                if let Some(d) = find_dosing(&toks[i + p.len()..end]) {
                    return Some(format!("{drug} … {d}"));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Vec<String> {
        tokens(s)
    }

    #[test]
    fn tokens_split_on_punctuation_and_lowercase() {
        assert_eq!(t("Band-Aid, don't"), vec!["band", "aid", "don", "t"]);
    }

    #[test]
    fn waiting_phrases_are_pressure_in_both_spellings() {
        // Verification V-17: "Do not wait ..." slipped past a list that had only "don't wait".
        for text in [
            "Don't wait to file.",
            "Don\u{2019}t wait to file.",
            "Do not wait to see flames.",
            "DO NOT WAIT",
        ] {
            assert!(!find_phrases(&t(text), BANNED_PHRASES).is_empty(), "{text}");
        }
        assert!(find_phrases(&t("Do not let the water wait."), BANNED_PHRASES).is_empty());
    }

    #[test]
    fn brands_match_whole_words_only() {
        assert_eq!(
            find_phrases(&t("A LifeStraw filter"), BRANDS),
            vec!["lifestraw"]
        );
        assert_eq!(
            find_phrases(&t("Goal-Zero panel"), BRANDS),
            vec!["goal zero"]
        );
        assert!(find_phrases(&t("goals are zero-cost"), BRANDS).is_empty());
        // the brand is matched by its full name, so the ordinary word stays usable
        assert!(find_phrases(&t("less reliance on stored water"), BRANDS).is_empty());
    }

    #[test]
    fn dosing_patterns_are_found() {
        assert!(find_dosing(&t("take 200 mg")).is_some());
        assert!(find_dosing(&t("two tablets now")).is_some());
        assert!(find_dosing(&t("every 4 to 6 hours")).is_some());
        assert!(find_dosing(&t("every 6 hours")).is_some());
        assert!(find_dosing(&t("3 times a day")).is_some());
        assert!(find_dosing(&t("twice daily")).is_some());
        assert!(find_dosing(&t("keep a 7- to 10-day supply")).is_none());
        assert!(find_dosing(&t("mix one packet with 1 liter of clean water")).is_none());
    }

    #[test]
    fn drug_with_dose_is_found_but_drug_alone_is_not() {
        assert!(find_drug_dose(&t("ibuprofen 200 mg every 6 hours")).is_some());
        assert!(find_drug_dose(&t("Potassium iodide 130 mg")).is_some());
        assert!(find_drug_dose(&t("infant pain reliever with acetaminophen")).is_none());
    }

    #[test]
    fn firearm_words_are_found() {
        assert_eq!(
            find_phrases(&t("store the firearm locked"), FIREARM_TOKENS),
            vec!["firearm"]
        );
        assert!(find_phrases(&t("troubleshooting the generator"), FIREARM_TOKENS).is_empty());
    }

    #[test]
    fn conditional_spans_are_kept_or_dropped_cleanly() {
        let text = "Warm the core first. {if:avalanche}In avalanche country, carry a beacon.{/if} Check on neighbours.";
        assert_eq!(
            apply_conditions(text, |_| true),
            "Warm the core first. In avalanche country, carry a beacon. Check on neighbours."
        );
        assert_eq!(
            apply_conditions(text, |_| false),
            "Warm the core first. Check on neighbours."
        );
        // At the end of a paragraph, and inside a sentence.
        let end = "Stay inside. {if:tsunami}Do not go back to the shore.{/if}\n\nNext.";
        assert_eq!(apply_conditions(end, |_| false), "Stay inside. \n\nNext.");
        let clause =
            "Leave if you are told to{if:landslide}, or if you feel unsafe near a slope{/if}.";
        assert_eq!(
            apply_conditions(clause, |_| false),
            "Leave if you are told to."
        );
        assert_eq!(
            apply_conditions(clause, |h| h == "landslide"),
            "Leave if you are told to, or if you feel unsafe near a slope."
        );
        // At the start of a line.
        assert_eq!(
            apply_conditions(
                "{if:drought}In a drought, save water.{/if} Then rest.",
                |_| false
            ),
            "Then rest."
        );
    }

    #[test]
    fn malformed_conditional_spans_are_reported() {
        assert!(condition_problems("a {if:avalanche}b{/if} c", &["avalanche"]).is_empty());
        assert!(!condition_problems("a {if:tornado}b{/if}", &["avalanche"]).is_empty());
        assert!(!condition_problems("a {if:avalanche}b", &["avalanche"]).is_empty());
        assert!(
            !condition_problems("a {if:avalanche}b {if:avalanche}c{/if}", &["avalanche"])
                .is_empty()
        );
        assert!(!condition_problems("a b{/if}", &["avalanche"]).is_empty());
        assert!(!condition_problems("{if:avalanche}a\n\nb{/if}", &["avalanche"]).is_empty());
    }

    #[test]
    fn home_conditions_parse_and_pick_the_right_homes() {
        use rr_types::HousingKind::*;
        assert_eq!(
            Condition::parse("avalanche"),
            Ok(Condition::Hazard("avalanche".to_owned()))
        );
        assert_eq!(
            Condition::parse("home:apartment_high_rise"),
            Ok(Condition::Home(vec![ApartmentHighRise]))
        );
        let flats = Condition::parse("not_home:apartment_low_rise|apartment_high_rise").unwrap();
        assert_eq!(
            flats,
            Condition::NotHome(vec![ApartmentLowRise, ApartmentHighRise])
        );
        assert_eq!(flats.for_home(Rowhouse), Some(true));
        assert_eq!(flats.for_home(ApartmentHighRise), Some(false));
        assert_eq!(
            Condition::parse("home:mobile_home")
                .unwrap()
                .for_home(MobileHome),
            Some(true)
        );
        assert_eq!(
            Condition::parse("tornado").unwrap().for_home(Detached),
            None
        );
        assert!(Condition::parse("home:castle").is_err());
        assert!(Condition::parse("home:").is_err());
        assert!(Condition::parse("home:detached|detached").is_err());
    }

    /// A household for the condition tests: a high-rise flat, someone who is deaf, a generator,
    /// SNAP, and tornadoes likely enough to matter.
    struct Flat;

    impl HouseholdFacts for Flat {
        fn hazard_relevant(&self, hazard: &str) -> bool {
            hazard == "tornado"
        }
        fn home(&self) -> rr_types::HousingKind {
            rr_types::HousingKind::ApartmentHighRise
        }
        fn has_access_need(&self, need: &str) -> bool {
            need == "hearing"
        }
        fn has_item(&self, item: &str) -> bool {
            item == "power_generator"
        }
        fn has_benefit(&self, benefit: &str) -> bool {
            benefit == "snap_wic"
        }
    }

    #[test]
    fn need_has_and_benefit_conditions_parse() {
        assert_eq!(
            Condition::parse("need:hearing|vision"),
            Ok(Condition::Need(vec!["hearing".into(), "vision".into()]))
        );
        assert_eq!(
            Condition::parse("has:power_generator"),
            Ok(Condition::Has(vec!["power_generator".into()]))
        );
        assert_eq!(
            Condition::parse("benefit:snap_wic|federal_pay"),
            Ok(Condition::Benefit(vec![
                "snap_wic".into(),
                "federal_pay".into()
            ]))
        );
        assert!(Condition::parse("need:telepathy").is_err());
        assert!(Condition::parse("need:").is_err());
        assert!(Condition::parse("need:hearing|hearing").is_err());
        assert!(Condition::parse("benefit:lottery").is_err());
        assert!(Condition::parse("has:Power Generator").is_err());
        assert!(Condition::parse("has:a||b").is_err());
        // They say nothing about the kind of home on their own.
        let need = Condition::parse("need:hearing").unwrap();
        assert_eq!(need.for_home(rr_types::HousingKind::Detached), None);
    }

    #[test]
    fn every_condition_kind_is_applied_for_a_household() {
        let text = "A. {if:tornado}B.{/if} {if:hurricane}C.{/if} {if:home:apartment_high_rise}D.{/if} \
                    {if:not_home:apartment_high_rise}E.{/if} {if:need:hearing|vision}F.{/if} \
                    {if:need:dialysis}G.{/if} {if:has:power_generator}H.{/if} \
                    {if:has:water_drum_55gal}I.{/if} {if:benefit:snap_wic}J.{/if} \
                    {if:benefit:va}K.{/if} Z.";
        assert_eq!(apply_conditions_for(text, &Flat), "A. B. D. F. H. J. Z.");
        // A household-free view keeps every span.
        assert_eq!(
            apply_conditions(text, |_| true),
            "A. B. C. D. E. F. G. H. I. J. K. Z."
        );
        // A malformed condition keeps its text.
        assert_eq!(apply_conditions_for("{if:need:x}Y.{/if}", &Flat), "Y.");
    }

    #[test]
    fn conditions_are_checked_against_their_scope() {
        let items: std::collections::BTreeSet<String> =
            ["power_generator".to_owned()].into_iter().collect();
        let strict = ConditionScope {
            hazards: Some(&["tornado"]),
            items: Some(&items),
        };
        assert!(condition_problems_in("{if:tornado}a{/if}", strict).is_empty());
        assert!(!condition_problems_in("{if:hurricane}a{/if}", strict).is_empty());
        assert!(condition_problems_in("{if:has:power_generator}a{/if}", strict).is_empty());
        assert!(!condition_problems_in("{if:has:jetpack}a{/if}", strict).is_empty());
        assert!(!condition_problems_in("{if:need:telepathy}a{/if}", strict).is_empty());
        // Plan blocks may name any hazard, including one contract v2 adds, but not a made-up one.
        let open = ConditionScope {
            hazards: None,
            items: Some(&items),
        };
        assert!(condition_problems_in("{if:hurricane}a{/if}", open).is_empty());
        assert!(condition_problems_in("{if:wildfire_smoke}a{/if}", open).is_empty());
        assert!(!condition_problems_in("{if:zombies}a{/if}", open).is_empty());
        // Spans still do not nest, whatever their kind.
        assert!(
            !condition_problems_in("{if:need:hearing}a {if:has:power_generator}b{/if}", open)
                .is_empty()
        );
    }

    #[test]
    fn home_conditions_are_validated_and_applied() {
        // A home condition is allowed in any block, whatever it applies to.
        let body = "Shelter now. {if:not_home:apartment_high_rise}Use the basement.{/if} \
                    {if:home:apartment_high_rise}Stay on or below the 10th floor.{/if} Then wait.";
        assert!(condition_problems(body, &[]).is_empty());
        assert!(!condition_problems("{if:home:castle}x{/if}", &[]).is_empty());
        assert!(!condition_problems("{if:not_home:}x{/if}", &[]).is_empty());
        let keep = |kind: rr_types::HousingKind| {
            apply_conditions(body, |id| {
                Condition::parse(id)
                    .ok()
                    .and_then(|c| c.for_home(kind))
                    .unwrap_or(true)
            })
        };
        assert_eq!(
            keep(rr_types::HousingKind::Rowhouse),
            "Shelter now. Use the basement. Then wait."
        );
        assert_eq!(
            keep(rr_types::HousingKind::ApartmentHighRise),
            "Shelter now. Stay on or below the 10th floor. Then wait."
        );
    }
}
