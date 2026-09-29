//! Policy 1.8.0: the bare word `impact` leaves the SLOP-F003 lexicon. The
//! rule matches by substring, so the entry fired on `impacted`, `impacts`,
//! and every neutral sentence that names what a change reaches, which the
//! guard itself files as content. The entries that rank a consequence stay,
//! `high-impact` among them, and SLOP-S002 keeps the `Impact` heading on the
//! bug-report profile.

mod common;

use ai_slop::{policy, Profile};
use common::{assert_invariants, has_rule, run};

fn f003_fires(text: &str) -> bool {
    let report = run(text, Profile::PublicBugReport);
    assert_invariants(text, &report);
    has_rule(&report, "SLOP-F003")
}

// --- lexicon pin ------------------------------------------------------------

/// The bare word is gone and the hyphenated compound is not: a later edit
/// that puts `impact` back, or takes `high-impact` with it, fails here.
#[test]
fn f003_lexicon_drops_bare_impact_and_keeps_high_impact() {
    let pkg = policy::load().unwrap();
    let f003 = pkg.rule_by_id("SLOP-F003").unwrap();
    assert!(
        !f003.terms.iter().any(|t| t == "impact"),
        "bare impact is back in the F003 lexicon"
    );
    assert!(
        f003.terms.iter().any(|t| t == "high-impact"),
        "high-impact left the F003 lexicon with the bare word"
    );
    assert_eq!(f003.terms.len(), 26, "the F003 lexicon carries 26 entries");
}

// --- keep-tests: every surviving entry still fires ---------------------------

/// Each remaining lexicon entry, dropped into a plain sentence on the
/// bug-report profile, still produces an F003 finding. The loop reads the
/// lexicon from the loaded policy, so a new entry is covered the day it
/// lands and a silent one is caught the same day.
#[test]
fn f003_every_surviving_entry_fires_on_public_bug_report() {
    let pkg = policy::load().unwrap();
    let f003 = pkg.rule_by_id("SLOP-F003").unwrap();
    assert!(!f003.terms.is_empty());
    for term in &f003.terms {
        let text = format!("The parser drops the field, {term} for callers.\n");
        assert!(
            f003_fires(&text),
            "surviving F003 entry went silent: {term:?}"
        );
    }
}

/// The compound that ranks the consequence keeps firing in both cases.
#[test]
fn f003_high_impact_compound_still_fires() {
    for t in [
        "This is a high-impact bug for every caller.\n",
        "HIGH-IMPACT: the fourth field is gone.\n",
    ] {
        assert!(f003_fires(t), "high-impact went silent: {t:?}");
    }
}

/// The verdict phrases beside the dropped word are untouched.
#[test]
fn f003_verdict_phrases_still_fire() {
    for t in [
        "This must be fixed before the next release.\n",
        "The parser bug is severe and callers see it at once.\n",
        "Fix this immediately.\n",
        "An attacker can easily reach the parser with crafted input.\n",
    ] {
        assert!(f003_fires(t), "verdict phrase went silent: {t:?}");
    }
}

// --- newly silent: sentences that only matched on the bare word ------------

/// Naming what a change reaches is the mechanism the guard files as content.
#[test]
fn f003_neutral_impact_prose_is_silent() {
    for t in [
        "The impact of the change is limited to callers of the fourth field.\n",
        "Impacted callers read None instead of the fourth field.\n",
        "This impacts every caller that reads the fourth field.\n",
        "The fix has no impact on the first three fields.\n",
    ] {
        assert!(!f003_fires(t), "bare impact still fires: {t:?}");
    }
}

/// The literal `Impact` heading no longer carries an F003 finding. SLOP-S002
/// still reports it as a verdict heading on the bug-report profile, and that
/// rule is unchanged.
#[test]
fn f003_impact_heading_is_left_to_s002() {
    let t = "# Title: parser drops trailing field\n\n## Observed\n\nThe parser returns three fields.\n\n## Expected\n\nFour fields.\n\n## Impact\n\nCallers reading the fourth field get None.\n\n## Repro\n\nRun the parser on the sample.\n";
    let report = run(t, Profile::PublicBugReport);
    assert_invariants(t, &report);
    assert!(
        !has_rule(&report, "SLOP-F003"),
        "F003 fired on the heading: {:?}",
        common::rule_ids(&report)
    );
    let s002 = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-S002")
        .expect("S002 keeps the verdict heading");
    assert_eq!(common::snippet(s002), "Impact");
    assert_eq!(s002.state, "violation");
}

// --- word edges ------------------------------------------------------------

/// Every entry matches on word edges, and a hyphen is a word edge, so the
/// negated and mechanism compounds carry their own exemptions. The plain
/// entries and the adverb keep firing.
#[test]
fn f003_matches_on_word_edges() {
    for t in [
        "The retry loop must persevere through the outage.\n",
        "The non-urgent queue drains nightly.\n",
        "The fault is non-severe and logged.\n",
        "The bug is unexploitable on 64-bit builds.\n",
        "The bug is non-exploitable on 64-bit builds.\n",
        "Self-remediation runs after the alert.\n",
        "Auto-remediation runs after the alert.\n",
    ] {
        assert!(!f003_fires(t), "F003 fired inside a compound: {t:?}");
    }
    for t in [
        "The outage is severe for every caller.\n",
        "The parser is severely broken on this input.\n",
        "This is urgent for every caller.\n",
        "The bug is exploitable from the network.\n",
        "Remediation is a one-line change.\n",
    ] {
        assert!(f003_fires(t), "F003 went silent: {t:?}");
    }
}
