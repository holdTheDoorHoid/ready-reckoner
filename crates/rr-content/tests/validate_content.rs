//! Runs the content validator over everything embedded from `content/`.
//!
//! Errors fail the build. Warnings are printed (`cargo test -p rr-content -- --nocapture`) and are
//! listed with reasons in the content agent's report.

use rr_content::validate::summary;

#[test]
fn embedded_content_parses_and_passes_the_validator() {
    let content = rr_content::try_content().expect("content parses");
    let report = rr_content::validate_embedded();
    for w in report.warnings() {
        println!("{w}");
    }
    let s = summary(content);
    println!(
        "content {}: {} items ({} free), {} citations ({} priors), {} guidance blocks, {} glossary terms; {} warnings",
        rr_content::CONTENT_VERSION,
        s.items,
        s.free_items,
        s.citations,
        s.prior_citations,
        s.guidance,
        s.glossary,
        report.warnings().count()
    );
    let errors: Vec<String> = report.errors().map(ToString::to_string).collect();
    assert!(
        errors.is_empty(),
        "{} content errors:\n{}",
        errors.len(),
        errors.join("\n")
    );
}

#[test]
fn content_version_is_stable_and_names_the_hash() {
    assert!(rr_content::CONTENT_VERSION.starts_with("2026."));
    assert!(rr_content::CONTENT_VERSION.ends_with(&rr_content::CONTENT_HASH[..8]));
    // parsing twice gives identical content (no hidden state, no clock)
    let a = rr_content::Content::from_files(rr_content::embedded_files()).unwrap();
    let b = rr_content::Content::from_files(rr_content::embedded_files()).unwrap();
    assert_eq!(a, b);
}
