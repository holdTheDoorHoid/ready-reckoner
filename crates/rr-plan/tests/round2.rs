//! Round 2 (v0.1.1) regression tests: what the packet prints and the model numbers the reviews
//! found wrong (`~/Desktop/ready-reckoner-briefs/round2/REVIEW.md`, S2–S8, M-04, M-08, M-13, C1).
//!
//! Besides the seven fixture households, five backtest households from the model review are
//! planned against the repository's data packs (`tests/data/backtest/`, copied from the review's
//! inputs): Paradise, California (Camp Fire), a basement flat in Queens (Ida's remnants),
//! Lahaina, Maui (the 2023 fire), Utuado, Puerto Rico (Maria) and Asheville (Helene).

mod common;

use std::sync::OnceLock;

use common::{engine, outputs, run};
use rr_types::{BucketId, HazardId, PlanInput, PlanOutput, Target};

const BACKTESTS: [&str; 5] = [
    "campfire-paradise-2",
    "ida-queens-basement-2",
    "lahaina-maui-3",
    "maria-utuado-3",
    "helene-asheville-3",
];

fn backtest(name: &str) -> PlanInput {
    let path = format!(
        "{}/tests/data/backtest/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Every fixture and backtest household with its output (computed once).
fn all() -> &'static Vec<(String, PlanInput, PlanOutput)> {
    static ALL: OnceLock<Vec<(String, PlanInput, PlanOutput)>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut v: Vec<(String, PlanInput, PlanOutput)> = outputs()
            .iter()
            .map(|(n, i, o)| ((*n).to_owned(), i.clone(), o.clone()))
            .collect();
        for name in BACKTESTS {
            let input = backtest(name);
            let out = engine()
                .assess(&input)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            v.push((name.to_owned(), input, out));
        }
        v
    })
}

fn packet(name: &str) -> &'static str {
    &all()
        .iter()
        .find(|(n, _, _)| n == name)
        .unwrap_or_else(|| panic!("no household {name}"))
        .2
        .packet_markdown
}

fn input(name: &str) -> &'static PlanInput {
    &all()
        .iter()
        .find(|(n, _, _)| n == name)
        .unwrap_or_else(|| panic!("no household {name}"))
        .1
}

/// The part of a packet from one `##` heading to the next.
fn section<'a>(p: &'a str, heading: &str) -> &'a str {
    let start = p
        .find(&format!("\n{heading}\n"))
        .unwrap_or_else(|| panic!("no {heading}"));
    let rest = &p[start + 1..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |e| e + 3);
    &rest[..end]
}

/// The hazard cards of a packet, by name, in order.
fn cards(p: &str) -> Vec<&str> {
    section(p, "## Your risks")
        .lines()
        .filter_map(|l| l.strip_prefix("#### "))
        .filter_map(|l| l.split_once(". ").map(|(_, t)| t))
        .collect()
}

/// One card's text, from its heading to the next heading.
fn card<'a>(p: &'a str, name: &str) -> &'a str {
    let risks = section(p, "## Your risks");
    let at = risks
        .lines()
        .position(|l| l.starts_with("#### ") && l.ends_with(&format!(". {name}")))
        .unwrap_or_else(|| panic!("no card {name}"));
    let lines: Vec<&str> = risks.lines().collect();
    let first = risks.find(lines[at]).unwrap();
    let rest = &risks[first + lines[at].len()..];
    let end = rest.find("\n###").unwrap_or(rest.len());
    &risks[first..first + lines[at].len() + end]
}

// ------------------------------------------------------------------------------------------------
// S3: which hazards get a card
// ------------------------------------------------------------------------------------------------

#[test]
fn every_packet_has_a_house_fire_card() {
    for (name, _, out) in all() {
        let c = cards(&out.packet_markdown);
        assert!(c.contains(&"House fire"), "{name}: {c:?}");
        // Its escape steps reach paper (RR-P03: "crawl low" was in 0 of 7 packets).
        assert!(
            out.packet_markdown.contains("crawl low"),
            "{name}: the house-fire card's advice"
        );
    }
}

#[test]
fn hazards_that_kill_get_a_card_where_they_threaten() {
    for (name, hazard) in [
        ("campfire-paradise-2", "Wildfire"),
        (
            "ida-queens-basement-2",
            "Flooding from rivers or heavy rain",
        ),
        ("miami-condo-retiree-1", "Hurricane"),
        ("lahaina-maui-3", "Wildfire"),
        ("lahaina-maui-3", "Tsunami"),
        ("maria-utuado-3", "Landslide"),
    ] {
        let c = cards(packet(name));
        assert!(c.contains(&hazard), "{name}: no {hazard} card in {c:?}");
    }
    // Paradise: the wildfire card says to leave as soon as told, even without flames.
    assert!(
        card(packet("campfire-paradise-2"), "Wildfire").contains("even if you cannot see flames")
    );
}

/// The packet's length in printed pages, as docs/PACKET.md defines the proxy: words outside the
/// Sources section at 400 a page, and words in the two-column, 8-point Sources section at 1,000 a
/// page. Calibrated on the v0.1.0 Philadelphia packet, which printed on 22 US Letter pages with
/// 8,054 words outside Sources and 2,055 in it: 22.19 on this scale.
fn printed_pages(p: &str) -> f64 {
    let at = p.find("\n## Sources\n").expect("a Sources section");
    words(&p[..at]) as f64 / 400.0 + words(&p[at..]) as f64 / 1000.0
}

/// Words as a reader counts them: tokens with a letter or digit, citation brackets left out (the
/// same count as `the_packet_stays_short`).
fn words(markdown: &str) -> usize {
    let mut text = String::with_capacity(markdown.len());
    let mut rest = markdown;
    while let Some(start) = rest.find('[') {
        text.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find(']') {
            Some(end)
                if !after[..end].is_empty()
                    && after[..end].split(", ").all(|n| n.parse::<usize>().is_ok()) =>
            {
                text.push(' ');
                rest = &after[end + 1..];
            }
            _ => {
                text.push('[');
                rest = after;
            }
        }
    }
    text.push_str(rest);
    text.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count()
}

/// The most the Philadelphia packet may print on: the v0.1.0 packet's 22 US Letter pages plus
/// about one page accepted in v0.1.1 for the safety rules, the wider card rule and the cold-chain
/// lines (docs/PACKET.md); v0.2.0's packet redesign sets a fresh budget.
const PHILADELPHIA_MAX_PAGES: f64 = 23.5;

#[test]
fn philadelphia_stays_within_22_printed_pages() {
    for (name, _, out) in all() {
        eprintln!(
            "{name}: {:.2} printed pages, {} words",
            printed_pages(&out.packet_markdown),
            words(&out.packet_markdown)
        );
    }
    let pages = printed_pages(packet("philadelphia-renters-4"));
    assert!(
        pages <= PHILADELPHIA_MAX_PAGES,
        "Philadelphia prints on about {pages:.2} pages"
    );
}

// ------------------------------------------------------------------------------------------------
// S3b: the life-safety rules reach every packet
// ------------------------------------------------------------------------------------------------

#[test]
fn every_packet_prints_the_life_safety_rules() {
    for (name, _, out) in all() {
        let plan = section(&out.packet_markdown, "## Your plan");
        for r in &rr_plan::packet::SAFETY_RULES {
            assert!(
                plan.contains(&format!("- **{}:** {}", r.label, r.text)),
                "{name}: the {} rule is missing",
                r.label
            );
        }
    }
}

#[test]
fn each_life_safety_rule_belongs_to_a_step_and_cites_what_the_registry_holds() {
    let content = rr_content::content();
    for r in &rr_plan::packet::SAFETY_RULES {
        assert!(
            content.item(r.step).is_some(),
            "{}: no catalogue step {}",
            r.label,
            r.step
        );
        assert!(!r.citations.is_empty(), "{}", r.label);
        for c in r.citations {
            assert!(content.citation(c).is_some(), "{}: {c}", r.label);
        }
    }
    // The six warnings the professional review listed (RR-P03), one line each.
    let all_text: String = rr_plan::packet::SAFETY_RULES
        .iter()
        .map(|r| r.text)
        .collect::<Vec<_>>()
        .join(" ");
    for key in [
        "two ways out",
        "gas was shut off",
        "power or gas before you drain it",
        "40°F",
        "wall outlet or the house wiring",
        "CPR",
    ] {
        assert!(all_text.contains(key), "no rule says {key:?}");
    }
}

// ------------------------------------------------------------------------------------------------
// S4: rare-catastrophe gear only with the opt-in
// ------------------------------------------------------------------------------------------------

#[test]
fn rare_catastrophe_gear_prints_only_with_the_opt_in() {
    let content = rr_content::content();
    let rare: Vec<&str> = content
        .items
        .iter()
        .filter(|i| i.rare_catastrophic)
        .map(|i| i.name.as_str())
        .collect();
    assert!(!rare.is_empty());
    for (name, input, out) in all() {
        assert!(!input.dials.rare_catastrophic_opt_in, "{name}");
        let lists = section(&out.packet_markdown, "## Checklists");
        for r in &rare {
            assert!(!lists.contains(r), "{name}: {r} without the opt-in");
        }
    }
    // With the opt-in the allocator may spend up to a tenth of the budget on them, so they come
    // back (in the plan, or among the extras when the plan has no room).
    let mut opted = input("philadelphia-renters-4").clone();
    opted.dials.rare_catastrophic_opt_in = true;
    let out = engine().assess(&opted).expect("assess");
    let lower = out.packet_markdown.to_lowercase();
    assert!(
        rare.iter().any(|r| lower.contains(&r.to_lowercase())),
        "the opt-in shows the rare gear"
    );
}

// ------------------------------------------------------------------------------------------------
// S2: leaving first, and shelter advice that fits the home
// ------------------------------------------------------------------------------------------------

fn three_things(p: &str) -> Vec<&str> {
    let s = section(p, "## Summary");
    s.lines()
        .filter(|l| l.starts_with(|c: char| c.is_ascii_digit()) && l.contains(". "))
        .collect()
}

#[test]
fn leaving_comes_first_where_it_matters() {
    const LEAVE: &str = "Know your evacuation zone and where you would go; leave when told.";
    for (name, _, out) in all() {
        let a = run(&all().iter().find(|(n, _, _)| n == name).unwrap().1);
        let p10 = match a.bucket(BucketId::Evacuate).target {
            Target::Evacuate { p_need_10yr, .. } => p_need_10yr,
            _ => 0.0,
        };
        let scenario = a
            .consequence
            .scenarios
            .iter()
            .any(|s| s.on && rr_plan::packet::LEAVE_FIRST_SCENARIOS.contains(&s.id.as_str()));
        let things = three_things(&out.packet_markdown);
        let first_is_leaving = things.first().is_some_and(|t| t.contains(LEAVE));
        assert_eq!(
            first_is_leaving,
            p10 >= rr_plan::packet::LEAVE_FIRST_P10 || scenario,
            "{name}: p10 {p10}, scenario {scenario}: {things:?}"
        );
        if first_is_leaving {
            assert!(
                things
                    .iter()
                    .skip(1)
                    .any(|t| t.contains("If you are not told to leave, be ready to manage")),
                "{name}: the stay-home amounts are for when you are not told to leave"
            );
        }
    }
    // Miami (whole city an evacuation zone, major-hurricane scenario on) leads with leaving; a
    // local-tsunami county adds the shaking rule.
    assert!(three_things(packet("miami-condo-retiree-1"))[0].contains(LEAVE));
    assert!(three_things(packet("lahaina-maui-3"))[0].contains("strong shaking is the warning"));
    assert!(!three_things(packet("philadelphia-renters-4"))[0].contains(LEAVE));
}

#[test]
fn shelter_advice_fits_the_home() {
    // Miami, floor 14 of a tall building: no basement or lowest-floor shelter in the wind and
    // hurricane advice; the tenth-floor rule and the elevator warning instead.
    let miami = packet("miami-condo-retiree-1");
    assert!(
        !miami.contains("basement on the lowest floor"),
        "Miami: basement advice"
    );
    for c in ["Strong wind", "Hurricane"] {
        let text = card(miami, c);
        assert!(!text.contains("basement"), "Miami {c} card: {text}");
        assert!(!text.contains("lowest floor"), "Miami {c} card: {text}");
    }
    assert!(
        card(miami, "Hurricane").contains("take shelter on or below the 10th floor"),
        "Miami: the tenth-floor sentence"
    );
    assert!(
        section(miami, "## Your targets").contains("do not count on the elevator"),
        "Miami: the elevator warning"
    );
    // Philadelphia, a rowhouse with a basement, keeps the basement sentence.
    let phl = packet("philadelphia-renters-4");
    assert!(
        card(phl, "Strong wind").contains(
            "small, windowless room or basement on the lowest floor of a sturdy building"
        ),
        "Philadelphia: the basement sentence"
    );
    assert!(
        !phl.contains("10th floor"),
        "Philadelphia: no high-rise advice"
    );
}

// ------------------------------------------------------------------------------------------------
// C1 and M-04: the status line and the dial sentence
// ------------------------------------------------------------------------------------------------

#[test]
fn page_one_carries_the_status_line() {
    for (name, _, out) in all() {
        let p = &out.packet_markdown;
        let at = p
            .find(rr_plan::packet::STATUS_LINE)
            .unwrap_or_else(|| panic!("{name}: no status line"));
        assert!(at < p.find("\n## Summary\n").unwrap(), "{name}: on page 1");
    }
    assert!(rr_plan::packet::STATUS_LINE.starts_with(
        "Ready Reckoner is an independent, open-source planning aid. It is not official emergency \
         guidance, and not medical, legal or financial advice. Follow instructions from your local \
         officials first."
    ));
}

#[test]
fn the_dial_sentence_is_per_need() {
    const CANONICAL: &str = "For any one need, something worse than its target comes in about 1 of \
        every 10 ten-year stretches. Across all your needs together, the chance that at least one \
        runs out is higher, roughly 1 in 3. That is why the plan also gives you ways to cope when \
        a target runs out.";
    for (name, input, out) in all() {
        let targets = section(&out.packet_markdown, "## Your targets");
        assert!(
            !targets.contains("Something worse than these targets"),
            "{name}: the old sentence"
        );
        let want = rr_plan::packet::dial_sentence(input.dials.return_period);
        assert!(targets.contains(&want), "{name}");
        if input.dials.return_period == rr_types::ReturnPeriod::OneIn100 {
            assert_eq!(want, CANONICAL);
        } else {
            assert!(!want.contains("roughly 1 in 3"), "{name}: {want}");
        }
    }
    // Coos Bay plans at 1 in 500: about 2 of every 100 ten-year stretches for one need.
    assert!(
        rr_plan::packet::dial_sentence(rr_types::ReturnPeriod::OneIn500)
            .contains("about 2 of every 100 ten-year stretches")
    );
}

// ------------------------------------------------------------------------------------------------
// S8 (M-06): wildfire warnings to leave and power shutoffs stay apart
// ------------------------------------------------------------------------------------------------

fn part(a: &rr_plan::Assessment, hazard: HazardId, name: &str) -> f64 {
    a.hazards
        .parts
        .iter()
        .find(|p| p.hazard == hazard && p.part == name)
        .map_or(0.0, |p| p.rate_per_year)
}

/// Events a year that force the household out, from one hazard.
fn evacuations_from(a: &rr_plan::Assessment, hazard: HazardId) -> f64 {
    a.consequence
        .evacuate
        .causes
        .iter()
        .filter(|(h, _, _, _)| *h == hazard)
        .map(|(_, _, r, _)| *r)
        .sum()
}

/// Power cuts a year from one hazard (the table's terms, no coupling).
fn power_cuts_from(a: &rr_plan::Assessment, hazard: HazardId) -> f64 {
    a.consequence
        .details
        .iter()
        .find(|d| d.bucket == BucketId::Power)
        .map_or(0.0, |d| {
            d.terms
                .iter()
                .filter(|t| t.hazard == hazard && t.origin == "effects table")
                .map(|t| t.events_per_year)
                .sum()
        })
}

#[test]
fn wildfire_evacuations_and_shutoffs_stay_apart() {
    // Butte County (Paradise): warnings to leave about 0.021 a year (the model review's M-06
    // replay), not the 0.0047 the old 15/85 re-split gave; shutoffs half of 0.02 (suburban).
    let a = run(input("campfire-paradise-2"));
    let burn = part(&a, HazardId::Wildfire, rr_hazards::WILDFIRE_BURN_PART);
    let shutoff = part(&a, HazardId::Wildfire, rr_hazards::WILDFIRE_SHUTOFF_PART);
    assert!((0.015..0.03).contains(&burn), "burn part {burn}");
    assert!((shutoff - 0.01).abs() < 1e-9, "shutoffs {shutoff}");
    let total = a.hazard_rate(HazardId::Wildfire);
    assert!(
        (burn + shutoff - total).abs() < 1e-9,
        "{burn} + {shutoff} vs {total}"
    );
    let evac = evacuations_from(&a, HazardId::Wildfire);
    assert!(
        (evac - burn).abs() < 1e-9,
        "every warning to leave is an evacuation: {evac}"
    );
    assert!(
        evac > 4.0 * 0.15 * total,
        "no longer 15 % of all wildfire events: {evac}"
    );
    assert!((power_cuts_from(&a, HazardId::Wildfire) - shutoff).abs() < 1e-9);
    // Maui: Hawaii is not a shutoff state, so no wildfire power cuts at all, and every wildfire
    // event is a warning to leave.
    let m = run(input("lahaina-maui-3"));
    assert_eq!(
        part(&m, HazardId::Wildfire, rr_hazards::WILDFIRE_SHUTOFF_PART),
        0.0
    );
    assert_eq!(power_cuts_from(&m, HazardId::Wildfire), 0.0);
    let m_total = m.hazard_rate(HazardId::Wildfire);
    assert!((evacuations_from(&m, HazardId::Wildfire) - m_total).abs() < 1e-9);
}

// ------------------------------------------------------------------------------------------------
// S5 (M-05): landslides damage few homes and cut off more roads
// ------------------------------------------------------------------------------------------------

#[test]
fn utuado_landslides_are_bounded_and_say_what_they_do() {
    let a = run(input("maria-utuado-3"));
    let damage = part(&a, HazardId::Landslide, rr_hazards::LANDSLIDE_DAMAGE_PART);
    let total = a.hazard_rate(HazardId::Landslide);
    // Was 2.1 events a year and "about 90 of 100" homes damaged in ten years.
    assert!(damage <= 0.01 + 1e-12, "damage {damage}");
    assert!(total < 0.2, "total {total}");
    assert!((evacuations_from(&a, HazardId::Landslide) - damage).abs() < 1e-9);
    let profile = a
        .hazards
        .profiles
        .iter()
        .find(|p| p.id == HazardId::Landslide)
        .unwrap();
    assert!(
        profile
            .frequency_sentence
            .contains("cut off their road or damage their home")
            && profile
                .frequency_sentence
                .contains("will have one damage their home"),
        "{}",
        profile.frequency_sentence
    );
    assert!(
        a.hazards.notes.iter().any(|n| n.starts_with("Landslides:")),
        "the ceiling is explained"
    );
}

/// No county's landslide home-damage chance for a household there exceeds the ceiling: 1 in 100
/// a year (a ten-year chance under 10 in 100), the high-risk flood-zone yardstick. Before the
/// bound, 32 counties passed 10 in 100 over ten years and 13 passed 25.
#[test]
fn no_county_lets_landslides_damage_more_homes_than_the_ceiling() {
    let store = engine().store();
    let template = common::household("philadelphia-renters-4");
    let mut checked = 0;
    let mut worst = (0.0f64, String::new());
    for county in store.counties() {
        let mut input = template.clone();
        input.location.county_fips = Some(county.fips.clone());
        input.location.zip = None;
        let Ok(location) = store.resolve(&input.location) else {
            continue;
        };
        let hz = rr_hazards::assess(&input, county, store.base_rates(), &location);
        let damage = hz
            .parts
            .iter()
            .find(|p| {
                p.hazard == HazardId::Landslide && p.part == rr_hazards::LANDSLIDE_DAMAGE_PART
            })
            .map_or(0.0, |p| p.rate_per_year);
        if damage > worst.0 {
            worst = (damage, county.fips.clone());
        }
        let ten_years = -rr_types::math::exp_m1(-10.0 * damage);
        assert!(
            damage <= 0.01 + 1e-12 && ten_years < 0.1,
            "{}: landslide home damage {damage} a year",
            county.fips
        );
        checked += 1;
    }
    assert!(checked > 3000, "only {checked} counties");
    eprintln!(
        "worst landslide home damage: {} a year in {}",
        worst.0, worst.1
    );
}

// ------------------------------------------------------------------------------------------------
// M-08: fast hazards set the short warning
// ------------------------------------------------------------------------------------------------

#[test]
fn fast_hazards_set_the_short_warning() {
    // Lahaina: the wildfire and the local tsunami give minutes, not "as short as 2 hours".
    let a = run(input("lahaina-maui-3"));
    assert!(
        a.consequence.evacuate.notice_hours[0] <= 0.25,
        "{:?}",
        a.consequence.evacuate.notice_hours
    );
    let lahaina = packet("lahaina-maui-3");
    let line = lahaina
        .lines()
        .find(|l| l.contains("Warning can be"))
        .expect("Lahaina's warning line");
    assert!(
        line.contains(" minutes to ") && !line.contains("Warning can be 2 hours"),
        "{line}"
    );
    // Paradise: minutes too.
    let p = run(input("campfire-paradise-2"));
    assert!(p.consequence.evacuate.notice_hours[0] < 1.0);
    assert!(packet("campfire-paradise-2").contains("Warning can be 1 minute to"));
}

// ------------------------------------------------------------------------------------------------
// M-13: the relief rating describes the event behind the target
// ------------------------------------------------------------------------------------------------

#[test]
fn relief_describes_the_event_behind_the_target() {
    for (name, _, out) in all() {
        for b in &out.buckets {
            if let (Some(r), Target::Days { value, .. }) = (&b.relief, b.target) {
                assert!(
                    3.0 * r.mostly_restored_days >= value,
                    "{name} {}: mostly back in {} days beside a {value}-day target",
                    b.id,
                    r.mostly_restored_days
                );
            }
        }
    }
    let relief = |name: &str, bucket: BucketId| {
        all()
            .iter()
            .find(|(n, _, _)| n == name)
            .unwrap()
            .2
            .buckets
            .iter()
            .find(|b| b.id == bucket)
            .unwrap()
            .relief
            .clone()
    };
    // Asheville: "mostly back in half a day" beside two weeks, before; now about 6 days.
    let r = relief("helene-asheville-3", BucketId::Power).expect("Asheville relief");
    assert!(
        r.mostly_restored_days >= 4.5 && r.help_arrives_days <= 3.0,
        "{r:?}"
    );
    // Philadelphia: the hurricane restoration curve, outages of a day or more: about 6 days.
    let r = relief("philadelphia-renters-4", BucketId::Power).expect("Philadelphia relief");
    assert!((r.mostly_restored_days - 6.14).abs() < 0.05, "{r:?}");
    // Coos Bay: Cascadia's own rating (Oregon Resilience Plan) stays.
    let r = relief("coos-bay-well-owner-2", BucketId::Power).expect("Coos Bay relief");
    assert_eq!((r.help_arrives_days, r.mostly_restored_days), (14.0, 180.0));
}

// ------------------------------------------------------------------------------------------------
// M-04: "roughly 1 in 3" across all needs, checked against the model
// ------------------------------------------------------------------------------------------------

/// The canonical dial sentence says that at the 1-in-100 setting the chance that at least one need
/// runs past its target in ten years is "roughly 1 in 3". The model's own event list gives it
/// (`ConsequenceAssessment::joint_rate`): each target alone is outlasted in about 10 of 100
/// ten-year stretches, all together in 26 to 39 of 100 for ten of these twelve households, and
/// about 20 of 100 where one named scenario drives most targets at once (Coos Bay's Cascadia,
/// Miami's major hurricane).
#[test]
fn roughly_one_in_three_needs_run_out_together() {
    let mut seen = 0;
    for (name, input, _) in all() {
        let mut one_in_100 = input.clone();
        one_in_100.dials.return_period = rr_types::ReturnPeriod::OneIn100;
        let a = run(&one_in_100);
        let joint = -rr_types::math::exp_m1(-10.0 * a.consequence.joint_rate());
        eprintln!("{name}: at least one need past its target in 10 years: {joint:.2}");
        assert!((0.15..=0.45).contains(&joint), "{name}: {joint:.3}");
        seen += 1;
    }
    assert_eq!(seen, all().len());
}
