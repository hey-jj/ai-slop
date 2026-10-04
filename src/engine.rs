//! Compiled matching engines: two global Aho-Corasick automatons split by
//! case mode, and one multi-pattern DFA served through an overlapping
//! forward-plus-reverse adapter. All engines compile once per process.

use crate::extract::{Doc, F_BODY, F_SUBJECT, F_TRAILER};
use crate::input::Prepared;
use crate::policy::{self, MatchKindSpec, PolicyPackage, Scope, View};
use crate::views::NormView;
use crate::{AnalysisError, Config, Field, Stance};
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind as AcMatchKind};
use regex_automata::hybrid::dfa::{Cache, OverlappingState, DFA};
use regex_automata::nfa::thompson;
use regex_automata::util::syntax;
use regex_automata::{Anchored, Input, MatchKind, PatternID};
use std::collections::HashSet;
use std::ops::Range;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct Hit {
    pub rule: usize,
    pub span: Range<usize>,
    pub quoted: bool,
    pub field: Field,
    /// Set by structural checks whose exact tier is heuristic.
    pub force_candidate: bool,
    /// Set by rules that report what they could not check.
    pub force_hint: bool,
    /// For word-set and regex hits, the exact text matched in the haystack.
    /// Norm-scope rules use the norm view. Other rules use the source. The
    /// trigger-fidelity check renders the reported source slice and checks
    /// that it contains this text. A mapping mismatch returns an
    /// instrumentation error and prevents a finding at the wrong bytes.
    /// Structural checks carry `None` when they have no trigger.
    pub trigger: Option<String>,
    /// For a hit in a decoded link destination, the parser-decoded text of
    /// the region that contains the span. Trigger fidelity checks this text
    /// with pulldown semantics. The raw spelling can hide a trigger behind
    /// references such as `&lowbar;` and `&period;` that the enumerated
    /// entity table cannot resolve. The render_key path returned exit 30 on
    /// those inputs and aborted the whole report with an instrumentation
    /// error. This field preserves the decoded text.
    pub decoded: Option<String>,
    /// Instrument figure appended to the finding message (never a trigger:
    /// it does not exist in the source, so it must not enter the
    /// trigger-fidelity check). Used by ratio instruments (SLOP-C009) whose
    /// finding is a computed number over the whole document.
    pub detail: Option<String>,
}

impl Hit {
    pub fn new(rule: usize, span: Range<usize>) -> Hit {
        Hit {
            rule,
            span,
            quoted: false,
            field: Field::Whole,
            force_candidate: false,
            force_hint: false,
            trigger: None,
            decoded: None,
            detail: None,
        }
    }
}

struct RxMeta {
    rule: usize,
    trim_start: usize,
    trim_end: usize,
    /// The pattern begins/ends with `\b`. The DFA matched the ASCII
    /// `(?-u:\b)` prefilter form. The edge is re-validated against the real
    /// Unicode word-boundary rule (`is_xid_continue` XOR across the edge)
    /// before the hit is accepted, so an ASCII boundary inside a non-ASCII
    /// token ("éwhy") cannot fire. Interior `\b` occurrences stay ASCII-only:
    /// their match positions are unrecoverable from the DFA, and every policy
    /// pattern is ASCII around interior boundaries.
    bound_start: bool,
    bound_end: bool,
    /// Maximum match width in bytes. Every policy pattern is bounded-width
    /// (the build rejects unbounded quantifiers), so this is `Some(w)` and
    /// bounds the reverse start-recovery window to `w` bytes, which is what
    /// keeps the overlapping adapter linear. `None` is only the degenerate
    /// no-finite-maximum fallback and drops back to the region low bound.
    max_width: Option<usize>,
}

pub struct CompiledPolicy {
    pub pkg: PolicyPackage,
    ac_ci: AhoCorasick,
    ac_ci_meta: Vec<usize>,
    ac_cs: AhoCorasick,
    ac_cs_meta: Vec<usize>,
    rx_fwd: DFA,
    rx_rev: DFA,
    rx_meta: Vec<RxMeta>,
}

/// Per-call scratch caches for the lazy DFAs. The compiled machines are
/// shared. The caches are created once per analysis.
pub struct RxCaches {
    fwd: Cache,
    rev: Cache,
}

impl CompiledPolicy {
    pub fn caches(&self) -> RxCaches {
        RxCaches {
            fwd: self.rx_fwd.create_cache(),
            rev: self.rx_rev.create_cache(),
        }
    }
}

static COMPILED: OnceLock<Result<CompiledPolicy, String>> = OnceLock::new();

pub fn compiled() -> Result<&'static CompiledPolicy, String> {
    COMPILED.get_or_init(build).as_ref().map_err(|e| e.clone())
}

/// Rewrite policy patterns into DFA-compatible form. `\b` becomes the ASCII
/// word boundary (a PREFILTER, pattern-edge occurrences are re-validated
/// against Unicode boundaries in `scan_rx`), and the two single-character
/// look-around forms become consuming UNICODE `\w` characters with one-CHAR
/// span trims recorded. Unicode `\w` (not ASCII) so an edge
/// like `café--style` or `变量--值` produces a DFA candidate at all. The
/// consumed char is then held to `is_xid_continue` in `scan_rx`.
fn rewrite_pattern(p: &str) -> Result<(String, usize, usize, bool, bool), String> {
    let mut pat = p.to_string();
    let mut trim_start = 0usize;
    let mut trim_end = 0usize;
    if let Some(rest) = pat.strip_prefix(r"(?<=\w)") {
        pat = format!(r"\w{rest}");
        trim_start = 1;
    }
    if let Some(rest) = pat.strip_suffix(r"(?=\w)") {
        pat = format!(r"{rest}\w");
        trim_end = 1;
    }
    if pat.contains("(?<=") || pat.contains("(?<!") || pat.contains("(?!") {
        return Err(format!("unsupported look-around in pattern {p}"));
    }
    if pat.contains("(?=") {
        return Err(format!("unsupported look-ahead in pattern {p}"));
    }
    // Pattern-edge `\b` positions map to the final span's edges, so those two
    // (unlike interior `\b`) can be post-filtered per match.
    let bound_start = pat.strip_prefix("(?i)").unwrap_or(&pat).starts_with(r"\b");
    let bound_end = pat.ends_with(r"\b");
    let pat = pat.replace(r"\b", r"(?-u:\b)");
    Ok((pat, trim_start, trim_end, bound_start, bound_end))
}

/// Validate a rewritten pattern against the locked bounded-width policy and return
/// its maximum match width in bytes.
///
/// The bespoke overlapping forward+reverse adapter recovers each match start
/// with a reverse search. On an unbounded-width pattern that window is the
/// whole region and a run of overlapping match ends makes it quadratic
/// (`turn\d+...\d+` measured ~12s at 256KB). The dependency decision therefore
/// bans every unbounded-width quantifier (`*`, `+`, `{n,}`) at policy build,
/// whitespace included. Unbounded whitespace is not exempt: a pattern ending in
/// `\s+` (e.g. Q001's `\?\s+`) manufactures a fresh match end at every
/// whitespace byte, each an O(region) reverse scan, which reproduces the exact
/// quadratic (a `\?\s+` tail measured 10.66s at 256KB). Bounded forms are
/// required everywhere: `.*`/`\d+`/`\w+` → `[^.\r\n]{0,120}` and friends.
/// `\s+`/`\s*` → `\s{1,N}`/`\s{0,N}`.
///
/// `Ok(Some(w))` gives the maximum width in bytes. `Ok(None)` covers a HIR
/// without a finite maximum. Fully bounded patterns have a finite maximum,
/// and the caller falls back to the region bound otherwise. `Err` indicates a
/// banned pattern or parse failure.
fn validate_bounded_width(pat: &str) -> Result<Option<usize>, String> {
    let hir = regex_syntax::parse(pat)
        .map_err(|e| format!("pattern {pat} failed width-validation parse: {e}"))?;
    if unbounded_repetition(&hir) {
        return Err(format!(
            "pattern {pat} has an unbounded-width quantifier (*, +, or {{n,}}); \
             bounded forms are required, whitespace included \
             (e.g. {{0,120}} for text, \\s{{1,8}} for whitespace)"
        ));
    }
    Ok(hir.properties().maximum_len())
}

/// True if the HIR contains any unbounded repetition (`max == None`). No class
/// is exempt: unbounded whitespace at a match end is as quadratic as any other
/// unbounded run.
fn unbounded_repetition(hir: &regex_syntax::hir::Hir) -> bool {
    use regex_syntax::hir::HirKind;
    match hir.kind() {
        HirKind::Repetition(rep) => rep.max.is_none() || unbounded_repetition(&rep.sub),
        HirKind::Capture(c) => unbounded_repetition(&c.sub),
        HirKind::Concat(v) | HirKind::Alternation(v) => v.iter().any(unbounded_repetition),
        _ => false,
    }
}

fn build() -> Result<CompiledPolicy, String> {
    let pkg = policy::load()?;

    let mut ci_pats: Vec<String> = Vec::new();
    let mut ci_meta: Vec<usize> = Vec::new();
    let mut cs_pats: Vec<String> = Vec::new();
    let mut cs_meta: Vec<usize> = Vec::new();
    let mut rx_pats: Vec<String> = Vec::new();
    let mut rx_meta: Vec<RxMeta> = Vec::new();

    for (idx, rule) in pkg.rules.iter().enumerate() {
        if rule.lifecycle == policy::Lifecycle::Deprecated {
            continue;
        }
        if rule.kind == MatchKindSpec::WordSet {
            for term in &rule.terms {
                if term.is_empty() {
                    return Err(format!("rule {} has an empty term", rule.id));
                }
                if rule.case_sensitive {
                    cs_pats.push(term.clone());
                    cs_meta.push(idx);
                } else {
                    ci_pats.push(term.clone());
                    ci_meta.push(idx);
                }
            }
        }
        for p in &rule.patterns {
            let (pat, ts, te, bs, be) = rewrite_pattern(p)?;
            let max_width = validate_bounded_width(&pat)?;
            rx_pats.push(pat);
            rx_meta.push(RxMeta {
                rule: idx,
                trim_start: ts,
                trim_end: te,
                bound_start: bs,
                bound_end: be,
                max_width,
            });
        }
    }

    let ac_ci = AhoCorasickBuilder::new()
        .match_kind(AcMatchKind::Standard)
        .ascii_case_insensitive(true)
        .build(&ci_pats)
        .map_err(|e| format!("case-insensitive automaton: {e}"))?;
    let ac_cs = AhoCorasickBuilder::new()
        .match_kind(AcMatchKind::Standard)
        .build(&cs_pats)
        .map_err(|e| format!("case-sensitive automaton: {e}"))?;

    let syn = syntax::Config::new()
        .unicode(true)
        .utf8(true)
        .multi_line(true);
    // One multi-pattern machine with MatchKind::All. The lazy DFA is used
    // because full determinization of the bounded-window patterns
    // (`.{1,160}` under an unanchored prefix) is exponential. Laziness keeps
    // the same automaton semantics while materializing only reachable
    // states, with the cache size as the load-time bound.
    let fwd = DFA::builder()
        .configure(
            DFA::config()
                .match_kind(MatchKind::All)
                .starts_for_each_pattern(true)
                .cache_capacity(8 * 1024 * 1024),
        )
        .syntax(syn)
        .build_many(&rx_pats)
        .map_err(|e| format!("forward dfa: {e}"))?;
    let rev = DFA::builder()
        .configure(
            DFA::config()
                .match_kind(MatchKind::All)
                .starts_for_each_pattern(true)
                .cache_capacity(8 * 1024 * 1024),
        )
        .thompson(thompson::Config::new().reverse(true))
        .syntax(syn)
        .build_many(&rx_pats)
        .map_err(|e| format!("reverse dfa: {e}"))?;

    // Empty-matchable patterns are banned.
    let mut probe_cache = fwd.create_cache();
    for (pid, pat) in rx_pats.iter().enumerate() {
        let input = Input::new("").anchored(Anchored::Pattern(PatternID::new_unchecked(pid)));
        if let Ok(Some(_)) = fwd.try_search_fwd(&mut probe_cache, &input) {
            return Err(format!("pattern {pat} can match empty"));
        }
    }

    Ok(CompiledPolicy {
        pkg,
        ac_ci,
        ac_ci_meta: ci_meta,
        ac_cs,
        ac_cs_meta: cs_meta,
        rx_fwd: fwd,
        rx_rev: rev,
        rx_meta,
    })
}

fn field_of(flags: u8) -> Field {
    if flags & F_SUBJECT != 0 {
        Field::Subject
    } else if flags & F_TRAILER != 0 {
        Field::Trailers
    } else if flags & F_BODY != 0 {
        Field::Body
    } else {
        Field::Whole
    }
}

fn word_bounded(hay: &str, span: &Range<usize>) -> bool {
    let before_ok = hay[..span.start]
        .chars()
        .next_back()
        .map(|c| !unicode_ident::is_xid_continue(c))
        .unwrap_or(true);
    let after_ok = hay[span.end..]
        .chars()
        .next()
        .map(|c| !unicode_ident::is_xid_continue(c))
        .unwrap_or(true);
    before_ok && after_ok
}

/// Real Unicode word boundary at `at`: exactly one side of the position is a
/// word (xid_continue) character, the Unicode analog of `\b`, sharing
/// `word_bounded`'s character class. Out-of-text sides count as non-word.
fn unicode_word_boundary(hay: &str, at: usize) -> bool {
    let before = hay[..at]
        .chars()
        .next_back()
        .map(unicode_ident::is_xid_continue)
        .unwrap_or(false);
    let after = hay[at..]
        .chars()
        .next()
        .map(unicode_ident::is_xid_continue)
        .unwrap_or(false);
    before != after
}

/// Case-insensitive covering-phrase check for `[rule.exemptions]`. A phrase
/// exempts a hit when it contains the hit span. A plain phrase matches
/// anywhere, inside a longer token too, so `cpu utilization` covers
/// `vCPU utilization`. A phrase that starts with `\b` must open on a word
/// edge, so `\bform i` covers `direct form I` and never `platform I`. A
/// phrase that ends with `\b` must close on a word edge, so `upstream ref\b`
/// never covers `upstream reference`.
///
/// A space in a phrase matches a run of whitespace. When that run holds a
/// line break, the match is read again in the source text through `origin`
/// (the norm view and its source), where each gap must be spaces and tabs
/// with at most one line break. A line wrap keeps the exemption. A blank
/// line, a heading boundary with or without a blank line after it, or a new
/// list item ends it. `lines_are_blocks` is true for formats that read each
/// source line as its own block, such as a commit body or plain text.
fn exempted(
    hay: &str,
    span: &Range<usize>,
    phrases: &[String],
    origin: Option<(&NormView, &str)>,
    lines_are_blocks: bool,
) -> bool {
    if phrases.is_empty() {
        return false;
    }
    let win_start =
        crate::widen_to_char_boundaries(hay, span.start.saturating_sub(60)..span.start).start;
    let win_end =
        crate::widen_to_char_boundaries(hay, span.end..(span.end + 60).min(hay.len())).end;
    let window = hay[win_start..win_end].to_lowercase();
    // Lowercasing can change byte lengths for non-ASCII. Recompute the match
    // position by lowercasing the prefix.
    let rel_start = hay[win_start..span.start].to_lowercase().len();
    let rel_end = rel_start + hay[span.start..span.end].to_lowercase().len();
    let is_word = |c: char| unicode_ident::is_xid_continue(c);
    for raw in phrases {
        let (phrase, open) = match raw.strip_prefix("\\b") {
            Some(p) => (p, true),
            None => (raw.as_str(), false),
        };
        let (phrase, close) = match phrase.strip_suffix("\\b") {
            Some(p) => (p, true),
            None => (phrase, false),
        };
        if phrase.is_empty() {
            continue;
        }
        for (s, _) in window.char_indices() {
            if s > rel_start {
                break;
            }
            let Some(e) = phrase_end(&window, s, phrase, None) else {
                continue;
            };
            let before = if s == 0 {
                hay[..win_start].chars().next_back()
            } else {
                window[..s].chars().next_back()
            };
            let open_ok = !open
                || !phrase.chars().next().is_some_and(is_word)
                || !before.is_some_and(is_word);
            let after = if e == window.len() {
                hay[win_end..].chars().next()
            } else {
                window[e..].chars().next()
            };
            let close_ok = !close || !after.is_some_and(is_word);
            if s <= rel_start
                && e >= rel_end
                && open_ok
                && close_ok
                && wrap_ok(
                    hay,
                    win_start,
                    &window,
                    s..e,
                    phrase,
                    origin,
                    lines_are_blocks,
                )
            {
                return true;
            }
        }
    }
    false
}

/// True when a phrase match at `m` in the lowercased `window` holds no line
/// break, or when every line break in it is a wrap. The norm view writes a
/// block join as a line break with an empty source range and a hard break as
/// one with its own source bytes. In markdown a soft wrap is already a space,
/// so a block join inside the match is a real boundary, such as a heading
/// followed by its paragraph, and ends the match. Where each source line is
/// its own block, a block join is a wrapped line, and a line that opens with
/// `#` ends the match instead. In both cases the source text must still match
/// the phrase with at most one line break per gap, which rules out a blank
/// line and a new list item.
fn wrap_ok(
    hay: &str,
    win_start: usize,
    window: &str,
    m: Range<usize>,
    phrase: &str,
    origin: Option<(&NormView, &str)>,
    lines_are_blocks: bool,
) -> bool {
    if !window[m.clone()].contains(['\n', '\r']) {
        return true;
    }
    let (Some(hs), Some(he)) = (
        hay_offset(hay, win_start, m.start),
        hay_offset(hay, win_start, m.end),
    ) else {
        return false;
    };
    if let Some((n, _)) = origin {
        let block_join = n.segs.iter().any(|seg| {
            seg.norm.start >= hs
                && seg.norm.end <= he
                && seg.src.is_empty()
                && n.text[seg.norm.clone()].contains(['\n', '\r'])
        });
        if block_join && !lines_are_blocks {
            return false;
        }
    }
    let text = match origin {
        Some((n, src)) => match n.to_source(hs..he) {
            Some(r) => crate::widen_to_char_boundaries(src, r),
            None => return false,
        },
        None => hs..he,
    };
    let full = match origin {
        Some((_, src)) => src,
        None => hay,
    };
    let line_start = full[..text.start].rfind(['\n', '\r']).map_or(0, |i| i + 1);
    if lines_are_blocks && full[line_start..].trim_start().starts_with('#') {
        return false;
    }
    let source = &full[text];
    let lower = source.to_lowercase();
    phrase_end(&lower, 0, phrase, Some(1)) == Some(lower.len())
}

/// The `hay` offset whose lowercased prefix from `win_start` is `rel` bytes
/// long, or `None` when `rel` falls inside one lowercased character.
fn hay_offset(hay: &str, win_start: usize, rel: usize) -> Option<usize> {
    let mut acc = 0usize;
    for (i, c) in hay[win_start..].char_indices() {
        if acc == rel {
            return Some(win_start + i);
        }
        if acc > rel {
            return None;
        }
        acc += c.to_lowercase().map(char::len_utf8).sum::<usize>();
    }
    (acc == rel).then_some(hay.len())
}

/// The end of `phrase` matched at `start` in `text`, or `None` when it does
/// not match there. A space in the phrase matches one or more spaces, tabs,
/// and line breaks. A line break is `\n`, a bare `\r`, or `\r\n` counted as
/// one. `max_breaks` caps the line breaks in each such run when set. Every
/// other byte matches itself.
fn phrase_end(text: &str, start: usize, phrase: &str, max_breaks: Option<usize>) -> Option<usize> {
    let t = text.as_bytes();
    let mut i = start;
    for &b in phrase.as_bytes() {
        if b == b' ' {
            let from = i;
            let mut breaks = 0usize;
            while let Some(&c) = t.get(i) {
                match c {
                    b' ' | b'\t' => i += 1,
                    // A line break is `\n`, a bare `\r`, or `\r\n` read as one.
                    b'\r' | b'\n' if max_breaks.is_none_or(|m| breaks < m) => {
                        breaks += 1;
                        i += if c == b'\r' && t.get(i + 1) == Some(&b'\n') {
                            2
                        } else {
                            1
                        };
                    }
                    _ => break,
                }
            }
            if i == from {
                return None;
            }
        } else if t.get(i) == Some(&b) {
            i += 1;
        } else {
            return None;
        }
    }
    Some(i)
}

/// Function words and the imperative verbs that open marketing copy. A
/// lexicon word capitalized after one of these is a heading, a sentence
/// start, or a slogan, never a product name, so `The Bedrock of Our Approach`
/// and `Experience Seamless Integration` still fire.
const PROPER_NOUN_STOPWORDS: [&str; 83] = [
    "a",
    "an",
    "the",
    "this",
    "that",
    "these",
    "those",
    "our",
    "your",
    "their",
    "its",
    "my",
    "his",
    "her",
    "we",
    "you",
    "they",
    "it",
    "i",
    "in",
    "on",
    "of",
    "for",
    "with",
    "to",
    "and",
    "or",
    "but",
    "at",
    "by",
    "from",
    "as",
    "is",
    "are",
    "be",
    "into",
    "how",
    "why",
    "what",
    "via",
    "experience",
    "deliver",
    "discover",
    "introducing",
    "unlock",
    "build",
    "create",
    "achieve",
    "enjoy",
    "meet",
    "transform",
    "elevate",
    "empower",
    "explore",
    "embrace",
    "get",
    "try",
    "start",
    "make",
    "go",
    "join",
    "choose",
    "find",
    "use",
    "bring",
    "take",
    "turn",
    "let",
    "give",
    "learn",
    "see",
    "run",
    "grow",
    "scale",
    "boost",
    "simplify",
    "streamline",
    "maximize",
    "accelerate",
    "drive",
    "power",
    "ship",
    "launch",
];

/// A capitalized lexicon hit that directly follows another capitalized word,
/// one space apart, reads as the second word of a proper noun such as
/// `Amazon Bedrock`. The check runs only for rules whose `match.params`
/// sets `proper_noun_compound = true`. The preceding word must not be a
/// function word or a term of any rule that sets the flag, so a title-case
/// stack such as `Unlock Seamless Workflows` still fires. A hyphen joins the
/// preceding word, so `World-Class Seamless` reads `world-class`, a lexicon
/// term, and still fires. A soft line wrap
/// reads as one space. A block break or terminal punctuation leaves no word
/// one space before the term, so a sentence start never qualifies.
fn proper_noun_compound(cp: &CompiledPolicy, hay: &str, span: &Range<usize>) -> bool {
    if !hay[span.clone()]
        .chars()
        .next()
        .is_some_and(char::is_uppercase)
    {
        return false;
    }
    let before = &hay[..span.start];
    let Some(gap) = before.chars().next_back() else {
        return false;
    };
    if gap != ' ' && gap != '\u{a0}' {
        return false;
    }
    let lead = &before[..before.len() - gap.len_utf8()];
    let word_start = lead
        .char_indices()
        .rev()
        .take_while(|&(_, c)| unicode_ident::is_xid_continue(c) || c == '-')
        .last()
        .map(|(i, _)| i);
    let Some(word_start) = word_start else {
        return false;
    };
    let prev = &lead[word_start..];
    if !prev.chars().next().is_some_and(char::is_uppercase) {
        return false;
    }
    let prev_lower = prev.to_lowercase();
    if PROPER_NOUN_STOPWORDS.contains(&prev_lower.as_str()) {
        return false;
    }
    !cp.pkg
        .rules
        .iter()
        .filter(|r| proper_noun_flag(r))
        .any(|r| r.terms.contains(&prev_lower))
}

/// True when the rule opts into the proper-noun compound exemption.
fn proper_noun_flag(rule: &policy::Rule) -> bool {
    rule.params
        .as_table()
        .and_then(|t| t.get("proper_noun_compound"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Case-sensitive covering-literal check for `profile_exemptions`. A literal
/// must contain the hit span and stand alone as a verdict token. The caller
/// supplies the current profile's literals. Both the word-set and regex paths
/// use this check through `accept_word_hit` and `scan_rx`.
///
/// A covering literal only exempts a STANDALONE token:
/// - its edges sit on word boundaries in `hay`, so an embedded spelling
///   (`AVOCA[DO NOT BUILD] LIST`) never covers anything.
/// - it is not continued by a following word. A verdict label ends at end
///   of text, a line break, punctuation, or a table-cell `|`, never at a
///   continuing alphanumeric word, so `We DO NOT BUILD trust by ...` stays
///   a finding.
fn exempted_cs(hay: &str, span: &Range<usize>, literals: &[String]) -> bool {
    if literals.is_empty() {
        return false;
    }
    let max_len = literals.iter().map(|l| l.len()).max().unwrap_or(0);
    let win_start =
        crate::widen_to_char_boundaries(hay, span.start.saturating_sub(max_len)..span.start).start;
    let win_end =
        crate::widen_to_char_boundaries(hay, span.end..(span.end + max_len).min(hay.len())).end;
    let window = &hay[win_start..win_end];
    let rel_start = span.start - win_start;
    let rel_end = span.end - win_start;
    for lit in literals {
        let mut at = 0usize;
        while let Some(pos) = window[at..].find(lit.as_str()) {
            let s = at + pos;
            let e = s + lit.len();
            if s <= rel_start
                && e >= rel_end
                && word_bounded(hay, &(win_start + s..win_start + e))
                && standalone_token_end(hay, win_start + e)
            {
                return true;
            }
            at = s + 1;
        }
    }
    false
}

/// True when the text after a covering literal reads as the end of a
/// standalone token: end of text, a line break, punctuation, or a table-cell
/// `|`, possibly after a short inline space run, but never an alphanumeric
/// continuation word. The peek is bounded at 8 space/tab units, mirroring
/// the parsers' whitespace bound. A longer run reads as layout, and layout
/// ends a label.
fn standalone_token_end(hay: &str, from: usize) -> bool {
    let mut seen = 0usize;
    for c in hay[from..].chars() {
        match c {
            ' ' | '\t' => {
                seen += 1;
                if seen > 8 {
                    return true;
                }
            }
            c if c.is_alphanumeric() => return false,
            _ => return true,
        }
    }
    true
}

fn cjk_present(s: &str) -> bool {
    s.chars().any(|c| {
        let u = c as u32;
        (0x3040..=0x30FF).contains(&u)
            || (0x3400..=0x4DBF).contains(&u)
            || (0x4E00..=0x9FFF).contains(&u)
            || (0xAC00..=0xD7AF).contains(&u)
            || (0xF900..=0xFAFF).contains(&u)
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScanCtx {
    Norm,
    RawAll,
    RawProse,
    LinkUrl,
    Code,
    Heading,
    Comment,
}

fn rule_in_ctx(rule: &policy::Rule, ctx: ScanCtx) -> bool {
    match ctx {
        ScanCtx::Norm => rule.view == View::Norm && rule.scope == Scope::None,
        ScanCtx::RawAll => rule.view == View::Raw && rule.scope == Scope::None,
        ScanCtx::RawProse => rule.view == View::Prose && rule.scope == Scope::None,
        ScanCtx::LinkUrl => rule.scope == Scope::LinkUrl || rule.id == "SLOP-P002",
        ScanCtx::Code => rule.scope == Scope::Code,
        ScanCtx::Heading => rule.scope == Scope::Heading,
        ScanCtx::Comment => rule.scope == Scope::Comment,
    }
}

pub fn scan_all(
    cp: &CompiledPolicy,
    prepared: &Prepared,
    doc: &Doc,
    norm: &NormView,
    config: &Config,
) -> Result<Vec<Hit>, AnalysisError> {
    let src = prepared.text.as_str();
    let mut hits: Vec<Hit> = Vec::new();
    let mut caches = cp.caches();

    // 1. Word sets over the norm view.
    scan_ac_haystack(
        cp,
        config,
        norm.text.as_str(),
        None,
        ScanCtx::Norm,
        Some(norm),
        src,
        &mut hits,
    );

    // 2. Injection family over every raw byte.
    if !src.is_empty() {
        scan_ac_haystack(cp, config, src, None, ScanCtx::RawAll, None, src, &mut hits);
    }

    // 3. Link-URL regions.
    for r in &doc.link_url_regions {
        scan_ac_haystack(
            cp,
            config,
            src,
            Some(r.clone()),
            ScanCtx::LinkUrl,
            None,
            src,
            &mut hits,
        );
    }

    // 4. Case-sensitive word set over prose and link-URL regions.
    for (r, flags) in &doc.prose_regions {
        scan_ac_cs(cp, config, src, r.clone(), *flags, &mut hits);
    }
    for r in &doc.link_url_regions {
        scan_ac_cs(cp, config, src, r.clone(), 0, &mut hits);
    }

    // 5. Comment scope (api-docs profile).
    for c in &doc.html_comments {
        scan_ac_comment(cp, config, src, c, &mut hits);
    }

    // 6. Regex set over the norm view.
    scan_rx(
        cp,
        &mut caches,
        config,
        norm.text.as_str(),
        0..norm.text.len(),
        ScanCtx::Norm,
        Some(norm),
        src,
        0,
        &mut hits,
    )?;

    // 7. Regex set over raw (control and zero-width characters).
    scan_rx(
        cp,
        &mut caches,
        config,
        src,
        0..src.len(),
        ScanCtx::RawAll,
        None,
        src,
        0,
        &mut hits,
    )?;

    // 8. Regex set over prose regions in source coordinates.
    for (r, flags) in &doc.prose_regions {
        let mut sub = Vec::new();
        scan_rx(
            cp,
            &mut caches,
            config,
            src,
            r.clone(),
            ScanCtx::RawProse,
            None,
            src,
            *flags,
            &mut sub,
        )?;
        hits.extend(sub);
    }

    // 8b. Regex set over link-URL regions (SLOP-P002's generalized forms).
    // Source coordinates, exactly like the AC link-URL pass above.
    for r in &doc.link_url_regions {
        scan_rx(
            cp,
            &mut caches,
            config,
            src,
            r.clone(),
            ScanCtx::LinkUrl,
            None,
            src,
            0,
            &mut hits,
        )?;
    }

    // 8c. Decoded link destinations: where the raw spelling hides the
    // pattern behind backslash escapes or character references the parser (or
    // the browser, for autolinks) resolves, run the link-URL rules over the
    // decoded text too and map each hit back onto the raw region.
    for (r, decoded) in &doc.link_url_decoded {
        scan_link_url_decoded(cp, &mut caches, config, src, r, decoded, &mut hits)?;
    }

    // 9. Code scope.
    for code in &doc.code_regions {
        if code.fenced && is_text_info(&code.info) {
            continue;
        }
        scan_rx(
            cp,
            &mut caches,
            config,
            src,
            code.range.clone(),
            ScanCtx::Code,
            None,
            src,
            0,
            &mut hits,
        )?;
    }

    // 10. Heading scope: each heading text is its own haystack so anchors
    // hold at the heading start.
    for h in &doc.headings {
        if h.text_range.start >= h.text_range.end {
            continue;
        }
        let hay = &src[h.text_range.clone()];
        let mut sub = Vec::new();
        scan_rx(
            cp,
            &mut caches,
            config,
            hay,
            0..hay.len(),
            ScanCtx::Heading,
            None,
            src,
            0,
            &mut sub,
        )?;
        for mut hit in sub {
            hit.span.start += h.text_range.start;
            hit.span.end += h.text_range.start;
            hits.push(hit);
        }
    }

    resolve_overlaps(&mut hits);
    Ok(hits)
}

fn is_text_info(info: &str) -> bool {
    let lang = info.split_whitespace().next().unwrap_or("");
    matches!(
        lang,
        "text" | "txt" | "plain" | "plaintext" | "md" | "markdown" | "prose"
    )
}

#[allow(clippy::too_many_arguments)]
fn scan_ac_haystack(
    cp: &CompiledPolicy,
    config: &Config,
    hay: &str,
    region: Option<Range<usize>>,
    ctx: ScanCtx,
    norm: Option<&NormView>,
    src: &str,
    hits: &mut Vec<Hit>,
) {
    let input = match &region {
        Some(r) => aho_corasick::Input::new(hay).range(r.clone()),
        None => aho_corasick::Input::new(hay),
    };
    for m in cp.ac_ci.find_overlapping_iter(input) {
        let rule_idx = cp.ac_ci_meta[m.pattern().as_usize()];
        let rule = &cp.pkg.rules[rule_idx];
        if !rule_in_ctx(rule, ctx) {
            continue;
        }
        let span = m.start()..m.end();
        accept_word_hit(cp, config, rule_idx, hay, span, ctx, norm, src, hits);
    }
}

fn scan_ac_cs(
    cp: &CompiledPolicy,
    config: &Config,
    src: &str,
    region: Range<usize>,
    flags: u8,
    hits: &mut Vec<Hit>,
) {
    let input = aho_corasick::Input::new(src).range(region);
    for m in cp.ac_cs.find_overlapping_iter(input) {
        let rule_idx = cp.ac_cs_meta[m.pattern().as_usize()];
        let rule = &cp.pkg.rules[rule_idx];
        if rule.view != View::Prose && rule.scope != Scope::LinkUrl {
            continue;
        }
        let span = m.start()..m.end();
        if rule.boundary_word && !word_bounded(src, &span) {
            continue;
        }
        let field = field_of(flags);
        if rule.stance(config.profile, field) == Stance::Off {
            continue;
        }
        let mut hit = Hit::new(rule_idx, span);
        hit.field = field;
        hits.push(hit);
    }
}

fn scan_ac_comment(
    cp: &CompiledPolicy,
    config: &Config,
    src: &str,
    comment: &crate::extract::HtmlComment,
    hits: &mut Vec<Hit>,
) {
    let hay = &src[comment.content_range.clone()];
    for m in cp
        .ac_ci
        .find_overlapping_iter(aho_corasick::Input::new(hay))
    {
        let rule_idx = cp.ac_ci_meta[m.pattern().as_usize()];
        let rule = &cp.pkg.rules[rule_idx];
        if !rule_in_ctx(rule, ScanCtx::Comment) {
            continue;
        }
        if rule.stance(config.profile, Field::Whole) == Stance::Off {
            continue;
        }
        let span = comment.content_range.start + m.start()..comment.content_range.start + m.end();
        hits.push(Hit::new(rule_idx, span));
    }
}

#[allow(clippy::too_many_arguments)]
fn accept_word_hit(
    cp: &CompiledPolicy,
    config: &Config,
    rule_idx: usize,
    hay: &str,
    span: Range<usize>,
    ctx: ScanCtx,
    norm: Option<&NormView>,
    src: &str,
    hits: &mut Vec<Hit>,
) {
    let rule = &cp.pkg.rules[rule_idx];
    let (field, quoted) = match (ctx, norm) {
        (ScanCtx::Norm, Some(n)) => {
            let f = n.flags_at(span.start);
            (field_of(f), n.all_quoted(&span))
        }
        _ => (Field::Whole, false),
    };
    if rule.stance(config.profile, field) == Stance::Off {
        return;
    }
    if rule.boundary_word && !word_bounded(hay, &span) {
        return;
    }
    if rule.block_start {
        if let Some(n) = norm {
            if !n.is_block_start(span.start) {
                return;
            }
        }
    }
    // A rule may anchor part of its lexicon instead of all of it. Entries
    // listed in `match.params.block_start_only` fire only where they open a
    // sentence, a line, or a list item. The rest of the same lexicon is read
    // anywhere. Only a matched span reaches here, so the list costs nothing
    // to consult.
    if let Some(anchored) = rule
        .params
        .as_table()
        .and_then(|t| t.get("block_start_only"))
        .and_then(|v| v.as_array())
    {
        let matched = hay[span.clone()].to_lowercase();
        if anchored
            .iter()
            .filter_map(|v| v.as_str())
            .any(|e| e == matched)
        {
            match norm {
                Some(n) if n.is_block_start(span.start) => {}
                _ => return,
            }
        }
    }
    // The filler rule's "overall" entry fires only at block start.
    if rule.id == "SLOP-T001" {
        let matched = hay[span.clone()].to_ascii_lowercase();
        if matched == "overall" {
            match norm {
                Some(n) if n.is_block_start(span.start) => {}
                _ => return,
            }
        }
    }
    let origin = match (ctx, norm) {
        (ScanCtx::Norm, Some(n)) => Some((n, src)),
        _ => None,
    };
    let lines_are_blocks = config.input_format != crate::InputFormat::Markdown;
    if exempted(hay, &span, &rule.exemptions, origin, lines_are_blocks) {
        return;
    }
    if proper_noun_flag(rule) && proper_noun_compound(cp, hay, &span) {
        return;
    }
    if exempted_cs(hay, &span, &rule.profile_exemptions[config.profile.index()]) {
        return;
    }
    // Deployment scrub override narrows the scrub list.
    if rule.id == "SLOP-W001" {
        if let Some(list) = &config.deployment.scrub_overrides {
            let matched = hay[span.clone()].to_lowercase();
            if !list.iter().any(|t| t.to_lowercase() == matched) {
                return;
            }
        }
    }
    // Widen each span in its own coordinate system, as in `scan_rx`. Norm hits map through
    // `to_source` and widen against `src`. Other hits widen against `hay`.
    // The caller then rebases slice-local spans.
    let source_span = match (ctx, norm) {
        (ScanCtx::Norm, Some(n)) => match n.to_source(span.clone()) {
            Some(s) => crate::widen_to_char_boundaries(src, s),
            None => return,
        },
        _ => crate::widen_to_char_boundaries(hay, span.clone()),
    };
    if source_span.start >= source_span.end {
        return;
    }
    let mut hit = Hit::new(rule_idx, source_span);
    hit.field = field;
    hit.quoted = quoted;
    // A match inside a fully-folded single-script token (no Latin
    // witness, folded because every char was a table confusable) is the
    // conservative candidate path, the rare genuine foreign word that folds
    // onto an English lexicon term must reach a judge, not hard-block.
    if let (ScanCtx::Norm, Some(n)) = (ctx, norm) {
        if n.span_has_flag(&span, crate::extract::F_FULL_FOLD) {
            hit.force_candidate = true;
        }
    }
    hit.trigger = Some(hay[span].to_string());
    hits.push(hit);
}

/// Run the link-URL passes (case-insensitive word set, case-sensitive
/// word set, regex set) over the decoded text of one link destination, then
/// map each hit back into source coordinates: the exact position of the
/// matched trigger inside the raw region when it occurs there literally,
/// else the whole region as the fail-safe (fidelity-safe, `render_key` resolves
/// the escape/reference spellings, so the whole-region slice still renders
/// to the trigger).
fn scan_link_url_decoded(
    cp: &CompiledPolicy,
    caches: &mut RxCaches,
    config: &Config,
    src: &str,
    region: &Range<usize>,
    decoded: &str,
    hits: &mut Vec<Hit>,
) -> Result<(), AnalysisError> {
    if region.start >= region.end || decoded.is_empty() {
        return Ok(());
    }
    let mut sub = Vec::new();
    scan_ac_haystack(
        cp,
        config,
        decoded,
        None,
        ScanCtx::LinkUrl,
        None,
        decoded,
        &mut sub,
    );
    scan_ac_cs(cp, config, decoded, 0..decoded.len(), 0, &mut sub);
    scan_rx(
        cp,
        caches,
        config,
        decoded,
        0..decoded.len(),
        ScanCtx::LinkUrl,
        None,
        decoded,
        0,
        &mut sub,
    )?;
    let raw = &src[region.clone()];
    for mut hit in sub {
        let trigger = match &hit.trigger {
            Some(t) => t.clone(),
            // The cs word-set pass records no trigger. The decoded slice at
            // the hit's span is the matched text.
            None => match decoded.get(hit.span.clone()) {
                Some(t) => t.to_string(),
                None => continue,
            },
        };
        hit.span = match raw.find(&trigger) {
            Some(p) => region.start + p..region.start + p + trigger.len(),
            None => region.clone(),
        };
        hit.trigger = Some(trigger);
        hit.decoded = Some(decoded.to_string());
        hits.push(hit);
    }
    Ok(())
}

/// The bespoke overlapping adapter over regex-automata's DFAs: the forward
/// DFA yields (pattern, end) pairs, the reverse DFA anchored to the pattern
/// and bounded by the region recovers the start.
#[allow(clippy::too_many_arguments)]
fn scan_rx(
    cp: &CompiledPolicy,
    caches: &mut RxCaches,
    config: &Config,
    hay: &str,
    region: Range<usize>,
    ctx: ScanCtx,
    norm: Option<&NormView>,
    src: &str,
    flags: u8,
    hits: &mut Vec<Hit>,
) -> Result<(), AnalysisError> {
    if region.start >= region.end || cp.rx_meta.is_empty() {
        return Ok(());
    }
    let input = Input::new(hay).range(region.clone());
    let mut state = OverlappingState::start();
    let mut seen: HashSet<(usize, usize, usize)> = HashSet::new();
    loop {
        cp.rx_fwd
            .try_search_overlapping_fwd(&mut caches.fwd, &input, &mut state)
            .map_err(|e| AnalysisError::Instrumentation(format!("forward dfa search: {e}")))?;
        let Some(hm) = state.get_match() else { break };
        let pid = hm.pattern();
        let end = hm.offset();
        let meta = &cp.rx_meta[pid.as_usize()];
        let rule = &cp.pkg.rules[meta.rule];
        if !rule_in_ctx(rule, ctx) {
            continue;
        }
        // Bound the reverse start-recovery window by the pattern's max width.
        // The true start is at most `max_width` bytes before `end`, so a
        // window of that size always contains it while capping each reverse
        // search at O(width) instead of O(region), the fix for the quadratic
        // adapter. Every policy pattern is bounded, so this always bites. The
        // region-bound fallback is dead-defensive.
        let rev_lo = match meta.max_width {
            Some(w) => region.start.max(end.saturating_sub(w)),
            None => region.start,
        };
        let rin = Input::new(hay)
            .range(rev_lo..end)
            .anchored(Anchored::Pattern(pid));
        let start = match cp
            .rx_rev
            .try_search_rev(&mut caches.rev, &rin)
            .map_err(|e| AnalysisError::Instrumentation(format!("reverse dfa search: {e}")))?
        {
            Some(h) => h.offset(),
            None => continue,
        };
        if !seen.insert((pid.as_usize(), start, end)) {
            continue;
        }
        // The DFA matched with ASCII-boundary and Unicode-`\w` PREFILTER
        // forms. Validate each candidate's edges against real Unicode word
        // boundaries before accepting. A trimmed look-around edge consumed
        // one CHAR (possibly multi-byte) that must be a genuine word char.
        // A pattern-edge `\b` must sit at a genuine boundary. An ASCII
        // boundary inside one xid token (`éwhy`) is rejected here.
        let mut mstart = start;
        let mut mend = end;
        if meta.trim_start > 0 {
            match hay[mstart..mend].chars().next() {
                Some(c) if unicode_ident::is_xid_continue(c) => mstart += c.len_utf8(),
                _ => continue,
            }
        }
        if meta.trim_end > 0 {
            match hay[mstart..mend].chars().next_back() {
                Some(c) if unicode_ident::is_xid_continue(c) => mend -= c.len_utf8(),
                _ => continue,
            }
        }
        let mut span = mstart..mend;
        if span.start >= span.end {
            continue;
        }
        span = crate::widen_to_char_boundaries(hay, span);
        if (meta.bound_start && !unicode_word_boundary(hay, span.start))
            || (meta.bound_end && !unicode_word_boundary(hay, span.end))
        {
            continue;
        }

        // Rule-specific post filters from the guard text.
        match rule.id.as_str() {
            "SLOP-P003" => {
                if hay[span.clone()].starts_with('\u{3010}') && cjk_present(&hay[span.clone()]) {
                    continue;
                }
                // A dagger-digit pair at line start defines a footnote and
                // receives the policy exemption. Inline pairs still fire.
                if hay[span.clone()].starts_with(['†', '‡']) {
                    let ls = hay[..span.start].rfind('\n').map(|p| p + 1).unwrap_or(0);
                    if hay[ls..span.start].trim().is_empty() {
                        continue;
                    }
                }
            }
            "SLOP-M004" => {
                let m = &hay[span.clone()];
                let is_zero_width = m.chars().all(|c| {
                    matches!(
                        c,
                        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
                    )
                });
                if is_zero_width {
                    let before = hay[..span.start].chars().next_back();
                    let after = hay[span.end..].chars().next();
                    let inside_word = before.map(unicode_ident::is_xid_continue).unwrap_or(false)
                        && after.map(unicode_ident::is_xid_continue).unwrap_or(false);
                    if !inside_word {
                        continue;
                    }
                }
            }
            "SLOP-C004" => {
                // Sentence-start arm only: that alternation's span begins
                // with the consumed [.!?] boundary. The line-start arm never
                // starts on punctuation.
                let first = hay[span.clone()].chars().next();
                if matches!(first, Some('.' | '!' | '?')) {
                    // A digit run that opens its line is a list marker
                    // ("2. Granted ..."), so the period is structure. A digit
                    // that follows other text on the line closes a real
                    // sentence ("version 2. Granted ..."), so only the marker
                    // suppresses.
                    let before = &hay[..span.start];
                    if before
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_ascii_digit())
                    {
                        let line = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
                        if before[line..]
                            .trim_start()
                            .chars()
                            .all(|c| c.is_ascii_digit())
                        {
                            continue;
                        }
                    }
                    // An abbreviation or mid-sentence period ("e.g. while
                    // ...") is not a sentence end, SLOP-C007's terminal
                    // test, reused, decides.
                    if first == Some('.')
                        && !crate::rules::contrast::period_is_terminal(hay, span.start + 1)
                    {
                        continue;
                    }
                    // The span the reader gets opens at the concession word.
                    // The terminal punctuation and the whitespace after it
                    // licensed the match and belong to the sentence before,
                    // which in a list is a whole block earlier.
                    let mut at = span.start + first.map_or(0, char::len_utf8);
                    while let Some(c) = hay[at..].chars().next() {
                        if c.is_whitespace() {
                            at += c.len_utf8();
                        } else {
                            break;
                        }
                    }
                    if at < span.end {
                        span.start = at;
                    }
                } else if crate::rules::contrast::temporal_while(&hay[span.clone()]) {
                    continue;
                }
            }
            "SLOP-E002" => {
                let prefix = hay[..span.start].trim_end();
                if prefix.ends_with("MUST")
                    || prefix.ends_with("SHOULD")
                    || prefix.ends_with("SHALL")
                    || prefix.ends_with("MAY")
                {
                    continue;
                }
            }
            _ => {}
        }

        if exempted_cs(hay, &span, &rule.profile_exemptions[config.profile.index()]) {
            continue;
        }

        let (field, quoted) = match (ctx, norm) {
            (ScanCtx::Norm, Some(n)) => {
                let f = n.flags_at(span.start);
                (field_of(f), n.all_quoted(&span))
            }
            _ => (field_of(flags), false),
        };
        if rule.stance(config.profile, field) == Stance::Off {
            continue;
        }
        // Widen to char boundaries in the coordinate system the span lives in.
        // A Norm hit's `to_source` already returns source coordinates, widened
        // against `src`. Every other context matched inside `hay`, so the span
        // is in `hay` coordinates (== `src` for whole-source scans, a heading
        // slice otherwise) and must be widened against `hay`. The heading loop
        // rebases the slice-local span to source coords AFTER this returns.
        // Widening a heading-local span against the full `src` dragged a heading
        // span end across a multi-byte char sitting at the same byte offset near
        // the document start (`aaaaaİ …\n\n# Impact` → `Impact `).
        let source_span = match (ctx, norm) {
            (ScanCtx::Norm, Some(n)) => match n.to_source(span.clone()) {
                Some(s) => crate::widen_to_char_boundaries(src, s),
                None => continue,
            },
            _ => crate::widen_to_char_boundaries(hay, span.clone()),
        };
        if source_span.start >= source_span.end {
            continue;
        }
        let mut hit = Hit::new(meta.rule, source_span);
        hit.field = field;
        hit.quoted = quoted;
        hit.trigger = Some(hay[span].to_string());
        hits.push(hit);
    }
    Ok(())
}

/// Policy-layer overlap resolution: within one rule, a span contained in a
/// wider span of the same rule merges into it. Filtering already happened,
/// so an inactive longer rule cannot suppress an active shorter one.
pub fn resolve_overlaps(hits: &mut Vec<Hit>) {
    hits.sort_by(|a, b| {
        (a.rule, a.span.start, std::cmp::Reverse(a.span.end)).cmp(&(
            b.rule,
            b.span.start,
            std::cmp::Reverse(b.span.end),
        ))
    });
    let mut out: Vec<Hit> = Vec::new();
    for h in hits.drain(..) {
        if let Some(prev) = out.last() {
            if prev.rule == h.rule && h.span.start >= prev.span.start && h.span.end <= prev.span.end
            {
                continue;
            }
        }
        out.push(h);
    }
    *hits = out;
}

#[cfg(test)]
mod bounded_width_tests {
    use super::validate_bounded_width;

    // F2: the whitespace exemption is gone. A pattern whose match can END on an
    // unbounded whitespace run is exactly what made the adapter quadratic, so
    // the gate must reject it, no class is exempt.
    #[test]
    fn unbounded_trailing_whitespace_is_rejected() {
        assert!(
            validate_bounded_width(r"\?\s+").is_err(),
            "trailing \\s+ must be rejected"
        );
        assert!(
            validate_bounded_width(r"foo\s*").is_err(),
            "trailing \\s* must be rejected"
        );
        assert!(
            validate_bounded_width(r"a\s+b").is_err(),
            "interior \\s+ must be rejected"
        );
    }

    #[test]
    fn unbounded_nonwhitespace_is_still_rejected() {
        assert!(validate_bounded_width(r"\d+").is_err());
        assert!(validate_bounded_width(r".*").is_err());
        assert!(validate_bounded_width(r"[^.]*").is_err());
    }

    #[test]
    fn bounded_whitespace_and_text_are_accepted() {
        assert!(matches!(validate_bounded_width(r"\?\s{1,8}"), Ok(Some(_))));
        assert!(matches!(
            validate_bounded_width(r"foo\s{0,8}bar"),
            Ok(Some(_))
        ));
        assert!(matches!(
            validate_bounded_width(r"[^.\r\n]{0,120}"),
            Ok(Some(_))
        ));
        // A single, unquantified \s is bounded and fine.
        assert!(matches!(validate_bounded_width(r"a\sb"), Ok(Some(_))));
    }
}
