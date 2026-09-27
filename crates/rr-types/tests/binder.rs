//! The binder's JSON is the contract the web app renders (DESIGN-DELTA-v3 §4.1): each block and
//! inline is an object with exactly one key, compared here with literal strings, and a whole
//! binder reads back unchanged.

mod common;

use rr_types::Binder;
use rr_types::binder::*;

fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap()
}

fn back<T: serde::de::DeserializeOwned + serde::Serialize + PartialEq + std::fmt::Debug>(
    value: &T,
) {
    let text = json(value);
    let again: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&again, value, "{text}");
}

fn t(s: &str) -> Inline {
    Inline::T(s.into())
}

#[test]
fn every_inline_has_its_one_key_shape() {
    let cases: Vec<(Inline, &str)> = vec![
        (t("Leave now."), r#"{"t":"Leave now."}"#),
        (Inline::B("Get out.".into()), r#"{"b":"Get out."}"#),
        (Inline::Cite(vec![3, 7]), r#"{"cite":[3,7]}"#),
        (
            Inline::Link(Link {
                to: "home".into(),
                text: "Tab 3, Home".into(),
            }),
            r#"{"link":{"to":"home","text":"Tab 3, Home"}}"#,
        ),
        (Inline::Blank(24), r#"{"blank":24}"#),
    ];
    for (inline, want) in cases {
        assert_eq!(json(&inline), want);
        back(&inline);
    }
    for bad in [
        r#"{"t":"a","b":"c"}"#,
        r#"{"text":"a"}"#,
        r#""t""#,
        r#"{"link":{"to":"home"}}"#,
        r#"{"link":{"to":"home","text":"x","extra":1}}"#,
        r#"{"blank":-1}"#,
        r#"{"blank":300}"#,
    ] {
        assert!(serde_json::from_str::<Inline>(bad).is_err(), "{bad}");
    }
}

#[test]
fn every_block_has_its_one_key_shape() {
    let cases: Vec<(Block, &str)> = vec![
        (
            Block::Heading(Heading {
                level: 2,
                text: "Do first".into(),
            }),
            r#"{"heading":{"level":2,"text":"Do first"}}"#,
        ),
        (
            Block::Para(vec![t("Stay low."), Inline::Cite(vec![1])]),
            r#"{"para":[{"t":"Stay low."},{"cite":[1]}]}"#,
        ),
        (
            Block::Bullets(vec![vec![t("a")], vec![t("b")]]),
            r#"{"bullets":[[{"t":"a"}],[{"t":"b"}]]}"#,
        ),
        (
            Block::Numbered(vec![vec![t("one")]]),
            r#"{"numbered":[[{"t":"one"}]]}"#,
        ),
        (
            Block::Steps(vec![Step {
                text: vec![Inline::B("Get out.".into())],
                memory: true,
            }]),
            r#"{"steps":[{"text":[{"b":"Get out."}],"memory":true}]}"#,
        ),
        (
            Block::Fields(vec![
                FieldRow {
                    label: "Meeting place near home".into(),
                    value: Some("The corner mailbox".into()),
                    lines: 1,
                },
                FieldRow {
                    label: "Out-of-area contact".into(),
                    value: None,
                    lines: 2,
                },
            ]),
            r#"{"fields":[{"label":"Meeting place near home","value":"The corner mailbox","lines":1},{"label":"Out-of-area contact","lines":2}]}"#,
        ),
        (
            Block::Table(Table {
                header: vec!["Name".into()],
                rows: vec![vec![vec![t("Rosa")]]],
            }),
            r#"{"table":{"header":["Name"],"rows":[[[{"t":"Rosa"}]]]}}"#,
        ),
        (
            Block::Callout(Callout {
                kind: CalloutKind::Warning,
                title: Some("Careful".into()),
                blocks: vec![Block::Para(vec![t("x")])],
            }),
            r#"{"callout":{"kind":"warning","title":"Careful","blocks":[{"para":[{"t":"x"}]}]}}"#,
        ),
        (
            Block::Callout(Callout {
                kind: CalloutKind::Note,
                title: None,
                blocks: vec![],
            }),
            r#"{"callout":{"kind":"note","blocks":[]}}"#,
        ),
        (
            Block::Decision(Decision {
                question: "Leave or stay?".into(),
                branches: vec![
                    Branch {
                        when: vec![t("Leave if told to.")],
                        then: vec![t("Go.")],
                        go_to: Some("getting_out".into()),
                    },
                    Branch {
                        when: vec![t("Stay if the roads are flooded.")],
                        then: vec![t("Move up.")],
                        go_to: None,
                    },
                ],
            }),
            r#"{"decision":{"question":"Leave or stay?","branches":[{"when":[{"t":"Leave if told to."}],"then":[{"t":"Go."}],"go_to":"getting_out"},{"when":[{"t":"Stay if the roads are flooded."}],"then":[{"t":"Move up."}]}]}}"#,
        ),
        (
            Block::MapSlot(MapSlot {
                id: "neighbourhood".into(),
                kind: MapSlotKind::Neighbourhood,
                caption: "Your neighbourhood".into(),
            }),
            r#"{"map_slot":{"id":"neighbourhood","kind":"neighbourhood","caption":"Your neighbourhood"}}"#,
        ),
        (
            Block::Cards(vec![Card {
                title: "Ana".into(),
                lines: vec![vec![t("555-0110")]],
            }]),
            r#"{"cards":[{"title":"Ana","lines":[[{"t":"555-0110"}]]}]}"#,
        ),
        (
            Block::Log(Log {
                columns: vec!["Date".into(), "What".into()],
                rows: 20,
            }),
            r#"{"log":{"columns":["Date","What"],"rows":20}}"#,
        ),
        (Block::page_break(), r#"{"page_break":true}"#),
    ];
    let mut keys: Vec<String> = Vec::new();
    for (block, want) in cases {
        assert_eq!(json(&block), want);
        back(&block);
        let v: serde_json::Value = serde_json::from_str(want).unwrap();
        let obj = v.as_object().unwrap();
        assert_eq!(obj.len(), 1, "{want}");
        keys.extend(obj.keys().cloned());
    }
    keys.dedup();
    assert_eq!(
        keys,
        [
            "heading",
            "para",
            "bullets",
            "numbered",
            "steps",
            "fields",
            "table",
            "callout",
            "decision",
            "map_slot",
            "cards",
            "log",
            "page_break"
        ]
    );
    for bad in [
        r#"{"heading":{"level":1,"text":"a"},"para":[]}"#,
        r#"{"heading":{"level":1}}"#,
        r#"{"heading":{"level":1,"text":"a","size":3}}"#,
        r#"{"quote":[]}"#,
        r#"{"page_break":false}"#,
        r#"{"page_break":null}"#,
        r#"{"steps":[{"text":[]}]}"#,
        r#"{"callout":{"kind":"danger","blocks":[]}}"#,
        r#"{"map_slot":{"id":"a","kind":"street","caption":"b"}}"#,
    ] {
        assert!(serde_json::from_str::<Block>(bad).is_err(), "{bad}");
    }
}

#[test]
fn page_kinds_fits_and_ids_serialise_as_the_delta_names_them() {
    assert_eq!(
        PageKind::STRS,
        [
            "cover",
            "how_to_use",
            "quick_start",
            "index",
            "contacts",
            "person",
            "wallet_cards",
            "home",
            "place",
            "neighbourhood",
            "getting_out",
            "pets",
            "vehicles",
            "documents",
            "inventory",
            "risks_glance",
            "checklist",
            "after",
            "log",
            "sources"
        ]
    );
    assert_eq!(Fit::STRS, ["one", "two", "flow"]);
    assert_eq!(CalloutKind::STRS, ["stop", "warning", "note", "decision"]);
    assert_eq!(MapSlotKind::STRS, ["region", "area", "neighbourhood"]);
    let page = Page {
        id: "check_tornado".into(),
        title: "Tornado".into(),
        kind: PageKind::Checklist,
        fit: Fit::One,
        blocks: vec![],
    };
    assert_eq!(
        json(&page),
        r#"{"id":"check_tornado","title":"Tornado","kind":"checklist","fit":"one","blocks":[]}"#
    );
    let source = SourceEntry {
        n: 1,
        title: "Tornadoes".into(),
        publisher: "FEMA / Ready.gov".into(),
        year: None,
        url: None,
        expert: true,
    };
    assert_eq!(
        json(&source),
        r#"{"n":1,"title":"Tornadoes","publisher":"FEMA / Ready.gov","expert":true}"#
    );
}

#[test]
fn a_whole_binder_round_trips_and_passes_its_own_check() {
    let b = common::samples::binder();
    back(&b);
    assert_eq!(b.check(), Vec::<String>::new());
    // Fields in the delta's order (serde_json's `Value` sorts keys, so read the text).
    let text = json(&b);
    assert!(
        text.starts_with(
            r#"{"title":"Your emergency binder","generated_on":"2026-10-01","household":"2 adults","location":"Philadelphia County, Pennsylvania (ZIP code 19147)","status_line":"Ready Reckoner is an independent, open-source planning aid.","review_by":"2027-10-01","parts":[{"id":"people","tab":2,"title":"People","short_title":"People","pages":[{"id":"wallet_cards","title":"Wallet cards","kind":"wallet_cards","fit":"flow","blocks":["#
        ),
        "{text}"
    );
    assert!(
        text.ends_with(
            r#"],"sources":[{"n":1,"title":"Home Fires","publisher":"FEMA / Ready.gov","year":2026,"url":"https://www.ready.gov/home-fires","expert":false}],"credits":["Uses National Risk Index data; not endorsed by FEMA."]}"#
        ),
        "{text}"
    );
    let v: serde_json::Value = serde_json::to_value(&b).unwrap();
    // Unknown fields are rejected at every level.
    let mut extra = v.clone();
    extra["parts"][0]["colour"] = serde_json::json!("red");
    assert!(serde_json::from_value::<Binder>(extra).is_err());
    let mut extra = v;
    extra["parts"][0]["pages"][0]["blocks"][0]["cards"][0]["icon"] = serde_json::json!("x");
    assert!(serde_json::from_value::<Binder>(extra).is_err());
}
