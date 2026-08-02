//! Calibration coverage against real-world markdown: table-cell
//! barriers — no match may fuse across the `|` cell delimiter; A002
//! `harness` narrowed to the verb-with-object slop
//! form; and mention-vs-use — the code-span authoring convention for
//! quoted banned-word lists, with the plain-prose enumeration residual pinned.

mod common;

use ai_slop::Profile;
use common::{assert_invariants, has_rule, run};

// --- Table cells scanned as prose must not fuse across cell delimiters ------

/// S001 (`^--\s{1,8}\S`) and M001 (`\s--\s`) can otherwise fire on table
/// placeholder-dash cells by pairing one cell's `--` with the NEXT cell's
/// text across the Block newline. The cell-end barrier must stop both.
#[test]
fn placeholder_dash_cells_no_longer_fuse_across_cell_boundaries() {
    let text = "# Audit\n\n\
        | Crate | Verdict | Notes |\n\
        | --- | --- | --- |\n\
        | serde | -- | not audited |\n\
        | tokio | -- | pending |\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    assert!(
        !has_rule(&report, "SLOP-S001"),
        "S001 fused across cells: {:?}",
        common::rule_ids(&report)
    );
    assert!(
        !has_rule(&report, "SLOP-M001"),
        "M001 fused across cells: {:?}",
        common::rule_ids(&report)
    );
}

/// Genuine slop INSIDE one cell must still fire: the barrier sits at the cell
/// end only, never inside it, and cell interiors remain scanned prose.
#[test]
fn genuine_slop_inside_a_single_cell_still_fires() {
    let text = "| Item | Note |\n\
        | --- | --- |\n\
        | widget | a truly game-changer design |\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    let a001: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "SLOP-A001")
        .collect();
    assert_eq!(a001.len(), 1, "in-cell lexicon hit fires");
    let span = &a001[0].spans[0];
    assert_eq!(&text[span.start..span.end], "game-changer");
}

/// A signature line inside one cell is in-cell content, not cross-cell
/// fusion: the block-start position of the cell's own text must survive the
/// barrier (which is why the barrier is at the cell END, not the start).
#[test]
fn signature_shape_within_one_cell_still_fires() {
    let text = "| Item | Note |\n\
        | --- | --- |\n\
        | -- Claude | sig in cell |\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    let s001: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "SLOP-S001")
        .collect();
    assert_eq!(s001.len(), 1, "in-cell signature fires");
    let span = &s001[0].spans[0];
    assert!(text[span.start..span.end].starts_with("-- C"));
}

/// The table's leading edge: a paragraph ending `--` directly before a table
/// must not pair with the first header cell into S001's shape.
#[test]
fn prose_before_a_table_does_not_fuse_into_the_first_cell() {
    let text = "--\n\n| Alpha | Beta |\n| --- | --- |\n| a | b |\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    assert!(
        !has_rule(&report, "SLOP-S001"),
        "S001 fused prose into the table head: {:?}",
        common::rule_ids(&report)
    );
}

/// A normal prose document is untouched by the table barriers: the real
/// signature shape still fires, and an ordinary paragraph raises nothing new.
#[test]
fn normal_prose_is_unaffected_by_table_barriers() {
    let text = "The parser handles nested lists.\n\n-- Claude\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    assert!(
        has_rule(&report, "SLOP-S001"),
        "prose signature still fires"
    );

    let clean = "The parser handles nested lists without recursion.\n";
    let report = run(clean, Profile::InternalDoc);
    assert_invariants(clean, &report);
    assert!(
        report.findings.iter().all(|f| f.state != "violation"),
        "clean prose stays clean: {:?}",
        common::rule_ids(&report)
    );
}

// --- A002 `harness` narrowed to the verb-with-object slop form --------------

fn a002_fires(text: &str) -> bool {
    run(text, Profile::Readme)
        .findings
        .iter()
        .any(|f| f.rule_id == "SLOP-A002" && f.state == "violation")
}

/// FN regression: a determiner-only form silently
/// misses determiner-less verb slop. The exact verified-FN repros — each
/// clean under a determiner-only calibration — must fire.
#[test]
fn a002_determiner_less_verb_slop_fires() {
    for text in [
        // The three verified-FN repros, isolated and re-verified.
        "You can harness machine learning without extra setup.",
        "The SDK lets you harness modern APIs with one call.",
        "Use it to harness advanced language models in CI.",
        // Determiner-less variants: imperative start, AI-domain objects,
        // plural-subject + AI object, modal and helper-verb signals.
        "Harness modern tooling in one step.",
        "Developers harness AI daily.",
        "Teams can harness LLMs for code review.",
        "It helps harness generative AI safely.",
        "Let's harness large language models today.",
        "We will harness neural networks here.",
    ] {
        assert!(a002_fires(text), "determiner-less verb slop missed: {text}");
    }
}

/// NOUN uses of `harness` dominate real technical prose and must pass. The
/// rule requires the slop VERB construction — determiner+object, a preceding
/// verb/subject signal, the sentence-start imperative, an AI-domain object,
/// or the `harnessing` gerund; every noun use passes structurally.
#[test]
fn a002_harness_verb_with_object_fires() {
    for text in [
        "You can harness the power of X here.",
        "It harnesses the capabilities of the runtime.",
        "This lets you harness its potential.",
        "We harness your existing pipeline.",
        "Harnessing its potential is straightforward.",
        "By harnessing the capabilities of the compiler, it checks more.",
    ] {
        assert!(a002_fires(text), "verb-form slop did not fire: {text}");
    }
}

#[test]
fn a002_harness_noun_uses_do_not_fire() {
    for text in [
        "The test harness runs nightly.",
        "The orchestration harness deploys the fleet.",
        "The CI harness caches builds between runs.",
        "The harness ran without failures.",
        "Each harness writes its logs to disk.",
        "We added three harnesses for the parser.",
    ] {
        assert!(!a002_fires(text), "noun use fired: {text}");
    }
}

/// Every determiner-less/other-determiner verb
/// form confirmed as a silent FN must fire — in the imperative
/// (standalone) carrier via the sentence-start form AND in a signaled
/// carrier via the verb-context alternation.
#[test]
fn a002_confirmed_verb_forms_fire_in_both_carriers() {
    for phrase in [
        "harness machine learning",
        "harness advanced models",
        "harness data at scale",
        "harness LLMs",
        "harness real-time data",
        "harness such power",
        "harness all the power",
        "harness a modern API",
        "harness an advanced model",
        "harness some existing data",
        "harness that capability",
        "harness those models",
    ] {
        let standalone = format!("Harness{} today.", &phrase[7..]);
        assert!(
            a002_fires(&standalone),
            "imperative carrier missed: {standalone}"
        );
        let signaled = format!("You can {phrase} today.");
        assert!(a002_fires(&signaled), "signaled carrier missed: {signaled}");
    }
}

/// The decisive pair: `these` fired while `those` shipped clean, purely
/// from a half-covered demonstrative set. The closed
/// 4-member paradigm (this/that/these/those) makes the pair behave
/// IDENTICALLY in the unsignaled third-person carrier neither verb signal
/// nor imperative reaches.
#[test]
fn a002_demonstrative_paradigm_is_symmetric() {
    for det in ["this", "that", "these", "those"] {
        let text = format!("The platform harnesses {det} model daily.");
        assert!(a002_fires(&text), "demonstrative asymmetry: {text}");
    }
}

/// Boundary pins for the harness calibration, so neither residual is
/// accidental. Over-fire side (accepted, FN-safety first): a sentence-start
/// noun compound matches the imperative form and FIRES — a documented FP,
/// waivable, never a miss. Miss side (the documented residual): a bare
/// plural-noun subject with a base verb and a non-AI object is structurally
/// identical to a noun compound ("harness telemetry") and is not matched.
#[test]
fn a002_harness_calibration_boundaries_are_pinned() {
    assert!(
        a002_fires("Harness configuration lives in rig.toml."),
        "sentence-start over-fire is the accepted side of the boundary"
    );
    // Covering "harness that capability" costs the relativizer
    // over-fire — accepted, documented, waivable, never a miss.
    assert!(
        a002_fires("We ship a harness that runs nightly."),
        "relativizer over-fire is the accepted side of the boundary"
    );
    assert!(
        !a002_fires("Teams harness telemetry pipelines in production."),
        "documented residual (b) changed shape"
    );
    // Residual (a): unsignaled third-person subject + article object.
    assert!(
        !a002_fires("The platform harnesses a modern API under the hood."),
        "documented residual (a) changed shape"
    );
}

// --- C-family vs the cell barrier, both sides pinned -----------------------

/// C006's \s{1,8} gap cannot cross the cell barrier, so
/// the cross-cell contrast is SUPPRESSED (working-as-designed —
/// attacker-unrealistic as organic slop, candidate tier even in-cell), while
/// the same phrase inside one cell still fires.
#[test]
fn c006_cross_cell_suppressed_but_single_cell_fires() {
    let cross = "| A | B |\n| --- | --- |\n| simple | but flexible |\n";
    let report = run(cross, Profile::InternalDoc);
    assert_invariants(cross, &report);
    assert!(
        !has_rule(&report, "SLOP-C006"),
        "cross-cell C006 should be suppressed by the barrier: {:?}",
        common::rule_ids(&report)
    );
    let single = "| A | B |\n| --- | --- |\n| x | simple but flexible |\n";
    let report = run(single, Profile::InternalDoc);
    assert_invariants(single, &report);
    assert!(
        has_rule(&report, "SLOP-C006"),
        "in-cell C006 must still fire: {:?}",
        common::rule_ids(&report)
    );
}

/// C005's [^.!?] classes admit U+FFFD and the newline, so
/// the cross-cell tricolon BRIDGE persists — candidate tier, surfaced, never
/// silence. Pinned so the asymmetry with C006 stays deliberate; excluding
/// U+FFFD from the C-family classes would silence contrast-slop legitimately
/// spanning an inline-code barrier (a real FN) and must not be done casually.
#[test]
fn c005_cross_cell_bridge_persists_as_candidate() {
    let text = "| A | B | C |\n| --- | --- | --- |\n\
        | fast | Linux, macOS, and Windows | reliable |\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    let c005: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "SLOP-C005")
        .collect();
    assert!(
        !c005.is_empty(),
        "C005 bridge disappeared — if intentional, re-document the accepted edge"
    );
    for f in &c005 {
        assert_eq!(f.state, "candidate", "C005 must stay candidate tier");
    }
}

/// The other three homographs are untouched by the harness narrowing.
#[test]
fn a002_other_homographs_still_fire_in_bare_prose() {
    for text in [
        "This opens a new realm of possibilities.",
        "Users navigate complexity with ease.",
        "The testing landscape keeps changing.",
    ] {
        assert!(a002_fires(text), "homograph did not fire: {text}");
    }
}

// --- Mention-vs-use on quoted banned-word lists -----------------------------
//
// Decision: NO prose-list downgrade. The FN-safe authoring
// conventions are code spans / fenced code (excluded by segmentation) and
// blockquotes (deterministic candidate downgrade with provenance). A
// downgrade keyed on "list items under an avoid/banned heading" was rejected:
// LLMs produce "avoid"-headed lists organically, and any genuine slop
// sentence can be authored as a list item under one — a silent-FN channel.

/// The convention works: a style guide quoting every banned term in code
/// spans and a fenced block carries no ornamental/filler finding at all.
#[test]
fn banned_words_in_code_spans_and_fences_do_not_fire() {
    let text = "# Style guide\n\n\
        Never use these words: `delve`, `game-changer`, `robust`, `essentially`.\n\n\
        The banned list in fenced form:\n\n\
        ```text\ndelve\ngame-changer\nessentially\n```\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.rule_id == "SLOP-A001" || f.rule_id == "SLOP-T001"),
        "quoted-in-code banned words fired: {:?}",
        common::rule_ids(&report)
    );
    assert!(
        report.findings.iter().all(|f| f.state != "violation"),
        "style guide carries violations: {:?}",
        common::rule_ids(&report)
    );
}

/// The blockquote convention: quoted banned words downgrade to candidate
/// with claimed-quotation provenance — surfaced, never silent, not blocking.
#[test]
fn banned_words_in_a_blockquote_downgrade_to_candidate() {
    let text = "# Style guide\n\n> Avoid: delve, game-changer.\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    let a001: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "SLOP-A001")
        .collect();
    assert_eq!(a001.len(), 2);
    for f in &a001 {
        assert_eq!(f.state, "candidate", "quotation downgrade applies");
        assert_eq!(f.provenance, "claimed-quotation");
    }
}

/// Residual pin: PLAIN-PROSE enumeration of banned words still fires as a
/// violation. Deliberate — see the module comment above.
/// If this test ever goes red because someone added a prose-list downgrade,
/// that change must first prove it cannot hide genuine slop.
#[test]
fn plain_prose_banned_word_enumeration_still_fires() {
    let text = "# Style guide\n\nWords to avoid:\n\n- delve\n- game-changer\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    let a001: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "SLOP-A001" && f.state == "violation")
        .collect();
    assert_eq!(a001.len(), 2, "plain-prose mentions stay violations");
}

/// The guardrail the rejected downgrade was measured against: genuine slop in
/// ordinary prose — including inside a list under an "avoid" heading — fires.
#[test]
fn genuine_slop_in_prose_and_avoid_lists_still_fires() {
    let text = "We delve into the internals of the parser.\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    assert!(has_rule(&report, "SLOP-A001"));

    let text = "Mistakes to avoid:\n\n- Forgetting to delve into the config first.\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id == "SLOP-A001" && f.state == "violation"),
        "slop sentence under an avoid heading must still fire"
    );
}

/// Diagnosis pin (A001 on crate names in audit tables): a
/// single-cell
/// lexicon word — a crate NAMED `robust` or `Vibrant` — is NOT cross-cell
/// fusion and deliberately still fires. Distinguishing a name column from a
/// description cell is not decidable mechanically, and a cells-are-data
/// downgrade would hide genuine slop written in a description cell — a
/// silent-FN channel. The authoring convention is code spans: `robust` in
/// backticks is excluded by segmentation (see the mention-vs-use tests). Pinned so the
/// residual is visible, not accidental.
#[test]
fn single_cell_lexicon_word_is_a_documented_residual_not_fusion() {
    let text = "| Crate | Verdict |\n\
        | --- | --- |\n\
        | robust | clean |\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    let a001: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "SLOP-A001")
        .collect();
    assert_eq!(a001.len(), 1, "single-cell lexicon word still fires");
    let span = &a001[0].spans[0];
    assert_eq!(&text[span.start..span.end], "robust");

    // The convention: the same table with the crate name in a code span is
    // clean — the code-span exclusion plus its barrier cover it.
    let text = "| Crate | Verdict |\n\
        | --- | --- |\n\
        | `robust` | clean |\n";
    let report = run(text, Profile::InternalDoc);
    assert_invariants(text, &report);
    assert!(
        !has_rule(&report, "SLOP-A001"),
        "code-span crate name must not fire: {:?}",
        common::rule_ids(&report)
    );
}

// --- SLOP-C007 apophatic self-definition: adjudicated boundary pins ---------
//
// The spec's 16 positive and 16 negative boundary examples, pinned so the T1
// suppression classifier and the T2-T4 trigger regexes survive future tuning.
// Positives are third-person self-description; negatives are imperatives,
// second-person directives, parenthetical interpolations, and shapes owned by
// SLOP-C001/C003. The deny-list homograph FN and the api-docs relax behavior
// are pinned separately below.

const C007_POSITIVES: &[&str] = &[
    "Findings judge house style, not authorship.",
    "The report carries evidence, not verdicts.",
    "This tool is a linter, not a detector.",
    "It measures diction, not intent.",
    "The digest identifies the policy, not the tarball.",
    "Errors are surfaced, never swallowed.",
    "The cache is an optimization, not a source of truth.",
    "This limit is a floor, not a ceiling.",
    "The skill gates drafts, not people.",
    "The check enforces style, not correctness.",
    "The goal is clarity, not coverage.",
    "Waivers document exceptions, not permissions.",
    "The check isn't about speed, it's about correctness.",
    "Configuration is not a convenience but a contract.",
    "The scanner is not a formatter. It is a gate.",
    "Profiles describe the artifact, not the author.",
];

const C007_NEGATIVES: &[&str] = &[
    "Use tabs, not spaces.",
    "Never commit secrets, not even in fixtures.",
    "Do not retry on 4xx.",
    "You cannot call this from a signal handler, not even with a lock held.",
    "Prefer &str, not String, in argument position.",
    "Pass --force, not -f, to override.",
    "Rust, not C, was chosen for the rewrite.",
    "The tests cover ASCII but not UTF-16.",
    "If not set, the default applies.",
    "Whether or not the flag is present, parsing proceeds.",
    "The parser accepts CRLF, not because it is valid, but because real files contain it.",
    "Use exponential backoff rather than fixed sleeps.",
    "When in doubt, use the builder, not the raw constructor.",
    "404 Not Found is returned for missing keys.",
    "> \"It's not a bug, it's a feature.\"",
];

#[test]
fn c007_positive_boundaries_fire_as_experimental_candidates() {
    for text in C007_POSITIVES {
        let t = format!("{text}\n");
        let report = run(&t, Profile::Readme);
        assert_invariants(&t, &report);
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "SLOP-C007")
            .unwrap_or_else(|| panic!("C007 silent on positive {text:?}"));
        assert_eq!(f.state, "candidate", "{text:?}");
        assert_eq!(
            f.lifecycle, "experimental",
            "{text:?}: experimental lifecycle reports without gating"
        );
    }
}

#[test]
fn c007_negative_boundaries_stay_silent() {
    for text in C007_NEGATIVES {
        let t = format!("{text}\n");
        let report = run(&t, Profile::Readme);
        assert_invariants(&t, &report);
        assert!(
            !has_rule(&report, "SLOP-C007"),
            "C007 fired on negative {text:?}: {:?}",
            common::rule_ids(&report)
        );
    }
}

/// Degenerate-tail calibration: an empty or WHITESPACE-ONLY span between
/// the keyword and the terminal is not a noun phrase, so the T1 parser must
/// stay silent on it. The long-run variant (more whitespace than the 8-char
/// keyword-boundary skip consumes) is the regression pin: before the
/// content check it parsed the residual spaces as an "NP" and fired with a
/// snippet like `, not         .`. A real NP still fires as candidate.
#[test]
fn c007_whitespace_only_np_is_silent() {
    for text in [
        "Findings, not   .\n",              // short run: consumed by the skip
        "Findings, not            .\n",     // long run: residual ws is the "NP"
        "Findings, never \t \t    \t  .\n", // mixed space/tab, `never` keyword
    ] {
        let report = run(text, Profile::Readme);
        assert_invariants(text, &report);
        assert!(
            !has_rule(&report, "SLOP-C007"),
            "C007 fired on whitespace-only NP {text:?}: {:?}",
            common::rule_ids(&report)
        );
    }

    // Control: the canonical specimen with a real NP still fires.
    let t = "Findings judge house style, not authorship.\n";
    let report = run(t, Profile::Readme);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-C007")
        .expect("real NP must still fire");
    assert_eq!(f.state, "candidate");
}

/// ACCEPTED FALSE NEGATIVE (KNOWN-EDGES): C007 tail matching is
/// ASCII-whitespace-only. A non-ASCII space (here U+00A0 NBSP) between the
/// keyword and the noun phrase fails the keyword's right-boundary check, so
/// the tail does not fire. Accepted as attacker-unrealistic; this test
/// characterizes the behavior, it does not endorse widening the match.
#[test]
fn c007_nonascii_space_tail_is_an_accepted_false_negative() {
    let t = "Findings judge house style, not\u{00A0}authorship.\n";
    let report = run(t, Profile::Readme);
    assert_invariants(t, &report);
    assert!(
        !has_rule(&report, "SLOP-C007"),
        "the NBSP accepted-FN pin moved: {:?}",
        common::rule_ids(&report)
    );
}

/// The documented deny-list false negative: `Set` is the noun/verb homograph
/// on the imperative opener list, so this descriptive sentence is wrongly
/// suppressed. Accepted by design — the classifier's bias is FP-safety, and
/// every suppression doubt resolves toward silence.
#[test]
fn c007_denylist_homograph_is_an_accepted_false_negative() {
    let t = "Set operations return unions, not lists.\n";
    let report = run(t, Profile::Readme);
    assert!(
        !has_rule(&report, "SLOP-C007"),
        "the deny-list FN pin moved: {:?}",
        common::rule_ids(&report)
    );
}

/// Subject-elided rustdoc contrast: candidate under readme, advisory-only
/// under api-docs (`relax` drops candidate to advisory reporting), which is
/// the profile stance that carries the legitimate operand-contrast case the
/// mood classifier cannot reach.
#[test]
fn c007_api_docs_relax_drops_to_advisory() {
    let t = "Returns a reference, not a copy.\n";
    let readme = run(t, Profile::Readme);
    let f = readme
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-C007")
        .expect("candidate under readme");
    assert_eq!(f.state, "candidate");
    assert_eq!(f.lifecycle, "experimental");

    let api = run(t, Profile::ApiDocs);
    let f = api
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-C007")
        .expect("still reported under api-docs");
    assert_eq!(f.lifecycle, "advisory", "relax: candidate reports advisory");
    assert_eq!(api.result_state, "no_findings", "advisory never gates");
}

/// Span and trigger fidelity for the T1 evaluator: the reported source slice
/// is exactly the comma-not tail of the canonical specimen.
#[test]
fn c007_canonical_specimen_span_is_the_tail() {
    let t = "Findings judge house style, not authorship.\n";
    let report = run(t, Profile::Readme);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-C007")
        .expect("canonical specimen fires");
    let span = &f.spans[0];
    assert_eq!(&t[span.start..span.end], ", not authorship.");
    assert_eq!(common::snippet(f), ", not authorship.");
}

/// T1 sites inside code formatting are mentions, never prose: the engine's
/// segmentation must keep the canonical specimen silent when fenced.
#[test]
fn c007_quoted_in_code_never_fires() {
    let t = "Prose line.\n\n```\nFindings judge house style, not authorship.\n```\n";
    let report = run(t, Profile::Readme);
    assert!(
        !has_rule(&report, "SLOP-C007"),
        "C007 fired from inside a code fence"
    );
}
