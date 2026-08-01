//! Section 12.3: `source[start..end] == snippet` for every finding on every
//! input, segmentation total coverage, duplicate substrings, nested
//! markdown, multibyte and CRLF cases.

mod common;

use ai_slop::Profile;
use common::{assert_invariants, run};

const INPUTS: &[&str] = &[
    "We delve into things.\n",
    "Plain text, no markdown constructs at all.",
    "# Head\n\nA — dash and a ; semicolon.\n\n```rust\nlet x = 1; // delve\n```\n",
    "CRLF line one.\r\nWe delve here.\r\n\r\nMore.\r\n",
    "Entity dash &mdash; here and &#8212; there.\n",
    "Zero width de\u{200B}lve evasion.\n",
    "Multibyte céfé — naïve delve résumé.\n",
    "> quoted; semicolon and — dash inside a blockquote\n",
    "- item one with *emphasis*\n- [link text](https://example.com/?utm_source=chatgpt&x=1)\n",
    "delve delve delve\n\ndelve again delve\n",
    "Nested [link with *emph inside* text](https://e.io) in a list:\n\n- > quote with **bold delve** here\n",
    "| a | b |\n|---|---|\n| delve | — |\n",
    "Footnote[^1] text.\n\n[^1]: the note has a — dash\n",
    "Autolink <https://example.com/?utm_source=chatgpt.com> in prose.\n",
    "Heading *with emph*\n===\n\nbody\n",
    "Term split across a soft\nbreak: game\nchanger indeed.\n",
    "An i\u{0301}nput with combining marks and delve.\n",
    "*This* is **not** what it seems, but rather — well.\n",
];

#[test]
fn span_invariant_over_inputs() {
    for text in INPUTS {
        for profile in [
            Profile::PublicBugReport,
            Profile::Readme,
            Profile::InternalDoc,
        ] {
            let report = run(text, profile);
            assert_invariants(text, &report);
        }
    }
}

#[test]
fn duplicate_substring_occurrences_get_distinct_spans() {
    let text = "delve one delve two delve three\n";
    let report = run(text, Profile::Readme);
    let spans: Vec<(usize, usize)> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "SLOP-A001")
        .map(|f| (f.spans[0].start, f.spans[0].end))
        .collect();
    assert_eq!(spans, vec![(0, 5), (10, 15), (20, 25)]);
    for (s, e) in spans {
        assert_eq!(&text[s..e], "delve");
    }
}

#[test]
fn commit_and_text_formats_hold_invariants() {
    let commit =
        "feat: add thing\n\nBody line; with issues — here.\n\nSigned-off-by: A B <a@b.c>\n";
    let mut config = ai_slop::Config::new(Profile::CommitMessage);
    config.input_format = ai_slop::InputFormat::Commit;
    let report = ai_slop::analyze(commit.as_bytes(), &config).unwrap();
    assert_invariants(commit, &report);

    let mut config = ai_slop::Config::new(Profile::InternalDoc);
    config.input_format = ai_slop::InputFormat::Text;
    let text = "plain text with — a dash\nand a second; line\n";
    let report = ai_slop::analyze(text.as_bytes(), &config).unwrap();
    assert_invariants(text, &report);
}

#[test]
fn bom_offsets_index_the_post_bom_payload() {
    let payload = "\u{FEFF}We delve here.\n";
    let stripped = &payload[3..];
    let report =
        ai_slop::analyze(payload.as_bytes(), &ai_slop::Config::new(Profile::Readme)).unwrap();
    assert_invariants(stripped, &report);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-A001")
        .expect("delve fires");
    assert_eq!(&stripped[f.spans[0].start..f.spans[0].end], "delve");
    // The artifact hash covers the original bytes as received.
    assert_eq!(report.artifact.bytes, payload.len());
}
