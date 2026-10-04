//! A whole-document finding anchors on the first character of the payload,
//! and that character is not always one byte wide.
//!
//! A rule that reports about the whole document still
//! has to name a span, and it names the document's first character. Anchoring
//! on a hardcoded `0..1` cuts an emoji, an em dash, or an accented letter in
//! half, and a span landing inside a character fails the span invariant, so
//! the run reports an instrumentation error and exits 30 with no findings at
//! all. The invariant helper in `common` has always checked this. Nothing ever
//! handed it a document that opened on a multi-byte character.
//!
//! One test per anchoring site, so a new site that reintroduces the raw range
//! has somewhere obvious to fail.

mod common;

use ai_slop::{analyze, Config, InputFormat, Profile};
use common::{assert_invariants, run};

/// The three leading characters from the report, each several bytes wide.
const EMOJI: &str = "\u{1F389}";
const EM_DASH: &str = "\u{2014}";
const ACCENT: &str = "\u{e9}";

/// Every finding sits on character boundaries, and the one anchored at the
/// document start covers the whole first character.
fn assert_anchor(text: &str, profile: Profile, rule: &str) {
    let report = run(text, profile);
    assert_invariants(text, &report);
    let width = text.chars().next().expect("a first character").len_utf8();
    let anchored: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .collect();
    assert_eq!(
        anchored.len(),
        1,
        "{rule} did not report: {:?}",
        common::rule_ids(&report)
    );
    let span = &anchored[0].spans[0];
    assert_eq!(
        (span.start, span.end),
        (0, width),
        "{rule} anchored on {} bytes of a {width}-byte character",
        span.end - span.start
    );
}

/// The reported case. A contrast-density hint anchors the document, and the
/// document opens on a four-byte emoji.
#[test]
fn density_anchor_takes_a_whole_emoji() {
    let t = format!("{EMOJI} The rule reports the span, not the sentence.\n");
    assert_anchor(&t, Profile::InternalDoc, "SLOP-C009");
}

/// The same anchor behind a three-byte em dash.
#[test]
fn density_anchor_takes_a_whole_em_dash() {
    let t = format!("{EM_DASH} The rule reports the span, not the sentence.\n");
    assert_anchor(&t, Profile::InternalDoc, "SLOP-C009");
}

/// And behind a two-byte accented letter.
#[test]
fn density_anchor_takes_a_whole_accented_letter() {
    let t = format!("{ACCENT}clair notes, not the sentence.\n");
    assert_anchor(&t, Profile::InternalDoc, "SLOP-C009");
}

/// The structural word-cap rule anchors the same way.
#[test]
fn word_cap_anchor_takes_a_whole_character() {
    let body = "The parser handles nesting well. ".repeat(250);
    for lead in [EMOJI, EM_DASH, ACCENT] {
        let t = format!("{lead}clair notes. {body}");
        assert_anchor(&t, Profile::CommitMessage, "SLOP-X003");
    }
}

/// The excluded-content hint in the coverage family, where most of the
/// document is a fenced block.
#[test]
fn excluded_content_anchor_takes_a_whole_character() {
    for lead in [EMOJI, EM_DASH, ACCENT] {
        let t = format!(
            "{lead} short prose line.\n\n```rust\n{}\n```\n",
            "let value = compute(input);\n".repeat(20)
        );
        assert_anchor(&t, Profile::InternalDoc, "SLOP-H002");
    }
}

/// The input-shape hint in the same family, driven by mixed line endings.
#[test]
fn mixed_line_ending_anchor_takes_a_whole_character() {
    for lead in [EMOJI, EM_DASH, ACCENT] {
        let t = format!("{lead} first line.\r\nSecond line.\nThird line.\r\n");
        assert_anchor(&t, Profile::InternalDoc, "SLOP-H003");
    }
}

/// The release-body contract, the one anchoring site that needs deployment
/// configuration to reach.
#[test]
fn release_body_anchor_takes_a_whole_character() {
    for lead in [EMOJI, EM_DASH, ACCENT] {
        let text = format!("{lead} the notes say one thing.\n");
        let mut config = Config::new(Profile::ReleaseNotes);
        config.input_format = InputFormat::Markdown;
        config.deployment.expected_release_body = Some("Something else entirely.".to_string());
        let report = analyze(text.as_bytes(), &config).expect("analyze must succeed");
        assert_invariants(&text, &report);
        let width = text.chars().next().expect("a first character").len_utf8();
        let found = report
            .findings
            .iter()
            .find(|f| f.rule_id == "SLOP-K004")
            .expect("the release-body contract reports");
        assert_eq!(
            (found.spans[0].start, found.spans[0].end),
            (0, width),
            "K004 anchored on part of a character"
        );
    }
}

/// The whole failure mode in one assertion: a document opening on a multi-byte
/// character analyzes successfully, on every prose profile. The
/// manifest profile is out because it parses TOML, where a line of prose is
/// unsupported input whatever it opens on.
#[test]
fn a_multibyte_opening_never_fails_the_run() {
    for lead in [EMOJI, EM_DASH, ACCENT, "\u{4e2d}", "\u{1F1FA}\u{1F1F8}"] {
        for profile in Profile::ALL
            .into_iter()
            .filter(|p| *p != Profile::CargoMetadata)
        {
            let text = format!("{lead} The rule reports the span, not the sentence.\n");
            let config = Config::new(profile);
            let report = analyze(text.as_bytes(), &config)
                .unwrap_or_else(|e| panic!("{profile:?} failed on a {lead} opening: {e}"));
            assert_invariants(&text, &report);
        }
    }
}
