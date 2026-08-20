//! 0.1.7 harvest proposals (policy 1.4.0): the E002 internal-doc verdict-token
//! allowlist via `profile_exemptions`, the X004 house-report-template
//! exemption, the W001 hyphen-form technical-compound exemption literals, and
//! the C004 sentence-boundary hardening with the mid-paragraph concession
//! demoted to the skill's hand-read (reconstruction fixture pinned clean).

mod common;

use ai_slop::{policy, Profile};
use common::{assert_invariants, has_rule, run};

// --- P1: SLOP-E002 profile_exemptions, case-sensitive and profile-scoped ----

/// The exact ledger verdict token, standing alone — its own clause end or a
/// table cell — is internal-doc vocabulary: the E002 hit on its `NOT` is
/// suppressed because the listed literal covers the span as a standalone
/// token.
#[test]
fn e002_standalone_do_not_build_is_exempt_on_internal_doc() {
    for t in [
        "The verdict stands: DO NOT BUILD.\n",
        "| Crate | Verdict |\n| --- | --- |\n| widget | DO NOT BUILD |\n",
    ] {
        let report = run(t, Profile::InternalDoc);
        assert_invariants(t, &report);
        assert!(
            !has_rule(&report, "SLOP-E002"),
            "the internal-doc verdict token fired: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// The same bytes on an outbound surface stay a candidate: the exemption is
/// scoped to the profiles the policy lists, never global.
#[test]
fn e002_do_not_build_still_fires_on_public_bug_report() {
    let t = "The verdict stands: DO NOT BUILD.\n";
    let report = run(t, Profile::PublicBugReport);
    assert_invariants(t, &report);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-E002")
        .expect("E002 must stay hot outside internal-doc");
    assert_eq!(f.state, "candidate");
}

/// A continuing word after the literal means the bytes are running prose
/// wearing the verdict's spelling, not a verdict label: the covering literal
/// exempts nothing and both NOTs of the sentence fire.
#[test]
fn e002_do_not_build_mid_sentence_still_fires_on_internal_doc() {
    let t = "We DO NOT BUILD trust by shipping unreviewed patches.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    assert!(
        has_rule(&report, "SLOP-E002"),
        "mid-sentence DO NOT BUILD must stay a finding: {:?}",
        common::rule_ids(&report)
    );
}

/// The covering match is word-bounded: the literal spelled across a word's
/// interior (`AVOCA[DO NOT BUILD] LIST`) covers nothing.
#[test]
fn e002_embedded_literal_spelling_still_fires_on_internal_doc() {
    let t = "Add it to the AVOCADO NOT BUILD LIST today.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    assert!(
        has_rule(&report, "SLOP-E002"),
        "embedded literal spelling must stay a finding: {:?}",
        common::rule_ids(&report)
    );
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
    assert_eq!(pkg.version, "1.5.0");
    assert_eq!(pkg.digest, policy::compute_digest());
    let cp = ai_slop::engine::compiled().unwrap();
    let snapshot = ai_slop::skill::generate(&cp.pkg);
    assert!(snapshot.contains(&pkg.digest));
    assert!(snapshot.contains("policy version: 1.5.0"));
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

// --- P4: SLOP-W001 hyphen forms of the technical port compounds ---------------

/// The listed technical compounds are exemption literals, hyphen spelling
/// included. Only these stay silent — hyphenation itself suppresses nothing.
#[test]
fn w001_hyphenated_technical_compounds_do_not_fire() {
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

/// A scrub word hyphen-joined to an ornament is still the scrub word: the
/// blocking provenance rule cannot be bypassed by hyphenation. These four
/// gated CLEAN under the reverted blanket hyphen suppression.
#[test]
fn w001_hyphenated_hype_compounds_still_fire() {
    for text in [
        "A research-backed cleanup of the framing layer.\n",
        "The audit-ready module ships this week.\n",
        "A straight-port of the original decoder.\n",
        "We take an upstream-first approach to fixes.\n",
    ] {
        let report = run(text, Profile::Readme);
        assert_invariants(text, &report);
        assert!(
            has_rule(&report, "SLOP-W001"),
            "hyphenated scrub word bypassed W001: {text:?} {:?}",
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

/// `-` is a word-boundary character everywhere: a hyphenated carrier of an
/// ornamental lexicon term fires A001, exemption literals notwithstanding.
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

// --- P7: SLOP-C004 stays line-start; hand-read owns the mid-paragraph form ---

/// The mid-paragraph "While X, Y" concession is the skill's hand-read shape,
/// not the rule's: a 958-file corpus probe measured the widened
/// sentence-start matcher mostly on temporal `while` and legitimate human
/// contrasts (a negation-restricted variant kept temporal-while FPs and lost
/// the genuine concessions), so the arm was demoted back to the 0.1.6
/// line-start pattern.
#[test]
fn c004_mid_paragraph_while_concession_is_hand_read() {
    let t = "The BEL form keeps the text. While the parser recognizes BEL as a terminator, an ST-terminated sequence consumes the rest of the line.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    assert!(
        !has_rule(&report, "SLOP-C004"),
        "mid-paragraph concession is hand-read, not rule-caught: {:?}",
        common::rule_ids(&report)
    );
}

/// Line-start coverage is the 0.1.6 behavior, preserved.
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

/// The staged-agreement pattern keeps its sentence-boundary arm, and that
/// boundary is now a REAL one: a list marker's period ("2. Granted") and an
/// abbreviation's period ("e.g. granted") no longer open a match, via the
/// ordinal test and SLOP-C007's own terminal test.
///
/// The ordinal test replaced a blanket digit test that suppressed after ANY
/// digit-final text, which is why "The list has 2. Granted, ..." was pinned
/// silent here through 0.1.8. Evidence for the change: in that sentence the
/// numeral is the object of `has` and the period closes a real sentence,
/// which is the same shape as "version 2. Granted, ...". The two cannot both
/// hold, and both now fire. Only a digit run that OPENS its line is a list
/// marker, and the marker case is read in the text format, where a line
/// break survives into the norm view. Under markdown a numbered item is
/// structure the extractor strips, and the line-start alternative reads the
/// remaining opening on its own, which is a separate reporting path.
#[test]
fn c004_sentence_boundary_arm_rejects_list_markers_and_abbreviations() {
    let marker = "2. Granted, the code is shorter, but it hides the cost.\n";
    let mut text_cfg = common::cfg(Profile::InternalDoc);
    text_cfg.input_format = ai_slop::InputFormat::Text;
    let report = ai_slop::analyze(marker.as_bytes(), &text_cfg).expect("analyze must succeed");
    assert_invariants(marker, &report);
    assert!(
        !has_rule(&report, "SLOP-C004"),
        "a list marker opened a match: {:?}",
        common::rule_ids(&report)
    );

    let abbrev = "See the docs, e.g. granted, the flag is set, but the cache stays cold.\n";
    let report = run(abbrev, Profile::InternalDoc);
    assert_invariants(abbrev, &report);
    assert!(
        !has_rule(&report, "SLOP-C004"),
        "an abbreviation opened a match: {:?}",
        common::rule_ids(&report)
    );
    for t in [
        "It shipped early. Granted, the code is shorter, but it hides the cost.\n",
        "The list has 2. Granted, the code is shorter, but it hides the cost.\n",
        "The stable protocol is version 2. Granted, the code is shorter, but it hides the cost.\n",
    ] {
        let report = run(t, Profile::InternalDoc);
        assert_invariants(t, &report);
        assert!(
            has_rule(&report, "SLOP-C004"),
            "real sentence boundary lost: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// The reconstruction fixture (the original session sentence was reworded
/// before filing, so this is a labeled same-shape substitute): with the
/// sentence-start arm demoted to the skill's hand-read, the machine
/// expectation is a clean pass — the concession sentence is the hand-read's
/// to rule, and no X004 because the report carries the house template (P2).
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
    assert_eq!(ids, Vec::<(&str, &str)>::new(), "fixture drifted");
}
