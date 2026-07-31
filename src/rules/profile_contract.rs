//! profile-contract family: per-profile mechanical format rules
//! SLOP-K001 through SLOP-K007. SLOP-K008 is a word-set rule served by the
//! engine.

use crate::engine::{CompiledPolicy, Hit};
use crate::extract::Doc;
use crate::input::{line_ranges, FormatData, Prepared};
use crate::Config;
use std::ops::Range;

pub const HANDLED: &[&str] = &[
    "SLOP-K001",
    "SLOP-K002",
    "SLOP-K003",
    "SLOP-K004",
    "SLOP-K005",
    "SLOP-K006",
    "SLOP-K007",
];

fn word_hit_in(hay: &str, base: usize, word: &str) -> Option<Range<usize>> {
    let lower = hay.to_ascii_lowercase();
    let mut at = 0usize;
    while let Some(pos) = lower[at..].find(word) {
        let s = at + pos;
        let e = s + word.len();
        let before_ok = !lower[..s]
            .chars()
            .next_back()
            .map(unicode_ident::is_xid_continue)
            .unwrap_or(false);
        let after_ok = !lower[e..]
            .chars()
            .next()
            .map(unicode_ident::is_xid_continue)
            .unwrap_or(false);
        // Identifiers containing forbidden substrings do not fire; a hyphen
        // joined token like `non-critical-path` is treated as an identifier.
        let hyphenated = lower[..s].ends_with('-') || lower[e..].starts_with('-');
        if before_ok && after_ok && !hyphenated {
            return Some(base + s..base + e);
        }
        at = e;
    }
    None
}

pub fn evaluate(
    cp: &CompiledPolicy,
    prepared: &Prepared,
    doc: &Doc,
    config: &Config,
    hits: &mut Vec<Hit>,
) {
    let src = prepared.text.as_str();

    // K001: bug-report title.
    if let Some(idx) = super::active(cp, config, "SLOP-K001") {
        let rule = &cp.pkg.rules[idx];
        let (title_range, title) = match doc.headings.first() {
            Some(h) => (h.text_range.clone(), h.text.clone()),
            None => {
                let first = line_ranges(src).into_iter().next().unwrap_or(0..0);
                let text = src[first.clone()].to_string();
                (first, text)
            }
        };
        let max_chars = super::param_i64(rule, "max_title_chars").unwrap_or(80) as usize;
        if title.chars().count() > max_chars {
            hits.push(Hit::new(idx, title_range.clone()));
        }
        let forbidden: Vec<String> = rule
            .params
            .as_table()
            .and_then(|t| t.get("forbid_title_words"))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        for word in &forbidden {
            if let Some(span) = word_hit_in(&src[title_range.clone()], title_range.start, word) {
                hits.push(Hit::new(idx, span));
            }
        }
    }

    // K002: commit subject format.
    if let Some(idx) = super::active(cp, config, "SLOP-K002") {
        if let FormatData::Commit(split) = &prepared.format {
            let rule = &cp.pkg.rules[idx];
            let subject = &src[split.subject.clone()];
            let max_chars = super::param_i64(rule, "max_subject_chars").unwrap_or(72) as usize;
            if subject.chars().count() > max_chars {
                hits.push(Hit::new(idx, split.subject.clone()));
            }
            if subject.trim_end().ends_with('.') {
                hits.push(Hit::new(idx, split.subject.clone()));
            }
            let after_prefix = conventional_prefix(subject);
            match after_prefix {
                None => {
                    hits.push(Hit::new(idx, split.subject.clone()));
                }
                Some(rest_at) => {
                    // Imperative heuristic on the first word: reports
                    // candidate, never violation.
                    let rest = &subject[rest_at..];
                    let first = rest.split_whitespace().next().unwrap_or("");
                    let lower = first.to_ascii_lowercase();
                    let non_imperative = lower.ends_with("ed")
                        || (lower.ends_with("ing") && lower.len() > 4)
                        || (lower.ends_with('s') && !lower.ends_with("ss") && lower.len() > 3);
                    if first
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false)
                        || non_imperative
                    {
                        let s = split.subject.start + rest_at;
                        let mut h = Hit::new(idx, s..s + first.len());
                        h.force_candidate = true;
                        hits.push(h);
                    }
                }
            }
        }
    }

    // K003: changelog structure.
    if let Some(idx) = super::active(cp, config, "SLOP-K003") {
        let rule = &cp.pkg.rules[idx];
        let whitelist: Vec<String> = rule
            .params
            .as_table()
            .and_then(|t| t.get("section_whitelist"))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        for h in doc.headings.iter().filter(|h| h.level >= 2) {
            let name = h.text.to_ascii_lowercase();
            let version_like = name.chars().any(|c| c.is_ascii_digit())
                && (name.contains('.') || name.contains('v') || name.contains('['));
            if whitelist.iter().any(|w| &name == w) {
                continue;
            }
            if version_like {
                if !has_iso_date(&name) {
                    hits.push(Hit::new(idx, h.text_range.clone()));
                }
                continue;
            }
            hits.push(Hit::new(idx, h.text_range.clone()));
        }
    }

    // K004: release-notes body equals the configured changelog entry.
    if let Some(idx) = super::active(cp, config, "SLOP-K004") {
        match &config.deployment.expected_release_body {
            Some(expected) => {
                let got: Vec<&str> = src
                    .lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .collect();
                let want: Vec<&str> = expected
                    .lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .collect();
                let max_extra = super::param_i64(&cp.pkg.rules[idx], "max_extra_pointer_lines")
                    .unwrap_or(1) as usize;
                let ok = got == want
                    || (got.len() <= want.len() + max_extra
                        && want.iter().all(|l| got.contains(l)));
                if !ok {
                    hits.push(Hit::new(idx, 0..src.len().min(1)));
                }
            }
            None => {
                let mut h = Hit::new(idx, 0..0);
                h.force_hint = true;
                hits.push(h);
            }
        }
    }

    // K005: readme License section.
    if let Some(idx) = super::active(cp, config, "SLOP-K005") {
        let license_heading = doc
            .headings
            .iter()
            .enumerate()
            .find(|(_, h)| h.text.eq_ignore_ascii_case("license"));
        match license_heading {
            None => {
                hits.push(Hit::new(idx, 0..0));
            }
            Some((i, h)) => match &config.deployment.expected_license_wording {
                Some(expected) => {
                    let section = doc
                        .sections
                        .get(i)
                        .map(|s| s.range.clone())
                        .unwrap_or(h.range.start..src.len());
                    let body = collapse_ws(&src[section.clone()]);
                    if !body.contains(&collapse_ws(expected)) {
                        hits.push(Hit::new(idx, h.range.clone()));
                    }
                }
                None => {
                    let mut hint = Hit::new(idx, h.range.clone());
                    hint.force_hint = true;
                    hits.push(hint);
                }
            },
        }
    }

    // K006: cargo-metadata fields.
    if let Some(idx) = super::active(cp, config, "SLOP-K006") {
        if let FormatData::Manifest(info) = &prepared.format {
            let rule = &cp.pkg.rules[idx];
            let max_desc = super::param_i64(rule, "max_description_chars").unwrap_or(160) as usize;
            let kw_exact = super::param_i64(rule, "keywords_exact").unwrap_or(5) as usize;
            let cat_min = super::param_i64(rule, "categories_min").unwrap_or(2) as usize;
            let cat_max = super::param_i64(rule, "categories_max").unwrap_or(3) as usize;
            match (&info.description, &info.description_span) {
                (Some(d), Some(span)) => {
                    if d.chars().count() > max_desc {
                        hits.push(Hit::new(idx, span.clone()));
                    }
                    if let Some(name) = &info.name {
                        if d.to_ascii_lowercase()
                            .starts_with(&name.to_ascii_lowercase())
                        {
                            hits.push(Hit::new(idx, span.clone()));
                        }
                    }
                }
                _ => hits.push(Hit::new(idx, 0..0)),
            }
            match &info.keywords {
                Some((n, span)) if *n != kw_exact => hits.push(Hit::new(idx, span.clone())),
                None => hits.push(Hit::new(idx, 0..0)),
                _ => {}
            }
            match &info.categories {
                Some((n, span)) if *n < cat_min || *n > cat_max => {
                    hits.push(Hit::new(idx, span.clone()))
                }
                None => hits.push(Hit::new(idx, 0..0)),
                _ => {}
            }
        }
    }

    // K007: readme version agreement.
    if let Some(idx) = super::active(cp, config, "SLOP-K007") {
        let mut found: Vec<(Range<usize>, String)> = Vec::new();
        for code in &doc.code_regions {
            let body = &src[code.range.clone()];
            let mut at = 0usize;
            while let Some(pos) = body[at..].find("= \"") {
                let vstart = at + pos + 3;
                if let Some(close) = body[vstart..].find('"') {
                    let val = &body[vstart..vstart + close];
                    if !val.is_empty()
                        && val.chars().all(|c| c.is_ascii_digit() || c == '.')
                        && val.contains('.')
                    {
                        found.push((
                            code.range.start + vstart..code.range.start + vstart + close,
                            val.to_string(),
                        ));
                    }
                    at = vstart + close + 1;
                } else {
                    break;
                }
            }
        }
        match &config.deployment.expected_version {
            Some(expected) => {
                for (span, val) in &found {
                    if val != expected && !val.starts_with(expected.as_str()) {
                        hits.push(Hit::new(idx, span.clone()));
                    }
                }
            }
            None => {
                if let Some((span, _)) = found.first() {
                    let mut h = Hit::new(idx, span.clone());
                    h.force_hint = true;
                    hits.push(h);
                }
            }
        }
    }
}

fn conventional_prefix(subject: &str) -> Option<usize> {
    let colon = subject.find(':')?;
    let head = &subject[..colon];
    let (ty, scope_ok) = match head.find('(') {
        Some(open) => {
            let close = head.rfind(')')?;
            if close + 1 != head.len() && !(head.ends_with('!') && close + 2 == head.len()) {
                return None;
            }
            (&head[..open], close > open + 1)
        }
        None => (head.strip_suffix('!').unwrap_or(head), true),
    };
    let ty = ty.strip_suffix('!').unwrap_or(ty);
    if ty.is_empty() || !ty.chars().all(|c| c.is_ascii_lowercase()) || !scope_ok {
        return None;
    }
    if !subject[colon + 1..].starts_with(' ') {
        return None;
    }
    Some(colon + 2)
}

fn has_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 10 {
        return false;
    }
    for i in 0..=b.len() - 10 {
        let w = &b[i..i + 10];
        if w[0].is_ascii_digit()
            && w[1].is_ascii_digit()
            && w[2].is_ascii_digit()
            && w[3].is_ascii_digit()
            && w[4] == b'-'
            && w[5].is_ascii_digit()
            && w[6].is_ascii_digit()
            && w[7] == b'-'
            && w[8].is_ascii_digit()
            && w[9].is_ascii_digit()
        {
            return true;
        }
    }
    false
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
