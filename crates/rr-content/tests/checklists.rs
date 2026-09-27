//! The incident checklists in `content/checklists/` (DESIGN-DELTA-v3 §5): the two exemplars the
//! content workstreams copy parse, pass the validator, name their sources' titles in their
//! footnotes (as guidance blocks do), and
//! render for real households: filled where the household answered, blank markers where it did
//! not, steps kept or dropped by its conditions.

use rr_content::checklist::{self, Onset, blank_marker};
use rr_content::policy::HouseholdFacts;
use rr_content::{Checklist, GuidanceKind};
use rr_types::{AgeBand, Contact, PlanInput, PoweredDevice};

fn content() -> &'static rr_content::Content {
    rr_content::content()
}

fn block(id: &str) -> &'static Checklist {
    content()
        .checklist(id)
        .unwrap_or_else(|| panic!("no checklist `{id}`"))
}

/// A household as the conditions see it, from its plan input (every hazard counts as relevant).
struct Household<'a>(&'a PlanInput);

impl HouseholdFacts for Household<'_> {
    fn hazard_relevant(&self, _: &str) -> bool {
        true
    }
    fn home(&self) -> rr_types::HousingKind {
        self.0.housing.kind
    }
    fn has_access_need(&self, need: &str) -> bool {
        self.0
            .people
            .iter()
            .any(|p| p.access_needs.iter().any(|n| n.as_str() == need))
    }
    fn has_item(&self, _: &str) -> bool {
        false
    }
    fn has_benefit(&self, benefit: &str) -> bool {
        self.0
            .finances
            .benefits
            .iter()
            .any(|b| b.as_str() == benefit)
    }
    fn has_children(&self) -> bool {
        self.0.people.iter().any(|p| {
            matches!(
                p.age_band,
                AgeBand::Infant | AgeBand::Toddler | AgeBand::Child | AgeBand::Teen
            )
        })
    }
    fn has_pets(&self) -> bool {
        let p = &self.0.pets;
        p.dogs + p.cats + p.small + p.large_animals > 0
    }
    fn has_vehicle(&self) -> bool {
        !self.0.mobility.vehicles.is_empty()
    }
    fn has_powered_device(&self) -> bool {
        self.0
            .people
            .iter()
            .any(|p| p.medical.powered_device != PoweredDevice::None)
    }
}

fn contact(c: Option<&Contact>) -> Option<String> {
    let c = c?;
    let parts: Vec<&str> = [c.name.as_deref(), c.phone.as_deref()]
        .into_iter()
        .flatten()
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// The household's answer for a placeholder, as the binder would fill it (a sketch of rr-plan's
/// job: the family plan's v2 fields, the v3 home and neighbourhood groups, the county).
fn answer(input: &PlanInput, name: &str, county: &str) -> Option<String> {
    let plan = input.family_plan.as_ref();
    let home = plan.and_then(|p| p.home.as_ref());
    let hood = plan.and_then(|p| p.neighbourhood.as_ref());
    match name {
        "meeting_near" => plan?.meeting_place_near.clone(),
        "meeting_far" => plan?.meeting_place_far.clone(),
        "shelter_home" => plan?.shelter_spot_home.clone(),
        "shelter_work" => plan?.shelter_spot_work.clone(),
        "where_go" => plan?.where_we_would_go.clone(),
        "out_of_area_contact" => contact(plan?.out_of_area_contact.as_ref()),
        "gas_shutoff" => plan?.shutoff_gas.clone(),
        "water_shutoff" => plan?.shutoff_water.clone(),
        "electric_panel" => plan?.shutoff_electric.clone(),
        "electric_utility" => contact(home?.electric_utility.as_ref()),
        "gas_utility" => contact(home?.gas_utility.as_ref()),
        "water_utility" => contact(home?.water_utility.as_ref()),
        "hospital" => contact(hood?.hospital.as_ref()),
        "pharmacy" => contact(hood?.pharmacy.as_ref()),
        "alerts" => hood?.alerts.clone(),
        "county" => Some(county.to_owned()),
        other => panic!("an exemplar uses `{{{other}}}`, which the sketch does not fill"),
    }
}

fn render(id: &str, household: &str, county: &str) -> checklist::ChecklistSections {
    let input = rr_types::fixtures::get(household).unwrap();
    block(id).render_for(&Household(&input), &|name: &str| {
        answer(&input, name, county)
    })
}

fn all_text(s: &checklist::ChecklistSections) -> String {
    let mut parts = vec![s.use_when.clone()];
    for list in [
        &s.do_first,
        &s.then,
        &s.leave_or_stay,
        &s.where_who,
        &s.do_not,
        &s.when_over,
    ] {
        parts.extend(list.iter().cloned());
    }
    parts.join("\n")
}

#[test]
fn the_exemplars_exist_and_pass_the_validator() {
    for (id, hazard) in [
        ("check_house_fire", "hazard:house_fire"),
        ("check_tornado", "hazard:tornado"),
    ] {
        let c = block(id);
        assert_eq!(c.meta.kind, Some(GuidanceKind::Checklist));
        assert_eq!(c.onset, Onset::Now, "{id}");
        assert_eq!(c.pages, 1);
        assert_eq!(
            content().checklist_for(hazard).map(|c| c.meta.id.as_str()),
            Some(id)
        );
        assert!(
            c.word_count() <= checklist::WORDS_ONE_PAGE,
            "{id}: {} words",
            c.word_count()
        );
        let grade = c.reading_grade().unwrap();
        assert!(grade <= 8.0, "{id}: grade {grade:.1}");
        println!("{id}: {} words, grade {grade:.1}", c.word_count());
    }
    let report = rr_content::validate_embedded();
    let about: Vec<String> = report
        .findings
        .iter()
        .filter(|f| f.location.starts_with("checklists/check_"))
        .map(ToString::to_string)
        .collect();
    assert!(about.is_empty(), "{about:#?}");
    // The catalogue lists them with the guidance blocks.
    let catalogue = content().catalogue();
    let listed: Vec<&str> = catalogue
        .guidance
        .iter()
        .filter(|g| g.kind == Some(GuidanceKind::Checklist))
        .map(|g| g.id.as_str())
        .collect();
    assert!(listed.contains(&"check_house_fire") && listed.contains(&"check_tornado"));
    assert!(content().checklist_for("hazard:zombies").is_none());
}

#[test]
fn checklist_footnotes_copy_the_registry() {
    for c in content().checklists() {
        for (id, text) in c.footnote_definitions() {
            let cited = content()
                .citation(&id)
                .unwrap_or_else(|| panic!("{}: unknown citation {id}", c.meta.id));
            let title = cited.title.trim_end_matches('.');
            assert!(
                text.contains(title),
                "{}: footnote {id} does not name the source's title `{title}`",
                c.meta.id
            );
        }
    }
}

#[test]
fn the_exemplars_render_for_philadelphia() {
    // Philadelphia: a rowhouse, one car, a dog, children; its v3 answers give the alerts line,
    // and its v2 family plan is empty, so the meeting places are blanks.
    let fire = render(
        "check_house_fire",
        "philadelphia-renters-4",
        "Philadelphia County, Pennsylvania",
    );
    assert_eq!(fire.do_first.len(), 5);
    assert_eq!(
        fire.do_first[0],
        "**Get out.** Leave by the nearest safe way; do not stop for anything.[^usfa_home_fire_escape_plans]"
    );
    let blank = blank_marker(30);
    assert_eq!(
        fire.leave_or_stay[0],
        format!(
            "**Leave if** there is fire or smoke in the home. Go to {blank}. {{ref:neighbourhood}}"
        )
    );
    assert_eq!(
        fire.where_who[0],
        format!("Meeting place near home: {blank}")
    );
    assert_eq!(fire.where_who[1], "Second way out of each bedroom:");

    let tornado = render(
        "check_tornado",
        "philadelphia-renters-4",
        "Philadelphia County, Pennsylvania",
    );
    let first: Vec<&str> = tornado
        .do_first
        .iter()
        .map(|s| s.split("**").nth(1).unwrap())
        .collect();
    assert_eq!(
        first,
        [
            "Go to your shelter now.",
            "Get down low, inside.",
            "Cover your head.",
            "Stay away from windows."
        ],
        "a rowhouse keeps the house step and drops the apartment and mobile-home ones"
    );
    assert!(
        tornado.then.iter().any(|s| s.contains(
            "Follow the warning on The city's text alerts, and the news radio station until it ends."
        )),
        "{:?}",
        tornado.then
    );
    assert!(
        tornado.then.iter().any(|s| s.contains("stay buckled in")),
        "one car"
    );
    assert!(
        !tornado.then.iter().any(|s| s.contains("elevators")),
        "not a flat"
    );
    let text = all_text(&tornado);
    assert!(!text.contains("{if:") && !text.contains("{/if}"));
    for (name, _) in checklist::PLACEHOLDERS {
        assert!(!text.contains(&format!("{{{name}}}")), "{name} left in");
    }
}

#[test]
fn the_exemplars_render_with_detroits_own_words() {
    // Detroit filled in the v2 family plan: meeting places, shelter spot, where it would go.
    let input = rr_types::fixtures::get("detroit-snap-3").unwrap();
    let plan = input.family_plan.as_ref().unwrap();
    let fire = render(
        "check_house_fire",
        "detroit-snap-3",
        "Wayne County, Michigan",
    );
    let near = plan.meeting_place_near.as_deref().unwrap();
    let go = plan.where_we_would_go.as_deref().unwrap();
    assert_eq!(
        fire.where_who[0],
        format!("Meeting place near home: {near}")
    );
    assert!(
        fire.leave_or_stay[2].contains(go),
        "{:?}",
        fire.leave_or_stay
    );
    let tornado = render("check_tornado", "detroit-snap-3", "Wayne County, Michigan");
    let shelter = plan.shelter_spot_home.as_deref().unwrap();
    assert_eq!(
        tornado.do_first[0],
        format!("**Go to your shelter now.** Get to {shelter}.[^ready_gov_tornadoes]")
    );
    assert!(
        !tornado.then.iter().any(|s| s.contains("car")),
        "Detroit has no car: the car steps are dropped"
    );
}

#[test]
fn the_exemplars_render_blanks_for_a_household_with_no_answers() {
    // Chicago answered none of the optional questions: every placeholder is a blank marker,
    // except the county, which the engine always knows.
    let tornado = render(
        "check_tornado",
        "chicago-student-zero-budget-1",
        "Cook County, Illinois",
    );
    let blank30 = blank_marker(30);
    assert_eq!(
        tornado.do_first[0],
        format!("**Go to your shelter now.** Get to {blank30}.[^ready_gov_tornadoes]")
    );
    assert_eq!(
        tornado.where_who,
        [
            format!("Shelter spot at home: {blank30}"),
            format!("Shelter spot at work or school: {blank30}")
        ]
    );
    assert!(
        tornado
            .then
            .iter()
            .any(|s| s.contains(&format!("Follow the warning on {blank30} until it ends.")))
    );
    for text in [
        all_text(&tornado),
        all_text(&render(
            "check_house_fire",
            "chicago-student-zero-budget-1",
            "Cook County, Illinois",
        )),
    ] {
        for marker in [
            "{if:",
            "{/if}",
            "{shelter_home}",
            "{meeting_near}",
            "{where_go}",
            "{alerts}",
        ] {
            assert!(!text.contains(marker), "{marker} left in:\n{text}");
        }
        // Cross-references and footnotes are kept for rr-plan.
        assert!(text.contains("{ref:getting_out}") && text.contains("[^"));
    }
}
