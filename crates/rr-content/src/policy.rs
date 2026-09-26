//! The word lists behind the content policy (`docs/PRINCIPLES.md` §8–§9,
//! `docs/CONTENT_STANDARDS.md` §3–§5) and small token matchers that use them.
//!
//! Matching works on lowercase alphanumeric tokens, so punctuation and hyphens never hide a word:
//! "Band-Aid", "band aid" and "BAND AID" all become the tokens `band aid`. No regular-expression
//! engine is needed, which keeps the WebAssembly build small.

/// The placeholder the engine replaces with the household's own natural-frequency sentence.
pub const FREQUENCY_PLACEHOLDER: &str = "{frequency}";

/// Opens a span of a hazard-family guidance block that is about one of its hazards only:
/// `{if:avalanche}In avalanche country, get training ...{/if}`. The packet keeps the span when
/// that hazard is likely enough for the household (a ten-year chance of at least 1 in 100) and
/// drops it otherwise; every household-free view keeps it. The hazard must be one the block
/// `applies_to`. Spans do not nest and stay within one paragraph.
pub const CONDITION_OPEN: &str = "{if:";

/// Closes a conditional span (see [`CONDITION_OPEN`]).
pub const CONDITION_CLOSE: &str = "{/if}";

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

/// Problems with the conditional spans of a guidance body: an unclosed or nested span, or a
/// hazard id that is not one of `allowed` (the block's own `hazard:` targets).
pub fn condition_problems(body: &str, allowed: &[&str]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find(CONDITION_OPEN) {
        let after_open = &rest[start + CONDITION_OPEN.len()..];
        let Some(id_end) = after_open.find('}') else {
            problems.push("a `{if:` marker has no closing brace".to_owned());
            break;
        };
        let id = after_open[..id_end].trim();
        if !allowed.contains(&id) {
            problems.push(format!(
                "`{{if:{id}}}` names a hazard this block does not apply to (its applies_to lists {allowed:?})"
            ));
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
}
