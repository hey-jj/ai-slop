//! SLOP-A004 inflated diction, plus the W001 scrub-list change that unbans
//! the bare word AI. The A004 regression anchor is a tool description built
//! from inflated noun stacks, which the rule must flag, against a set of
//! plain technical sentences the rule must leave alone.

mod common;

use ai_slop::Profile;
use common::{assert_invariants, has_rule, run};

// --- SLOP-A004 inflated diction ---------------------------------------------

/// A tool description carrying both tells: the tool-noun stack "coverage
/// instrument" and the participial noun stack "generated-text defects".
const INFLATED_DESCRIPTION: &str = "Deterministic detector and coverage instrument \
    for generated-text defects in outbound technical artifacts\n";

#[test]
fn an_inflated_tool_description_fires_inflated_diction() {
    let report = run(INFLATED_DESCRIPTION, Profile::Readme);
    assert_invariants(INFLATED_DESCRIPTION, &report);
    let a004: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "SLOP-A004")
        .collect();
    assert!(
        a004.len() >= 2,
        "both noun stacks must fire: {:?}",
        common::rule_ids(&report)
    );
    assert!(a004.iter().all(|f| f.state == "candidate"));
    let spans: Vec<&str> = a004
        .iter()
        .map(|f| &INFLATED_DESCRIPTION[f.spans[0].start..f.spans[0].end])
        .collect();
    assert!(spans.contains(&"coverage instrument"), "{spans:?}");
    assert!(spans.contains(&"generated-text defects"), "{spans:?}");
}

#[test]
fn the_inflated_description_fires_in_the_manifest_profile_too() {
    let manifest = format!(
        "[package]\nname = \"t\"\nversion = \"1.0.0\"\ndescription = \"{}\"\nkeywords = [\"a\",\"b\",\"c\",\"d\",\"e\"]\ncategories = [\"parsing\",\"no-std\"]\n",
        INFLATED_DESCRIPTION.trim()
    );
    let mut config = ai_slop::Config::new(Profile::CargoMetadata);
    config.input_format = ai_slop::InputFormat::Manifest;
    let report = ai_slop::analyze(manifest.as_bytes(), &config).unwrap();
    assert!(
        report.findings.iter().any(|f| f.rule_id == "SLOP-A004"),
        "manifest description must run through A004: {:?}",
        common::rule_ids(&report)
    );
}

#[test]
fn inflated_word_set_fires_on_the_curated_tells() {
    for text in [
        "The service utilizes a queue to schedule work.",
        "This module facilitates communication between the two processes.",
        "The aforementioned flag controls both paths.",
        "We operationalize the checklist in CI.",
    ] {
        let report = run(text, Profile::InternalDoc);
        assert!(has_rule(&report, "SLOP-A004"), "missed: {text}");
    }
}

/// Plain technical prose, including dense-but-legitimate sentences, must not
/// fire. These are the calibration anchors for list membership.
#[test]
fn plain_technical_prose_does_not_fire_inflated_diction() {
    for text in [
        // A normal bug report sentence.
        "The parser returns an error when the input ends inside a fence.",
        // A normal README line.
        "ai-slop lints bug reports, commit messages, and changelogs before they ship.",
        // Dense but legitimate API prose.
        "The reverse DFA recovers the start offset for each matched pattern span.",
        // A normal fix description.
        "Restarting the worker clears the stale cache entry and the retry succeeds.",
        // Exempt resource metrics, including multi-word collocations.
        "CPU utilization stays under 80 percent under sustained load.",
        "Cache utilization improves when the working set fits in L2.",
        "Connection pool utilization peaked at 92 percent.",
        // The verb sense of instrument.
        "We instrument the allocator to count peak usage.",
    ] {
        let report = run(text, Profile::InternalDoc);
        assert!(!has_rule(&report, "SLOP-A004"), "false positive on: {text}");
    }
}

/// Homograph senses of the pattern words stay silent: instrument as a
/// measured, financial, or musical noun follows none of the tool-stack
/// modifiers.
#[test]
fn instrument_homographs_do_not_fire() {
    for text in [
        "An oscilloscope is a measurement instrument.",
        "A bond is a financial instrument, not a loan.",
        "The cockpit instrument panel failed during the test.",
        "The cello is a bowed string instrument.",
        "The instrumentation error path exits with code 30.",
    ] {
        let report = run(text, Profile::InternalDoc);
        assert!(!has_rule(&report, "SLOP-A004"), "false positive on: {text}");
    }
}

// --- SLOP-W001 scrub change: bare AI is no longer a scrub word ---------------

#[test]
fn bare_ai_no_longer_scrubs_but_the_rest_of_the_list_holds() {
    let report = run(
        "This linter flags AI slop in outbound text.",
        Profile::Readme,
    );
    assert!(
        !has_rule(&report, "SLOP-W001"),
        "AI must not scrub: {:?}",
        common::rule_ids(&report)
    );
    let report = run("The upstream maintainers were notified.", Profile::Readme);
    assert!(
        has_rule(&report, "SLOP-W001"),
        "scrub list lost more than ai"
    );
}
