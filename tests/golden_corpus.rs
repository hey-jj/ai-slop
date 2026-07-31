//! Section 12.1 and 12.6: golden corpus seeded from the two incident-program
//! bug reports, plus a golden segmentation map for a representative
//! document.

mod common;

use ai_slop::Profile;
use common::{assert_invariants, run};

fn read_fixture(name: &str) -> String {
    let path = format!("{}/fixtures/golden/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn gamut_report_findings_are_stable() {
    let text = read_fixture("lightningcss-gamut-mapping-early-return-desaturated-fallback.md");
    let report = run(&text, Profile::PublicBugReport);
    assert_invariants(&text, &report);
    // Adjudicated expectations: the only blocking finding is the title
    // length breach.
    let blocking: Vec<&str> = report
        .findings
        .iter()
        .filter(|f| f.lifecycle == "blocking" && !f.waived)
        .map(|f| f.rule_id.as_str())
        .collect();
    assert_eq!(blocking, vec!["SLOP-K001"]);
}

#[test]
fn recursion_report_findings_are_stable() {
    let text = read_fixture("lightningcss-unbounded-parser-recursion-stack-overflow.md");
    let report = run(&text, Profile::PublicBugReport);
    assert_invariants(&text, &report);
    let blocking: Vec<&str> = report
        .findings
        .iter()
        .filter(|f| f.lifecycle == "blocking" && !f.waived)
        .map(|f| f.rule_id.as_str())
        .collect();
    // Title length plus one contrast candidate ("rather than").
    assert_eq!(blocking, vec!["SLOP-K001", "SLOP-C003"]);
}

#[test]
fn golden_segmentation_map() {
    let doc = "# Title\n\nProse one.\n\n```rust\nlet x = 1;\n```\n\nProse `two` end.\n";
    let report = run(doc, Profile::InternalDoc);
    let seg: Vec<(usize, usize, &str)> = report
        .coverage
        .segmentation
        .excluded
        .iter()
        .map(|e| (e.start, e.end, e.reason.as_str()))
        .collect();
    assert_eq!(
        seg,
        vec![
            (0, 2, "structure"),
            (7, 9, "structure"),
            (19, 21, "structure"),
            (21, 43, "code_fence"),
            (43, 45, "structure"),
            (51, 56, "inline_code"),
            (61, 62, "structure"),
        ],
        "segmentation drifted"
    );
    assert_eq!(report.coverage.segmentation.prose_bytes, doc.len() - 36);
    assert_eq!(report.coverage.sections.len(), 1);
    assert_eq!(report.coverage.sections[0].title, "Title");
}

#[test]
fn manifest_profile_end_to_end() {
    let manifest = br#"
[package]
name = "demo"
version = "1.0.0"
description = "A very seamless parser for demo files"
keywords = ["a", "b", "c"]
"#;
    let mut config = ai_slop::Config::new(Profile::CargoMetadata);
    let report = ai_slop::analyze(manifest, &config).unwrap();
    let ids = common::rule_ids(&report);
    // The description runs through the prose rule set.
    assert!(ids.contains(&"SLOP-A001"), "seamless missed: {ids:?}");
    assert!(ids.contains(&"SLOP-I001"), "very missed: {ids:?}");
    // Field checks: 3 keywords instead of 5, categories missing.
    assert!(
        report
            .findings
            .iter()
            .filter(|f| f.rule_id == "SLOP-K006")
            .count()
            >= 2,
        "K006 field checks missed: {ids:?}"
    );
    // A payload that does not parse as a manifest is unsupported input.
    config.input_format = ai_slop::InputFormat::Manifest;
    assert!(matches!(
        ai_slop::analyze(b"not = a manifest", &config),
        Err(ai_slop::AnalysisError::UnsupportedInput(_))
    ));
}
