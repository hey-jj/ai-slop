//! Section 12.10: the tool must lint its own repository. Provider-artifact
//! and ornamental tokens quoted in code formatting must not fire, and the
//! README must carry no violation at all.

mod common;

use ai_slop::Profile;
use common::{assert_invariants, run};

#[test]
fn readme_self_lint_is_violation_free() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")).unwrap();
    let report = run(&text, Profile::Readme);
    assert_invariants(&text, &report);
    let violations: Vec<&str> = report
        .findings
        .iter()
        .filter(|f| f.state == "violation")
        .map(|f| f.rule_id.as_str())
        .collect();
    assert!(
        violations.is_empty(),
        "README carries violations: {violations:?}"
    );
    // The standing fixture fence quotes provider tokens and ornamental
    // words; none may fire from quoted-in-code content.
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.family == "provider-artifact" || f.family == "ornamental"),
        "quoted-in-code content fired: {:?}",
        common::rule_ids(&report)
    );
}

#[test]
fn quoted_tokens_in_code_never_fire_but_prose_uses_do() {
    // The same token inline: quoted in code formatting, then in prose. The
    // prose occurrence is a registered expected finding, never suppressed.
    let text = "The trailer `Co-authored-by: Claude` is banned.\n\nGenerated with Claude Code.\n";
    let report = run(text, Profile::Readme);
    let p001: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "SLOP-P001")
        .collect();
    assert_eq!(p001.len(), 1, "exactly the prose occurrence fires");
    let span = &p001[0].spans[0];
    assert_eq!(&text[span.start..span.end], "Generated with Claude Code");
}

#[test]
fn fixture_files_hold_invariants() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/golden");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let report = run(&text, Profile::PublicBugReport);
        assert_invariants(&text, &report);
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.family == "provider-artifact" || f.family == "ornamental"),
            "{path:?} fired provider or ornamental rules"
        );
    }
}
