//! Acceptance conditions for the raw-source input guard, measured with the
//! shipped line test so the numbers cannot drift from the code. Every score
//! is taken after extraction, so the code-block exclusion is the real one.

use ai_slop::{extract, input, Config, InputFormat, Profile};
use std::ops::Range;

/// Percent of non-blank outside-code lines that carry Rust structure.
fn score(bytes: &[u8], profile: Profile) -> (usize, usize, usize) {
    let mut config = Config::new(profile);
    config.input_format = InputFormat::Markdown;
    let prepared = input::prepare(bytes, &config).expect("prepare");
    let doc = extract::build_doc(&prepared, &config).expect("extract");
    let code_blocks: Vec<Range<usize>> = doc
        .regions
        .iter()
        .filter(|r| r.kind == extract::RegionKind::CodeBlock)
        .map(|r| r.range.clone())
        .collect();
    let (code, nonblank) = input::rust_line_counts(&prepared.text, &code_blocks);
    let pct = (code * 100).checked_div(nonblank).unwrap_or(0);
    (code, nonblank, pct)
}

fn repo(rel: &str) -> Vec<u8> {
    std::fs::read(format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"))).expect("repo file")
}

fn source_files() -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![format!("{}/src", env!("CARGO_MANIFEST_DIR"))];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path.to_string_lossy().to_string());
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path.to_string_lossy().to_string());
            }
        }
    }
    out.sort();
    out
}

/// Condition 1: every Rust source file in the tree reads as source.
#[test]
fn guard_fires_on_every_source_file() {
    let files = source_files();
    assert!(files.len() >= 20, "expected the whole src tree: {files:?}");
    let mut lowest = 100usize;
    for path in &files {
        let bytes = std::fs::read(path).expect("source file");
        let (code, nonblank, pct) = score(&bytes, Profile::ApiDocs);
        println!("source {path}: {code}/{nonblank} = {pct}%");
        lowest = lowest.min(pct);
        let mut config = Config::new(Profile::ApiDocs);
        config.input_format = InputFormat::Markdown;
        let prepared = input::prepare(&bytes, &config).expect("prepare");
        let doc = extract::build_doc(&prepared, &config).expect("extract");
        let code_blocks: Vec<Range<usize>> = doc
            .regions
            .iter()
            .filter(|r| r.kind == extract::RegionKind::CodeBlock)
            .map(|r| r.range.clone())
            .collect();
        assert!(
            input::rust_source_shape(&prepared.text, &code_blocks).is_some(),
            "{path} passed the guard at {pct}%"
        );
    }
    println!("lowest source score: {lowest}%");
}

/// Condition 2: the prose the project ships contributes no source lines, and
/// condition 3: the separation between the lowest source score and the
/// highest prose score clears twenty points.
#[test]
fn prose_surfaces_score_zero_and_stay_clear_of_source() {
    let mut highest_prose = 0usize;
    for (rel, profile) in [
        ("README.md", Profile::Readme),
        ("CHANGELOG.md", Profile::Changelog),
        ("skills/ai-slop/SKILL.md", Profile::InternalDoc),
        ("skills/ai-slop/references/rules.md", Profile::InternalDoc),
    ] {
        let (code, nonblank, pct) = score(&repo(rel), profile);
        println!("prose {rel}: {code}/{nonblank} = {pct}%");
        highest_prose = highest_prose.max(pct);
        assert_eq!(code, 0, "{rel} contributed source lines");
    }

    let lowest_source = source_files()
        .iter()
        .map(|p| score(&std::fs::read(p).expect("source file"), Profile::ApiDocs).2)
        .min()
        .expect("source files");
    println!("separation: {lowest_source}% source vs {highest_prose}% prose");
    assert!(
        lowest_source >= highest_prose + 20,
        "separation is {lowest_source} against {highest_prose}"
    );
}

/// A definition list writes several words after its colon, so the field-line
/// shape never counts it, bullet markers included.
#[test]
fn definition_lists_are_not_field_lines() {
    let doc = "\
- name: the person who signed the waiver,\n\
- reason: the sentence a reader would accept,\n\
- expiry: the date the authority approved,\n\
reason: the sentence a reader would accept,\n\
signer: the human who owns the record,\n";
    let (code, _, _) = score(doc.as_bytes(), Profile::InternalDoc);
    assert_eq!(code, 0, "a definition list read as field lines");
}
