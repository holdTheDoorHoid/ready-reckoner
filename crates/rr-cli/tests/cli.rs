//! Smoke tests for every `rr` subcommand, run as a real process against the fixture households,
//! on the fixture counties (`--fixtures`) and on the committed data pack (`--data`).
//!
//! Where a command prints numbers, the test computes the same numbers by calling the engine
//! in-process and checks the command printed those, so a formatting slip or a wrong source shows
//! up here rather than in a packet.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use assert_cmd::Command;
use predicates::prelude::*;
use rr_plan::{CountySource, Engine, FixtureSource};
use rr_types::{
    BucketId, Catalogue, CountyRecord, Explanation, PlanInput, PlanOutput, ReturnPeriod, Target,
};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn data_dir() -> String {
    format!("{ROOT}/data")
}

fn household(name: &str) -> String {
    format!("{ROOT}/fixtures/households/{name}.json")
}

fn rr() -> Command {
    Command::cargo_bin("rr").expect("the rr binary")
}

/// `rr --fixtures ...`
fn fixtures() -> Command {
    let mut c = rr();
    c.arg("--fixtures");
    c
}

/// `rr --data <repo>/data ...`
fn pack() -> Command {
    let mut c = rr();
    c.args(["--data", &data_dir()]);
    c
}

/// An empty scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("rr-cli")
        .join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

fn engine() -> &'static Engine<FixtureSource> {
    static E: OnceLock<Engine<FixtureSource>> = OnceLock::new();
    E.get_or_init(|| Engine::with_fixtures().expect("fixture engine"))
}

fn fixture(name: &str) -> PlanInput {
    rr_types::fixtures::get(name).expect("fixture household")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("utf-8")
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8(out.stderr.clone()).expect("utf-8")
}

fn manifest_pack_version() -> String {
    let m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(format!("{}/manifest.json", data_dir())).unwrap())
            .unwrap();
    m["pack_version"].as_str().unwrap().to_owned()
}

// ---------------------------------------------------------------------------------------- plan

#[test]
fn plan_prints_the_packet_and_the_json_the_engine_makes() {
    let input = fixture("philadelphia-renters-4");
    let expected = engine().assess(&input).unwrap();
    let out = fixtures()
        .args(["plan", "--household", &household("philadelphia-renters-4")])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), expected.packet_markdown);
    for heading in rr_plan::packet::SECTION_HEADINGS {
        assert!(stdout(&out).contains(heading), "{heading}");
    }
    // By fixture name, as JSON: the golden bytes.
    let out = fixtures()
        .args([
            "plan",
            "--household",
            "philadelphia-renters-4",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), rr_plan::to_json(&expected));
}

#[test]
fn plan_writes_both_files_into_out() {
    let dir = scratch("plan-both");
    fixtures()
        .args(["plan", "--household", &household("miami-condo-retiree-1")])
        .args(["--format", "both", "--out"])
        .arg(&dir)
        .assert()
        .success()
        .stderr(predicate::str::contains("wrote"));
    let md = std::fs::read_to_string(dir.join("miami-condo-retiree-1.md")).unwrap();
    let json = std::fs::read_to_string(dir.join("miami-condo-retiree-1.json")).unwrap();
    let parsed: PlanOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(md, parsed.packet_markdown);
    assert_eq!(parsed.location.county_fips, "12086");
    // Both needs somewhere to put two files.
    fixtures()
        .args([
            "plan",
            "--household",
            "miami-condo-retiree-1",
            "--format",
            "both",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--out"));
}

#[test]
fn plan_reads_standard_input() {
    let json = std::fs::read_to_string(household("chicago-student-zero-budget-1")).unwrap();
    let expected = engine()
        .assess(&fixture("chicago-student-zero-budget-1"))
        .unwrap();
    fixtures()
        .args(["plan", "--household", "-"])
        .write_stdin(json)
        .assert()
        .success()
        .stdout(expected.packet_markdown);
}

#[test]
fn validation_problems_exit_2_with_every_problem_listed() {
    let dir = scratch("invalid");
    let mut v: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(household("philadelphia-renters-4")).unwrap(),
    )
    .unwrap();
    v["people"] = serde_json::json!([]);
    v["location"]["zip"] = serde_json::json!("191");
    let bad = dir.join("bad.json");
    std::fs::write(&bad, v.to_string()).unwrap();
    let out = fixtures()
        .args(["plan", "--household"])
        .arg(&bad)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err = stderr(&out);
    assert!(err.contains("problems to fix"), "{err}");
    assert!(
        err.contains("  - people: ") && err.contains("[no_people]"),
        "{err}"
    );
    assert!(
        err.contains("  - location.zip: ") && err.contains("[zip_format]"),
        "{err}"
    );
    // Not a PlanInput at all.
    v["sprinklers"] = serde_json::json!(true);
    std::fs::write(&bad, v.to_string()).unwrap();
    fixtures()
        .args(["plan", "--household"])
        .arg(&bad)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("[schema]"));
    // A flag can fix what it replaces: the file's bad ZIP code is overridden by --county.
    v.as_object_mut().unwrap().remove("sprinklers");
    v["people"] = serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string(household("philadelphia-renters-4")).unwrap(),
    )
    .unwrap()["people"]
        .clone();
    std::fs::write(&bad, v.to_string()).unwrap();
    fixtures()
        .args(["risks", "--household"])
        .arg(&bad)
        .args(["--county", "42101"])
        .assert()
        .success();
}

#[test]
fn unknown_places_exit_2_and_say_what_is_loaded() {
    fixtures()
        .args([
            "plan",
            "--household",
            "philadelphia-renters-4",
            "--zip",
            "10001",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("We don't have ZIP code 10001"))
        .stderr(predicate::str::contains(
            "Only the seven fixture counties are loaded",
        ));
    fixtures()
        .args([
            "targets",
            "--household",
            "philadelphia-renters-4",
            "--county",
            "42999",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("42101  Philadelphia, PA"));
    fixtures()
        .args(["plan", "--household", "no-such-household"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("coos-bay-well-owner-2"));
}

#[test]
fn dials_and_scenario_toggles_reach_the_engine() {
    let run = |extra: &[&str]| -> String {
        let out = fixtures()
            .args([
                "plan",
                "--household",
                "coos-bay-well-owner-2",
                "--format",
                "json",
            ])
            .args(extra)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", stderr(&out));
        stdout(&out)
    };
    let off: PlanOutput = serde_json::from_str(&run(&["--scenario", "cascadia_m9=off"])).unwrap();
    let s = off
        .scenarios
        .iter()
        .find(|s| s.id == "cascadia_m9")
        .unwrap();
    assert!(!s.on, "the toggle turned Cascadia off");
    let mut input = fixture("coos-bay-well-owner-2");
    input.dials.return_period = ReturnPeriod::OneIn10;
    input.dials.climate = rr_types::ClimateHorizon::Y2050;
    input.dials.water_level = rr_types::WaterLevel::Survival;
    let expected = rr_plan::to_json(&engine().assess(&input).unwrap());
    let got = run(&[
        "--return-period",
        "one_in_10",
        "--climate",
        "y2050",
        "--water-level",
        "survival",
    ]);
    assert!(
        got == expected,
        "{}",
        rr_plan::golden::diff(&expected, &got)
    );
    // A toggle for a scenario that does not apply here is pointed out, not silently dropped.
    fixtures()
        .args([
            "plan",
            "--household",
            "coos-bay-well-owner-2",
            "--scenario",
            "hayward_m7=on",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "--scenario hayward_m7=on changes nothing",
        ));
}

#[test]
fn plan_runs_on_the_data_pack() {
    let out = pack()
        .args([
            "plan",
            "--household",
            "philadelphia-renters-4",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let o: PlanOutput = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(o.data_pack_version, manifest_pack_version());
    assert_eq!(o.location.county_fips, "42101");
    assert_eq!(o.location.zip.as_deref(), Some("19147"));
    assert!(!o.register.is_empty() && !o.packet_markdown.is_empty());
}

#[test]
fn without_a_pack_the_default_falls_back_to_the_fixtures_with_a_note() {
    let dir = scratch("no-pack");
    rr().current_dir(&dir)
        .args(["risks", "--household", "philadelphia-renters-4"])
        .assert()
        .success()
        .stderr(predicate::str::contains("no data pack in ./data"))
        .stdout(predicate::str::contains("the seven fixture counties"));
    // An explicit --data that holds no pack is an error, not a silent fallback.
    rr().args(["--data"])
        .arg(&dir)
        .args(["data", "info"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("manifest.json is missing"));
}

// ------------------------------------------------------------------------------ risks, targets

#[test]
fn risks_lists_the_register_with_sentences_and_sources() {
    let a = engine().run(&fixture("philadelphia-renters-4")).unwrap();
    let out = fixtures()
        .args(["risks", "--household", "philadelphia-renters-4"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("Ranked, most important first"), "{text}");
    let first = &a.hazards.profiles[0];
    assert!(text.contains(&first.name));
    // Every hazard and its natural-frequency sentence (wrapped, so compare the words).
    let words: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    for p in &a.hazards.profiles {
        assert!(words.contains(&p.name), "{}", p.name);
        assert!(
            words.contains(&p.frequency_sentence),
            "{}",
            p.frequency_sentence
        );
        for s in &p.sources {
            assert!(text.contains(s.as_str()), "{s}");
        }
    }
    if a.hazards
        .profiles
        .iter()
        .any(|p| p.display == rr_types::HazardDisplay::RareCatastrophic)
    {
        assert!(text.contains("Rare but severe"));
    }
    assert!(text.contains("\nSources\n"));
    assert!(text.contains("fema_nri_v120"));
}

fn days(t: &Target) -> (f32, f32, f32) {
    match *t {
        Target::Days { value, low, high } | Target::Months { value, low, high } => {
            (value, low, high)
        }
        _ => (0.0, 0.0, 0.0),
    }
}

#[test]
fn targets_prints_each_bucket_with_its_range_and_relief() {
    let a = engine().run(&fixture("philadelphia-renters-4")).unwrap();
    let out = fixtures()
        .args(["targets", "--household", "philadelphia-renters-4"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    for b in &a.buckets {
        assert!(text.contains(&b.name), "{}", b.name);
        if b.id.kind() == rr_types::BucketKind::Duration {
            let (v, lo, hi) = days(&b.target);
            let phrase = rr_cli::format::target_days(f64::from(v), f64::from(lo), f64::from(hi));
            assert!(text.contains(&phrase), "{}: {phrase}", b.name);
        }
    }
    assert!(text.contains("Help arrives") && text.contains("Mostly back"));
    assert!(text.contains("Enough for this household: "));
}

#[test]
fn the_sweep_shows_every_dial_and_matches_the_engine_at_each() {
    let out = fixtures()
        .args(["targets", "--household", "coos-bay-well-owner-2", "--sweep"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("1 in 500 *"),
        "Coos Bay's own dial is marked:\n{text}"
    );
    let input = fixture("coos-bay-well-owner-2");
    let base = engine().run(&input).unwrap();
    let row = text
        .lines()
        .find(|l| l.trim_start().starts_with(BucketId::WaterOut.name()))
        .expect("the no-water row");
    for (i, rp) in ReturnPeriod::ALL.iter().enumerate() {
        let mut at = input.clone();
        at.dials.return_period = *rp;
        let a = engine().run(&at).unwrap();
        let (v, _, _) = days(&a.bucket(BucketId::WaterOut).target);
        // The cell starts with the ladder value, as the engine rounds it.
        let expected = rr_cli::format::number(f64::from(v));
        assert!(
            row.split("  ")
                .filter(|c| !c.trim().is_empty())
                .nth(1 + i)
                .is_some_and(|c| c.trim().starts_with(&expected)),
            "{rp}: expected {expected} in {row}"
        );
        // The re-run agrees with the consequence crate's own dial table from a single run.
        let d = base
            .consequence
            .details
            .iter()
            .find(|d| d.bucket == BucketId::WaterOut)
            .unwrap();
        assert_eq!(d.dial_table[i].return_period_years, rp.years());
        assert_eq!(d.dial_table[i].ladder_days, v, "{rp}");
    }
    assert!(text.contains("Plan done by month") && text.contains("Purchases in the plan"));
}

// ------------------------------------------------------------------------------------ explain

#[test]
fn explain_gives_words_arithmetic_and_sources() {
    let input = fixture("philadelphia-renters-4");
    let expected = engine()
        .explain(rr_types::ExplainKind::Bucket, "power", &input)
        .unwrap();
    let out = fixtures()
        .args([
            "explain",
            "bucket",
            "power",
            "--household",
            "philadelphia-renters-4",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.starts_with(&expected.title));
    assert!(
        text.contains("The arithmetic") && text.contains("Λ("),
        "{text}"
    );
    for c in &expected.sources {
        assert!(text.contains(c.id.as_str()), "{}", c.id);
    }
    let out = fixtures()
        .args([
            "explain",
            "bucket",
            "power",
            "--household",
            "philadelphia-renters-4",
            "--json",
        ])
        .output()
        .unwrap();
    let e: Explanation = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(e, expected);
    // Every warning the plan raises can be explained.
    let plan = engine().assess(&input).unwrap();
    for w in &plan.warnings {
        fixtures()
            .args([
                "explain",
                "warning",
                &w.id,
                "--household",
                "philadelphia-renters-4",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains(w.message.as_str()));
    }
}

#[test]
fn explain_an_unknown_id_suggests_real_ones() {
    fixtures()
        .args([
            "explain",
            "hazard",
            "heat_wav",
            "--household",
            "philadelphia-renters-4",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Did you mean: heat_wave"));
    fixtures()
        .args([
            "explain",
            "requirement",
            "water_out.nothing",
            "--household",
            "philadelphia-renters-4",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("water_out.water_gallons"));
}

// --------------------------------------------------------------------- catalogue, citations

#[test]
fn catalogue_lists_items_and_its_json_is_the_engine_catalogue() {
    let content = rr_content::content();
    rr().args(["catalogue"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Catalogue: {} items",
            content.items.len()
        )));
    // Byte for byte what the engine's catalogue() serialises to (serde_json's default float
    // parser is not correctly rounded, so compare the text, not a parsed copy).
    let out = rr().args(["catalogue", "--json"]).output().unwrap();
    assert_eq!(stdout(&out), rr_plan::to_json(&engine().catalogue()));
    let out = rr()
        .args(["catalogue", "--category", "water", "--json"])
        .output()
        .unwrap();
    let c: Catalogue = serde_json::from_str(&stdout(&out)).unwrap();
    assert!(!c.items.is_empty() && c.items.iter().all(|i| i.category == "water"));
    rr().args(["catalogue", "--category", "watr"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("water"));
}

#[test]
fn citations_list_the_registry_and_the_missing_ids() {
    let content = rr_content::content();
    rr().args(["citations"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Sources: {} in content/citations.toml",
            content.citations.len()
        )))
        .stdout(predicate::str::contains("fema_nri_v120"));
    for mut cmd in [fixtures(), pack()] {
        let out = cmd.args(["citations", "--missing"]).output().unwrap();
        assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
        let text = stdout(&out);
        for id in rr_plan::provenance::AWAITING_CONTENT {
            assert!(
                text.contains(id) && text.contains("awaited"),
                "{id}: {text}"
            );
        }
    }
}

// ------------------------------------------------------------------------------------- county

#[test]
fn county_search_and_show() {
    fixtures()
        .args(["county", "search", "phila"])
        .assert()
        .success()
        .stdout(predicate::str::contains("42101  Philadelphia"));
    pack()
        .args(["county", "search", "Cook,", "IL"])
        .assert()
        .success()
        .stdout(predicate::str::contains("17031  Cook"));
    fixtures()
        .args(["county", "search", "zzzz"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("No county matches"));
    let out = pack().args(["county", "show", "42101"]).output().unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.starts_with("Philadelphia, Pennsylvania (FIPS 42101)"),
        "{text}"
    );
    assert!(text.contains("National Risk Index, per hazard") && text.contains("Power outages"));
    let out = fixtures()
        .args(["county", "show", "42101", "--json"])
        .output()
        .unwrap();
    let record = engine().store().county("42101").unwrap();
    assert_eq!(stdout(&out), rr_plan::to_json(record));
    let c: CountyRecord = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(c.fips, "42101");
    fixtures()
        .args(["county", "show", "42999"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("42101 Philadelphia"));
}

// --------------------------------------------------------------------------------------- data

#[test]
fn data_info_and_verify() {
    fixtures()
        .args(["data", "info"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "no data pack; fixture counties in use",
        ))
        .stdout(predicate::str::contains("FEMA National Risk Index"));
    fixtures()
        .args(["data", "verify"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "no data pack; fixture counties in use",
        ));
    let version = manifest_pack_version();
    let out = pack().args(["data", "info"]).output().unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(&version) && text.contains("Refreshed"),
        "{text}"
    );
    assert!(text.contains("FEMA National Risk Index") && text.contains("accessed"));
    let out = pack().args(["data", "verify"]).output().unwrap();
    let text = stdout(&out);
    assert!(out.status.success(), "{text}{}", stderr(&out));
    assert!(
        text.contains("rr-etl verify") && text.contains("Everything checks out."),
        "{text}"
    );
}

#[test]
fn a_corrupt_pack_file_fails_its_checksum() {
    let dir = scratch("corrupt-pack");
    let src = Path::new(ROOT).join("data");
    for entry in walk(&src) {
        let rel = entry.strip_prefix(&src).unwrap();
        let to = dir.join(rel);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(&entry, &to).unwrap();
    }
    let states = dir.join("core/states.csv");
    let mut bytes = std::fs::read(&states).unwrap();
    let last = bytes.len() - 2;
    bytes[last] ^= 1;
    std::fs::write(&states, bytes).unwrap();
    rr().args(["--data"])
        .arg(&dir)
        .args(["county", "show", "42101"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("does not match its checksum"));
    rr().args(["--data"])
        .arg(&dir)
        .args(["data", "verify"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("core/states.csv"));
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "raw" || n == "tmp") {
                continue;
            }
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

// ----------------------------------------------------------------------------- golden, doctor

#[test]
fn golden_compares_with_the_helper_and_update_is_a_no_op_when_they_match() {
    let files = 2 * rr_types::fixtures::RAW.len();
    let out = rr().args(["golden"]).output().unwrap();
    let text = stdout(&out);
    assert!(text.contains("rr-plan's golden helper"), "{text}");
    let helper_matches = rr_plan::golden::compare_all().is_ok();
    assert_eq!(out.status.success(), helper_matches, "{text}");
    if helper_matches {
        assert!(
            text.contains(&format!("{files} of {files} golden files match.")),
            "{text}"
        );
        // Rewriting goldens that already match changes nothing.
        rr().args(["golden", "--update"])
            .assert()
            .success()
            .stdout(predicate::str::contains("None changed."));
        assert!(rr_plan::golden::compare_all().is_ok());
    }
    // --update never takes another source.
    rr().args(["--fixtures", "golden", "--update"])
        .assert()
        .code(2);
    // A read-only comparison on the data pack reports every file either way.
    let out = pack().args(["golden"]).output().unwrap();
    let text = stdout(&out);
    assert!(text.contains("Rendered by data pack"), "{text}");
    assert!(
        text.contains(&format!("of {files} golden files match.")),
        "{text}"
    );
}

#[test]
fn doctor_runs_every_fixture() {
    for mut cmd in [fixtures(), pack()] {
        let out = cmd.args(["doctor", "--runs", "1"]).output().unwrap();
        let text = stdout(&out);
        assert!(out.status.success(), "{text}{}", stderr(&out));
        for (name, _) in rr_types::fixtures::RAW {
            assert!(text.contains(name), "{name}");
        }
        assert!(
            text.contains("Median ms") && text.contains("Result: OK"),
            "{text}"
        );
    }
}

#[test]
fn usage_errors_exit_2() {
    rr().args(["plan"]).assert().code(2);
    rr().args(["targets", "--household", "x", "--return-period", "1in100"])
        .assert()
        .code(2);
    rr().args(["--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("rr "));
}
