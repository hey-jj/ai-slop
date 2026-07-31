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
