//! SLOP-I006 hedging litotes and SLOP-I007 understatement hedge: every member
//! pinned to the span a writer rewrites, the honest negations that stay
//! silent under both rules, the seams with the contrast and filler rules, the
//! quoted-region behavior, and the profile stances. The SLOP-F002 word-edge
//! regression sits at the end.

mod common;

use ai_slop::Profile;
use common::{assert_invariants, has_rule, run};

fn hits(report: &ai_slop::Report, id: &str) -> Vec<String> {
    report
        .findings
        .iter()
        .filter(|f| f.rule_id == id)
        .map(common::snippet)
        .collect()
}

fn i006(report: &ai_slop::Report) -> Vec<String> {
    hits(report, "SLOP-I006")
}

fn i007(report: &ai_slop::Report) -> Vec<String> {
    hits(report, "SLOP-I007")
}

/// The fixed arm. One case per spelling, pinned to the phrase itself, and
/// every hit is a violation.
#[test]
fn i006_every_member_fires_on_its_span() {
    for (text, span) in [
        (
            "This is no small feat for a single maintainer.\n",
            "no small feat",
        ),
        (
            "Migrating the schema was no simple task.\n",
            "was no simple task",
        ),
        (
            "The rollout is not without its challenges.\n",
            "not without its challenges",
        ),
        (
            "Debugging the linker is far from trivial.\n",
            "far from trivial",
        ),
        (
            "It is hardly surprising that the build broke.\n",
            "hardly surprising",
        ),
        (
            "It's not exactly trivial to reproduce.\n",
            "not exactly trivial",
        ),
        (
            "The codebase isn't exactly simple.\n",
            "isn't exactly simple",
        ),
        (
            "The codebase isn\u{2019}t exactly simple.\n",
            "isn\u{2019}t exactly simple",
        ),
        ("Getting this to compile is no mean feat.\n", "no mean feat"),
        (
            "The result leaves something to be desired.\n",
            "leaves something to be desired",
        ),
        (
            "It would not be wrong to say the design is fragile.\n",
            "It would not be wrong to say",
        ),
        ("This is, to put it mildly, a mess.\n", "to put it mildly"),
        (
            "The API surface is not inconsiderable.\n",
            "not inconsiderable",
        ),
        (
            "Setup is not for the faint of heart.\n",
            "not for the faint of heart",
        ),
        ("The cost is not unimportant.\n", "not unimportant"),
        (
            "The change is not inconsequential.\n",
            "not inconsequential",
        ),
        (
            "The shape is not unfamiliar to the team.\n",
            "not unfamiliar",
        ),
        ("Shipping this was no easy feat.\n", "no easy feat"),
        ("That is not a small feat.\n", "not a small feat"),
        (
            "A green run is no small achievement.\n",
            "no small achievement",
        ),
        (
            "The port is no small undertaking.\n",
            "no small undertaking",
        ),
        (
            "The delay is due in no small part to the cache.\n",
            "in no small part",
        ),
        (
            "The gain comes in no small measure from batching.\n",
            "in no small measure",
        ),
        (
            "The upgrade remains no easy task.\n",
            "remains no easy task",
        ),
        (
            "The plan is not without difficulties.\n",
            "not without difficulties",
        ),
        ("The choice is not without irony.\n", "not without irony"),
        ("The fix is far from simple.\n", "far from simple"),
        ("The failure is hardly trivial.\n", "hardly trivial"),
        ("The numbers are less than stellar.\n", "less than stellar"),
        (
            "The docs leave much to be desired.\n",
            "leave much to be desired",
        ),
        (
            "It wouldn't be wrong to say the cache is dead.\n",
            "It wouldn't be wrong to say",
        ),
        ("It's safe to say the parser is done.\n", "It's safe to say"),
        ("It is not hard to see why.\n", "It is not hard to see"),
        (
            "It is not hard to imagine a crash here.\n",
            "It is not hard to imagine",
        ),
        (
            "The output was odd, to say the least.\n",
            "to say the least",
        ),
        ("The config is not rocket science.\n", "not rocket science"),
        (
            "This is not exactly a walk in the park.\n",
            "not exactly a walk in the park",
        ),
        (
            "The migration is not a walk in the park.\n",
            "not a walk in the park",
        ),
        (
            "The cost is not to be underestimated.\n",
            "not to be underestimated",
        ),
        (
            "The cost is not to be overlooked.\n",
            "not to be overlooked",
        ),
        (
            "The risk is not to be taken lightly.\n",
            "not to be taken lightly",
        ),
        (
            "The gain is not to be sneezed at.\n",
            "not to be sneezed at",
        ),
        (
            "The build is slow, not to mention flaky.\n",
            ", not to mention",
        ),
        ("This isn't rocket science.\n", "isn't rocket science"),
        (
            "The setup isn't for the faint of heart.\n",
            "isn't for the faint of heart",
        ),
        (
            "The risk isn't to be underestimated.\n",
            "isn't to be underestimated",
        ),
        (
            "The rollout wasn't without its challenges.\n",
            "wasn't without its challenges",
        ),
        (
            "The API surface isn't inconsiderable.\n",
            "isn't inconsiderable",
        ),
    ] {
        let report = run(text, Profile::PublicBugReport);
        assert_invariants(text, &report);
        assert_eq!(i006(&report), vec![span.to_string()], "{text:?}");
        assert!(i007(&report).is_empty(), "I007 also fired on {text:?}");
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "SLOP-I006")
            .unwrap();
        assert_eq!(f.state, "violation", "{text:?}");
    }
}

/// The contextual arm. One case per spelling, pinned to the phrase itself,
/// and every hit is a candidate.
#[test]
fn i007_every_member_fires_on_its_span() {
    for (text, span) in [
        (
            "It is not uncommon for the cache to miss.\n",
            "not uncommon",
        ),
        (
            "It is not entirely clear why the test flakes.\n",
            "not entirely clear",
        ),
        (
            "The current latency is less than ideal.\n",
            "less than ideal",
        ),
        ("The design is not without merit.\n", "not without merit"),
        (
            "Porting to ARM is not a trivial undertaking.\n",
            "not a trivial undertaking",
        ),
        ("The benchmark numbers are not the best.\n", "not the best"),
        ("The load is not insignificant.\n", "not insignificant"),
        (
            "The queue is not quite right after failover.\n",
            "not quite right",
        ),
        ("A retry is not unusual here.\n", "not unusual"),
        ("A second pass is not unreasonable.\n", "not unreasonable"),
        ("A crash is not unlikely under load.\n", "not unlikely"),
        ("The fix is not inexpensive.\n", "not inexpensive"),
        ("The result is not unexpected.\n", "not unexpected"),
        ("The team is not unaware of the gap.\n", "not unaware"),
        ("The change is not unwelcome.\n", "not unwelcome"),
        (
            "The parser is not incapable of recovery.\n",
            "not incapable",
        ),
        (
            "The field is not irrelevant to the digest.\n",
            "not irrelevant",
        ),
        ("The order is not illogical.\n", "not illogical"),
        ("A stale read is not unheard of.\n", "not unheard of"),
        ("Misses are not infrequent.\n", "not infrequent"),
        ("The cache misses not infrequently.\n", "not infrequently"),
        ("A rewrite is not a trivial task.\n", "not a trivial task"),
        ("This is not a trivial matter.\n", "not a trivial matter"),
        (
            "Recovery is not a trivial problem.\n",
            "not a trivial problem",
        ),
        ("This is not a trivial change.\n", "not a trivial change"),
        (
            "The port is not a trivial exercise.\n",
            "not a trivial exercise",
        ),
        ("The move is not an easy task.\n", "not an easy task"),
        ("Rollback is not a simple matter.\n", "not a simple matter"),
        ("The leak is not a minor issue.\n", "not a minor issue"),
        (
            "The patch took no small amount of work.\n",
            "no small amount",
        ),
        (
            "The plan is not without any risk.\n",
            "not without any risk",
        ),
        (
            "The plan is not without its risks.\n",
            "not without its risks",
        ),
        ("The proposal is not without value.\n", "not without value"),
        ("The cache is not without a cost.\n", "not without a cost"),
        (
            "The rule is not without precedent.\n",
            "not without precedent",
        ),
        (
            "The choice is not without controversy.\n",
            "not without controversy",
        ),
        ("The delay is not without reason.\n", "not without reason"),
        (
            "The retry is not without consequences.\n",
            "not without consequences",
        ),
        (
            "The docs are not entirely accurate.\n",
            "not entirely accurate",
        ),
        ("The claim is not entirely true.\n", "not entirely true"),
        ("The guess is not entirely wrong.\n", "not entirely wrong"),
        (
            "The result is not entirely surprising.\n",
            "not entirely surprising",
        ),
        (
            "The cause is not entirely obvious.\n",
            "not entirely obvious",
        ),
        ("The fix is not entirely certain.\n", "not entirely certain"),
        ("We are not entirely sure.\n", "not entirely sure"),
        ("The layout is not exactly ideal.\n", "not exactly ideal"),
        ("The path is not exactly fast.\n", "not exactly fast"),
        ("The port is not quite there.\n", "not quite there"),
        ("The budget is not quite enough.\n", "not quite enough"),
        ("The layout is not quite ideal.\n", "not quite ideal"),
        ("The path is not particularly fast.\n", "not particularly"),
        ("The path is not especially fast.\n", "not especially"),
        ("The code is not overly complex.\n", "not overly"),
        ("The API is not terribly consistent.\n", "not terribly"),
        ("The patch isn't quite right.\n", "isn't quite right"),
        ("The cause isn't entirely clear.\n", "isn't entirely clear"),
        ("Timeouts aren't uncommon under load.\n", "aren't uncommon"),
        ("The plan isn't without merit.\n", "isn't without merit"),
        ("The heap wasn't terribly fragmented.\n", "wasn't terribly"),
        ("The layout is far from ideal.\n", "far from ideal"),
        ("The fix is far from perfect.\n", "far from perfect"),
        ("The cause is far from certain.\n", "far from certain"),
        ("The cause is far from clear.\n", "far from clear"),
        ("The question is far from settled.\n", "far from settled"),
        ("The path is less than optimal.\n", "less than optimal"),
        (
            "It is not unreasonable to expect a retry.\n",
            "It is not unreasonable to",
        ),
        ("The result is not the worst.\n", "not the worst"),
        ("The error text could be better.\n", "could be better"),
        (
            "The docs have room for improvement.\n",
            "room for improvement",
        ),
    ] {
        let report = run(text, Profile::PublicBugReport);
        assert_invariants(text, &report);
        assert_eq!(i007(&report), vec![span.to_string()], "{text:?}");
        assert!(i006(&report).is_empty(), "I006 also fired on {text:?}");
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "SLOP-I007")
            .unwrap();
        assert_eq!(f.state, "candidate", "{text:?}");
    }
}

/// Honest negations. Proof and probability, comparison, code review
/// tolerance, the quantifier, the bare distance words, and the frames the
/// spec lists as stated misses stay silent under both rules.
#[test]
fn honest_negations_are_silent_under_both_rules() {
    for text in [
        "The tool does not support nested transactions.\n",
        "Not all inputs are validated before parsing.\n",
        "The buffer is not fully drained when close returns.\n",
        "The cache is not always warm after failover.\n",
        "A successful write does not necessarily imply durability.\n",
        "A collision is not impossible because the hash has finite width.\n",
        "The patch is not incorrect, but it omits the 32-bit path.\n",
        "The two crashes are not unrelated.\n",
        "The wire format is not unlike CBOR.\n",
        "No simple task may invoke the privileged runner.\n",
        "The buffer holds not less than 32 bytes.\n",
        "The response is not without a Content-Type header.\n",
        "The node has barely enough memory for the job.\n",
        "The hook hardly ever fires under load.\n",
        "The value is not exactly 1.0 after rounding.\n",
        "The queue is not yet initialized.\n",
        "The field is not included in the digest.\n",
        "The token is not invalid; its status is unknown.\n",
        "She chose not to mention the delay.\n",
        "The two files are far from each other on disk.\n",
        "The two formats are not dissimilar.\n",
        "The count is not inaccurate.\n",
        "The reader is not indifferent to the order.\n",
        "The mirror is not unavailable during the sync.\n",
        "The export is not illegal in that region.\n",
        "The fix is not really a fix.\n",
        "The reply is not completely wrong.\n",
        "The node scarcely reaches the floor.\n",
        "The change is far from the hot path.\n",
        "The result is less than the threshold.\n",
        "A simple task runs in one pass.\n",
        "The park has a walk along the river.\n",
        "The mention of the delay came late.\n",
    ] {
        let report = run(text, Profile::PublicBugReport);
        assert!(
            i006(&report).is_empty(),
            "I006 fired on honest negation {text:?}: {:?}",
            i006(&report)
        );
        assert!(
            i007(&report).is_empty(),
            "I007 fired on honest negation {text:?}: {:?}",
            i007(&report)
        );
    }
}

/// The two-sentence reframe still reports SLOP-C002, and a litotes inside
/// the same block reports beside it. No rule coalesces across rules.
#[test]
fn seam_with_the_contrast_rules() {
    let t = "It is not a linter. It is a gate, and the setup is no small feat.\n";
    let report = run(t, Profile::PublicBugReport);
    assert_invariants(t, &report);
    assert!(has_rule(&report, "SLOP-C002"));
    assert_eq!(i006(&report), vec!["no small feat"]);

    let t = "The build is slow, not to mention flaky.\n";
    let report = run(t, Profile::PublicBugReport);
    assert!(has_rule(&report, "SLOP-C007"));
    assert_eq!(i006(&report), vec![", not to mention"]);
}

/// The filler lexicon owns the saying frame, so the sentence reports
/// SLOP-T001 once and never SLOP-I006.
#[test]
fn it_goes_without_saying_belongs_to_the_filler_rule() {
    let t = "It goes without saying that tests must pass.\n";
    let report = run(t, Profile::PublicBugReport);
    assert_eq!(hits(&report, "SLOP-T001"), vec!["It goes without saying"]);
    assert!(i006(&report).is_empty());
}

/// Inline code and fenced code are segmented out, so a document that quotes
/// the shapes in code can describe them without firing.
#[test]
fn code_regions_are_silent() {
    for text in [
        "The tell is `no small feat` in a README, and `not uncommon` in a comment.\n",
        "```\nThis is no small feat.\nIt is not uncommon.\n```\n",
    ] {
        let report = run(text, Profile::Readme);
        assert!(i006(&report).is_empty(), "{text:?}: {:?}", i006(&report));
        assert!(i007(&report).is_empty(), "{text:?}: {:?}", i007(&report));
    }
}

/// A claimed quotation downgrades the fixed arm to candidate and drops the
/// contextual arm, since a quoted hedge belongs to the quoted author.
#[test]
fn quoted_regions_downgrade_the_fixed_arm_and_drop_the_contextual_arm() {
    let t = "> This is no small feat for a single maintainer.\n";
    let report = run(t, Profile::Readme);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-I006")
        .expect("I006 reports inside the quotation");
    assert_eq!(f.state, "candidate");
    assert_eq!(f.provenance, "claimed-quotation");

    let t = "> It is not uncommon for the cache to miss.\n";
    let report = run(t, Profile::Readme);
    assert!(i007(&report).is_empty(), "{:?}", i007(&report));
}

/// The fixed arm applies at violation tier in every profile, internal-doc
/// included. The contextual arm relaxes to advisory in internal-doc and
/// blocks as a candidate everywhere else.
#[test]
fn profile_stances() {
    let a = "This is no small feat for a single maintainer.\n";
    let b = "It is not uncommon for the cache to miss.\n";
    let profiles = [
        Profile::PublicBugReport,
        Profile::PublicComment,
        Profile::Readme,
        Profile::Changelog,
        Profile::ReleaseNotes,
        Profile::CommitMessage,
        Profile::ApiDocs,
        Profile::InternalDoc,
    ];
    for profile in profiles {
        let report = run(a, profile);
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "SLOP-I006")
            .unwrap_or_else(|| panic!("I006 silent under {profile:?}"));
        assert_eq!(f.state, "violation", "{profile:?}");
        assert_eq!(f.lifecycle, "blocking", "{profile:?}");
    }
    for profile in profiles {
        let report = run(b, profile);
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "SLOP-I007")
            .unwrap_or_else(|| panic!("I007 silent under {profile:?}"));
        assert_eq!(f.state, "candidate", "{profile:?}");
        if profile == Profile::InternalDoc {
            assert_eq!(f.lifecycle, "advisory");
            assert_eq!(report.result_state, "no_findings");
        } else {
            assert_eq!(f.lifecycle, "blocking", "{profile:?}");
        }
    }
}

/// Regression: SLOP-F002 matched `re-verified` as a substring inside
/// `signature-verified`, and a README line saying that capture claims are
/// read and never signature-verified failed the gate. Every entry now
/// matches on word edges. A hyphen compound ending in `verified` or
/// `confirmed` names a mechanism.
#[test]
fn f002_is_silent_inside_signature_verified() {
    let t = "# Title\n\nCapture claims are read, not signature-verified.\n";
    let report = run(t, Profile::Readme);
    assert_invariants(t, &report);
    assert!(
        !has_rule(&report, "SLOP-F002"),
        "F002 fired inside signature-verified: {:?}",
        hits(&report, "SLOP-F002")
    );
    for text in [
        "The fix is unverified.\n",
        "The report is unconfirmed.\n",
        "The digest is hash-confirmed on read.\n",
    ] {
        let report = run(text, Profile::Readme);
        assert!(!has_rule(&report, "SLOP-F002"), "{text:?}");
    }
}

/// Keep-test: the standalone word `re-verified` still fires, at the start of
/// a sentence, mid-sentence, and before a period.
#[test]
fn f002_standalone_re_verified_still_fires() {
    for (text, span) in [
        ("We re-verified the fix after the patch.\n", "re-verified"),
        ("Re-verified on a clean checkout.\n", "Re-verified"),
        ("The result was re-verified.\n", "re-verified"),
    ] {
        let report = run(text, Profile::Readme);
        assert_eq!(
            hits(&report, "SLOP-F002"),
            vec![span.to_string()],
            "{text:?}"
        );
    }
}

/// The plain verification claims fire as before.
#[test]
fn f002_plain_claims_still_fire() {
    for (text, span) in [
        ("The bug was confirmed by two runs.\n", "confirmed by"),
        ("The patch was double-checked.\n", "double-checked"),
        ("The change was reviewed twice.\n", "was reviewed"),
        ("The result is verified.\n", "verified"),
    ] {
        let report = run(text, Profile::Readme);
        assert_eq!(
            hits(&report, "SLOP-F002"),
            vec![span.to_string()],
            "{text:?}"
        );
    }
}

/// SLOP-K008 and SLOP-T003 had the same shape as SLOP-F002: a single-word
/// entry read as the tail of a hyphenated compound. The negated guarantee
/// compounds and the audio compound stay silent, and the plain entries and
/// the verb forms keep firing.
#[test]
fn k008_and_t003_match_on_word_edges() {
    for text in [
        "Delivery is non-guaranteed under backpressure.\n",
        "The order is un-guaranteed across shards.\n",
        "The map is non-thread-safe.\n",
        "The queue is non-lock-free.\n",
        "The compare is non-constant-time on this target.\n",
    ] {
        let report = run(text, Profile::ApiDocs);
        assert!(
            !has_rule(&report, "SLOP-K008"),
            "{text:?}: {:?}",
            hits(&report, "SLOP-K008")
        );
    }
    for (text, span) in [
        ("Delivery is guaranteed under backpressure.\n", "guaranteed"),
        ("The map is thread-safe.\n", "thread-safe"),
        ("The queue is lock-free.\n", "lock-free"),
    ] {
        let report = run(text, Profile::ApiDocs);
        assert_eq!(
            hits(&report, "SLOP-K008"),
            vec![span.to_string()],
            "{text:?}"
        );
    }
    for text in [
        "Remastering the samples took an hour.\n",
        "The audio re-mastering step took an hour.\n",
    ] {
        let report = run(text, Profile::Readme);
        assert!(
            !has_rule(&report, "SLOP-T003"),
            "{text:?}: {:?}",
            hits(&report, "SLOP-T003")
        );
    }
    for (text, span) in [
        ("Mastering the CLI takes a week.\n", "Mastering"),
        ("This guide demystifies the parser.\n", "demystifies"),
        ("The parser is demystified in chapter two.\n", "demystified"),
        ("Let's dive in.\n", "Let's dive in"),
    ] {
        let report = run(text, Profile::Readme);
        assert_eq!(
            hits(&report, "SLOP-T003"),
            vec![span.to_string()],
            "{text:?}"
        );
    }
}
