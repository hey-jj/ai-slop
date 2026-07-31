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
