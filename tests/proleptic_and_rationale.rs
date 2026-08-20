//! SLOP-C010 proleptic capability denial and SLOP-F004 rationale leak: the
//! two arms, the imperative exclusion, the seam between the two rules, and
//! the anchors that keep ordinary prose out.

mod common;

use ai_slop::Profile;
use common::{assert_invariants, has_rule, run};

fn hits<'a>(report: &'a ai_slop::Report, id: &str) -> Vec<&'a ai_slop::Finding> {
    report.findings.iter().filter(|f| f.rule_id == id).collect()
}

fn detail(report: &ai_slop::Report, id: &str) -> String {
    report
        .findings
        .iter()
        .find(|f| f.rule_id == id)
        .map(|f| f.message.clone())
        .unwrap_or_default()
}

// --- SLOP-C010 ---------------------------------------------------------------

/// The density arm: a denial and a hedge sharing one block. This is the
/// owner's specimen.
#[test]
fn c010_two_clauses_in_one_block_report() {
    let t = "It reads text. It does not detect authorship, and no finding is evidence that a person or a model wrote anything.\n";
    let report = run(t, Profile::Readme);
    assert_invariants(t, &report);
    assert!(
        has_rule(&report, "SLOP-C010"),
        "the denial stack stayed silent: {:?}",
        common::rule_ids(&report)
    );
    // One finding per qualifying clause, each carrying the block count.
    let found = hits(&report, "SLOP-C010");
    assert_eq!(found.len(), 2, "one finding per qualifying clause");
    for f in &found {
        assert!(
            f.message.contains("arm A, one of 2 denied capabilities"),
            "each finding names its arm and the count: {}",
            f.message
        );
    }
    // A denial with a subject reports its segment; a hedge reports the whole
    // comma-delimited clause. Both carry content only: no coordinator at the
    // front, no comma or terminal stop at the back. The byte offsets are
    // pinned because the span is what a reader is asked to rewrite.
    let spans: Vec<String> = found.iter().map(|f| common::snippet(f)).collect();
    assert_eq!(spans[0], "It does not detect authorship", "segment span");
    assert_eq!(
        spans[1], "no finding is evidence that a person or a model wrote anything",
        "clause span for the open complement"
    );
    assert_eq!(
        (found[0].spans[0].start, found[0].spans[0].end),
        (15, 44),
        "segment span bytes"
    );
    assert_eq!(
        (found[1].spans[0].start, found[1].spans[0].end),
        (50, 112),
        "clause span bytes"
    );
}

/// Every span this rule reports carries clause content and no delimiter at
/// either end, whether the clause closes on a comma or on the sentence's
/// terminal stop.
#[test]
fn c010_spans_exclude_every_delimiter() {
    for t in [
        "It reads text. Never scores voice.\n",
        "The tools read text. The tool does not detect authorship.\n",
        "It does not detect authorship, never scores voice, and makes no claim about intent.\n",
        "It reads prose. None of the rules judges voice!\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        let found = hits(&report, "SLOP-C010");
        assert!(!found.is_empty(), "no finding to check: {t:?}");
        for f in found {
            let span = common::snippet(f);
            let first = span.chars().next().expect("non-empty span");
            let last = span.chars().next_back().expect("non-empty span");
            assert!(
                !first.is_whitespace() && !matches!(first, ',' | ';' | '.' | '!' | '?'),
                "span opens on a delimiter: {span:?}"
            );
            assert!(
                !last.is_whitespace() && !matches!(last, ',' | ';' | '.' | '!' | '?'),
                "span closes on a delimiter: {span:?}"
            );
        }
    }
}

/// The adjacency arm: one denial beside a sentence describing the same
/// subject affirmatively.
#[test]
fn c010_single_clause_beside_an_affirmative_sentence_reports() {
    let t = "It scans all regions including code and comments. It never scores authorship.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    assert!(
        has_rule(&report, "SLOP-C010"),
        "the adjacency arm stayed silent: {:?}",
        common::rule_ids(&report)
    );
}

/// Family 1, spelling B: a negative subject opening the clause is its own
/// negation, and the capability verb follows inside the window.
#[test]
fn c010_negative_subject_spelling_reports() {
    for t in [
        "It reads text. No rule scores voice.\n",
        "It reads prose. None of the rules judges voice.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            has_rule(&report, "SLOP-C010"),
            "the negative-subject spelling stayed silent: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// Coreference is a bare closed-set pronoun on either side, or the same
/// tool-noun lemma on both. A noun and its plural are one lemma. Two
/// different nouns are two different things, so the cross-noun pair goes
/// silent and needs the pronoun form or the stack arm instead.
#[test]
fn c010_arm_b_coreference_needs_a_pronoun_or_the_same_lemma() {
    for t in [
        "The tool reads text. It does not detect authorship.\n",
        "The rules read text. The rule does not detect authorship.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            has_rule(&report, "SLOP-C010"),
            "coreference lost: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
    for t in [
        "The linter reads text. The tool does not detect authorship.\n",
        "The test finished at noon. The tool does not detect authorship.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            !has_rule(&report, "SLOP-C010"),
            "two different nouns coreferred: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// Every capability-verb form qualifies in the subject-bearing spellings,
/// the participle included. Only the subjectless spelling excludes it.
#[test]
fn c010_subject_spellings_take_every_verb_form() {
    for t in [
        "This reads text. This is not accurately judging writers.\n",
        "Nothing here is measuring intent. Nothing now is judging authors.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            has_rule(&report, "SLOP-C010"),
            "a participle was refused: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// A product name opens a sentence whatever its case. The shared terminal
/// test reads a lowercase follower as a continuation, which is right for
/// prose and wrong for a name that is spelled lowercase.
#[test]
fn c010_lowercase_product_names_qualify() {
    let t = "unslop does not detect authorship. unslop never judges writers.\n";
    let report = run(t, Profile::Readme);
    assert_invariants(t, &report);
    assert_eq!(
        hits(&report, "SLOP-C010").len(),
        2,
        "both denials must report: {:?}",
        common::rule_ids(&report)
    );
}

/// The open hedge spans one or two wildcard tokens, and the closed-set test
/// reads the head noun, the last of them before the copula. That closes the
/// one-adjective evasion without catching a sentence about samples.
#[test]
fn c010_open_hedges_read_their_head_noun() {
    for t in [
        "It reads text. No single finding is evidence of intent.\n",
        "It reads text. No finding is evidence of intent.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            has_rule(&report, "SLOP-C010"),
            "a tool-noun head was missed: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
    for t in [
        "The study measured cortisol. No single sample is evidence of chronic stress.\n",
        "It reads text. No banana is evidence of intent.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            !has_rule(&report, "SLOP-C010"),
            "a foreign head reached the adjacency arm: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// A product name is a referent in its own right. Two sides match on the
/// same name, and a name paired with a tool noun does not, which is the
/// ruled miss.
#[test]
fn c010_product_names_corefer_only_with_themselves() {
    let same = "ai-slop reads text. ai-slop does not detect authorship.\n";
    let report = run(same, Profile::Readme);
    assert_invariants(same, &report);
    assert!(
        has_rule(&report, "SLOP-C010"),
        "a product name failed to corefer with itself: {:?}",
        common::rule_ids(&report)
    );

    let mixed = "ai-slop reads text. The tool does not detect authorship.\n";
    let report = run(mixed, Profile::Readme);
    assert_invariants(mixed, &report);
    assert!(
        !has_rule(&report, "SLOP-C010"),
        "a name coreferred with a tool noun: {:?}",
        common::rule_ids(&report)
    );
}

/// A family-2 clause needs its own closed-set subject before the adjacency
/// arm can test coreference. Without one it counts toward the stack arm and
/// nothing else. The requirement is family 2's alone: the subjectless
/// spelling stays subjectless.
#[test]
fn c010_foreign_subject_hedges_reach_arm_a_only() {
    let alone = "It reads text. No banana is evidence of intent.\n";
    let report = run(alone, Profile::Readme);
    assert_invariants(alone, &report);
    assert!(
        !has_rule(&report, "SLOP-C010"),
        "a foreign-subject hedge reached the adjacency arm: {:?}",
        common::rule_ids(&report)
    );

    let stacked = "It does not detect authorship, and no banana is evidence of intent.\n";
    let report = run(stacked, Profile::Readme);
    assert_invariants(stacked, &report);
    assert_eq!(
        hits(&report, "SLOP-C010").len(),
        2,
        "the stack arm must count the foreign-subject hedge: {:?}",
        common::rule_ids(&report)
    );

    // D3's requirement is family 2's alone.
    let elided = "It reads text. Never scores voice.\n";
    let report = run(elided, Profile::Readme);
    assert!(
        has_rule(&report, "SLOP-C010"),
        "the subjectless spelling lost its arm B: {:?}",
        common::rule_ids(&report)
    );
}

/// A capability verb is required. Denying a function is an honest scope
/// fact, and an adjectival denial carries no verb at all. Both stay for the
/// reread. `that` left the subject set, because as a relativizer it collides
/// with ordinary prose.
#[test]
fn c010_denials_without_a_capability_verb_stay_silent() {
    for t in [
        "It scans all regions including code and comments. It is never demotable or agent-waivable.\n",
        "The tool reads text. The guard never fires on irregularity.\n",
        "The linter reads text. That does not detect authorship.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            !has_rule(&report, "SLOP-C010"),
            "a stated miss reported: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// The crate's own shipped prose, which the unamended clause spec reported.
/// These sentences are the reason family 1 needs the capability verb.
#[test]
fn c010_stays_off_the_crates_own_prose() {
    for t in [
        "Exit 0 means the check completed with nothing blocking. Before you trust it, read the coverage block.\n",
        "A green check means the rules found nothing, and these classes are what the rules cannot find.\n",
        "This section primes the reread: these classes are what the rules cannot find.\n",
        "A value that is not a table stops the load.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            !has_rule(&report, "SLOP-C010"),
            "the crate's own prose reported: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// The general partner test is what Arm B uses. A restatement of what the
/// thing does is one affirmative sentence among others, with no special
/// matcher behind it, so a trailing adverb or a question mark changes
/// nothing.
#[test]
fn c010_any_affirmative_sentence_partners_the_denial() {
    for t in [
        "The linter reads text. It does not detect authorship.\n",
        "The linter reads text carefully. It does not detect authorship.\n",
        "It reads text every morning. It does not detect authorship.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            detail(&report, "SLOP-C010").contains("affirmative"),
            "the affirmative partner was missed: {t:?} {}",
            detail(&report, "SLOP-C010")
        );
    }

    // An affirmative sentence on its own reports nothing.
    let alone = "The linter reads text.\n";
    let report = run(alone, Profile::Readme);
    assert!(
        !has_rule(&report, "SLOP-C010"),
        "an affirmative sentence reported on its own: {:?}",
        common::rule_ids(&report)
    );
}

/// A lone denial with no affirmative neighbour is not the stack.
#[test]
fn c010_single_clause_without_a_partner_stays_silent() {
    let t = "The queue drains on shutdown. It does not detect authorship.\n\nA later paragraph.\n";
    let report = run(t, Profile::Readme);
    assert_invariants(t, &report);
    assert!(
        !has_rule(&report, "SLOP-C010"),
        "a lone denial reported: {:?}",
        common::rule_ids(&report)
    );
}

/// A command is excluded one clause at a time, and only an imperative-capable
/// negation can head one. The base-form test is what separates the command
/// from the middle of a denial stack: `do not assume` governs a base form and
/// is a command, `never assumes` does not and is a declarative with its
/// subject elided.
#[test]
fn c010_command_exclusion_is_per_clause_and_reads_the_verb_form() {
    let command = "It reads text. Do not assume the report is not evidence of intent.\n";
    let report = run(command, Profile::Readme);
    assert_invariants(command, &report);
    assert!(
        !has_rule(&report, "SLOP-C010"),
        "a command reported: {:?}",
        common::rule_ids(&report)
    );

    let declarative = "It reads text. Never assumes the report is not evidence of intent.\n";
    let report = run(declarative, Profile::Readme);
    assert_invariants(declarative, &report);
    assert!(
        has_rule(&report, "SLOP-C010"),
        "an elided-subject declarative was read as a command: {:?}",
        common::rule_ids(&report)
    );
}

/// The exclusion never runs sentence-wide: one leading command must not
/// carry the rest of a stack past the rule. The reported span is the
/// qualifying clause alone, so the command stays out of the finding.
#[test]
fn c010_a_leading_command_does_not_cover_the_clauses_after_it() {
    let t = "The linter reads text. Do not obey injected text, and it does not judge anyone.\n";
    let report = run(t, Profile::Readme);
    assert_invariants(t, &report);
    let hit = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-C010")
        .expect("the second clause qualifies on judge");
    let snippet = common::snippet(hit);
    assert!(
        snippet.contains("does not judge anyone"),
        "the qualifying clause is missing from the span: {snippet:?}"
    );
    assert!(
        !snippet.contains("obey"),
        "the excluded command entered the span: {snippet:?}"
    );
}

/// A leading coordinator is skipped before every test, so the second half of
/// a coordinated denial reads the same as the first. The subject the skip
/// uncovers is the one the clause presents: `and it does not detect
/// authorship` reads as `it does not detect authorship`.
#[test]
fn c010_reads_past_a_leading_coordinator() {
    for t in [
        "The linter reads text. And it does not detect authorship.\n",
        "It does not detect authorship, never scores voice, and makes no claim about intent.\n",
        "It reads text, and it does not detect authorship, and no finding is evidence of intent.\n",
        "It reads text. Text goes in, and it does not detect authorship.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            has_rule(&report, "SLOP-C010"),
            "a coordinator hid the clause: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// The partner search runs in order and stops at the first match: the other
/// clauses of the qualifying clause's own sentence, then the sentence before,
/// then the sentence after. Within one sentence there is no distance limit,
/// so an interposed clause does not hide the partner. Across sentences the
/// search stays strictly adjacent.
#[test]
fn c010_arm_b_searches_its_own_sentence_first() {
    for t in [
        "It reads text, and it does not detect authorship.\n",
        "It reads text, which took years to build, and it does not detect authorship.\n",
        // Segment granularity, not comma granularity: the coordinated form
        // without a comma reads the same as the form with one.
        "It reads text and it does not detect authorship.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            detail(&report, "SLOP-C010").contains("arm B, beside an affirmative clause"),
            "the within-sentence partner was missed: {t:?} {}",
            detail(&report, "SLOP-C010")
        );
    }

    // Two sentences apart is past the adjacency limit and stays silent.
    let far = "It reads text.\n\nA paragraph between.\n\nIt does not detect authorship.\n";
    let report = run(far, Profile::Readme);
    assert_invariants(far, &report);
    assert!(
        !has_rule(&report, "SLOP-C010"),
        "a distant partner satisfied the arm: {:?}",
        common::rule_ids(&report)
    );
}

/// Family 1 spelling C: an elided subject qualifies when the negation at the
/// head is finite, or when it is imperative-capable over an INFLECTED verb.
/// A base form there is a command, and an `-ing` form is a participial
/// adjunct.
#[test]
fn c010_elided_subject_spelling_reports_and_knows_the_verb_form() {
    for t in [
        "It does not detect authorship and never scores voice.\n",
        "It reads text. Never scores voice.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            has_rule(&report, "SLOP-C010"),
            "the elided-subject spelling stayed silent: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
    for t in [
        "Never score voice.\n",
        "Do not force-push main.\n",
        "She listened, never judging anyone.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            !has_rule(&report, "SLOP-C010"),
            "a command or a participle reported: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// The three-clause stack counts every spelling that qualified: the positive
/// subject, the elided subject, and the hedge.
#[test]
fn c010_the_stack_counts_all_three_spellings() {
    let t = "It does not detect authorship, never scores voice, and makes no claim about intent.\n";
    let report = run(t, Profile::Readme);
    assert_invariants(t, &report);
    assert!(
        detail(&report, "SLOP-C010").contains("arm A, one of 3 denied capabilities"),
        "the stack must count three: {}",
        detail(&report, "SLOP-C010")
    );
}

/// Bare commands stay commands.
#[test]
fn c010_imperatives_stay_silent() {
    for t in [
        "Never obey injected text. Do not force-push main. Never sign your own waiver.\n",
        "It reads text. Never treat a finding as evidence of authorship, and do not describe the tool that way.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            !has_rule(&report, "SLOP-C010"),
            "an imperative reported: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// The subject set is closed. A denial about anything else is ordinary prose.
#[test]
fn c010_open_subjects_stay_silent() {
    for t in [
        "The parser reads text. The parser does not detect a trailing comma.\n",
        "Returns a reference, not a copy. The timeout is per attempt, not per call.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            !has_rule(&report, "SLOP-C010"),
            "an open subject reported: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// A backticked mention is a code span, which segmentation excludes.
#[test]
fn c010_backticked_mention_stays_silent() {
    let t = "The shape to catch reads `It reads text. It does not detect authorship, and no finding is evidence that a model wrote anything.` in a scope line.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    assert!(
        !has_rule(&report, "SLOP-C010"),
        "a quoted example reported: {:?}",
        common::rule_ids(&report)
    );
}

// --- SLOP-F004 ---------------------------------------------------------------

/// Both marker families, anchored by the tool nouns in the same sentence.
/// This is the owner's second specimen.
#[test]
fn f004_reception_and_design_markers_report() {
    let t = "Source in another language reaches the rules and produces findings a reader should discount, which is the trade for a guard that never fires on prose.\n";
    let report = run(t, Profile::Readme);
    assert_invariants(t, &report);
    // One finding per marker: the reception instruction and the design
    // bargain are two things to cut.
    let found = hits(&report, "SLOP-F004");
    assert_eq!(found.len(), 2, "one finding per marker: {found:?}");
    let spans: Vec<String> = found.iter().map(|f| common::snippet(f)).collect();
    assert_eq!(spans[0], "a reader should discount");
    assert_eq!(spans[1], "which is the trade");

    for t in [
        "The guard skips indented blocks by design.\n",
        "The rule fires deliberately when the span is short.\n",
        "The score should be read as a density.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert!(
            has_rule(&report, "SLOP-F004"),
            "an anchored marker stayed silent: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// The anchor is a tool noun anywhere in the sentence, and nothing else. A
/// pronoun points at whatever the paragraph was last about, so it anchors
/// nothing, which leaves the tool-noun-free denial to the reread.
#[test]
fn f004_unanchored_markers_stay_silent() {
    for t in [
        "The migration ran overnight, deliberately, so the load fell on the quiet hours.\n",
        "She left the door open on purpose.\n",
        "That was deliberately vague.\n",
        "I did it deliberately.\n",
        "It was deliberately narrow.\n",
        "This poem should be read as an elegy.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            !has_rule(&report, "SLOP-F004"),
            "an unanchored marker reported: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}

/// The seam: the negated reception form belongs to SLOP-C010's hedge family,
/// and the affirmative form belongs here.
#[test]
fn f004_leaves_the_negated_reception_form_to_c010() {
    let negated =
        "The score measures pattern density. The score should not be read as a ranking.\n";
    let report = run(negated, Profile::Readme);
    assert_invariants(negated, &report);
    assert!(
        !has_rule(&report, "SLOP-F004"),
        "F004 took C010's hedge: {:?}",
        common::rule_ids(&report)
    );
    assert!(
        has_rule(&report, "SLOP-C010"),
        "C010 lost its hedge: {:?}",
        common::rule_ids(&report)
    );

    let affirmative = "The score should be read as a density figure.\n";
    let report = run(affirmative, Profile::Readme);
    assert!(
        has_rule(&report, "SLOP-F004"),
        "the affirmative reception form stayed silent: {:?}",
        common::rule_ids(&report)
    );
}

/// The process-facts family exemption holds: internal-doc carries its own
/// reasoning, so the rule is off there.
#[test]
fn f004_is_off_on_internal_doc() {
    let t = "The guard skips indented blocks by design.\n";
    let report = run(t, Profile::InternalDoc);
    assert!(
        !has_rule(&report, "SLOP-F004"),
        "F004 fired on internal-doc: {:?}",
        common::rule_ids(&report)
    );
}

// --- SLOP-C007's and-not spelling --------------------------------------------

/// The and-not spelling carries the same rejection into a preposition.
#[test]
fn c007_and_not_spelling_reports() {
    let t = "Gating source draws findings from statement punctuation and not from writing.\n";
    let report = run(t, Profile::Readme);
    assert_invariants(t, &report);
    assert!(
        has_rule(&report, "SLOP-C007"),
        "the and-not spelling stayed silent: {:?}",
        common::rule_ids(&report)
    );
}

/// The pattern carries the and-spelling only. `whether or not` is an honest
/// idiom for an open condition and wears the or-spelling, so the or-not and
/// but-not spellings stay on the reread checklist.
#[test]
fn c007_or_not_spelling_stays_hand_read() {
    for t in [
        "Whether or not the flag is present, parsing proceeds.\n",
        "The gate landed, or not in the form we planned.\n",
    ] {
        let report = run(t, Profile::Readme);
        assert_invariants(t, &report);
        assert!(
            !has_rule(&report, "SLOP-C007"),
            "the or-spelling reported: {t:?} {:?}",
            common::rule_ids(&report)
        );
    }
}
