//! Section 12.4: per-rule predicate tests, generated over the package. For
//! every word-set rule, a prose use of its first term fires in a profile
//! where the rule applies, and the same term inside a code fence does not.

mod common;

use ai_slop::policy::{self, MatchKindSpec, Scope, View};
use ai_slop::{analyze, Config, Field, InputFormat, Profile, Stance};

fn first_active_profile(rule: &policy::Rule) -> Option<Profile> {
    Profile::ALL
        .into_iter()
        .find(|p| rule.stance(*p, Field::Whole) != Stance::Off)
}

fn term_is_exempt(rule: &policy::Rule, term: &str) -> bool {
    let lower = term.to_lowercase();
    rule.exemptions.iter().any(|e| e.contains(&lower))
}

#[test]
fn every_word_set_rule_fires_on_a_prose_positive() {
    let pkg = policy::load().unwrap();
    for rule in &pkg.rules {
        if rule.kind != MatchKindSpec::WordSet {
            continue;
        }
        if rule.lifecycle == policy::Lifecycle::Deprecated {
            continue;
        }
        // Scoped rules (link-url, comment) get their own targeted tests.
        if rule.scope != Scope::None {
            continue;
        }
        let Some(term) = rule.terms.iter().find(|t| !term_is_exempt(rule, t)) else {
            continue;
        };
        let Some(profile) = first_active_profile(rule) else {
            continue;
        };
        let (text, format) = if profile == Profile::CommitMessage {
            (
                format!("feat: subject\n\n{term} in the body.\n"),
                InputFormat::Commit,
            )
        } else if profile == Profile::CargoMetadata {
            (
                format!(
                    "[package]\nname = \"t\"\nversion = \"1.0.0\"\ndescription = \"{} in the description\"\nkeywords = [\"a\",\"b\",\"c\",\"d\",\"e\"]\ncategories = [\"parsing\",\"no-std\"]\n",
                    term.replace('"', "")
                ),
                InputFormat::Manifest,
            )
        } else {
            (
                format!("{term} appears in prose here.\n"),
                profile.default_format(),
            )
        };
        let mut config = Config::new(profile);
        config.input_format = format;
        let report = analyze(text.as_bytes(), &config).expect(&rule.id);
        assert!(
            report.findings.iter().any(|f| f.rule_id == rule.id),
            "{} did not fire on term {term:?} in profile {} (text {text:?})",
            rule.id,
            profile.as_str()
        );
    }
}

#[test]
fn no_word_set_rule_fires_from_inside_a_code_fence() {
    let pkg = policy::load().unwrap();
    for rule in &pkg.rules {
        if rule.kind != MatchKindSpec::WordSet || rule.lifecycle == policy::Lifecycle::Deprecated {
            continue;
        }
        // The injection family scans all regions by design; raw-view and
        // scoped rules are outside the prose segmentation guarantee.
        if rule.id == "SLOP-J001" || rule.view == View::Raw || rule.scope != Scope::None {
            continue;
        }
        let Some(profile) = first_active_profile(rule) else {
            continue;
        };
        if profile.default_format() != InputFormat::Markdown {
            continue;
        }
        let term = &rule.terms[0];
        let text = format!("Prose line.\n\n```\n{term}\n```\n");
        let config = Config::new(profile);
        let report = analyze(text.as_bytes(), &config).unwrap();
        assert!(
            !report.findings.iter().any(|f| f.rule_id == rule.id),
            "{} fired from inside a code fence on {term:?}",
            rule.id
        );
    }
}

#[test]
fn commit_subject_format_checks() {
    let mut config = Config::new(Profile::CommitMessage);
    config.input_format = InputFormat::Commit;

    let good = b"feat(parser): add span table\n\nExplains why.\n";
    let report = analyze(good, &config).unwrap();
    assert!(!report
        .findings
        .iter()
        .any(|f| f.rule_id == "SLOP-K002" && f.state == "violation"));

    for bad in [
        &b"Add span table\n"[..],
        &b"feat: add span table.\n"[..],
        &b"no conventional prefix here\n"[..],
    ] {
        let report = analyze(bad, &config).unwrap();
        assert!(
            report.findings.iter().any(|f| f.rule_id == "SLOP-K002"),
            "K002 missed {:?}",
            String::from_utf8_lossy(bad)
        );
    }

    let long = format!("feat: {}\n", "x".repeat(80));
    let report = analyze(long.as_bytes(), &config).unwrap();
    assert!(report.findings.iter().any(|f| f.rule_id == "SLOP-K002"));
}

// --- v0.1.5 FP narrowing: SLOP-F001 `I/O`, SLOP-V003 boundary flip ---------

/// F001 regression quartet: the `i = ["i/o"]` exemption kills the I/O false
/// positive while both genuine first-person markers keep firing.
#[test]
fn f001_io_exemption_quartet() {
    let config = Config::new(Profile::PublicBugReport);
    for benign in [
        "The bug corrupts I/O buffers on retry.\n",
        "Async I/O is slower on this path.\n",
    ] {
        let report = analyze(benign.as_bytes(), &config).unwrap();
        assert!(
            !report.findings.iter().any(|f| f.rule_id == "SLOP-F001"),
            "F001 fired on I/O in {benign:?}"
        );
    }
    for genuine in [
        "I ran the reproduction twice.\n",
        "We observed the failure under load.\n",
    ] {
        let report = analyze(genuine.as_bytes(), &config).unwrap();
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "SLOP-F001")
            .unwrap_or_else(|| panic!("F001 silent on {genuine:?}"));
        assert_eq!(f.state, "candidate");
    }
}

/// V003 regression quartet: the rule-wide boundary flip from none to word
/// kills the whole CLI/CI/API/GUI mid-token class, while phrase-edge word
/// boundaries keep every genuine offer firing.
#[test]
fn v003_word_boundary_quartet() {
    let config = Config::new(Profile::PublicBugReport);
    for benign in [
        "The CLI can also emit JSON.\n",
        "The API can also stream results.\n",
        "The GUI can also render a preview.\n",
    ] {
        let report = analyze(benign.as_bytes(), &config).unwrap();
        assert!(
            !report.findings.iter().any(|f| f.rule_id == "SLOP-V003"),
            "V003 fired mid-token in {benign:?}"
        );
    }
    let report = analyze(b"I can also update the docs if that helps.\n", &config).unwrap();
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-V003")
        .expect("genuine offer still fires");
    assert_eq!(f.state, "candidate");
}

/// The canary for the rule-wide flip: a multi-word entry ending mid-sentence
/// still matches with word-bounded edges.
#[test]
fn v003_multiword_entry_survives_the_boundary_flip() {
    let config = Config::new(Profile::PublicBugReport);
    let report = analyze(b"Let me know if you need anything else.\n", &config).unwrap();
    assert!(
        report.findings.iter().any(|f| f.rule_id == "SLOP-V003"),
        "multi-word V003 entry lost to the boundary flip"
    );
}

// --- SLOP-W002 oblique-provenance ------------------------------------------

/// Owner-approved provenance markers fire as candidates on the readme
/// profile (a hot profile via the default stance).
#[test]
fn w002_provenance_positives_fire_candidate_on_readme() {
    let config = Config::new(Profile::Readme);
    for text in [
        "The parser was reimplemented from scratch.\n",
        "Kept for API parity with the old interface.\n",
        "A drop-in replacement for serde_yaml.\n",
        "This crate is a reference implementation.\n",
        "It maintains parity with the original crate.\n",
    ] {
        let report = analyze(text.as_bytes(), &config).unwrap();
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "SLOP-W002")
            .unwrap_or_else(|| panic!("W002 silent on {text:?}"));
        assert_eq!(f.state, "candidate", "{text:?}");
    }
}

/// Domain uses of `provenance` (data, supply-chain) still fire and reach the
/// judge: adjudicating domain legitimacy is the human's call, never an
/// exemption. The assertion pins presence AND tier.
#[test]
fn w002_domain_provenance_reaches_the_judge_as_candidate() {
    let config = Config::new(Profile::Readme);
    let text = b"The build records supply-chain provenance for each artifact.\n";
    let report = analyze(text, &config).unwrap();
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SLOP-W002")
        .expect("domain provenance is a candidate for the judge, not an exemption");
    assert_eq!(f.state, "candidate");
}

/// De-dup against W001: `reference implementation of` is W001's violation and
/// W002 stays silent there; the bare noun phrase is W002's and W001 stays
/// silent.
#[test]
fn w002_dedups_reference_implementation_against_w001() {
    let config = Config::new(Profile::Readme);
    let report = analyze(
        b"The reference implementation of the algorithm is linked.\n",
        &config,
    )
    .unwrap();
    assert!(
        report.findings.iter().any(|f| f.rule_id == "SLOP-W001"),
        "W001 owns the trailing-of form"
    );
    assert!(
        !report.findings.iter().any(|f| f.rule_id == "SLOP-W002"),
        "W002 must not double-report the W001 form"
    );

    let report = analyze(b"This crate is a reference implementation.\n", &config).unwrap();
    assert!(
        report.findings.iter().any(|f| f.rule_id == "SLOP-W002"),
        "the bare form is W002's"
    );
    assert!(
        !report.findings.iter().any(|f| f.rule_id == "SLOP-W001"),
        "W001 requires the trailing of"
    );
}

/// internal-doc keeps its W001 stance for W002: naming process is the content
/// of internal docs, so the scrub family is off there.
#[test]
fn w002_internal_doc_is_off() {
    let config = Config::new(Profile::InternalDoc);
    let report = analyze(
        b"The parser was reimplemented; provenance is tracked per artifact.\n",
        &config,
    )
    .unwrap();
    assert!(
        !report.findings.iter().any(|f| f.rule_id == "SLOP-W002"),
        "W002 must be off for internal-doc"
    );
}
