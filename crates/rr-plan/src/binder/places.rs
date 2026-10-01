//! Tab 3, Home and places (DESIGN-DELTA-v3 §4.2; brief 2): the home (shut-offs, utilities,
//! where things are, the safest spots and the shelter plan), one page per place in the
//! household's life, the neighborhood (meeting places, help nearby, the state's pointers, the
//! county's hospitals with an emergency room, a map) and getting out (where to go, two ways out,
//! when to leave first, two maps).

use rr_types::binder::{Block, FieldRow, Inline, MapSlot, MapSlotKind, Page, PageKind, Table};
use rr_types::{BucketId, Place, PlaceKind};

use super::{Bx, b, contact_short, contact_text, heading, page, row, row_lines, t};
use crate::packet::{Ctx, text};

/// One place page: its id, title, the people who go there (by index) and the place, merged
/// from every person who named it.
#[derive(Debug, Clone)]
pub(super) struct PlaceEntry {
    pub id: String,
    pub title: String,
    pub people: Vec<usize>,
    pub place: Place,
}

/// The key places are told apart by: the trimmed, lower-cased name.
fn key(p: &Place) -> Option<String> {
    p.name
        .as_deref()
        .map(|n| n.trim().to_lowercase())
        .filter(|n| !n.is_empty())
}

/// The distinct places from the people's profiles, in the household's order: one per name
/// (trimmed, lower-cased), and one per person for a place with no name ("Where Dana spends the
/// day").
pub(super) fn distinct(cx: &Ctx<'_>) -> Vec<PlaceEntry> {
    let mut out: Vec<PlaceEntry> = Vec::new();
    for (i, person) in cx.a.input.people.iter().enumerate() {
        let Some(place) = person.profile.as_ref().and_then(|p| p.place.as_ref()) else {
            continue;
        };
        let k = key(place);
        if let Some(e) = k
            .as_ref()
            .and_then(|k| out.iter_mut().find(|e| key(&e.place).as_ref() == Some(k)))
        {
            e.people.push(i);
            merge(&mut e.place, place);
            continue;
        }
        let title = match place.name.as_deref() {
            Some(n) => n.to_owned(),
            None => format!("Where {} spends the day", super::person_title(i, person)),
        };
        out.push(PlaceEntry {
            id: format!("place_{}", out.len() + 1),
            title,
            people: vec![i],
            place: place.clone(),
        });
    }
    out
}

/// Fills `into`'s empty fields from `from`.
fn merge(into: &mut Place, from: &Place) {
    for (a, b) in [
        (&mut into.address, &from.address),
        (&mut into.phone, &from.phone),
        (&mut into.plan, &from.plan),
        (&mut into.pickup, &from.pickup),
        (&mut into.safest_spot, &from.safest_spot),
    ] {
        if a.is_none() {
            a.clone_from(b);
        }
    }
}

/// The place page a person's place is on.
pub(super) fn page_for(cx: &Ctx<'_>, person: usize) -> Option<String> {
    distinct(cx)
        .into_iter()
        .find(|e| e.people.contains(&person))
        .map(|e| e.id)
}

/// The kind of a place in words.
fn kind_words(k: PlaceKind) -> &'static str {
    match k {
        PlaceKind::Work => "work",
        PlaceKind::School => "school",
        PlaceKind::Childcare => "child care",
        PlaceKind::Other => "other",
    }
}

/// A place on one line, as written: "Riverside Warehouse (work)", or its kind alone.
pub(super) fn place_line(p: &Place, _who: &str) -> String {
    match p.name.as_deref() {
        Some(n) => format!("{n} ({})", kind_words(p.kind)),
        None => text::upper_first(kind_words(p.kind)),
    }
}

pub(super) fn pages(bx: &Bx<'_>, places: &[PlaceEntry]) -> Vec<Page> {
    let mut pages = vec![home(bx)];
    for (n, p) in places.iter().enumerate() {
        pages.push(place_page(bx, p, n == 0));
    }
    pages.push(neighbourhood(bx, places.is_empty()));
    pages.push(getting_out(bx));
    pages
}

fn title(id: &str) -> &'static str {
    super::FIXED_PAGES
        .iter()
        .find(|p| p.0 == id)
        .map_or("", |p| p.2)
}

/// The checklists that turn to the Home page, when the binder has them.
const HOME_CHECKLISTS: [&str; 13] = [
    "check_house_fire",
    "check_gas_leak_or_co",
    "check_water_leak",
    "check_power_outage",
    "check_shelter_in_place",
    "check_tornado",
    "check_severe_thunderstorm",
    "check_hurricane",
    "check_earthquake",
    "check_chemical_release",
    "check_wildfire_smoke",
    "check_nuclear_plant",
    "check_nuclear_attack",
];

/// "Checklists that turn here: (Tab 6, House fire) · (Tab 6, Tornado)": links to the pages
/// in this binder among `ids`.
fn turn_here(bx: &Bx<'_>, lead: &str, ids: &[&str]) -> Option<Block> {
    let links: Vec<Inline> = ids.iter().filter_map(|id| bx.link_inline(id)).collect();
    if links.is_empty() {
        return None;
    }
    let mut v = vec![b(lead.trim_end().to_owned())];
    for (i, l) in links.into_iter().enumerate() {
        v.push(t(if i > 0 { " · " } else { " " }));
        v.push(l);
    }
    Some(Block::Para(v))
}

/// The home (§4.2): the address, the shut-offs with their tools, the companies to call, the
/// insurer and the landlord, where things are, the safest spots and the neighbors who check;
/// then the shelter plan (`plan_shelter`) for this household, and the checklists that turn here.
fn home(bx: &Bx<'_>) -> Page {
    let a = bx.a();
    let fp = super::family(&bx.cx);
    let h = fp.home.clone().unwrap_or_default();
    let landlord = if a.input.housing.tenure == rr_types::Tenure::Rent {
        "Landlord"
    } else {
        "Mortgage company"
    };
    let mut rows: Vec<FieldRow> = vec![row("Address", h.address.as_deref())];
    let gas_home = matches!(
        a.input.housing.heating,
        rr_types::Heating::Gas | rr_types::Heating::Propane
    ) || a.input.housing.cooking == Some(rr_types::CookingFuel::Gas);
    if gas_home || fp.shutoff_gas.is_some() {
        rows.push(row("Gas shut-off and its tool", fp.shutoff_gas.as_deref()));
    }
    rows.push(row("Water shut-off", fp.shutoff_water.as_deref()));
    rows.push(row("Electrical panel", fp.shutoff_electric.as_deref()));
    rows.extend([
        row(
            "Electric company (outage number)",
            contact_short(h.electric_utility.as_ref()).as_deref(),
        ),
        row(
            "Gas company (outage number)",
            contact_short(h.gas_utility.as_ref()).as_deref(),
        ),
        row(
            "Water company (outage number)",
            contact_short(h.water_utility.as_ref()).as_deref(),
        ),
        row("Home insurer", contact_short(h.insurer.as_ref()).as_deref()),
        row("Policy number", h.policy_number.as_deref()),
        row(
            landlord,
            contact_text(h.landlord_or_mortgage.as_ref()).as_deref(),
        ),
    ]);
    let mut blocks = vec![Block::Fields(rows)];
    blocks.push(heading(2, "Where things are"));
    blocks.push(Block::Fields(vec![
        row("Emergency kit", h.where_kit.as_deref()),
        row("Documents", h.where_documents.as_deref()),
        row("Cash", h.where_cash.as_deref()),
        row("Spare keys", h.where_keys.as_deref()),
    ]));
    blocks.push(heading(2, "Safest spots"));
    blocks.push(Block::Fields(vec![
        row("Safest spot at home", fp.shelter_spot_home.as_deref()),
        row_lines(
            "Neighbors who check on us",
            fp.neighbours_who_check.as_deref(),
            2,
        ),
    ]));
    if let Some(g) = bx.cx.blocks_for("plan:shelter").into_iter().next() {
        let rendered = bx.cx.guidance(g, None, None);
        blocks.extend(bx.prose_blocks(&rendered));
    }
    blocks.extend(turn_here(
        bx,
        "Checklists that turn here: ",
        &HOME_CHECKLISTS,
    ));
    page("home", title("home"), PageKind::Home, blocks)
}

/// One place in the household's life (§4.2): who goes there, what it is, its address and phone,
/// its own plan, the pick-up rules and its safest spot, as written. The first place page also
/// carries the general work and school answers (the v2 family plan's).
fn place_page(bx: &Bx<'_>, e: &PlaceEntry, first: bool) -> Page {
    let a = bx.a();
    let who: Vec<String> = e
        .people
        .iter()
        .map(|&i| super::person_title(i, &a.input.people[i]))
        .collect();
    let p = &e.place;
    let mut blocks = vec![Block::Fields(vec![
        row("Who goes there", Some(&text::join_and(&who))),
        row("What it is", Some(&text::upper_first(kind_words(p.kind)))),
        row("Address", p.address.as_deref()),
        row("Phone", p.phone.as_deref()),
        row_lines("Its emergency plan", p.plan.as_deref(), 3),
        row_lines("Pick-up", p.pickup.as_deref(), 2),
        row("Safest spot there", p.safest_spot.as_deref()),
    ])];
    if first {
        blocks.extend(general_answers(bx));
    }
    let mut v = vec![t(
        "If you are apart when it happens, meet as your plan says: ",
    )];
    v.extend(bx.link_inline("neighbourhood"));
    v.push(t(". Who to call: "));
    v.extend(bx.link_inline("contacts"));
    v.push(t("."));
    blocks.push(Block::Para(v));
    page(&e.id, &e.title, PageKind::Place, blocks)
}

/// The v2 family plan's general answers about work and school, on the first place page or, with
/// no places, on the Neighborhood page.
fn general_answers(bx: &Bx<'_>) -> Vec<Block> {
    let fp = super::family(&bx.cx);
    let children = super::people::has_children(&bx.a().input.people);
    let mut rows = vec![row_lines(
        "What each of us does at work or school",
        fp.work_plans.as_deref(),
        2,
    )];
    if children || fp.school_pickup.is_some() {
        rows.push(row_lines(
            "Who picks up the children",
            fp.school_pickup.as_deref(),
            2,
        ));
    }
    rows.push(row(
        "Shelter spot at work or school",
        fp.shelter_spot_work.as_deref(),
    ));
    vec![heading(2, "Work and school"), Block::Fields(rows)]
}

/// The most hospitals the Neighborhood page lists.
pub const HOSPITALS_MAX: usize = 12;

/// The neighborhood (§4.2): meeting places, help nearby, how alerts arrive, the state's
/// pointers (the v2 packet's Local help), the county's hospitals with an emergency room (CMS),
/// and the neighborhood map.
fn neighbourhood(bx: &Bx<'_>, no_places: bool) -> Page {
    let cx = &bx.cx;
    let fp = super::family(cx);
    let hood = fp.neighbourhood.clone().unwrap_or_default();
    let mut blocks = vec![Block::Fields(vec![
        row("Meeting place near home", fp.meeting_place_near.as_deref()),
        row(
            "Meeting place outside the neighborhood",
            fp.meeting_place_far.as_deref(),
        ),
        row(
            "Hospital with an emergency room",
            contact_text(hood.hospital.as_ref()).as_deref(),
        ),
        row(
            "Urgent care",
            contact_text(hood.urgent_care.as_ref()).as_deref(),
        ),
        row("Pharmacy", contact_text(hood.pharmacy.as_ref()).as_deref()),
        row(
            "Community shelter",
            contact_text(hood.shelter.as_ref()).as_deref(),
        ),
        row(
            "County emergency office",
            contact_text(hood.county_emergency_office.as_ref()).as_deref(),
        ),
        row("How we get local alerts", hood.alerts.as_deref()),
    ])];
    if no_places {
        blocks.extend(general_answers(bx));
    }

    // The state's row: zones, registry, alerts, refills.
    let loc = &cx.a.location;
    match cx.content.states.row(&loc.state_abbr) {
        Some(state) => {
            blocks.push(heading(
                2,
                &format!(
                    "Where to start in {} (checked {})",
                    state.name,
                    text::date(state.checked)
                ),
            ));
            let rows: Vec<Vec<Vec<Inline>>> = state
                .lines()
                .into_iter()
                .map(|l| {
                    let mut cell = vec![t(l.text.clone())];
                    cell.extend(bx.cite(&l.sources));
                    vec![vec![t(l.topic)], cell]
                })
                .collect();
            blocks.push(Block::Table(Table {
                header: vec!["For".to_owned(), "Where to start".to_owned()],
                rows,
            }));
        }
        None => blocks.push(Block::Para(vec![t(
            "Your county emergency management office can tell you your evacuation zone, any \
             registry for people who may need help, and how to get local alerts.",
        )])),
    }

    // The county's hospitals with an emergency room (CMS), when the list is loaded.
    if let Some(list) = bx.extras.hospitals {
        let county = crate::packet::summary::place(cx);
        let released = list
            .released
            .map(|d| format!(" of {}", text::date(d)))
            .unwrap_or_default();
        if list.rows.is_empty() {
            let mut v = vec![t(format!(
                "The federal list of hospitals{released} shows no hospital with an emergency \
                 room in {county}. Find the nearest one outside the county, and write it in \
                 above."
            ))];
            v.extend(bx.cite_strs(&[HOSPITALS_CITATION]));
            blocks.push(Block::Para(v));
        } else {
            blocks.push(heading(
                2,
                &format!("Hospitals with emergency rooms in {county}"),
            ));
            let mut sorted: Vec<&crate::source::HospitalRow> = list.rows.iter().collect();
            sorted.sort_by(|x, y| x.name.cmp(&y.name).then(x.city.cmp(&y.city)));
            let more = sorted.len().saturating_sub(HOSPITALS_MAX);
            let rows: Vec<Vec<Vec<Inline>>> = sorted
                .iter()
                .take(HOSPITALS_MAX)
                .map(|h| {
                    vec![
                        vec![t(h.name.clone())],
                        vec![t(h.city.clone())],
                        vec![t(h.phone.clone())],
                    ]
                })
                .collect();
            let mut intro = vec![t(format!(
                "From the federal list of hospitals (CMS){released}. Check before you need it: \
                 a hospital can close or stop taking patients."
            ))];
            intro.extend(bx.cite_strs(&[HOSPITALS_CITATION]));
            blocks.push(Block::Para(intro));
            blocks.push(Block::Table(Table {
                header: vec!["Hospital".to_owned(), "City".to_owned(), "Phone".to_owned()],
                rows,
            }));
            if more > 0 {
                blocks.push(Block::Para(vec![t(format!(
                    "The list has {more} more in the county; these are the first {HOSPITALS_MAX} \
                     by name."
                ))]));
            }
        }
    }
    blocks.push(Block::MapSlot(MapSlot {
        id: "map-neighbourhood".to_owned(),
        kind: MapSlotKind::Neighbourhood,
        caption: "Map: your neighborhood".to_owned(),
    }));
    page(
        "neighbourhood",
        title("neighbourhood"),
        PageKind::Neighbourhood,
        blocks,
    )
}

/// The citation behind the county hospital table.
pub const HOSPITALS_CITATION: &str = "cms_hospital_general_information";

/// Getting out (§4.2): where we would go, two ways out, roadside assistance; leaving first
/// where it matters (the v2 summary's rule); the evacuation bucket's warning by cause and time
/// away; the evacuation checklist; the area and region maps.
fn getting_out(bx: &Bx<'_>) -> Page {
    let a = bx.a();
    let fp = super::family(&bx.cx);
    let mut rows = vec![
        row_lines("Where we would go", fp.where_we_would_go.as_deref(), 2),
        row("First way out", fp.routes.first().map(String::as_str)),
        row("Second way out", fp.routes.get(1).map(String::as_str)),
    ];
    let vehicles = !a.input.mobility.vehicles.is_empty() || !fp.vehicles.is_empty();
    if vehicles || fp.roadside_assistance.is_some() {
        rows.push(row(
            "Roadside assistance",
            fp.roadside_assistance.as_deref(),
        ));
    }
    let mut blocks = vec![Block::Fields(rows)];
    let leave = crate::packet::summary::leave_first(&bx.cx);
    if let Some(sentence) = &leave {
        blocks.push(Block::Callout(rr_types::binder::Callout {
            kind: rr_types::binder::CalloutKind::Warning,
            title: Some("Leaving comes first here".to_owned()),
            blocks: vec![Block::Para(bx.plain(sentence))],
        }));
    }
    // The evacuation bucket's sentences: the warning by cause and the time away (its first, the
    // ten-year chance, opens the leave-first box when that prints).
    let evac = a.bucket(BucketId::Evacuate);
    let skip = usize::from(leave.is_some());
    let sentences: Vec<&str> = evac
        .frequency_sentences
        .iter()
        .skip(skip)
        .map(String::as_str)
        .filter(|s| !leave.as_deref().is_some_and(|l| l.contains(s)))
        .collect();
    if !sentences.is_empty() {
        let mut v = bx.plain(&sentences.join(" "));
        v.extend(bx.cite(&evac.sources));
        blocks.push(Block::Para(v));
    }
    blocks.extend(turn_here(
        bx,
        "When you are told to leave: ",
        &[
            "check_evacuation_order",
            "check_wildfire",
            "check_hurricane",
            "check_flooding",
        ],
    ));
    for (id, kind, caption) in [
        ("map-area", MapSlotKind::Area, "Map: your city or county"),
        (
            "map-region",
            MapSlotKind::Region,
            "Map: your region and the ways out",
        ),
    ] {
        blocks.push(Block::MapSlot(MapSlot {
            id: id.to_owned(),
            kind,
            caption: caption.to_owned(),
        }));
    }
    page(
        "getting_out",
        title("getting_out"),
        PageKind::GettingOut,
        blocks,
    )
}
