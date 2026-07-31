//! emphasis family structural rules: SLOP-E001 emphasis-staged contrast and
//! SLOP-E003 bold-label lists. Both use parser emphasis events, never
//! literal asterisks.

use crate::engine::{CompiledPolicy, Hit};
use crate::extract::Doc;
use crate::input::Prepared;
use crate::Config;

pub const HANDLED: &[&str] = &["SLOP-E001", "SLOP-E003"];

pub fn evaluate(
    cp: &CompiledPolicy,
    prepared: &Prepared,
    doc: &Doc,
    config: &Config,
    hits: &mut Vec<Hit>,
) {
    let src = prepared.text.as_str();
    if let Some(idx) = super::active(cp, config, "SLOP-E001") {
        let rule = &cp.pkg.rules[idx];
        let words: Vec<String> = rule
            .params
            .as_table()
            .and_then(|t| t.get("emphasized_words"))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let followers: Vec<String> = rule
            .params
            .as_table()
            .and_then(|t| t.get("followed_by"))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let within = super::param_i64(rule, "followed_within").unwrap_or(120) as usize;
        for (range, inner) in &doc.emphasis {
            let word = inner.trim().to_ascii_lowercase();
            if !words.iter().any(|w| w == &word) {
                continue;
            }
            let window_end = crate::widen_to_char_boundaries(
                src,
                range.end..(range.end + within).min(src.len()),
            )
            .end;
            let after = src[range.end..window_end].to_ascii_lowercase();
            let followed = followers.iter().any(|f| {
                let mut at = 0usize;
                while let Some(pos) = after[at..].find(f.as_str()) {
                    let s = at + pos;
                    let before_ok = s == 0
                        || !after[..s]
                            .chars()
                            .next_back()
                            .map(|c| c.is_ascii_alphanumeric())
                            .unwrap_or(false);
                    let e = s + f.len();
                    let after_ok = !after[e..]
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_alphanumeric())
                        .unwrap_or(false);
                    if before_ok && after_ok {
                        return true;
                    }
                    at = s + 1;
                }
                false
            });
            if followed {
                hits.push(Hit::new(idx, range.clone()));
            }
        }
    }

    if let Some(idx) = super::active(cp, config, "SLOP-E003") {
        let rule = &cp.pkg.rules[idx];
        let min = super::param_i64(rule, "list_items_with_leading_bold_label").unwrap_or(3) as u64;
        if doc.stats.bold_label_items >= min {
            let span = doc.bold_label_ranges.first().cloned().unwrap_or(0..0);
            hits.push(Hit::new(idx, span));
        }
    }
}
