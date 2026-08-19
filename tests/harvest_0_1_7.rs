//! 0.1.7 harvest proposals (policy 1.4.0): the E002 internal-doc verdict-token
//! allowlist via `profile_exemptions`, the X004 house-report-template
//! exemption, the W001 hyphenated-compound suppression, and the C004
//! sentence-start concession widening with its reconstruction fixture.

mod common;

use ai_slop::{policy, Profile};
use common::{assert_invariants, has_rule, run};

// --- P1: SLOP-E002 profile_exemptions, case-sensitive and profile-scoped ----

/// The exact ledger verdict token is internal-doc vocabulary: the E002 hit on
/// its `NOT` is suppressed there because the listed literal covers the span.
#[test]
fn e002_do_not_build_is_exempt_on_internal_doc() {
    let t = "The verdict stands at DO NOT BUILD until the follow-up lands.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    assert!(
        !has_rule(&report, "SLOP-E002"),
        "the internal-doc verdict token fired: {:?}",
        common::rule_ids(&report)
    );
}

/// The same bytes on an outbound surface stay a candidate: the exemption is
/// scoped to the profiles the policy lists, never global.
#[test]
fn e002_do_not_build_still_fires_on_public_bug_report() {
    let t = "The verdict stands at DO NOT BUILD until the follow-up lands.\n";
    let report = run(t, Profile::PublicBugReport);
    assert_invariants(t, &report);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-E002")
        .expect("E002 must stay hot outside internal-doc");
    assert_eq!(f.state, "candidate");
}

/// The literal is case-SENSITIVE: a case variation of the token is not the
/// documented verdict vocabulary and still fires, even on internal-doc.
#[test]
fn e002_case_variation_still_fires_on_internal_doc() {
    let t = "The verdict stands at do NOT build until the follow-up lands.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    assert!(
        has_rule(&report, "SLOP-E002"),
        "case-varied token must still fire: {:?}",
        common::rule_ids(&report)
    );
}

/// Adjacent all-caps insistence on internal-doc is untouched by the
/// exemption: only spans the listed literal covers are suppressed.
#[test]
fn e002_other_insistence_on_internal_doc_still_fires() {
    let t = "The cache DOES rebuild on every run.\n";
    let report = run(t, Profile::InternalDoc);
    assert!(
        has_rule(&report, "SLOP-E002"),
        "unlisted insistence must still fire: {:?}",
        common::rule_ids(&report)
    );
}

/// Schema round-trip: the parsed package carries the E002 literal under
/// internal-doc only, and the digest over the extended package is
/// reproducible and flows into the snapshot.
#[test]
fn profile_exemptions_round_trip_and_digest() {
    let pkg = policy::load().unwrap();
    let rule = pkg.rule_by_id("SLOP-E002").unwrap();
    assert_eq!(
        rule.profile_exemptions[Profile::InternalDoc.index()],
        vec!["DO NOT BUILD".to_string()]
    );
    for p in Profile::ALL {
        if p != Profile::InternalDoc {
            assert!(
                rule.profile_exemptions[p.index()].is_empty(),
                "unexpected literals under {}",
                p.as_str()
            );
        }
    }
    assert_eq!(pkg.version, "1.4.0");
    assert_eq!(pkg.digest, policy::compute_digest());
    let cp = ai_slop::engine::compiled().unwrap();
    let snapshot = ai_slop::skill::generate(&cp.pkg);
    assert!(snapshot.contains(&pkg.digest));
    assert!(snapshot.contains("policy version: 1.4.0"));
}

// --- P2: SLOP-X004 house-report-template exemption --------------------------

const HOUSE_REPORT: &str = "# Parser drops the final row\n\n\
    ## Reproducer\n\nRun the parser on the attached two-row input.\n\n\
    ## Observed\n\nThe second row is missing from the output.\n\n\
    ## Expected\n\nBoth rows appear in the output.\n\n\
    ## Root cause\n\nThe loop stops one index early.\n";

/// The house template on public-bug-report is the declared exemption: a short
/// report with exactly the template sections after its title raises no X004.
#[test]
fn x004_house_template_is_exempt_on_public_bug_report() {
    let report = run(HOUSE_REPORT, Profile::PublicBugReport);
    assert_invariants(HOUSE_REPORT, &report);
    assert!(
        !has_rule(&report, "SLOP-X004"),
        "the house template fired X004: {:?}",
        common::rule_ids(&report)
    );
}

/// One extra heading breaks the exact match and X004 fires again, so the
/// exemption cannot widen past the declared template.
#[test]
fn x004_extra_heading_still_fires() {
    let doc = format!("{HOUSE_REPORT}\n## Impact\n\nUsers lose the last row.\n");
    let report = run(&doc, Profile::PublicBugReport);
    assert!(
        has_rule(&report, "SLOP-X004"),
        "extra heading must fire: {:?}",
        common::rule_ids(&report)
    );
}

/// Reordered sections are not the template: the match is order-sensitive.
#[test]
fn x004_reordered_headings_still_fire() {
    let doc = HOUSE_REPORT
        .replace("## Reproducer", "## TEMP")
        .replace("## Observed", "## Reproducer")
        .replace("## TEMP", "## Observed");
    let report = run(&doc, Profile::PublicBugReport);
    assert!(
        has_rule(&report, "SLOP-X004"),
        "reordered template must fire: {:?}",
        common::rule_ids(&report)
    );
}

/// The exemption is profile-scoped: the same headings on a readme still fire,
/// and internal-doc keeps X004 off entirely.
#[test]
fn x004_exemption_is_profile_scoped() {
    let report = run(HOUSE_REPORT, Profile::Readme);
    assert!(
        has_rule(&report, "SLOP-X004"),
        "readme is not in the exempt set: {:?}",
        common::rule_ids(&report)
    );
    let report = run(HOUSE_REPORT, Profile::InternalDoc);
    assert!(
        !has_rule(&report, "SLOP-X004"),
        "X004 is off on internal-doc"
    );
}

// --- P4: SLOP-W001 hyphenated compounds are single tokens --------------------

#[test]
fn w001_hyphenated_compounds_do_not_fire() {
    for text in [
        "The serial-port adapter reconnects after a reset.\n",
        "The port-forwarding rule maps 8080 to 80.\n",
    ] {
        let report = run(text, Profile::Readme);
        assert_invariants(text, &report);
        assert!(
            !has_rule(&report, "SLOP-W001"),
            "hyphenated compound fired: {text:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

#[test]
fn w001_standalone_scrub_words_still_fire() {
    for text in [
        "We port the parser to the new runtime.\n",
        "A ported routine handles the framing.\n",
    ] {
        let report = run(text, Profile::Readme);
        assert!(
            has_rule(&report, "SLOP-W001"),
            "standalone scrub word missed: {text:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// The suppression is W001-scoped: a hyphenated carrier of an ornamental
/// lexicon term still fires A001, so `-` did not become a global word char.
#[test]
fn hyphen_suppression_does_not_leak_to_other_rules() {
    let t = "| widget | a truly game-changer design |\n";
    let report = run(t, Profile::InternalDoc);
    assert!(
        has_rule(&report, "SLOP-A001"),
        "A001 lost its hyphenated term: {:?}",
        common::rule_ids(&report)
    );
}

// --- P7: SLOP-C004 concession at sentence starts ------------------------------

/// The escaped shape from the harvest ledger: a "While X, Y" concession
/// opening a sentence MID-paragraph, which the line-anchored pattern missed.
#[test]
fn c004_mid_paragraph_while_concession_fires() {
    let t = "The BEL form keeps the text. While the parser recognizes BEL as a terminator, an ST-terminated sequence consumes the rest of the line.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-C004")
        .expect("sentence-start concession must fire");
    assert_eq!(f.state, "candidate");
}

/// Line-start coverage is retained alongside the widening.
#[test]
fn c004_line_start_concession_still_fires() {
    let t = "While the cache warms, requests queue behind the lock.\n";
    let report = run(t, Profile::InternalDoc);
    assert!(
        has_rule(&report, "SLOP-C004"),
        "line-start concession lost: {:?}",
        common::rule_ids(&report)
    );
}

/// Mid-sentence temporal `while` (no sentence boundary before it) stays out
/// of the pattern's reach.
#[test]
fn c004_mid_sentence_temporal_while_stays_silent() {
    let t = "The daemon logs each frame while the socket stays open, then rotates the file.\n";
    let report = run(t, Profile::InternalDoc);
    assert!(
        !has_rule(&report, "SLOP-C004"),
        "mid-sentence temporal while fired: {:?}",
        common::rule_ids(&report)
    );
}

/// The reconstruction fixture (the original session sentence was reworded
/// before filing, so this is a labeled same-shape substitute): adjudicated
/// expectation is exactly one C004 candidate on the concession sentence, and
/// no X004 because the report carries the house template (P2).
#[test]
fn c004_reconstruction_fixture_expectation() {
    let path = format!(
        "{}/fixtures/golden/ansi-to-tui-osc-while-contrast-reconstruction.md",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).unwrap();
    let report = run(&text, Profile::PublicBugReport);
    assert_invariants(&text, &report);
    let ids: Vec<(&str, &str)> = report
        .findings
        .iter()
        .map(|f| (f.rule_id.as_str(), f.state.as_str()))
        .collect();
    assert_eq!(ids, vec![("SLOP-C004", "candidate")], "fixture drifted");
    // The span carries the sentence boundary the widened alternation matched,
    // then the concession clause up to its comma.
    let span = &report.findings[0].spans[0];
    assert!(
        text[span.start..span.end].contains("While the parser recognizes"),
        "C004 span moved off the concession sentence"
    );
}
