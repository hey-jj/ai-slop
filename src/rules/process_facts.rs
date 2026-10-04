//! process-facts family structural rule: SLOP-F004 rationale leak, a
//! sentence about the artifact that argues for the design. It leaves the
//! reader without the behavior and action instructions.
//!
//! Two marker families carry the shape. Design economics names the bargain
//! behind a choice (`which is the trade`, `at the cost of`, `by design`).
//! Reception instruction tells the reader how to take the text (`a reader
//! should discount`, `is best understood as`). Both are anchored on a tool
//! noun anywhere in the same sentence, so an ordinary sentence about the
//! world never fires. A pronoun is not an anchor: `it` and `this` point at
//! whatever the paragraph was last about, which is most of the time not the
//! artifact.
//!
//! The tool-noun set is SLOP-C010's, read from that rule's policy block, so
//! the two rules cannot drift apart.
//!
//! The negated reception form (`should not be read as`) belongs to
//! SLOP-C010's hedge family. Token-sequence matching keeps the two apart: the
//! affirmative marker `should be read as` cannot match across the `not`.
//!
//! The scan runs over the norm view and reuses the contrast module's bounded
//! sentence, clause, and token scans, so no new scanning primitive enters the
//! crate.

use crate::engine::{CompiledPolicy, Hit};
use crate::input::Prepared;
use crate::views::NormView;
use crate::Config;

pub const HANDLED: &[&str] = &["SLOP-F004"];

/// Read the shared tool-noun set from C010's policy block for C010 and F004.
/// An absent declaration returns an empty set and leaves this rule silent
/// without guessing a substitute. The policy CI test pins the set's contents
/// so that an absent set cannot ship.
pub(crate) fn shared_tool_nouns(cp: &CompiledPolicy) -> Vec<String> {
    cp.pkg
        .rule_by_id("SLOP-C010")
        .map(|r| super::contrast::str_list(r, "tool_nouns"))
        .unwrap_or_default()
}

pub fn evaluate(
    cp: &CompiledPolicy,
    prepared: &Prepared,
    norm: &NormView,
    config: &Config,
    hits: &mut Vec<Hit>,
) {
    let Some(idx) = super::active(cp, config, "SLOP-F004") else {
        return;
    };
    let rule = &cp.pkg.rules[idx];
    let markers = super::contrast::str_list(rule, "design_markers");
    let reception = super::contrast::str_list(rule, "reception_markers");
    let tool_nouns = shared_tool_nouns(cp);

    let text = norm.text.as_str();
    let src = prepared.text.as_str();
    for block in super::contrast::block_ranges(text) {
        for sentence in super::contrast::sentence_ranges(text, block, &[]) {
            let toks = super::contrast::tokenize(text, sentence.clone());
            if toks.is_empty() {
                continue;
            }
            // The anchor: a tool noun, singular or plural, anywhere in the
            // same sentence.
            if !toks.iter().any(|t| tool_nouns.contains(&t.word)) {
                continue;
            }
            // One finding per marker. Two reasons in one sentence are two
            // things to cut, and a reader answering the judge question needs
            // the span of each.
            let mut at = 0usize;
            while at < toks.len() {
                let design = super::contrast::phrase_at_any(&toks, at, &markers);
                let instruct = super::contrast::phrase_at_any(&toks, at, &reception);
                let (matched, detail) = match (design, instruct) {
                    (Some(d), _) => (d, "the bargain behind the design"),
                    (None, Some(r)) => (r, "how to take the text"),
                    (None, None) => {
                        at += 1;
                        continue;
                    }
                };
                let last = at + matched.1.len - 1;
                let span = toks[at].start..toks[last].end;
                at = last + 1;
                let Some(source_span) = norm.to_source(span.clone()) else {
                    continue;
                };
                let source_span = crate::widen_to_char_boundaries(src, source_span);
                if source_span.start >= source_span.end {
                    continue;
                }
                let mut hit = Hit::new(idx, source_span);
                hit.quoted = norm.all_quoted(&span);
                hit.trigger = Some(text[span].to_string());
                hit.detail = Some(detail.to_string());
                hits.push(hit);
            }
        }
    }
}
