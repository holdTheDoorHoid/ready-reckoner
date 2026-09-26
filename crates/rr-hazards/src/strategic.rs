//! The nuclear family's location term: the county strategic-exposure class (REVIEW §2.3;
//! `research/strategic-sites.md` §3 and §7; `data/core/strategic_sites.toml`).
//!
//! A county takes the highest class it qualifies for (A > C1 > B > C2 > D > E by f_S). f_S is the
//! chance that a large attack on the US brings serious local effects (blast or dangerous fallout)
//! to a county of the class: an expert estimate, shown only as a range. The "Why here" sentences
//! are the research's templates, word for word, filled from the pack's resolved sites, distances
//! and bearings; a placeholder the data cannot fill falls back to plainer words, never to a guess.

use rr_types::{StrategicClass, StrategicPlace};

use crate::params::Triple;

/// One class and f_S with its range (labels and templates: `data/core/strategic_sites.toml`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClassDef {
    /// The class.
    pub class: StrategicClass,
    /// f_S = P(serious local effects | a large attack on the US), PRIOR (REVIEW §2.3).
    pub f_s: Triple,
}

/// The six classes, in precedence order.
pub(crate) const CLASSES: [ClassDef; 6] = [
    ClassDef {
        class: StrategicClass::A,
        f_s: (0.9, 0.6, 0.99),
    },
    ClassDef {
        class: StrategicClass::C1,
        f_s: (0.6, 0.3, 0.9),
    },
    ClassDef {
        class: StrategicClass::B,
        f_s: (0.5, 0.2, 0.8),
    },
    ClassDef {
        class: StrategicClass::C2,
        f_s: (0.3, 0.1, 0.6),
    },
    ClassDef {
        class: StrategicClass::D,
        f_s: (0.15, 0.05, 0.4),
    },
    ClassDef {
        class: StrategicClass::E,
        f_s: (0.03, 0.01, 0.1),
    },
];

/// The definition of `class`.
pub(crate) fn class_def(class: StrategicClass) -> &'static ClassDef {
    CLASSES
        .iter()
        .find(|c| c.class == class)
        .expect("every class is defined")
}

/// Kilometres to miles, rounded to the nearest 10 (the template rule).
pub(crate) fn miles_rounded(km: f64) -> u32 {
    let mi = km * 0.621_371;
    ((mi / 10.0).round() * 10.0).max(10.0) as u32
}

/// A compass bearing (degrees clockwise from north) as one of the 16 points, in words.
pub(crate) fn compass16(bearing: f64) -> &'static str {
    const POINTS: [&str; 16] = [
        "north",
        "north-northeast",
        "northeast",
        "east-northeast",
        "east",
        "east-southeast",
        "southeast",
        "south-southeast",
        "south",
        "south-southwest",
        "southwest",
        "west-southwest",
        "west",
        "west-northwest",
        "northwest",
        "north-northwest",
    ];
    let b = bearing.rem_euclid(360.0);
    let i = ((b / 22.5).round() as usize) % 16;
    POINTS[i]
}

/// A state or territory's name from its two-letter code; the code itself if unknown.
pub(crate) fn state_name(code: &str) -> &str {
    const STATES: &[(&str, &str)] = &[
        ("AL", "Alabama"),
        ("AK", "Alaska"),
        ("AZ", "Arizona"),
        ("AR", "Arkansas"),
        ("CA", "California"),
        ("CO", "Colorado"),
        ("CT", "Connecticut"),
        ("DE", "Delaware"),
        ("DC", "the District of Columbia"),
        ("FL", "Florida"),
        ("GA", "Georgia"),
        ("GU", "Guam"),
        ("HI", "Hawaii"),
        ("ID", "Idaho"),
        ("IL", "Illinois"),
        ("IN", "Indiana"),
        ("IA", "Iowa"),
        ("KS", "Kansas"),
        ("KY", "Kentucky"),
        ("LA", "Louisiana"),
        ("ME", "Maine"),
        ("MD", "Maryland"),
        ("MA", "Massachusetts"),
        ("MI", "Michigan"),
        ("MN", "Minnesota"),
        ("MS", "Mississippi"),
        ("MO", "Missouri"),
        ("MT", "Montana"),
        ("NE", "Nebraska"),
        ("NV", "Nevada"),
        ("NH", "New Hampshire"),
        ("NJ", "New Jersey"),
        ("NM", "New Mexico"),
        ("NY", "New York"),
        ("NC", "North Carolina"),
        ("ND", "North Dakota"),
        ("OH", "Ohio"),
        ("OK", "Oklahoma"),
        ("OR", "Oregon"),
        ("PA", "Pennsylvania"),
        ("PR", "Puerto Rico"),
        ("RI", "Rhode Island"),
        ("SC", "South Carolina"),
        ("SD", "South Dakota"),
        ("TN", "Tennessee"),
        ("TX", "Texas"),
        ("UT", "Utah"),
        ("VT", "Vermont"),
        ("VA", "Virginia"),
        ("VI", "the US Virgin Islands"),
        ("WA", "Washington"),
        ("WV", "West Virginia"),
        ("WI", "Wisconsin"),
        ("WY", "Wyoming"),
    ];
    STATES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or(code, |(_, n)| *n)
}

/// "Wyoming, Nebraska and Colorado" from "WY, NE, CO" (names pass through unchanged).
pub(crate) fn states_in_words(list: &str) -> String {
    let names: Vec<&str> = list
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(state_name)
        .collect();
    match names.len() {
        0 => String::new(),
        1 => names[0].to_owned(),
        n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

/// The short form of a metro title: "Philadelphia-Camden-Wilmington, PA-NJ-DE-MD" →
/// "the Philadelphia metro area".
pub(crate) fn metro_short(title: &str) -> String {
    let first = title
        .split([',', '-'])
        .next()
        .map(str::trim)
        .unwrap_or(title);
    format!("the {first} metro area")
}

/// `{place}` in the C1, C2 and D sentences.
fn place_words(p: &StrategicPlace, county_and_state: &str) -> String {
    match p.kind.as_str() {
        "metro" => metro_short(&p.name),
        "port" | "refinery" => county_and_state.to_owned(),
        "capital_region" => "the National Capital Region".to_owned(),
        _ => p.name.clone(),
    }
}

/// `{reason}` in the C2 sentence: the qualifying reasons in words, joined.
fn reasons(places: &[StrategicPlace]) -> String {
    let mut out: Vec<&str> = Vec::new();
    // A refinery county is one place whose name joins its refineries with "; ".
    let refineries: usize = places
        .iter()
        .filter(|p| p.kind == "refinery")
        .map(|p| p.name.matches(';').count() + 1)
        .sum();
    for p in places {
        let r = match p.kind.as_str() {
            "metro" => "a metro area of more than a million people",
            "port" => "a major port",
            "refinery" if refineries > 1 => "several large oil refineries",
            "refinery" => "a large oil refinery",
            _ => "a major military base",
        };
        if !out.contains(&r) {
            out.push(r);
        }
    }
    match out.len() {
        0 => "a metro area, port, refinery or base of national importance".to_owned(),
        1 => out[0].to_owned(),
        n => format!("{} and {}", out[..n - 1].join(", "), out[n - 1]),
    }
}

/// The "Why here" sentence for a county of `class`, filled from its resolved places and the
/// distance and bearing to the place behind a downwind class. `county_and_state` names the
/// county for port and refinery counties ("Jefferson County, Texas").
pub(crate) fn why_here(
    class: StrategicClass,
    places: &[StrategicPlace],
    distance: Option<(f64, Option<f64>)>,
    county_and_state: &str,
) -> String {
    let first = places.first();
    match class {
        StrategicClass::A => {
            let site = match first {
                Some(p) if !p.role_plain.is_empty() => format!("{}: {}", p.name, p.role_plain),
                Some(p) => p.name.clone(),
                None => "a strategic military site".to_owned(),
            };
            format!(
                "Your county has, or is close to, {site}. In a large nuclear war, places like this \
                 are treated as likely targets. That does not mean an attack is likely. It means \
                 that if one happened, this area would face more danger than most."
            )
        }
        StrategicClass::B => {
            let field = places.iter().find(|p| p.kind == "icbm_field").or(first);
            let fields = match field.map(|p| states_in_words(&p.state)) {
                Some(s) if !s.is_empty() => format!("the nuclear missile fields in {s}"),
                _ => "the nuclear missile fields of the northern Great Plains".to_owned(),
            };
            let where_ = match distance {
                Some((km, Some(b))) => format!(
                    ", about {} miles to your {}",
                    miles_rounded(km),
                    compass16(b)
                ),
                Some((km, None)) => format!(", about {} miles away", miles_rounded(km)),
                None => String::new(),
            };
            format!(
                "You live downwind of {fields}{where_}. Winds here usually blow from the west. \
                 If those missile sites were ever attacked, radioactive fallout could drift over \
                 your area within a day or two. That is the main reason your chance is higher \
                 than in most places."
            )
        }
        StrategicClass::C1 => {
            let place = first.map_or_else(
                || {
                    "one of the largest cities, the capital region, or near a nuclear-weapons plant"
                        .to_owned()
                },
                |p| place_words(p, county_and_state),
            );
            format!(
                "You live in {place}. Because of its size and importance, a place like this is \
                 treated as a likely target in a large nuclear war. That does not mean an attack \
                 is likely, only that the danger here would be greater than in most places."
            )
        }
        StrategicClass::C2 => {
            let place = first.map_or_else(
                || county_and_state.to_owned(),
                |p| place_words(p, county_and_state),
            );
            format!(
                "You live in {place}: {}. Places like this could be targets in a large nuclear \
                 war, but they are less likely targets than the biggest cities and military sites.",
                reasons(places)
            )
        }
        StrategicClass::D => {
            let place = first.map_or_else(
                || "a likely target".to_owned(),
                |p| place_words(p, county_and_state),
            );
            let lead = match distance {
                Some((km, _)) => format!(
                    "You live about {} miles downwind of {place}.",
                    miles_rounded(km)
                ),
                None => format!("You live downwind of {place}."),
            };
            format!(
                "{lead} If that place were attacked, fallout could drift here, so your chance of \
                 serious effects is higher than in most rural areas, though still lower than near \
                 the target itself."
            )
        }
        StrategicClass::E => "None of the places treated as likely targets, and no missile-field \
             fallout path, is close to you. In a large nuclear war, the main effects here would be \
             shortages, power cuts and lost income, not blast or heavy fallout."
            .to_owned(),
    }
}

/// The sentence when the pack does not give the county's class.
pub(crate) const WHY_HERE_UNKNOWN: &str = "The data for where you live does not include its \
    strategic class yet, so this uses the average over every county in the country. Near \
    missile fields, command centres and the largest cities the chance is higher; far from them \
    it is lower.";

#[cfg(test)]
mod tests {
    use super::*;

    fn place(id: &str, name: &str, kind: &str, role: &str, state: &str) -> StrategicPlace {
        StrategicPlace {
            id: id.into(),
            name: name.into(),
            kind: kind.into(),
            role_plain: role.into(),
            state: state.into(),
            sources: Vec::new(),
            unverified: Vec::new(),
        }
    }

    #[test]
    fn the_filled_examples_of_the_research() {
        // research/strategic-sites.md §7, with the pack's own distances and bearings.
        let kitsap = why_here(
            StrategicClass::A,
            &[place(
                "kitsap_bangor",
                "Naval Base Kitsap-Bangor",
                "ssbn_base",
                "the home port of the Navy’s nuclear-missile submarines",
                "WA",
            )],
            None,
            "Kitsap County, Washington",
        );
        assert_eq!(
            kitsap,
            "Your county has, or is close to, Naval Base Kitsap-Bangor: the home port of the \
             Navy’s nuclear-missile submarines. In a large nuclear war, places like this are \
             treated as likely targets. That does not mean an attack is likely. It means that if \
             one happened, this area would face more danger than most."
        );
        let hays = why_here(
            StrategicClass::B,
            &[place(
                "warren_field",
                "F.E. Warren AFB missile field (90th Missile Wing)",
                "icbm_field",
                "a field of underground nuclear missile silos",
                "WY, NE, CO",
            )],
            Some((381.1, Some(304.0))),
            "Ellis County, Kansas",
        );
        assert!(hays.starts_with(
            "You live downwind of the nuclear missile fields in Wyoming, Nebraska and Colorado, \
             about 240 miles to your northwest. Winds here usually blow from the west."
        ));
        let phl = why_here(
            StrategicClass::C1,
            &[place(
                "metro_37980",
                "Philadelphia-Camden-Wilmington, PA-NJ-DE-MD",
                "metro",
                "",
                "",
            )],
            None,
            "Philadelphia County, Pennsylvania",
        );
        assert!(phl.starts_with("You live in the Philadelphia metro area. Because of its size"));
        let jefferson = why_here(
            StrategicClass::C2,
            &[
                place("port_7", "Beaumont, TX", "port", "", ""),
                place(
                    "refinery_48245",
                    "Motiva Enterprises LLC (Port Arthur), 656400 b/cd; ExxonMobil Refining & \
                     Supply Co (Beaumont), 612000 b/cd",
                    "refinery",
                    "",
                    "",
                ),
            ],
            None,
            "Jefferson County, Texas",
        );
        assert!(jefferson.starts_with(
            "You live in Jefferson County, Texas: a major port and several large oil refineries. \
             Places like this could be targets"
        ));
        let d = why_here(
            StrategicClass::D,
            &[place(
                "kcnsc",
                "Kansas City National Security Campus",
                "doe_weapons_complex",
                "a plant or laboratory that makes or looks after nuclear weapons",
                "MO",
            )],
            Some((107.8, Some(264.0))),
            "Saline County, Missouri",
        );
        assert!(d.starts_with(
            "You live about 70 miles downwind of Kansas City National Security Campus. If that \
             place were attacked"
        ));
        let e = why_here(StrategicClass::E, &[], None, "Coos County, Oregon");
        assert!(e.starts_with("None of the places treated as likely targets"));
    }

    #[test]
    fn compass_and_miles() {
        assert_eq!(compass16(0.0), "north");
        assert_eq!(compass16(292.5), "west-northwest");
        assert_eq!(compass16(304.0), "northwest");
        assert_eq!(compass16(359.0), "north");
        assert_eq!(compass16(-90.0), "west");
        assert_eq!(miles_rounded(381.1), 240);
        assert_eq!(miles_rounded(1.0), 10);
        assert_eq!(
            states_in_words("WY, NE, CO"),
            "Wyoming, Nebraska and Colorado"
        );
        assert_eq!(states_in_words("ND"), "North Dakota");
        assert_eq!(
            metro_short("Houston-Pasadena-The Woodlands, TX"),
            "the Houston metro area"
        );
    }

    #[test]
    fn classes_follow_the_review_and_precedence() {
        let order: Vec<StrategicClass> = CLASSES.iter().map(|c| c.class).collect();
        assert!(
            order.windows(2).all(|w| w[0] < w[1]),
            "declaration order is precedence"
        );
        for c in CLASSES {
            let (v, lo, hi) = c.f_s;
            assert!(0.0 < lo && lo <= v && v <= hi && hi < 1.0, "{}", c.class);
        }
        assert_eq!(class_def(StrategicClass::A).f_s, (0.9, 0.6, 0.99));
        assert_eq!(class_def(StrategicClass::E).f_s, (0.03, 0.01, 0.1));
    }
}
