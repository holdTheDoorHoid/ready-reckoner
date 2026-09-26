//! Every county in the repository's national data pack (`data/`), and the 22 households of the
//! round-2 backtest (`tests/data/backtest`, from the model review's frozen set), through
//! `assess`: the register's invariants hold everywhere, with or without the v2 exposure columns
//! (the pack in this branch predates them, so every county runs the fallbacks). Skipped, with a
//! message, when `data/manifest.json` is absent.

use std::path::{Path, PathBuf};

use rr_data::DataStore;
use rr_hazards::HazardAssessment;
use rr_types::{HazardDisplay, HazardId, PlanInput};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The core pack, loaded as the web app loads it (manifest first, every file checked).
fn store() -> Option<DataStore> {
    let dir = repo().join("data");
    let manifest_bytes = std::fs::read(dir.join("manifest.json")).ok()?;
    let manifest: rr_data::Manifest = serde_json::from_slice(&manifest_bytes).ok()?;
    let mut files: Vec<(String, Vec<u8>)> = vec![("manifest.json".to_owned(), manifest_bytes)];
    for f in &manifest.packs.get("core")?.files {
        files.push((f.path.clone(), std::fs::read(dir.join(&f.path)).ok()?));
    }
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let mut s = DataStore::new();
    s.load_many(&refs).ok()?;
    Some(s)
}

fn check(label: &str, a: &HazardAssessment) {
    let rare: Vec<_> = a
        .profiles
        .iter()
        .filter(|p| p.display == HazardDisplay::RareCatastrophic)
        .collect();
    assert_eq!(rare.len(), HazardId::RARE.len(), "{label}");
    assert_eq!(a.rates.len(), a.profiles.len() - rare.len(), "{label}");
    for r in &a.rates {
        assert!(!r.hazard.is_rare(), "{label}: {}", r.hazard);
        assert_eq!(r.validate(), Ok(()), "{label}: {}", r.hazard);
    }
    for p in &a.profiles {
        let [lo, hi] = p.rate_range;
        assert!(
            lo.is_finite() && 0.0 <= lo && lo <= p.rate_per_year && p.rate_per_year <= hi,
            "{label}: {} {lo} {} {hi}",
            p.id,
            p.rate_per_year
        );
        assert!(!p.frequency_sentence.contains("NaN"), "{label}: {}", p.id);
        assert!(p.frequency_sentence.ends_with('.'), "{label}: {}", p.id);
        if p.display == HazardDisplay::RareCatastrophic {
            assert!(
                p.range_only && p.anchor_sentence.is_some(),
                "{label}: {}",
                p.id
            );
        }
    }
    for c in &a.also_checked {
        assert!(c.rate_per_year < 1.0e-5, "{label}: {}", c.id);
    }
}

#[test]
fn every_county_in_the_pack() {
    let Some(s) = store() else {
        eprintln!("data/manifest.json not found: skipped");
        return;
    };
    let input = rr_types::fixtures::get("philadelphia-renters-4").unwrap();
    let mut n = 0;
    let mut classes_unknown = 0;
    let mut uasi_absent = Vec::new();
    for county in s.counties() {
        let Some(location) = s.location(&county.fips, None) else {
            continue;
        };
        let a = rr_hazards::assess(&input, county, s.base_rates(), &location);
        check(&county.fips, &a);
        let nuke = a
            .profiles
            .iter()
            .find(|p| p.id == HazardId::NuclearAttack)
            .unwrap();
        if nuke.location_factor.as_ref().unwrap().class == "unknown" {
            classes_unknown += 1;
        }
        if a.notes
            .iter()
            .any(|note| note.contains("urban-area funding shares are not loaded"))
        {
            uasi_absent.push(county.fips.clone());
        }
        n += 1;
    }
    // The pack carries the urban area's own UASI share for every county (0 outside the funded
    // areas), so no county falls back to a share of 0 for want of the column.
    assert!(
        uasi_absent.is_empty(),
        "{} counties without uasi_area_share: {:?}",
        uasi_absent.len(),
        &uasi_absent[..uasi_absent.len().min(10)]
    );
    assert!(n > 3000, "only {n} counties");
    // The strategic classes cover every county or none (a pack before or after the v2 exposure
    // files); awaiting: data-hazard — once they merge, none is unknown.
    assert!(
        classes_unknown == 0 || classes_unknown == n,
        "{classes_unknown} of {n} counties have no strategic class"
    );
}

#[test]
fn the_backtest_households() {
    let Some(s) = store() else {
        eprintln!("data/manifest.json not found: skipped");
        return;
    };
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/backtest");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    assert_eq!(
        paths.len(),
        22,
        "the frozen set has 22 event-household pairs"
    );
    for path in paths {
        let label = path.file_stem().unwrap().to_string_lossy().into_owned();
        let input = PlanInput::from_json(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{label}: {e:?}"));
        let fips = input
            .location
            .county_fips
            .clone()
            .expect("backtest households use a county");
        let county = s
            .county(&fips)
            .unwrap_or_else(|| panic!("{label}: no county {fips}"));
        let location = s.location(&fips, None).unwrap();
        let a = rr_hazards::assess(&input, county, s.base_rates(), &location);
        check(&label, &a);
        // The renters among them see eviction; the owners never do.
        let renter = input.housing.tenure == rr_types::Tenure::Rent;
        assert_eq!(
            a.profiles.iter().any(|p| p.id == HazardId::Eviction),
            renter,
            "{label}"
        );
    }
}
