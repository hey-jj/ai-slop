//! Section 12.14: density and structural threshold fixtures fire exactly on
//! the declared side of each boundary.

mod common;

use ai_slop::{analyze, Config, InputFormat, Profile};
use common::{has_rule, run};

fn filler_words(n: usize) -> String {
    (0..n)
        .map(|i| format!("w{i}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn lexicon_density_boundary_two_vs_three_per_500_words() {
    // Around 500 words with exactly 2 vs 3 counted lexicon hits.
    let base = filler_words(490);
    let two = format!("{base} delve tapestry end.");
    let report = run(&two, Profile::InternalDoc);
    assert!(!has_rule(&report, "SLOP-D001"), "2 hits fired D001");

    let three = format!("{base} delve tapestry testament end.");
    let report = run(&three, Profile::InternalDoc);
    assert!(has_rule(&report, "SLOP-D001"), "3 hits missed D001");
}

#[test]
fn opener_density_three_per_document() {
    let two = "Moreover, one.\n\nFurthermore, two.\n\nPlain third.\n";
    let report = run(two, Profile::InternalDoc);
    assert!(!has_rule(&report, "SLOP-D004"));

    let three = "Moreover, one.\n\nFurthermore, two.\n\nAdditionally, three.\n";
    let report = run(three, Profile::InternalDoc);
    assert!(has_rule(&report, "SLOP-D004"));
}

#[test]
fn bullet_heavy_commit_body_49_vs_51_pct() {
    let mut config = Config::new(Profile::CommitMessage);
    config.input_format = InputFormat::Commit;

    let body_49: String = (0..10)
        .map(|i| {
            if i < 4 {
                format!("- bullet {i}\n")
            } else {
                format!("line {i}\n")
            }
        })
        .collect();
    let commit = format!("feat: change\n\n{body_49}");
    let report = analyze(commit.as_bytes(), &config).unwrap();
    assert!(!has_rule(&report, "SLOP-X002"), "40% bullets fired X002");

    let body_60: String = (0..10)
        .map(|i| {
            if i < 6 {
                format!("- bullet {i}\n")
            } else {
                format!("line {i}\n")
            }
        })
        .collect();
    let commit = format!("feat: change\n\n{body_60}");
    let report = analyze(commit.as_bytes(), &config).unwrap();
    assert!(has_rule(&report, "SLOP-X002"), "60% bullets missed X002");
}

#[test]
fn over_structured_short_doc() {
    let doc = "# A\n\ntext\n\n## B\n\ntext\n\n## C\n\ntext\n\n## D\n\ntext\n";
    let report = run(doc, Profile::Readme);
    assert!(has_rule(&report, "SLOP-X004"));

    let long = format!(
        "# A\n\n{}\n\n## B\n\ntext\n\n## C\n\ntext\n\n## D\n\ntext\n",
        filler_words(400)
    );
    let report = run(&long, Profile::Readme);
    assert!(!has_rule(&report, "SLOP-X004"));
}

#[test]
fn boilerplate_skeleton_headings() {
    let doc = "# Report\n\n## Summary\n\nx\n\n## Key Changes\n\nx\n\n## Test Plan\n\nx\n";
    let report = run(doc, Profile::PublicBugReport);
    assert!(has_rule(&report, "SLOP-X001"));
    // Off outside public-bug-report and readme-relax.
    let report = run(doc, Profile::InternalDoc);
    assert!(!has_rule(&report, "SLOP-X001"));
}

#[test]
fn bold_label_list_needs_three_items() {
    let two = "- **Fast**: quick\n- **Safe**: sound\n";
    let report = run(two, Profile::Readme);
    assert!(!has_rule(&report, "SLOP-E003"));

    let three = "- **Fast**: quick\n- **Safe**: sound\n- **Clean**: neat\n";
    let report = run(three, Profile::Readme);
    assert!(has_rule(&report, "SLOP-E003"));
}

#[test]
fn verdict_heading_only_in_bug_report_profile() {
    let doc = "# Title\n\n## Impact\n\nbad things\n";
    let report = run(doc, Profile::PublicBugReport);
    assert!(has_rule(&report, "SLOP-S002"));
    let report = run(doc, Profile::Readme);
    assert!(!has_rule(&report, "SLOP-S002"));
}

#[test]
fn emphasis_staged_contrast() {
    let doc = "The parser *does* accept this input, but the writer rejects it.\n";
    let report = run(doc, Profile::Readme);
    assert!(has_rule(&report, "SLOP-E001"));

    let plain = "The parser *quickly* accepts this input, but slowly.\n";
    let report = run(plain, Profile::Readme);
    assert!(!has_rule(&report, "SLOP-E001"));
}
