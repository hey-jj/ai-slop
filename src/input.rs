//! Input contract: decoding, limits, BOM handling, commit-format split, and
//! manifest field extraction. Fail-closed boundary.

use crate::{AnalysisError, Config, InputFormat};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct CommitSplit {
    pub subject: Range<usize>,
    pub body: Range<usize>,
    pub trailers: Range<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct ManifestInfo {
    pub name: Option<String>,
    pub description: Option<String>,
    pub description_span: Option<Range<usize>>,
    /// Set when the description bytes in source equal the parsed value, so
    /// offsets can point inside the string literal.
    pub description_inner: Option<Range<usize>>,
    pub keywords: Option<(usize, Range<usize>)>,
    pub categories: Option<(usize, Range<usize>)>,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FormatData {
    Markdown,
    Text,
    Commit(CommitSplit),
    Manifest(ManifestInfo),
}

#[derive(Debug, Clone)]
pub struct Prepared {
    /// sha256 over the original bytes as received (pre BOM strip).
    pub sha256: String,
    pub original_len: usize,
    pub bom_stripped: bool,
    pub mixed_line_endings: bool,
    /// The post-BOM-strip payload every offset indexes.
    pub text: String,
    pub format: FormatData,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

pub fn prepare(input: &[u8], config: &Config) -> Result<Prepared, AnalysisError> {
    if input.len() > config.limits.max_bytes {
        return Err(AnalysisError::UnsupportedInput(format!(
            "input is {} bytes, over the {} byte limit",
            input.len(),
            config.limits.max_bytes
        )));
    }
    let sha256 = sha256_hex(input);
    let (payload, bom_stripped) = match input.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(rest) => (rest, true),
        None => (input, false),
    };
    let text = std::str::from_utf8(payload)
        .map_err(|e| {
            AnalysisError::UnsupportedInput(format!("invalid utf-8 at byte {}", e.valid_up_to()))
        })?
        .to_string();
    let has_crlf = text.contains("\r\n");
    let bare_lf = text
        .as_bytes()
        .iter()
        .enumerate()
        .any(|(i, &b)| b == b'\n' && (i == 0 || text.as_bytes()[i - 1] != b'\r'));
    let mixed_line_endings = has_crlf && bare_lf;

    let format = match config.input_format {
        InputFormat::Markdown => FormatData::Markdown,
        InputFormat::Text => FormatData::Text,
        InputFormat::Commit => FormatData::Commit(split_commit(&text)),
        InputFormat::Manifest => FormatData::Manifest(parse_manifest(&text)?),
    };

    Ok(Prepared {
        sha256,
        original_len: input.len(),
        bom_stripped,
        mixed_line_endings,
        text,
        format,
    })
}

/// Raw-Rust shape test for prose input, run by `analyze` AFTER extraction so
/// the prose/code split is the REAL extractor's segmentation: `code_blocks`
/// is the extractor's code-BLOCK region list, which covers backtick fences,
/// tilde `~~~` fences, and 4-space indented code blocks alike. A line that
/// overlaps any code block is code and never counts. A bug report with an
/// indented reproducer and a tilde-fenced Rust README both stay prose.
///
/// The guard reads Rust shape only. Source in other languages reaches the
/// prose rules and can produce findings unrelated to its writing. The guard
/// catches accidental source input. It provides no security boundary. A
/// writer who prefixes every line with a comment marker can pass it. This
/// comment records that limit.
///
/// Return `Some((rust_lines, nonblank_lines))` when at least
/// `RUST_GUARD_MIN_LINES` outside-code lines have Rust structure and reach
/// `RUST_GUARD_MIN_PCT` percent of nonblank outside-code lines.
const RUST_GUARD_MIN_LINES: usize = 8;
const RUST_GUARD_MIN_PCT: usize = 35;

pub fn rust_source_shape(text: &str, code_blocks: &[Range<usize>]) -> Option<(usize, usize)> {
    let (rust_lines, nonblank) = rust_line_counts(text, code_blocks);
    if rust_lines >= RUST_GUARD_MIN_LINES && rust_lines * 100 >= RUST_GUARD_MIN_PCT * nonblank {
        Some((rust_lines, nonblank))
    } else {
        None
    }
}

/// The raw counts behind `rust_source_shape`: lines carrying Rust structure,
/// and non-blank lines outside code blocks. Public so a measurement can score
/// a document without asking whether it crosses the thresholds.
pub fn rust_line_counts(text: &str, code_blocks: &[Range<usize>]) -> (usize, usize) {
    let mut rust_lines = 0usize;
    let mut nonblank = 0usize;
    for lr in line_ranges(text) {
        let t = text[lr.clone()].trim();
        if t.is_empty() {
            continue;
        }
        if code_blocks
            .iter()
            .any(|c| c.start < lr.end && lr.start < c.end)
        {
            continue;
        }
        nonblank += 1;
        if rust_shaped_line(t) {
            rust_lines += 1;
        }
    }
    (rust_lines, nonblank)
}

/// One trimmed line's Rust-shape test, in two arms.
///
/// Arm 1 needs no terminator, because the shape carries itself: an attribute
/// opener, a comment opener of any depth, or a line made only of structural
/// punctuation. In markdown that shape lives inside a fence, which
/// segmentation already excludes, so counting plain `//` costs no prose and
/// closes most of the comment-prefix evasion on the way past.
///
/// Arm 2 needs both halves. The line ends on a code terminator and either
/// opens on an item or binding keyword, optionally behind a visibility or
/// modifier word, or carries a path, arrow, or fat-arrow token, or has the
/// field-line shape. Requiring both halves is what keeps prose out: a
/// sentence opening on `use` or `type` ends on a period, and a sentence
/// ending on a semicolon opens on neither a keyword nor a path.
fn rust_shaped_line(t: &str) -> bool {
    // Arm 1.
    if t.starts_with("#[") || t.starts_with("#![") || t.starts_with("//") {
        return true;
    }
    if t.chars()
        .all(|c| matches!(c, '{' | '}' | '(' | ')' | '[' | ']' | ';' | ','))
    {
        return true;
    }

    // Arm 2, first half: a code terminator at the line end.
    if !t.ends_with(['{', '}', ';', '(', ')', ',', ']']) {
        return false;
    }
    // Arm 2, second half: a keyword opener, a code token, or a field line.
    const KEYWORDS: &[&str] = &[
        "fn", "struct", "enum", "impl", "trait", "mod", "use", "const", "static", "type", "let",
        "match", "extern",
    ];
    const MODIFIERS: &[&str] = &["pub", "pub(crate)", "async", "unsafe"];
    let mut words = t.split_whitespace();
    let mut head = words.next().unwrap_or("");
    if MODIFIERS.contains(&head) {
        head = words.next().unwrap_or("");
    }
    let head_word: &str = head
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("");
    KEYWORDS.contains(&head_word)
        || t.contains("::")
        || t.contains("->")
        || t.contains("=>")
        || field_line(t)
}

/// The field-line shape, kept tight on purpose: one identifier, a colon, one
/// type expression carrying no sentence structure, then a comma, optionally
/// behind `pub` or `pub(crate)`. A definition list writes several words after
/// its colon, so `- name: the person who signed,` never matches, and a bare
/// element line (a lone token and a comma) is not this shape and is not
/// counted at all.
fn field_line(t: &str) -> bool {
    let Some(body) = t.strip_suffix(',') else {
        return false;
    };
    let body = body
        .strip_prefix("pub(crate) ")
        .or_else(|| body.strip_prefix("pub "))
        .unwrap_or(body)
        .trim();
    let Some((name, ty)) = body.split_once(':') else {
        return false;
    };
    // A path separator means the colon was not the field colon.
    if name.ends_with(':') || ty.starts_with(':') {
        return false;
    }
    let name = name.trim();
    if name.is_empty()
        || !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || !name
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
    {
        return false;
    }
    // One type expression: an identifier, a path, a reference, or a generic,
    // written as a single whitespace-free token once references and generics
    // are allowed their own spaces.
    let ty = ty.trim();
    if ty.is_empty() {
        return false;
    }
    let compact: String = ty.chars().filter(|c| !c.is_whitespace()).collect();
    let spaced_words = ty.split_whitespace().count();
    // `&'a str` and `Vec<T, A>` keep their spaces, so a second word counts
    // only when the type carries generic or reference syntax.
    let syntactic = compact.contains(['<', '&', ':']);
    if spaced_words > 1 && !syntactic {
        return false;
    }
    compact.chars().all(|c| {
        c.is_alphanumeric()
            || matches!(
                c,
                '_' | ':' | '<' | '>' | '&' | '\'' | ',' | '[' | ']' | '(' | ')'
            )
    }) && compact
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || matches!(c, '_' | '&' | '(' | '['))
}

/// Byte range of each line, excluding the line terminator.
pub fn line_ranges(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let bytes = text.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            let mut end = i;
            if end > start && bytes[end - 1] == b'\r' {
                end -= 1;
            }
            out.push(start..end);
            start = i + 1;
        }
    }
    if start <= text.len() {
        out.push(start..text.len());
    }
    out
}

fn is_trailer_line(line: &str) -> bool {
    let Some(colon) = line.find(':') else {
        return false;
    };
    let key = &line[..colon];
    if key.is_empty() {
        return false;
    }
    key.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && line[colon + 1..].starts_with(' ')
}

fn split_commit(text: &str) -> CommitSplit {
    let lines = line_ranges(text);
    let subject = lines.first().cloned().unwrap_or(0..0);
    let body_start_line = if lines.len() > 1 { 1 } else { lines.len() };

    // Trailer block: trailing run of Key: value lines separated from the
    // body by a blank line, or forming the whole tail of the message.
    let mut trailer_first = lines.len();
    let mut i = lines.len();
    while i > body_start_line {
        let l = &text[lines[i - 1].clone()];
        if l.trim().is_empty() {
            if trailer_first < lines.len() {
                break;
            }
            i -= 1;
            continue;
        }
        if is_trailer_line(l) {
            trailer_first = i - 1;
            i -= 1;
        } else {
            break;
        }
    }
    let (body, trailers) = if trailer_first < lines.len() {
        let t_start = lines[trailer_first].start;
        let b_start = lines
            .get(body_start_line)
            .map(|r| r.start)
            .unwrap_or(text.len());
        (b_start..t_start.min(text.len()), t_start..text.len())
    } else {
        let b_start = lines
            .get(body_start_line)
            .map(|r| r.start)
            .unwrap_or(text.len());
        (b_start..text.len(), text.len()..text.len())
    };
    CommitSplit {
        subject,
        body,
        trailers,
    }
}

#[derive(Deserialize)]
struct ManifestDoc {
    package: Option<PackageTbl>,
}

#[derive(Deserialize)]
struct PackageTbl {
    name: Option<String>,
    version: Option<toml::Value>,
    description: Option<toml::Spanned<String>>,
    keywords: Option<toml::Spanned<Vec<String>>>,
    categories: Option<toml::Spanned<Vec<String>>>,
}

fn parse_manifest(text: &str) -> Result<ManifestInfo, AnalysisError> {
    let doc: ManifestDoc = toml::from_str(text)
        .map_err(|e| AnalysisError::UnsupportedInput(format!("manifest parse: {e}")))?;
    let Some(pkg) = doc.package else {
        return Err(AnalysisError::UnsupportedInput(
            "manifest has no [package] table".to_string(),
        ));
    };
    let mut info = ManifestInfo {
        name: pkg.name,
        version: pkg.version.and_then(|v| v.as_str().map(|s| s.to_string())),
        ..Default::default()
    };
    if let Some(desc) = pkg.description {
        let span = desc.span();
        let value = desc.into_inner();
        // If the literal bytes inside the quotes equal the parsed value the
        // description can be analyzed in place with exact offsets.
        let inner = span.start + 1..span.end.saturating_sub(1);
        if inner.start <= inner.end
            && inner.end <= text.len()
            && text.get(inner.clone()) == Some(value.as_str())
        {
            info.description_inner = Some(inner);
        }
        info.description_span = Some(span.start..span.end);
        info.description = Some(value);
    }
    if let Some(kw) = pkg.keywords {
        let span = kw.span();
        info.keywords = Some((kw.into_inner().len(), span.start..span.end));
    }
    if let Some(cat) = pkg.categories {
        let span = cat.span();
        info.categories = Some((cat.into_inner().len(), span.start..span.end));
    }
    Ok(info)
}
