//! Golden packets and outputs in `fixtures/golden/`. The default run compares and fails with a
//! readable diff; `RR_UPDATE_GOLDENS=1 cargo test -p rr-plan --test goldens` rewrites them. A
//! change to a golden must be explained in the commit message.

#[test]
fn goldens_match() {
    if std::env::var(rr_plan::golden::UPDATE_ENV).as_deref() == Ok("1") {
        let written = rr_plan::golden::write_all().expect("write goldens");
        assert_eq!(written.len(), 2 * rr_types::fixtures::RAW.len());
        return;
    }
    if let Err(report) = rr_plan::golden::compare_all() {
        panic!("{report}");
    }
}

#[test]
fn the_diff_is_readable() {
    let d = rr_plan::golden::diff("a\nb\nc\nd\n", "a\nb\nX\nd\n");
    assert!(
        d.contains("- c") && d.contains("+ X") && d.contains("  b"),
        "{d}"
    );
}
