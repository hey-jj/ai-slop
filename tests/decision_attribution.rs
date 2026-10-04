//! SLOP-V006 decision attribution: every spelling that credits a decision to
//! a role noun in the third person, the dated stamp joining its span, the
//! ordinary English the role nouns collide with, and the profile stances.

mod common;

use ai_slop::Profile;
use common::{assert_invariants, has_rule, run};

fn v006(report: &ai_slop::Report) -> Vec<String> {
    report
        .findings
        .iter()
        .filter(|f| f.rule_id == "SLOP-V006")
        .map(common::snippet)
        .collect()
}

/// The owner's specimen. The possessive with a decision noun, the date joined
/// to the span, one report.
#[test]
fn v006_dated_possessive_reports_once_with_the_date() {
    let t = "Owner's ruling, 2026-08-20: the profile stays.\n";
    let report = run(t, Profile::PublicBugReport);
    assert_invariants(t, &report);
    assert_eq!(v006(&report), vec!["Owner's ruling, 2026-08-20"]);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-V006")
        .unwrap();
    assert_eq!(f.state, "candidate");
}

/// One case per spelling, each pinned to the span a writer is asked to cut.
#[test]
fn v006_every_spelling_fires_on_its_span() {
    for (text, span) in [
        (
            "The owner ruled that the lexicon drops the bare word.\n",
            "The owner ruled that",
        ),
        (
            "The maintainer's call was to keep the gate.\n",
            "The maintainer's call",
        ),
        (
            "This entry was requested by the user.\n",
            "requested by the user",
        ),
        (
            "The owner-flagged stack ships as a rule.\n",
            "owner-flagged",
        ),
        (
            "Code sign-off is Fable, by owner decision.\n",
            "owner decision",
        ),
        ("Per the owner, the digest stays sealed.\n", "Per the owner"),
        (
            "The flag was added at the user's request.\n",
            "at the user's request",
        ),
        (
            "On the principal's instruction the run stopped.\n",
            "On the principal's instruction",
        ),
        (
            "Owner decision (2026-08-20): keep the profile.\n",
            "Owner decision (2026-08-20)",
        ),
        ("- Ruling: keep the profile.\n", "Ruling:"),
        ("- **Ruling:** keep the profile.\n", "Ruling:"),
        (
            "Decision, 2026-08-19: the profile stays.\n",
            "Decision, 2026-08-19:",
        ),
        (
            "Ruled by owner proxy, the rule is candidate tier.\n",
            "Ruled by owner proxy",
        ),
        (
            "The proxy signed off on 2026-08-20.\n",
            "The proxy signed off on 2026-08-20",
        ),
        (
            "The owner\u{2019}s verdict was to drop the entry.\n",
            "The owner\u{2019}s verdict",
        ),
        (
            "The reviewer approved it on 2026-08-20.\n",
            "The reviewer approved it on 2026-08-20",
        ),
        (
            "The owner ruled this out after the inspection.\n",
            "The owner ruled this",
        ),
        (
            "Approved by the maintainer's proxy.\n",
            "Approved by the maintainer's proxy",
        ),
        ("The owner wants the profile kept.\n", "The owner wants"),
        (
            "The orchestrator's call was to stop.\n",
            "The orchestrator's call",
        ),
        (
            "Owner directive (2026-08-24): stop the run.\n",
            "Owner directive (2026-08-24)",
        ),
        (
            "The maintainer asked for a smaller patch.\n",
            "The maintainer asked",
        ),
        ("Per your ruling, the digest stays.\n", "Per your ruling"),
        ("You ruled that the bare word leaves.\n", "You ruled"),
        (
            "The owner's request was a shorter README.\n",
            "The owner's request",
        ),
        ("The lead chose the second option.\n", "The lead chose"),
        ("Ruling (2026-08-20): keep.\n", "Ruling (2026-08-20):"),
        (
            "Directive of 2026-08-24 stands.\n",
            "Directive of 2026-08-24",
        ),
        (
            "Ruled by a Fable agent in the owner's stead.\n",
            "the owner's stead",
        ),
        (
            "Drafted under owner-proxy Ruling 011.\n",
            "owner-proxy Ruling",
        ),
        (
            "The owner has ruled on everything current.\n",
            "The owner has ruled",
        ),
        ("The owner greenlit the batch.\n", "The owner greenlit"),
        ("As you directed, the run stopped.\n", "As you directed"),
        (
            "Owner correction 2026-08-22: the gate relaxes.\n",
            "Owner correction 2026-08-22",
        ),
    ] {
        let report = run(text, Profile::PublicBugReport);
        assert_invariants(text, &report);
        assert_eq!(v006(&report), vec![span.to_string()], "{text:?}");
    }
}

/// The role nouns are ordinary English. Term-of-art compounds, config facts,
/// nouns that happen to share a spelling, and the loose verbs beside the
/// open roles (the user asked, the author said) stay silent.
#[test]
fn v006_ordinary_english_is_silent() {
    for text in [
        "Rate limits apply per user and per owner of a token.\n",
        "User-defined types and user-specified widths are supported.\n",
        "The value set by the user wins over the default.\n",
        "The rules read text and report every span.\n",
        "The decision tree splits on the first field.\n",
        "The call stack is printed on panic.\n",
        "The owner of the file can change its mode.\n",
        "The author's name appears in the header.\n",
        "The reviewer's comments are attached to the pull request.\n",
        "The lead developer's laptop runs the tests.\n",
        "The user session expires after an hour.\n",
        "A verdict from the court arrived on 2026-08-20.\n",
        "Decision records live under docs/adr.\n",
        "Ruled 2026-08-14 after the sweep.\n",
        "The verdicts were mixed.\n",
        "The user wants a faster export, and the author said so in the preface.\n",
        "Maintainer approval is required for a merge.\n",
        "The user asked for a dark theme.\n",
        "Under your account settings the theme changes.\n",
        "I said no, and you decided to wait.\n",
    ] {
        let report = run(text, Profile::PublicBugReport);
        assert!(
            v006(&report).is_empty(),
            "V006 fired on ordinary English {text:?}: {:?}",
            v006(&report)
        );
    }
}

/// The span carries the attribution and nothing after it, so the fact the
/// decision produced is left standing for the writer to keep.
#[test]
fn v006_span_stops_before_the_fact() {
    let t = "The owner's decision was to keep every profile name.\n";
    let report = run(t, Profile::Readme);
    assert_eq!(v006(&report), vec!["The owner's decision"]);
}

/// api-docs and internal-doc relax to advisory, which never gates. Everywhere
/// else the finding blocks as a candidate.
#[test]
fn v006_profile_stances() {
    let t = "The user approves the consent screen.\n";
    for profile in [Profile::ApiDocs, Profile::InternalDoc] {
        let report = run(t, profile);
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "SLOP-V006")
            .unwrap_or_else(|| panic!("V006 silent under {profile:?}"));
        assert_eq!(f.lifecycle, "advisory", "{profile:?}");
        assert_eq!(report.result_state, "no_findings", "{profile:?}");
    }
    for profile in [
        Profile::PublicBugReport,
        Profile::PublicComment,
        Profile::Readme,
        Profile::Changelog,
        Profile::ReleaseNotes,
        Profile::CommitMessage,
    ] {
        let report = run(t, profile);
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "SLOP-V006")
            .unwrap_or_else(|| panic!("V006 silent under {profile:?}"));
        assert_eq!(f.state, "candidate", "{profile:?}");
    }
}

/// The seam with SLOP-V005: the dated owner-verdict stamp reports under both
/// rules, and the roleless verb-plus-date stamp reports under V005 alone.
#[test]
fn v006_seam_with_the_ledger_stamp() {
    let t = "The owner rules this on 2026-08-18.\n";
    let report = run(t, Profile::Readme);
    assert!(has_rule(&report, "SLOP-V005"));
    assert_eq!(v006(&report), vec!["The owner rules this on 2026-08-18"]);

    let t = "The floor was measured 2026-08-01 across the boundary.\n";
    let report = run(t, Profile::Readme);
    assert!(has_rule(&report, "SLOP-V005"));
    assert!(v006(&report).is_empty());
}

/// Inline code is segmented out, so a skill that quotes the shape in
/// backticks can document it without firing on itself.
#[test]
fn v006_inline_code_is_silent() {
    let t = "The tell is `Owner's ruling, 2026-08-20` at the head of a paragraph.\n";
    let report = run(t, Profile::Readme);
    assert!(v006(&report).is_empty(), "{:?}", v006(&report));
}
