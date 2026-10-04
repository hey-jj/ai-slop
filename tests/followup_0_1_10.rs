//! The 0.1.10 follow-up round: C004's temporal-while split and its span at a
//! block boundary, C010's `and`-joined denial, the wider hedge wildcard, the
//! adverb the command test steps over, the participle that closes C007's
//! comma tail, and V002's anchored praise entries behind a run of decoration.

mod common;

use ai_slop::Profile;
use common::{assert_invariants, has_rule, run, snippet};

fn hits<'a>(report: &'a ai_slop::Report, id: &str) -> Vec<&'a ai_slop::Finding> {
    report.findings.iter().filter(|f| f.rule_id == id).collect()
}

fn fires(text: &str, id: &str) -> bool {
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    has_rule(&report, id)
}

// --- SLOP-C004 temporal while ------------------------------------------------

/// A progressive in front of the comma says the clause is about a stretch of
/// time, so the concession arm lets it go.
#[test]
fn c004_progressive_while_is_temporal_and_silent() {
    for t in [
        "While you are working, you might notice unexpected changes.\n",
        "While the index is rebuilding, queries fall back to the scan.\n",
        "While the workers were draining, the queue kept growing.\n",
    ] {
        assert!(!fires(t, "SLOP-C004"), "temporal while fired: {t}");
    }
}

/// A participle in the slot straight after the keyword names the activity that
/// filled a stretch of time.
#[test]
fn c004_participial_while_is_temporal_and_silent() {
    for t in [
        "While working on the migration, we found a race.\n",
        "While reviewing the patch, we found a second bug.\n",
        "While redistributing the work, you may charge a fee.\n",
    ] {
        assert!(!fires(t, "SLOP-C004"), "participial while fired: {t}");
    }
}

/// Eight participles introduce concessions, so they keep the
/// match.
#[test]
fn c004_concession_participles_are_held_out() {
    for word in [
        "acknowledging",
        "recognizing",
        "granting",
        "accepting",
        "conceding",
        "admitting",
        "noting",
        "allowing",
    ] {
        let t = format!("While {word} the risk, we proceeded anyway.\n");
        assert!(fires(&t, "SLOP-C004"), "{word} lost its concession");
    }
}

/// The participial drop asks for no finite verb between the keyword and the
/// comma. A concession that opens on an -ing word always carries one, and a
/// participial adjunct never does.
#[test]
fn c004_a_finite_verb_keeps_the_concession() {
    for t in [
        "While programming language parsers are usually written manually for more flexibility, this crate uses a generated one.\n",
        "While manipulating ASTs is the most flexible way to transform documents, operating on iterators is easier.\n",
        "While it can be slow, it is correct.\n",
    ] {
        assert!(fires(t, "SLOP-C004"), "a finite verb lost its concession: {t}");
    }
}

/// The list is read whole and never by suffix. An `-s` scan would call
/// `operations` and `parts` verbs and take back the clears the drop is for.
#[test]
fn c004_plural_nouns_are_not_finite_verbs() {
    for t in [
        "While reviewing some unsafe Vec::from_raw_parts operations within the library, we found a bug.\n",
        "While redistributing the Work or Derivative Works thereof, you may charge a fee.\n",
    ] {
        assert!(!fires(t, "SLOP-C004"), "a plural noun read as a verb: {t}");
    }
}

/// `be`, `been`, and `being` are deliberately off the list.
#[test]
fn c004_being_is_not_on_the_finite_list() {
    let t = "While being tested, the parser reports its own timings.\n";
    assert!(!fires(t, "SLOP-C004"), "being read as a finite verb");
}

/// A bare-form plural verb is finite without being on the list, so this
/// concession drops. Recorded in the guard beside the copula-adjective miss.
#[test]
fn c004_bare_form_plural_finite_is_the_recorded_residual() {
    let t = "While parsing tools handle this correctly, ours does not.\n";
    assert!(
        !fires(t, "SLOP-C004"),
        "the recorded residual stopped dropping"
    );
}

/// A quantifier pronoun ending in the same three letters is not a participle,
/// and the shared morphology list says so.
#[test]
fn c004_quantifier_pronouns_are_not_participles() {
    for t in [
        "While nothing changes, the queue keeps growing.\n",
        "While something is broken, the build still ships.\n",
    ] {
        assert!(fires(t, "SLOP-C004"), "a quantifier pronoun dropped: {t}");
    }
}

/// A copula with an adjective wears the same three parts as a real concession,
/// so it fires and the judge settles it. Recorded in the guard.
#[test]
fn c004_copula_adjective_is_the_inseparable_miss() {
    let t = "While the worker is busy with the first task, the queue keeps growing.\n";
    assert!(fires(t, "SLOP-C004"), "the recorded miss stopped firing");
}

/// Neither test reaches although or though.
#[test]
fn c004_participle_test_is_while_only() {
    let t = "Although working on the migration, we shipped on time.\n";
    assert!(fires(t, "SLOP-C004"), "although took the participle test");
}

/// A copula without a participle is the concession the rule exists for.
#[test]
fn c004_concessive_while_still_fires() {
    let t = "While the parser is slower, it handles more cases.\n";
    assert!(fires(t, "SLOP-C004"), "concessive while stayed silent");
}

/// A durative present carries no progressive, so it reads as a concession to
/// the test and goes to the judge. Recorded in the guard as a stated miss.
#[test]
fn c004_bare_durative_while_is_a_stated_miss() {
    let t = "While the build runs, grab a coffee.\n";
    assert!(fires(t, "SLOP-C004"), "the stated miss stopped firing");
}

/// Neither although nor though has a temporal reading, so neither gets the
/// progressive test.
#[test]
fn c004_although_and_though_stay_unqualified() {
    for t in [
        "Although you are working, the build keeps going.\n",
        "Though the tests are running, the branch is already merged.\n",
    ] {
        assert!(fires(t, "SLOP-C004"), "although or though went silent: {t}");
    }
}

/// The staged-agreement arm is licensed by the terminal punctuation of the
/// sentence before, which across two list items sits in an earlier block. The
/// span the reader gets opens at the concession word.
#[test]
fn c004_list_item_span_opens_at_the_concession_word() {
    let t = "- The parser handles nesting.\n- Granted, the depth limit is low, but it holds.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    let found = hits(&report, "SLOP-C004");
    assert_eq!(
        found.len(),
        1,
        "one concession: {:?}",
        common::rule_ids(&report)
    );
    assert_eq!(
        snippet(found[0]),
        "Granted, the depth limit is low, but",
        "span opens at the concession word"
    );
    assert_eq!(
        (found[0].spans[0].start, found[0].spans[0].end),
        (32, 68),
        "span bytes carry no prior-block punctuation"
    );
}

// --- SLOP-C010 the `and`-joined denial ---------------------------------------

/// A base-form denial behind `and` finishes a sentence that already named its
/// subject. Classify it as a statement about that subject. It is declarative.
#[test]
fn c010_and_joined_denial_reports_arm_b() {
    for (t, span) in [
        (
            "The rules read text and never detect authorship.\n",
            "never detect authorship",
        ),
        (
            "The rules read text and do not detect authorship.\n",
            "do not detect authorship",
        ),
        (
            "The rules are advisory and do not replace review.\n",
            "do not replace review",
        ),
    ] {
        let report = run(t, Profile::InternalDoc);
        assert_invariants(t, &report);
        let found = hits(&report, "SLOP-C010");
        assert_eq!(found.len(), 1, "one finding for {t}");
        assert_eq!(snippet(found[0]), span, "segment span for {t}");
        assert!(
            found[0].message.contains("arm B"),
            "the joined denial reports on the adjacency arm: {}",
            found[0].message
        );
    }
}

/// Every condition the shape needs, removed one at a time.
#[test]
fn c010_and_joined_denial_negatives_stay_silent() {
    for t in [
        // No sentence in front of it at all.
        "Never detect authorship.\n",
        // A turn marker instead of `and`.
        "The rules read text but never detect authorship.\n",
        "The rules read text so never detect authorship.\n",
        // A comma leaves the imperative reading open.
        "The rules read text, never detect authorship.\n",
        // Nothing earlier in the sentence names a closed-set subject.
        "Read the report and never judge by one finding.\n",
        // A new sentence starts the count over.
        "The rules read text. Never score voice.\n",
    ] {
        assert!(!fires(t, "SLOP-C010"), "the joined denial over-fired: {t}");
    }
}

/// The guard's own enumeration of the silent spellings ends on the shape it is
/// describing, and stays silent.
#[test]
fn c010_guard_enumeration_sentence_stays_silent() {
    let t = "The silent list is short: a bare Never detect authorship, the comma spelling, the but and so spellings, and Never score voice one sentence later.\n";
    assert!(!fires(t, "SLOP-C010"), "the guard's own sentence fires");
    let guard = ai_slop::engine::compiled()
        .expect("policy compiles")
        .pkg
        .rules
        .iter()
        .find(|r| r.id == "SLOP-C010")
        .expect("C010 is in the package")
        .guard
        .clone();
    assert!(
        guard.contains("and Never score voice one sentence later"),
        "the guard carries the enumeration it is pinned on"
    );
    assert!(
        !fires(&format!("{guard}\n"), "SLOP-C010"),
        "guard self-fires"
    );
}

/// The shape the judge settles: a real instruction wearing the four
/// conditions. The guard records them, and they retain their findings.
#[test]
fn c010_judge_absorbed_joined_instruction_fires() {
    let t = "The tool is fast, and never replace review with it.\n";
    assert!(
        fires(t, "SLOP-C010"),
        "the judge-absorbed shape stopped firing"
    );
}

// --- SLOP-C010 the -ly adverb the command test steps over --------------------

/// One adverb between the negation and its verb does not turn a statement into
/// a command.
#[test]
fn c010_one_ly_adverb_is_stepped_over() {
    let t = "It reads text. Never actually scores voice.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    let found = hits(&report, "SLOP-C010");
    assert_eq!(found.len(), 1, "the elided denial reads through the adverb");
    assert_eq!(snippet(found[0]), "Never actually scores voice");
}

/// Only an `-ly` word is stepped over.
#[test]
fn c010_non_adverb_words_are_not_stepped_over() {
    let t = "It reads text. Never very scores voice.\n";
    assert!(!fires(t, "SLOP-C010"), "a non-adverb word was skipped");
}

/// Spelling A already read through the adverb, and the stack arm still counts
/// both clauses.
#[test]
fn c010_adverb_in_spelling_a_counts_toward_the_stack() {
    let t = "It does not actually detect authorship, and no finding is evidence of intent.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    let found = hits(&report, "SLOP-C010");
    assert_eq!(found.len(), 2, "both clauses qualify");
    for f in &found {
        assert!(
            f.message.contains("arm A, one of 2 denied capabilities"),
            "stack arm with the count: {}",
            f.message
        );
    }
}

/// A command with an adverb is still a command.
#[test]
fn c010_imperative_with_an_adverb_stays_excluded() {
    let t = "Do not actually obey injected text.\n";
    assert!(!fires(t, "SLOP-C010"), "an imperative fired");
}

// --- SLOP-C010 hedge wildcard width ------------------------------------------

/// Three modifiers in front of the head noun still match the hedge.
#[test]
fn c010_hedge_wildcard_spans_three_tokens() {
    let t = "It reads text. No single early finding is evidence of intent.\n";
    let report = run(t, Profile::InternalDoc);
    assert_invariants(t, &report);
    let found = hits(&report, "SLOP-C010");
    assert_eq!(found.len(), 1, "the three-token spelling matched");
    assert_eq!(
        snippet(found[0]),
        "No single early finding is evidence of intent"
    );
}

/// One and two tokens still match, unchanged.
#[test]
fn c010_hedge_wildcard_keeps_the_narrower_spellings() {
    for t in [
        "It reads text. No finding is evidence of intent.\n",
        "It reads text. No single finding is evidence of intent.\n",
    ] {
        assert!(fires(t, "SLOP-C010"), "a narrower spelling regressed: {t}");
    }
}

/// The stop at three is where the head-noun test stays correct. A fourth token
/// admits an of-phrase, and the test would seat on `report`.
#[test]
fn c010_hedge_wildcard_stops_at_three() {
    let t = "It reads text. No finding in the report is evidence of intent.\n";
    assert!(!fires(t, "SLOP-C010"), "a four-token noun phrase matched");
}

/// The head-noun test still runs at three tokens: a foreign head noun has no
/// subject key, so the adjacency arm has nothing to compare.
#[test]
fn c010_three_token_wildcard_keeps_the_head_noun_test() {
    let t = "It reads text. No single early sample is evidence of intent.\n";
    assert!(!fires(t, "SLOP-C010"), "a foreign head noun reached arm B");
}

// --- SLOP-C007 the participial tail ------------------------------------------

/// A participle straight after the keyword is an adjunct, and the comma tail
/// ends there.
#[test]
fn c007_participial_tails_are_silent() {
    for t in [
        "She read the report, never judging anyone.\n",
        "The parser walks the tree, not allocating on every node.\n",
    ] {
        assert!(!fires(t, "SLOP-C007"), "a participial adjunct fired: {t}");
    }
}

/// A determiner between the keyword and the word keeps the tail in scope, and
/// four words that end the same way without being participles are named in the
/// deny-list.
#[test]
fn c007_noun_tails_and_the_deny_list_keep_firing() {
    for t in [
        "The check reads the file, not the sentence.\n",
        "The report lists the risks, not everything.\n",
        "The offset points at the token, not the beginning.\n",
        "The scan runs at emit, not during matching.\n",
        "The tool flags a shape, not a building.\n",
    ] {
        assert!(fires(t, "SLOP-C007"), "a noun tail went silent: {t}");
    }
}

// --- policy version ----------------------------------------------------------

/// The round changed rule text and patterns, so the policy version moves.
#[test]
fn policy_version_is_1_10_0() {
    let cp = ai_slop::engine::compiled().expect("policy compiles");
    assert_eq!(cp.pkg.version, "1.10.0");
}

// --- SLOP-V002 block-start anchoring and the emoji run -----------------------

fn v002_snippets(text: &str) -> Vec<String> {
    let report = run(text, Profile::Readme);
    assert_invariants(text, &report);
    hits(&report, "SLOP-V002")
        .iter()
        .map(|f| snippet(f))
        .collect()
}

/// A run of decoration in front of a phrase leaves the phrase at the opening
/// of its line, which is where a reader sees it.
#[test]
fn v002_praise_behind_an_emoji_still_opens_its_line() {
    let t = "\u{1F389} Great question!\n";
    assert_eq!(v002_snippets(t), vec!["Great question".to_string()]);
    assert!(
        has_rule(&run(t, Profile::Readme), "SLOP-M006"),
        "the emoji rule reports the decoration itself"
    );
}

/// Away from the opening the same words report what someone did.
#[test]
fn v002_praise_mid_sentence_is_silent() {
    for t in [
        "He asked a great question about the parser.\n",
        "The reviewer left a great point in the thread.\n",
        "She said you are absolutely right about the span.\n",
    ] {
        assert!(v002_snippets(t).is_empty(), "anchored praise fired: {t}");
    }
}

/// All eight take the position test, so no single entry walks around it.
#[test]
fn v002_all_eight_praise_entries_are_anchored() {
    for phrase in [
        "Great question",
        "Good question",
        "Excellent question",
        "That's a great question",
        "Great point",
        "Excellent point",
        "You're absolutely right",
        "You are absolutely right",
    ] {
        let opening = format!("{phrase}. The rest of the note follows.\n");
        assert!(
            !v002_snippets(&opening).is_empty(),
            "{phrase} went silent at a line opening"
        );
        let buried = format!(
            "The reviewer wrote that {}. Nothing else.\n",
            phrase.to_lowercase()
        );
        assert!(
            v002_snippets(&buried).is_empty(),
            "{phrase} fired mid-sentence"
        );
    }
}

/// A reviewer who opens a line with either catch means it, so the two entries
/// left the lexicon instead of joining the anchored set.
#[test]
fn v002_catch_entries_are_gone_from_the_lexicon() {
    for t in [
        "Good catch.\n",
        "Great catch.\n",
        "Good catch on the off-by-one.\n",
    ] {
        assert!(v002_snippets(t).is_empty(), "a catch entry fired: {t}");
    }
}

/// What follows such an opener still reports.
#[test]
fn v002_paired_residue_fires_on_the_second_phrase() {
    let t = "Good catch! You're absolutely right.\n";
    assert_eq!(
        v002_snippets(t),
        vec!["You're absolutely right".to_string()]
    );
}

/// The unanchored half of the lexicon is unchanged by the position test.
#[test]
fn v002_unanchored_entries_still_fire_anywhere() {
    let t = "The maintainer said I'd be happy to look at it tomorrow.\n";
    assert_eq!(v002_snippets(t), vec!["I'd be happy to".to_string()]);
}

// --- SLOP-V002 the concession entry ------------------------------------------

/// The concession reads the same wherever it sits, so the entry is unanchored.
#[test]
fn v002_fair_hit_fires_in_every_position() {
    for t in [
        "Fair hit. I'll revise the section.\n",
        "That's a fair hit on the design.\n",
        "Fair hit on the naming, I'll change it.\n",
    ] {
        assert!(
            !v002_snippets(t).is_empty(),
            "the concession stayed silent: {t}"
        );
    }
}

/// Two readings ride along and the judge takes both. Pinned as firing, since
/// neither is a regression.
#[test]
fn v002_fair_hit_collisions_reach_the_judge() {
    for t in [
        "The replay showed a fair hit to the shoulder.\n",
        "That was an unfair hit.\n",
    ] {
        assert!(
            !v002_snippets(t).is_empty(),
            "a collision the judge settles stopped firing: {t}"
        );
    }
}

/// Neighbouring concessions carry no entry.
#[test]
fn v002_nearby_concessions_stay_silent() {
    for t in [
        "Fair point, I'll change it.\n",
        "Fair enough.\n",
        "It's a fair cop.\n",
        "That was a fair knock.\n",
    ] {
        assert!(v002_snippets(t).is_empty(), "a neighbour fired: {t}");
    }
}

// --- block-start rules and ordinal markers -----------------------------------

/// The extractor strips a list marker before the norm view exists, so the
/// block-start test never meets an ordinal in either spelling. Both are pinned
/// so a change to that layer shows up here.
#[test]
fn block_start_rules_never_meet_an_ordinal_marker() {
    for t in [
        "1. First item.\n2. Moreover, the parser is slow.\n",
        "1. First item.\n2) Moreover, the parser is slow.\n",
        "- Moreover, the parser is slow.\n",
        "> Moreover, the parser is slow.\n",
        "# Moreover, the parser is slow.\n",
    ] {
        assert!(
            fires(t, "SLOP-T002"),
            "a stripped marker moved the opener: {t}"
        );
    }
}

/// A digit and a paren mid-line are text, not a marker, so the word after them
/// is mid-sentence.
#[test]
fn a_mid_line_ordinal_is_not_a_block_start() {
    let t = "The options are 2) Moreover as an entry.\n";
    assert!(!fires(t, "SLOP-T002"), "a mid-line ordinal opened a block");
}

/// A real terminal period still opens the next sentence, marker or not.
#[test]
fn a_terminal_period_before_an_opener_still_counts() {
    let t = "See item 3. Moreover, the parser is slow.\n";
    assert!(fires(t, "SLOP-T002"), "a terminal period stopped counting");
}

// --- block-start markers: bullet glyphs and the plain-text readers ----------

/// A bullet pasted from a rendered list leaves the phrase at the opening of
/// its line, the same way an emoji does.
#[test]
fn v002_praise_behind_a_bullet_glyph_opens_its_line() {
    for bullet in ["\u{2022}", "\u{2023}", "\u{2043}", "\u{2219}"] {
        let t = format!("{bullet} Great question!\n");
        assert_eq!(
            v002_snippets(&t),
            vec!["Great question".to_string()],
            "the bullet {bullet} moved the praise off its line"
        );
    }
}

/// The middle dot is a letter in Catalan and a separator in running prose, so
/// it stays out of the transparent set.
#[test]
fn a_middle_dot_is_not_leading_decoration() {
    let t = "\u{b7} Great question!\n";
    assert!(
        v002_snippets(t).is_empty(),
        "the middle dot was read as decoration"
    );
}

/// Plain text and commit bodies have no parser to strip a list marker, so the
/// reader takes it out and the first word opens its block. Every block-start
/// rule reports behind a marker on those formats.
#[test]
fn text_mode_marker_led_lines_reach_the_block_start_rules() {
    for marker in ["-", "*", "+", "\u{2022}", "1.", "2)", ">", "##"] {
        for (opener, rule) in [
            ("However, the parser is slow.", "SLOP-M003"),
            ("Moreover, the parser is slow.", "SLOP-T002"),
            ("Overall, the parser is slow.", "SLOP-T001"),
            ("Generated by an assistant.", "SLOP-S001"),
        ] {
            let t = format!("{marker} {opener}\n");
            let mut cfg = common::cfg(Profile::InternalDoc);
            cfg.input_format = ai_slop::InputFormat::Text;
            let report = ai_slop::analyze(t.as_bytes(), &cfg).expect("analyze must succeed");
            assert_invariants(&t, &report);
            assert!(
                has_rule(&report, rule),
                "{rule} stayed silent behind {marker:?} in text mode: {:?}",
                common::rule_ids(&report)
            );
        }
    }
}

/// A marker only opens a block when whitespace follows it, so a negative
/// number and a bare issue reference stay prose.
#[test]
fn a_marker_without_whitespace_is_not_a_marker() {
    for t in ["-3 degrees, however, is cold.\n", "#4 however, is open.\n"] {
        let mut cfg = common::cfg(Profile::InternalDoc);
        cfg.input_format = ai_slop::InputFormat::Text;
        let report = ai_slop::analyze(t.as_bytes(), &cfg).expect("analyze must succeed");
        assert_invariants(t, &report);
        assert!(
            !has_rule(&report, "SLOP-M003"),
            "a bare digit or hash opened a block: {t:?}"
        );
    }
}

/// The marker leaves the span, so a reported span never opens on it.
#[test]
fn text_mode_spans_never_open_on_a_marker() {
    let t = "- However, the parser is slow.\n";
    let mut cfg = common::cfg(Profile::InternalDoc);
    cfg.input_format = ai_slop::InputFormat::Text;
    let report = ai_slop::analyze(t.as_bytes(), &cfg).expect("analyze must succeed");
    assert_invariants(t, &report);
    let found: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "SLOP-M003")
        .collect();
    assert_eq!(found.len(), 1);
    assert_eq!(snippet(found[0]), "However");
    assert_eq!((found[0].spans[0].start, found[0].spans[0].end), (2, 9));
}

/// A run mixing decoration and markers still leaves the phrase at the opening,
/// in any order.
#[test]
fn mixed_decoration_and_marker_runs_open_the_line() {
    for lead in [
        "\u{1F389} \u{2023}",
        "\u{25AA} \u{1F389}",
        "- \u{2023}",
        "\u{2022} \u{1F389}",
    ] {
        let t = format!("{lead} Moreover, the parser is slow.\n");
        assert!(fires(&t, "SLOP-T002"), "a mixed run hid the opener: {lead}");
    }
}

/// A marker is structure, so it is not a word. Markdown never counted one and
/// the plain-text readers no longer do either.
#[test]
fn a_marker_is_not_counted_as_a_word() {
    let body = "alpha beta gamma delta epsilon zeta eta theta. ".repeat(40);
    let bulleted: String = (0..40).map(|i| format!("{}. {body}\n", i + 1)).collect();
    let mut text_cfg = common::cfg(Profile::InternalDoc);
    text_cfg.input_format = ai_slop::InputFormat::Text;
    let text = ai_slop::analyze(bulleted.as_bytes(), &text_cfg).expect("analyze must succeed");
    let mut md_cfg = common::cfg(Profile::InternalDoc);
    md_cfg.input_format = ai_slop::InputFormat::Markdown;
    let md = ai_slop::analyze(bulleted.as_bytes(), &md_cfg).expect("analyze must succeed");
    let rate = |r: &ai_slop::Report| {
        r.findings
            .iter()
            .find(|f| f.rule_id == "SLOP-D004")
            .map(|f| f.message.clone())
            .unwrap_or_default()
    };
    assert_eq!(
        rate(&text),
        rate(&md),
        "text and markdown disagree on the per-1000-words rate"
    );
}

/// One token may sit between the be-form and the participle. "were already
/// running" describes the same stretch of time "were running" does.
#[test]
fn c004_progressive_reads_over_one_intervening_adverb() {
    for t in [
        "While we were already running the testsuite, the exit code differed.\n",
        "While the parser is still parsing the header, the reader waits.\n",
        "While the job is currently running, the queue holds.\n",
        "While the build was just finishing, the cache expired.\n",
        "While the workers are never draining, the queue grows.\n",
        "While the index is quietly rebuilding, queries fall back.\n",
    ] {
        assert!(!fires(t, "SLOP-C004"), "the adverb gap stayed open: {t}");
    }
}

/// One is the cap. A second token means the copula governs something else.
#[test]
fn c004_progressive_skips_at_most_one_token() {
    let t = "While the parser is still half parsing the header, the reader waits.\n";
    assert!(fires(t, "SLOP-C004"), "two tokens were skipped");
}

/// A word qualifies for the step over by an -ly ending or by the closed list,
/// and by nothing else.
#[test]
fn c004_only_adverbs_are_stepped_over() {
    for t in [
        "While the parser is header parsing the file, the reader waits.\n",
        "While the queue is batch draining, the workers idle.\n",
    ] {
        assert!(fires(t, "SLOP-C004"), "a non-adverb was skipped: {t}");
    }
}

/// The copula-adjective miss widens with the gap and stays a miss.
#[test]
fn c004_copula_adjective_with_an_adverb_still_fires() {
    for t in [
        "While the API is already stable, the client changes weekly.\n",
        "While the result is still wrong, the tests pass.\n",
    ] {
        assert!(fires(t, "SLOP-C004"), "a recorded miss stopped firing: {t}");
    }
}
