//! contrast family structural rules.
//!
//! SLOP-C007 apophatic self-definition,
//! trigger T1 — the trailing negation tag (`, not <NP>.` / `, never <NP>.`).
//! The rule's T2-T4 trigger forms are declared as bounded `patterns` on the
//! policy block and served by the shared regex engine with span mapping and
//! trigger fidelity inherited; this module implements only the tail form,
//! which is where the imperative-opener and second-person suppression logic
//! lives. E001 precedent: bounded hand-rolled scans over policy params, no
//! regex, no new dependency.
//!
//! The scan runs over the norm view (NFC, entity decode, escape resolution,
//! invisible removal, soft-break folding; prose-only with U+FFFD barriers at
//! code spans, so flanking text never fuses across a code region). Every
//! window is bounded by policy params, honoring the crate-wide ban on
//! unbounded scans. FP-safety is the design bias: every suppression doubt
//! resolves toward silence, and the one deliberate inversion — a clause
//! whose start lies beyond the walk-back window fires by default — is the
//! spec's fail-toward-candidate-report choice.
//!
//! SLOP-C010 proleptic capability denial — a denial of a capability nobody
//! claimed, and the evidential hedge that rides with it. The scan reads
//! clauses rather than spans, because the shape is a stack: a restatement of
//! what the thing does, a denial of something it was never accused of, and a
//! hedge over the denial. Its two arms are the density arm (two qualifying
//! clauses in one block) and the adjacency arm (one qualifying clause beside
//! an affirmative sentence about the same subject). Both run over the same
//! norm view and the same bounded scans as C007.

use crate::engine::{CompiledPolicy, Hit};
use crate::input::Prepared;
use crate::views::NormView;
use crate::Config;

pub const HANDLED: &[&str] = &["SLOP-C007", "SLOP-C010"];

pub(crate) fn str_list(rule: &crate::policy::Rule, key: &str) -> Vec<String> {
    rule.params
        .as_table()
        .and_then(|t| t.get(key))
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_ascii_lowercase()))
                .collect()
        })
        .unwrap_or_default()
}

/// First word token of a clause: leading non-word characters (quotes,
/// brackets, barrier replacement chars) are skipped, then the maximal run of
/// alphanumerics plus apostrophes is collected, ASCII-lowercased, with the
/// typographic apostrophe folded so a norm-view `don\u{2019}t` still matches
/// the base-form deny-list entry `don't`.
fn first_token(clause: &str) -> String {
    let mut out = String::new();
    for c in clause.chars() {
        let c = if c == '\u{2019}' { '\'' } else { c };
        if c.is_alphanumeric() || c == '\'' {
            out.push(c.to_ascii_lowercase());
        } else if out.is_empty() {
            continue;
        } else {
            break;
        }
    }
    out
}

/// Word token beginning exactly at `at` (used for the interior-directive
/// check, where the position after `, ` or `then ` is already known).
fn token_at(clause_lower: &str, at: usize) -> String {
    first_token(&clause_lower[at..])
}

/// Word-bounded, case-insensitive containment of `needle` (already
/// lowercase) in `hay_lower` (already lowercase).
fn contains_word(hay_lower: &str, needle: &str) -> bool {
    let mut at = 0usize;
    while let Some(pos) = hay_lower[at..].find(needle) {
        let s = at + pos;
        let e = s + needle.len();
        let before_ok = hay_lower[..s]
            .chars()
            .next_back()
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true);
        let after_ok = hay_lower[e..]
            .chars()
            .next()
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true);
        if before_ok && after_ok {
            return true;
        }
        at = s + 1;
    }
    false
}

/// Bounded terminal test for a `.` met during the NP scan or the clause
/// walk-back. `dot_end` is the offset just past the `.` in `text`. A period
/// followed directly by an alphanumeric character is abbreviation- or
/// number-internal (`U.S`, `3.5`): not a terminal. A period followed by a
/// bounded ASCII space/tab run and then a lowercase continuation is
/// mid-sentence punctuation (`U.S. but`, `e.g. the`): not a terminal.
/// Everything else — end of text, a line break, an uppercase/digit/quote/
/// bracket/barrier follower, a whitespace run past the parser's 8-unit
/// bound — is a terminal, exactly as before this test existed. The peek is
/// O(1) and bounded, honoring the crate-wide ban on unbounded scans.
/// Accepted false negatives (KNOWN-EDGES): chat-style prose that starts
/// sentences lowercase reads a real terminal as a continuation and stays
/// silent, and an abbreviation followed by a capitalized word (`Mr. Smith`)
/// still reads as a terminal — both resolve toward silence or the
/// pre-existing behavior, never toward a new firing surface.
pub(crate) fn period_is_terminal(text: &str, dot_end: usize) -> bool {
    let mut chars = text[dot_end..].chars();
    let Some(first) = chars.next() else {
        return true; // end of text
    };
    if first.is_alphanumeric() {
        return false; // abbreviation- or number-internal
    }
    if first != ' ' && first != '\t' {
        // Line breaks end the block; quotes, brackets, punctuation, and the
        // U+FFFD barrier all sit on the terminal side.
        return true;
    }
    // Walk at most 8 ASCII space/tab units, mirroring the tail parser's own
    // whitespace bound.
    let mut seen = 1usize;
    loop {
        match chars.next() {
            Some(' ') | Some('\t') => {
                seen += 1;
                if seen > 8 {
                    return true;
                }
            }
            Some('\n') | Some('\r') => return true, // block end
            Some(c) => return !c.is_lowercase(),
            None => return true,
        }
    }
}

/// Parse the T1 tail shape starting at the comma at `comma`: up to 8
/// whitespace characters, `not` or `never` (case-insensitive, followed by
/// 1..=8 whitespace), then an NP of 1..=`np_max` bytes containing none of
/// `!?;:,\n` (nor a U+FFFD barrier) and at least one non-whitespace
/// character (a whitespace-only "NP" is not a noun phrase), closed by a
/// terminal `.`, `!`, or `?`. A non-terminal `.` (abbreviation-internal or
/// mid-sentence per `period_is_terminal`) is legal NP content. Returns the
/// exclusive end offset of the terminal punctuation. The
/// no-interior-comma constraint is what keeps the parenthetical
/// `X, not Y, verb ...` interpolation out of scope, and a word-bounded
/// `but` anywhere in the NP rejects the tail outright: a contrastive
/// continuation (`, not in the U.S. but in Asia.`) is the not-X-but-Y pair
/// form — SLOP-C008's territory and a legitimate contrast — never a bare
/// apophatic caveat.
/// Both whitespace loops match ASCII whitespace only (space/tab/LF/CR),
/// by design: a non-ASCII space inside a contrastive tail is an
/// attacker-unrealistic vector (see KNOWN-EDGES).
fn parse_tail(text: &str, comma: usize, np_max: usize) -> Option<usize> {
    let rest = text.get(comma + 1..)?;
    let mut i = 0usize;
    for c in rest.chars().take(8) {
        if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
            i += c.len_utf8();
        } else {
            break;
        }
    }
    let after_ws = &rest[i..];
    // `get` rather than direct slicing: the byte at the cut can sit inside a
    // multi-byte character, and a directly sliced prefix would panic there.
    let kw_len = if after_ws
        .get(..5)
        .is_some_and(|s| s.eq_ignore_ascii_case("never"))
    {
        5
    } else if after_ws
        .get(..3)
        .is_some_and(|s| s.eq_ignore_ascii_case("not"))
    {
        3
    } else {
        return None;
    };
    // The keyword must be followed by 1..=8 ASCII whitespace characters
    // (its right word boundary). ASCII-only by design, deliberately
    // narrower than a Unicode `\s{1,8}`: a non-ASCII space here is an
    // accepted false negative (see KNOWN-EDGES).
    let mut j = i + kw_len;
    let mut ws = 0usize;
    for c in rest[j..].chars().take(8) {
        if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
            ws += 1;
            j += c.len_utf8();
        } else {
            break;
        }
    }
    if ws == 0 {
        return None;
    }
    // NP scan: bounded, no clause punctuation, must close with a terminal,
    // and must carry at least one non-whitespace character — an empty or
    // whitespace-only span between the keyword and the terminal is not a
    // noun phrase.
    let np_start = j;
    let mut k = j;
    let mut np_has_content = false;
    for c in rest[np_start..].chars() {
        match c {
            '.' if !period_is_terminal(text, comma + 1 + k + 1) => {
                // Abbreviation-internal or mid-sentence period (`U.S.`,
                // `e.g.`): NP content, not a terminal.
                np_has_content = true;
                k += 1;
                if k - np_start > np_max {
                    return None;
                }
            }
            '.' | '!' | '?' => {
                if !np_has_content {
                    return None; // empty or whitespace-only NP
                }
                // A word-bounded `but` inside the tail means the negation
                // carries its own contrastive continuation ("not in the
                // U.S. but in Asia"): a not-X-but-Y pair, which is a
                // legitimate contrast shape and SLOP-C008's territory, not
                // a bare apophatic caveat. The comma-tail rule stays
                // silent. Bounded: the NP is at most `np_max` bytes.
                let np_lower = rest[np_start..k].to_ascii_lowercase();
                if contains_word(&np_lower, "but") {
                    return None;
                }
                return Some(comma + 1 + k + c.len_utf8());
            }
            ';' | ':' | ',' | '\n' | '\u{FFFD}' => return None,
            _ => {
                if !c.is_whitespace() {
                    np_has_content = true;
                }
                k += c.len_utf8();
                if k - np_start > np_max {
                    return None;
                }
            }
        }
    }
    None
}

/// Recover the clause start: walk back from the comma at most `window`
/// bytes to the nearest clause boundary — a line break, or terminal
/// punctuation (`.`, `!`, `?`, plus `:` per the design) followed by
/// whitespace — mirroring the engine's own block-start notion
/// (`NormView::is_block_start`) as a single bounded backward pass. A `.`
/// additionally goes through `period_is_terminal`, so an abbreviation
/// (`the U.S. market`) no longer truncates the recovered clause — the
/// suppression classifier sees the whole sentence, an FP-reducing change.
/// The `:` `!` `?` arms are untouched: a colon followed by lowercase is a
/// legitimate clause boundary and must stay one. Offset 0
/// counts as a boundary when it lies inside the window. `None` means the
/// window was exhausted without a boundary; the caller fires by default.
fn clause_start(text: &str, comma: usize, window: usize) -> Option<usize> {
    let lo = crate::widen_to_char_boundaries(text, comma.saturating_sub(window)..comma).start;
    let region = &text[lo..comma];
    for (off, c) in region.char_indices().rev() {
        let abs = lo + off;
        let boundary_end = match c {
            '\n' => Some(abs + 1),
            '.' | '!' | '?' | ':' => {
                let next = text[abs + c.len_utf8()..].chars().next();
                if matches!(next, Some(w) if w.is_whitespace())
                    && (c != '.' || period_is_terminal(text, abs + 1))
                {
                    Some(abs + c.len_utf8())
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(mut p) = boundary_end {
            // The clause proper starts after the whitespace run.
            for w in text[p..comma].chars() {
                if w.is_whitespace() {
                    p += w.len_utf8();
                } else {
                    break;
                }
            }
            return Some(p);
        }
    }
    if lo == 0 {
        return Some(0);
    }
    None
}

/// The section-3 suppression classifier over a recovered clause. True means
/// the site reads as a directive and stays silent.
fn suppressed(clause: &str, openers: &[String], second_person: &[String]) -> bool {
    let lower = clause.to_lowercase();
    // 1. Imperative opener: the clause's first token is on the base-form
    //    deny-list.
    let head = first_token(&lower);
    if !head.is_empty() && openers.contains(&head) {
        return true;
    }
    // 2. Second-person cue anywhere before the comma, word-bounded.
    if second_person.iter().any(|t| contains_word(&lower, t)) {
        return true;
    }
    // 3. A deny-list verb immediately after an interior `, ` or after
    //    `then ` — the leading-adverbial directive
    //    ("When in doubt, use the builder, not the raw constructor.").
    let mut at = 0usize;
    while let Some(pos) = lower[at..].find(", ") {
        let s = at + pos + 2;
        let tok = token_at(&lower, s);
        if !tok.is_empty() && openers.contains(&tok) {
            return true;
        }
        at = s;
    }
    let mut at = 0usize;
    while let Some(pos) = lower[at..].find("then ") {
        let s = at + pos;
        let before_ok = lower[..s]
            .chars()
            .next_back()
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true);
        if before_ok {
            let tok = token_at(&lower, s + 5);
            if !tok.is_empty() && openers.contains(&tok) {
                return true;
            }
        }
        at = s + 5;
    }
    false
}

pub fn evaluate(
    cp: &CompiledPolicy,
    prepared: &Prepared,
    norm: &NormView,
    config: &Config,
    hits: &mut Vec<Hit>,
) {
    evaluate_c007(cp, prepared, norm, config, hits);
    evaluate_c010(cp, prepared, norm, config, hits);
}

fn evaluate_c007(
    cp: &CompiledPolicy,
    prepared: &Prepared,
    norm: &NormView,
    config: &Config,
    hits: &mut Vec<Hit>,
) {
    let Some(idx) = super::active(cp, config, "SLOP-C007") else {
        return;
    };
    let rule = &cp.pkg.rules[idx];
    let np_max = super::param_i64(rule, "tail_np_max_bytes").unwrap_or(60) as usize;
    let window = super::param_i64(rule, "clause_window_bytes").unwrap_or(240) as usize;
    let openers = str_list(rule, "imperative_openers");
    let second_person = str_list(rule, "second_person");

    let text = norm.text.as_str();
    let src = prepared.text.as_str();
    for (comma, _) in text.char_indices().filter(|(_, c)| *c == ',') {
        let Some(tail_end) = parse_tail(text, comma, np_max) else {
            continue;
        };
        // Clause recovery within the bounded window. A recovered clause goes
        // through the suppression classifier; an exhausted window fires by
        // default (spec section 3: fail toward the candidate report).
        if let Some(cs) = clause_start(text, comma, window) {
            if suppressed(&text[cs..comma], &openers, &second_person) {
                continue;
            }
        }
        let span = comma..tail_end;
        // Map exactly as accept_word_hit does: through the segment table,
        // widened against the source. Trigger fidelity re-verifies the
        // reported slice at emit, so a mapping bug fails closed as exit 30
        // instead of surfacing a finding at the wrong bytes.
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
        hits.push(hit);
    }
}

// --- SLOP-C010 proleptic capability denial -----------------------------------

/// One word token of the norm view, with its absolute byte range. Hyphens
/// stay inside the token so a product name (`ai-slop`) and a hyphen-spelled
/// noun read as one word; the typographic apostrophe folds to the ASCII one
/// so `doesn\u{2019}t` matches the base-form entry `doesn't`.
pub(crate) struct Tok {
    pub start: usize,
    pub end: usize,
    pub word: String,
}

pub(crate) fn tokenize(text: &str, range: std::ops::Range<usize>) -> Vec<Tok> {
    let mut out: Vec<Tok> = Vec::new();
    let mut cur = String::new();
    let mut cur_start = range.start;
    for (off, raw) in text[range.clone()].char_indices() {
        let abs = range.start + off;
        let c = if raw == '\u{2019}' { '\'' } else { raw };
        if c.is_alphanumeric() || c == '\'' || c == '-' {
            if cur.is_empty() {
                cur_start = abs;
            }
            cur.push(c.to_ascii_lowercase());
        } else if !cur.is_empty() {
            out.push(Tok {
                start: cur_start,
                end: abs,
                word: std::mem::take(&mut cur),
            });
        }
    }
    if !cur.is_empty() {
        out.push(Tok {
            start: cur_start,
            end: range.end,
            word: cur,
        });
    }
    out
}

/// Does the whitespace-separated `phrase` sit at token `i`? A `*` element
/// matches any single token, which is what spells the `no <word> is evidence`
/// hedge as policy data instead of code.
pub(crate) fn phrase_at(toks: &[Tok], i: usize, phrase: &str) -> bool {
    phrase_match(toks, i, phrase).is_some()
}

/// What a phrase match consumed: how many tokens, and the index of the head
/// noun the wildcard ended on, when the phrase carries one. The head noun is
/// the last wildcard token before the copula, which is the subject an open
/// hedge is about.
pub(crate) struct PhraseMatch {
    pub len: usize,
    pub head: Option<usize>,
}

/// Match `phrase` at token `i`. A `*` element stands for one or two tokens,
/// tried longest first, so `no single finding is evidence` matches the same
/// entry as `no finding is evidence` and the two-token spelling cannot slip
/// past the list. Three tokens is a recorded miss, not an oversight: the cap
/// keeps the scan bounded and keeps a long noun phrase from swallowing the
/// copula.
pub(crate) fn phrase_match(toks: &[Tok], i: usize, phrase: &str) -> Option<PhraseMatch> {
    const WILDCARD_MAX: usize = 2;
    let parts: Vec<&str> = phrase.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    fn walk(
        toks: &[Tok],
        k: usize,
        parts: &[&str],
        head: Option<usize>,
    ) -> Option<(usize, Option<usize>)> {
        let Some((part, rest)) = parts.split_first() else {
            return Some((k, head));
        };
        if *part == "*" {
            for take in (1..=WILDCARD_MAX).rev() {
                if toks.len() < k + take {
                    continue;
                }
                if let Some(done) = walk(toks, k + take, rest, Some(k + take - 1)) {
                    return Some(done);
                }
            }
            return None;
        }
        match toks.get(k) {
            Some(t) if &t.word == part => walk(toks, k + 1, rest, head),
            _ => None,
        }
    }
    let (end, head) = walk(toks, i, &parts, None)?;
    if end > i {
        Some(PhraseMatch { len: end - i, head })
    } else {
        None
    }
}

/// The longest phrase from `phrases` sitting at token `i`, if any. Longest
/// wins so `makes no claims` beats the `makes no claim` prefix.
pub(crate) fn phrase_at_any<'a>(
    toks: &[Tok],
    i: usize,
    phrases: &'a [String],
) -> Option<(&'a str, PhraseMatch)> {
    phrases
        .iter()
        .filter_map(|p| phrase_match(toks, i, p).map(|m| (p.as_str(), m)))
        .max_by_key(|(_, m)| m.len)
}

/// First token index in `from..to` where any of `phrases` begins.
pub(crate) fn phrase_in(toks: &[Tok], from: usize, to: usize, phrases: &[String]) -> Option<usize> {
    (from..to.min(toks.len())).find(|&i| phrases.iter().any(|p| phrase_at(toks, i, p)))
}

/// The closed subject set, read at token `i`. Returns the token count the
/// subject spans and its comparison key. The three bare demonstratives share
/// one key: they name the same referent across a two-sentence stack, and
/// telling `it` from `this` there would make the adjacency arm brittle. A
/// tool noun matches in its singular and plural spelling alike.
/// A closed subject read at token `i`: the token count it spans, its
/// comparison key, and whether the subject carries its own negation.
///
/// Positive subjects are `it`, `this`, `the` plus a tool noun, and the
/// product names. Negative subjects are `no` plus a tool noun, `nothing`, and
/// `none of the` plus a tool noun. `that` is not in the set: as a relativizer
/// it collides with ordinary prose, and `they` was never in it.
pub(crate) struct Subject {
    pub span: usize,
    pub key: String,
    pub negative: bool,
}

/// The lemma of a tool noun: the plural and the singular share one key, so
/// `the rules` and `the rule` corefer while `the rule` and `the tool` do not.
fn lemma(noun: &str, tool_nouns: &[String]) -> String {
    match noun.strip_suffix('s') {
        Some(base) if tool_nouns.iter().any(|n| n == base) => base.to_string(),
        _ => noun.to_string(),
    }
}

pub(crate) fn subject_at(toks: &[Tok], i: usize, tool_nouns: &[String]) -> Option<Subject> {
    let w = &toks.get(i)?.word;
    let sub = |span, key: String, negative| {
        Some(Subject {
            span,
            key,
            negative,
        })
    };
    if w == "it" || w == "this" {
        return sub(1, "pronoun".to_string(), false);
    }
    if w == "nothing" {
        return sub(1, "pronoun".to_string(), true);
    }
    if w == "none"
        && toks.get(i + 1).map(|t| t.word.as_str()) == Some("of")
        && toks.get(i + 2).map(|t| t.word.as_str()) == Some("the")
    {
        let noun = &toks.get(i + 3)?.word;
        if tool_nouns.contains(noun) {
            return sub(4, format!("noun:{}", lemma(noun, tool_nouns)), true);
        }
        return None;
    }
    if w == "the" || w == "no" {
        let noun = &toks.get(i + 1)?.word;
        if tool_nouns.contains(noun) {
            return sub(2, format!("noun:{}", lemma(noun, tool_nouns)), w == "no");
        }
    }
    None
}

/// Two subjects name the same thing when either side is a bare closed-set
/// pronoun, or when both carry the same tool-noun lemma, where a noun and its
/// plural are one lemma. Two different nouns are two different things: `the
/// test` and `the tool` name separate objects as often as they name one.
fn same_referent(a: &str, b: &str) -> bool {
    a == b || a == "pronoun" || b == "pronoun"
}

/// What one segment contributes to the block.
///
/// Two ranges, because the two families edit differently. A denial with a
/// subject is rewritten as a segment, so spellings A, B, and C report the
/// coordinator-cut segment: that is the unit the writer changes. An
/// evidential hedge governs an open complement, so family 2 reports the
/// enclosing comma-delimited clause, which is where the phrase ends.
struct ClauseRead {
    segment: std::ops::Range<usize>,
    clause: std::ops::Range<usize>,
    qualifies: bool,
    /// True when family 2 is what qualified, which selects the wider span.
    open_complement: bool,
    subject: Option<String>,
    /// Set when this segment is itself an affirmative description of the
    /// artifact, which makes it an Arm B partner for its own sentence.
    affirmative: Option<String>,
}

impl ClauseRead {
    fn report_range(&self) -> std::ops::Range<usize> {
        if self.open_complement {
            self.clause.clone()
        } else {
            self.segment.clone()
        }
    }
}

struct C010Sets {
    tool_nouns: Vec<String>,
    products: Vec<String>,
    /// `do not`, `don't`, `never`: these can head a command.
    imperative_negations: Vec<String>,
    /// Finite forms, which require a subject and so never head a command.
    finite_negations: Vec<String>,
    /// Both sets together, for the window search and the affirmative test.
    negations: Vec<String>,
    capability_verbs: Vec<String>,
    hedges: Vec<String>,
    negation_window: usize,
    verb_window: usize,
    min_clauses: usize,
}

impl C010Sets {
    /// The product names read as one-token subjects beside the closed set.
    fn subject(&self, toks: &[Tok], i: usize) -> Option<Subject> {
        if let Some(w) = toks.get(i) {
            if self.products.contains(&w.word) {
                return Some(Subject {
                    span: 1,
                    key: format!("noun:{}", w.word),
                    negative: false,
                });
            }
        }
        subject_at(toks, i, &self.tool_nouns)
    }

    /// The subject key a bare noun carries: a tool noun by its lemma, or a
    /// product name. Anything else is a foreign subject and has no key, which
    /// is what keeps `no banana is evidence` out of the adjacency arm.
    fn subject_from_noun(&self, toks: &[Tok], i: usize) -> Option<String> {
        let w = &toks.get(i)?.word;
        if self.products.contains(w) {
            return Some(format!("noun:{w}"));
        }
        if self.tool_nouns.contains(w) {
            return Some(format!("noun:{}", lemma(w, &self.tool_nouns)));
        }
        None
    }

    /// End index of a negation phrase from `set` sitting at `i`, taking the
    /// longest match so `does not` beats a shorter overlap.
    fn negation_end(&self, toks: &[Tok], i: usize, set: &[String]) -> Option<usize> {
        set.iter()
            .filter(|n| phrase_at(toks, i, n))
            .map(|n| i + n.split_whitespace().count())
            .max()
    }
}

/// The three verb forms the rule tells apart, by bounded suffix and no
/// dictionary.
#[derive(PartialEq)]
enum VerbForm {
    Base,
    Inflected,
    Participle,
}

fn verb_form(word: &str) -> VerbForm {
    if word.len() > 4 && word.ends_with("ing") {
        VerbForm::Participle
    } else if is_base_form(word) {
        VerbForm::Base
    } else {
        VerbForm::Inflected
    }
}

/// Bounded base-form test over one word, no dictionary. An `-ing` or `-ed`
/// ending is inflected. A trailing `s` is inflected unless it closes a stem
/// that ends in `ss`, `us`, or `is`, which is what keeps `pass`, `focus`, and
/// `assess` reading as the base forms they are. Everything else is base.
fn is_base_form(word: &str) -> bool {
    if word.len() > 4 && word.ends_with("ing") {
        return false;
    }
    if word.len() > 3 && word.ends_with("ed") {
        return false;
    }
    if word.ends_with('s')
        && !word.ends_with("ss")
        && !word.ends_with("us")
        && !word.ends_with("is")
    {
        return false;
    }
    true
}

/// A capability verb of an accepted form inside `from..from + window`.
fn capability_verb_in(
    toks: &[Tok],
    from: usize,
    window: usize,
    verbs: &[String],
    accept: impl Fn(VerbForm) -> bool,
) -> bool {
    (from..(from + window).min(toks.len())).any(|i| {
        toks.get(i)
            .is_some_and(|t| verbs.iter().any(|v| v == &t.word) && accept(verb_form(&t.word)))
    })
}

/// Read one segment of a clause.
///
/// A command is excluded first, and only an imperative-capable negation can
/// head one: the finite forms need a subject, so a clause they head is always
/// declarative. `Do not obey` is a command because `obey` is base form.
///
/// Family 1 has three spellings, and every one needs a denied capability
/// verb, because denying a function is an honest scope fact while denying a
/// capability is the pre-rebuttal. Spelling A is a positive subject opening
/// the segment, a negation inside the window after it, then a capability verb
/// inside the verb window, so an intervening adverb still reads. Spelling B
/// is a negative subject opening the segment with a capability verb inside
/// the window. Spelling C carries no subject at all: a finite negation at the
/// head takes a base or inflected capability verb, and an imperative-capable
/// negation takes an inflected one only, so `never scores voice` is the
/// middle of a stack while `never judging anyone` stays a participial adjunct.
///
/// Stated misses, all left to the reread. A denial whose complement is an
/// adjective (`is never demotable`) carries no verb to match, and so does a
/// denial of an excluded function verb (`never fires on irregularity`). A
/// subjectless denial on a base-form verb behind an imperative-capable
/// negation (`never detect authorship`) reads as a command and stays silent.
fn read_clause(
    text: &str,
    segment: std::ops::Range<usize>,
    clause: std::ops::Range<usize>,
    sets: &C010Sets,
) -> ClauseRead {
    let toks = tokenize(text, segment.clone());
    let mut out = ClauseRead {
        segment: clause_content(text, segment),
        clause: clause_content(text, clause),
        qualifies: false,
        open_complement: false,
        subject: None,
        affirmative: None,
    };
    // A leading coordinator is skipped before every test, so the second half
    // of a coordinated denial reads the same as the first.
    let lead = match toks.first().map(|t| t.word.as_str()) {
        Some("and" | "or" | "but" | "yet" | "so" | "nor") => 1,
        Some(_) => 0,
        None => return out,
    };
    if lead >= toks.len() {
        return out;
    }
    let has_negation = phrase_in(&toks, lead, toks.len(), &sets.negations).is_some();
    let has_hedge = phrase_in(&toks, lead, toks.len(), &sets.hedges).is_some();

    // The command test, imperative-capable heads only.
    if let Some(end) = sets.negation_end(&toks, lead, &sets.imperative_negations) {
        if toks.get(end).is_some_and(|t| is_base_form(&t.word)) {
            return out;
        }
    }

    if let Some(subject) = sets.subject(&toks, lead) {
        out.subject = Some(subject.key.clone());
        if !subject.negative && !has_negation && !has_hedge {
            out.affirmative = Some(subject.key.clone());
        }
        let after_subject = lead + subject.span;
        // Spelling B: the negative subject is its own negation. Spelling A:
        // an explicit negation has to sit inside the window.
        let (verb_from, window) = if subject.negative {
            (Some(after_subject), sets.negation_window)
        } else {
            (
                phrase_in(
                    &toks,
                    after_subject,
                    after_subject + sets.negation_window,
                    &sets.negations,
                )
                .and_then(|at| sets.negation_end(&toks, at, &sets.negations)),
                sets.verb_window,
            )
        };
        if let Some(from) = verb_from {
            // Spellings A and B take every form of the verb, the participle
            // included: `is not accurately judging writers` denies the same
            // capability as `does not judge writers`.
            if capability_verb_in(&toks, from, window, &sets.capability_verbs, |_| true) {
                out.qualifies = true;
                return out;
            }
        }
    } else if let Some(end) = sets.negation_end(&toks, lead, &sets.finite_negations) {
        // Spelling C, finite head: base or inflected verb.
        if capability_verb_in(&toks, end, sets.verb_window, &sets.capability_verbs, |f| {
            f != VerbForm::Participle
        }) {
            out.qualifies = true;
            out.subject = Some("pronoun".to_string());
            return out;
        }
    } else if let Some(end) = sets.negation_end(&toks, lead, &sets.imperative_negations) {
        // Spelling C, imperative-capable head: inflected verb only. A base
        // form here is a command and never reached this far.
        if capability_verb_in(&toks, end, sets.verb_window, &sets.capability_verbs, |f| {
            f == VerbForm::Inflected
        }) {
            out.qualifies = true;
            out.subject = Some("pronoun".to_string());
            return out;
        }
    }
    // Family 2: the evidential hedge, which carries its own negation. An open
    // hedge is about its head noun, the last wildcard token before the
    // copula, so that is the subject coreference tests. A closed hedge takes
    // the clause's own closed subject.
    if let Some(at) = phrase_in(&toks, lead, toks.len(), &sets.hedges) {
        out.qualifies = true;
        out.open_complement = true;
        if out.subject.is_none() {
            let head = phrase_at_any(&toks, at, &sets.hedges).and_then(|(_, m)| m.head);
            out.subject = match head {
                Some(h) => sets.subject_from_noun(&toks, h),
                None => (lead..toks.len())
                    .find_map(|i| sets.subject(&toks, i))
                    .map(|s| s.key),
            };
        }
    }
    out
}

/// The subject of an affirmative sentence about the artifact: a positive
/// closed subject opens the sentence and no negation or hedge follows it.
fn affirmative_subject(
    text: &str,
    sentence: std::ops::Range<usize>,
    sets: &C010Sets,
) -> Option<String> {
    let toks = tokenize(text, sentence);
    let subject = sets.subject(&toks, 0)?;
    if subject.negative {
        return None;
    }
    if phrase_in(&toks, 0, toks.len(), &sets.negations).is_some() {
        return None;
    }
    if phrase_in(&toks, 0, toks.len(), &sets.hedges).is_some() {
        return None;
    }
    Some(subject.key)
}

/// Trim a range to the text it actually covers, so a reported span never
/// opens or closes on whitespace.
fn trim_range(text: &str, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let slice = &text[range.clone()];
    let start = range.start + (slice.len() - slice.trim_start().len());
    let end = range.start + slice.trim_end().len();
    start..end.max(start)
}

/// Split the norm text into blocks on line breaks. A line break ends a block
/// for every scan in this module, matching C007's own walk-back boundary.
pub(crate) fn block_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (off, c) in text.char_indices() {
        if c == '\n' {
            out.push(start..off);
            start = off + 1;
        }
    }
    out.push(start..text.len());
    out
}

/// True when the text after `at`, past a bounded run of spaces, opens on one
/// of `words`.
fn opens_with_word(text: &str, at: usize, words: &[String]) -> bool {
    if words.is_empty() || at >= text.len() {
        return false;
    }
    let rest = &text[at..];
    let trimmed = rest.trim_start_matches([' ', '\t']);
    if rest.len() - trimmed.len() > 8 {
        return false;
    }
    // The 64-byte peek is a bound, not an index the caller chose, so it is
    // widened to a char boundary before it slices.
    let end = crate::widen_to_char_boundaries(trimmed, 0..trimmed.len().min(64)).end;
    let toks = tokenize(trimmed, 0..end);
    toks.first()
        .is_some_and(|t| t.start == 0 && words.iter().any(|w| w == &t.word))
}

/// Split a block into sentences on terminal punctuation, reusing C007's
/// terminal-period test so an abbreviation never ends a sentence.
pub(crate) fn sentence_ranges(
    text: &str,
    block: std::ops::Range<usize>,
    lowercase_openers: &[String],
) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut start = block.start;
    for (off, c) in text[block.clone()].char_indices() {
        let abs = block.start + off;
        let terminal = match c {
            // The shared terminal test reads a lowercase follower as a
            // continuation, which is right for prose and wrong for a product
            // name spelled lowercase. A name from the closed set opens a
            // sentence whatever its case.
            '.' => {
                period_is_terminal(text, abs + 1)
                    || opens_with_word(text, abs + 1, lowercase_openers)
            }
            '!' | '?' => true,
            _ => false,
        };
        if terminal {
            let r = trim_range(text, start..abs + c.len_utf8());
            if r.start < r.end {
                out.push(r);
            }
            start = abs + c.len_utf8();
        }
    }
    let r = trim_range(text, start..block.end);
    if r.start < r.end {
        out.push(r);
    }
    out
}

/// Split a sentence into comma-delimited clauses. A semicolon splits too:
/// it separates independent clauses exactly as the comma does here.
fn clause_ranges(text: &str, sentence: std::ops::Range<usize>) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut start = sentence.start;
    for (off, c) in text[sentence.clone()].char_indices() {
        if c == ',' || c == ';' {
            let abs = sentence.start + off;
            let r = trim_range(text, start..abs);
            if r.start < r.end {
                out.push(r);
            }
            start = abs + c.len_utf8();
        }
    }
    let r = trim_range(text, start..sentence.end);
    if r.start < r.end {
        out.push(r);
    }
    out
}

/// A reported span carries clause content and no delimiter at either end. It
/// opens on the first non-whitespace byte after the coordinator that joined
/// this clause to what came before, and it closes on the last non-whitespace
/// byte of the content, so a mid-sentence comma or semicolon and a terminal
/// stop both stay outside the span a reader is asked to rewrite.
fn clause_content(text: &str, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    const COORDINATORS: &[&str] = &["and", "or", "but", "yet", "so", "nor"];
    let toks = tokenize(text, range.clone());
    let start = match (toks.first(), toks.get(1)) {
        (Some(head), Some(next)) if COORDINATORS.contains(&head.word.as_str()) => next.start,
        _ => range.start,
    };
    let mut end = range.end;
    while end > start {
        let Some(c) = text[start..end].chars().next_back() else {
            break;
        };
        if c.is_whitespace() || matches!(c, ',' | ';' | '.' | '!' | '?') {
            end -= c.len_utf8();
        } else {
            break;
        }
    }
    start..end.max(start)
}

/// Split one clause at its interior coordinators. The coordinator opens the
/// segment that follows it, where the leading-coordinator skip reads it, so a
/// coordinated denial presents each half to the family tests on its own. This
/// is the same single segmentation pass the exclusion and the partner search
/// use; there is no second splitter.
fn segment_ranges(text: &str, clause: std::ops::Range<usize>) -> Vec<std::ops::Range<usize>> {
    const COORDINATORS: &[&str] = &["and", "or", "but", "yet", "so", "nor"];
    let toks = tokenize(text, clause.clone());
    let mut out = Vec::new();
    let mut start = clause.start;
    for (i, t) in toks.iter().enumerate() {
        if i > 0 && COORDINATORS.contains(&t.word.as_str()) && t.start > start {
            let r = trim_range(text, start..t.start);
            if r.start < r.end {
                out.push(r);
            }
            start = t.start;
        }
    }
    let r = trim_range(text, start..clause.end);
    if r.start < r.end {
        out.push(r);
    }
    out
}

fn evaluate_c010(
    cp: &CompiledPolicy,
    prepared: &Prepared,
    norm: &NormView,
    config: &Config,
    hits: &mut Vec<Hit>,
) {
    let Some(idx) = super::active(cp, config, "SLOP-C010") else {
        return;
    };
    let rule = &cp.pkg.rules[idx];
    let sets = C010Sets {
        tool_nouns: str_list(rule, "tool_nouns"),
        products: str_list(rule, "product_names"),
        imperative_negations: str_list(rule, "imperative_negations"),
        finite_negations: str_list(rule, "finite_negations"),
        negations: {
            let mut all = str_list(rule, "imperative_negations");
            all.extend(str_list(rule, "finite_negations"));
            all
        },
        capability_verbs: str_list(rule, "capability_verbs"),
        hedges: str_list(rule, "hedge_markers"),
        negation_window: super::param_i64(rule, "negation_window_tokens").unwrap_or(4) as usize,
        verb_window: super::param_i64(rule, "verb_window_tokens").unwrap_or(3) as usize,
        min_clauses: super::param_i64(rule, "min_clauses").unwrap_or(2) as usize,
    };

    let text = norm.text.as_str();
    let src = prepared.text.as_str();
    for block in block_ranges(text) {
        if block.start >= block.end {
            continue;
        }
        let sentences = sentence_ranges(text, block, &sets.products);
        let mut reads: Vec<Vec<ClauseRead>> = Vec::with_capacity(sentences.len());
        for s in &sentences {
            let mut segs = Vec::new();
            for clause in clause_ranges(text, s.clone()) {
                for segment in segment_ranges(text, clause.clone()) {
                    segs.push(read_clause(text, segment, clause.clone(), &sets));
                }
            }
            reads.push(segs);
        }
        let qualifying: Vec<(usize, &ClauseRead)> = reads
            .iter()
            .enumerate()
            .flat_map(|(si, cs)| cs.iter().filter(|c| c.qualifies).map(move |c| (si, c)))
            .collect();

        // One finding per qualifying clause, so each denial stays separately
        // answerable and separately waivable.
        let mut reported: Vec<(std::ops::Range<usize>, String)> = Vec::new();
        if qualifying.len() >= sets.min_clauses {
            for (_, clause) in &qualifying {
                reported.push((
                    clause.report_range(),
                    format!(
                        "arm A, one of {} denied capabilities in this block",
                        qualifying.len()
                    ),
                ));
            }
        } else if qualifying.len() == 1 {
            let (si, clause) = qualifying[0];
            let Some(subject) = clause.subject.clone() else {
                continue;
            };
            // The partner search stops at the first match, in the ruled
            // order: the other segments of this sentence, then the sentence
            // before, then the sentence after. Within one sentence there is
            // no distance limit; across sentences the search stays strictly
            // adjacent. One finding per qualifying clause, however many
            // partners match.
            let within = reads[si].iter().any(|other| {
                !std::ptr::eq(other, clause)
                    && other
                        .affirmative
                        .as_deref()
                        .is_some_and(|k| same_referent(k, &subject))
            });
            let mut partner =
                within.then_some("arm B, beside an affirmative clause of its own sentence");
            if partner.is_none() {
                for n in [si.checked_sub(1), si.checked_add(1)].into_iter().flatten() {
                    let Some(s) = sentences.get(n) else {
                        continue;
                    };
                    if affirmative_subject(text, s.clone(), &sets)
                        .is_some_and(|k| same_referent(&k, &subject))
                    {
                        partner =
                            Some("arm B, beside an affirmative description of the same subject");
                        break;
                    }
                }
            }
            let Some(detail) = partner else {
                continue;
            };
            reported.push((clause.report_range(), detail.to_string()));
        }

        for (span, detail) in reported {
            // Map exactly as the C007 tail does: through the segment table,
            // widened against the source, with trigger fidelity re-verifying
            // the reported slice at emit.
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
            hit.detail = Some(detail);
            hits.push(hit);
        }
    }
}
