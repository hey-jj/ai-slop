//! Profile public-comment: the public-bug-report outbound discipline minus
//! the SLOP-K001 title contract. A PR or issue comment has no title, so the
//! title rule stays off while the size, structure, and lexicon rules keep
//! the public-bug-report stances.

mod common;

use ai_slop::Profile;
use common::{has_rule, run};

fn filler_words(n: usize) -> String {
    (0..n)
        .map(|i| format!("w{i}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A first line over 80 characters containing a forbidden title word: the
/// motivating misfire. K001 fires under public-bug-report and must not fire
/// under public-comment, which treats the same text only as a comment body.
const LONG_CRITICAL_FIRST_LINE: &str = "The parser treats the critical guard \
as optional and it drops the lock before the queue drain step finishes.\n\n\
The drain then races the producer.\n";

#[test]
fn oversized_comment_fires_x003_over_600_words() {
    let doc = format!("{}\n", filler_words(650));
    let report = run(&doc, Profile::PublicComment);
    assert!(has_rule(&report, "SLOP-X003"), "650 words missed X003");
}

#[test]
fn code_blocks_do_not_count_toward_the_x003_word_budget() {
    // 500 prose words plus a fenced block far larger than the budget. The
    // fence is excluded from the prose word count, so X003 stays quiet.
    let doc = format!(
        "{}\n\n```text\n{}\n```\n",
        filler_words(500),
        filler_words(900)
    );
    let report = run(&doc, Profile::PublicComment);
    assert!(
        !has_rule(&report, "SLOP-X003"),
        "fenced code counted toward the prose budget"
    );
}

#[test]
fn k001_stays_off_for_a_comment_and_fires_for_a_bug_report() {
    let report = run(LONG_CRITICAL_FIRST_LINE, Profile::PublicComment);
    assert!(
        !has_rule(&report, "SLOP-K001"),
        "K001 fired on a comment first line"
    );

    let report = run(LONG_CRITICAL_FIRST_LINE, Profile::PublicBugReport);
    assert!(
        has_rule(&report, "SLOP-K001"),
        "K001 missed the same text as a bug-report title"
    );
}

#[test]
fn over_structured_short_comment_fires_x004_with_no_exemption_set() {
    // Under 300 words with four headings. public-bug-report has an exempt
    // heading set for X004. Public-comment declares none, so the generic
    // structure rule applies as-is.
    let doc = "# A\n\ntext\n\n## B\n\ntext\n\n## C\n\ntext\n\n## D\n\ntext\n";
    let report = run(doc, Profile::PublicComment);
    assert!(has_rule(&report, "SLOP-X004"), "4 headings missed X004");
}
