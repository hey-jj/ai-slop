//! Section 12.12: for every rule x profile pair the resolved stance matches
//! the policy package, including commit field splits, the internal-doc
//! exemptions, and the public-bug-report tightenings.

use ai_slop::policy::{self, Tier};
use ai_slop::{Field, Profile, Stance};

fn expected_stance(rule_tbl: &toml::Value, profile: &str, field: &str) -> Stance {
    let profiles = rule_tbl
        .get("profiles")
        .and_then(|v| v.as_table())
        .expect("profiles table");
    let entry = profiles.get(profile).or_else(|| profiles.get("default"));
    let s = match entry {
        None => "apply",
        Some(toml::Value::String(s)) => s.as_str(),
        Some(toml::Value::Table(t)) => t.get(field).and_then(|v| v.as_str()).unwrap_or("apply"),
        Some(_) => panic!("bad stance value"),
    };
    match s {
        "apply" => Stance::Apply,
        "relax" => Stance::Relax,
        "off" => Stance::Off,
        other => panic!("unknown stance {other}"),
    }
}

#[test]
fn full_matrix_matches_the_package() {
    let pkg = policy::load().unwrap();
    let root: toml::Value = toml::from_str(policy::POLICY_TOML).unwrap();
    let raw_rules = root.get("rule").and_then(|v| v.as_array()).unwrap();
    assert_eq!(pkg.rules.len(), raw_rules.len());

    for (rule, raw) in pkg.rules.iter().zip(raw_rules) {
        assert_eq!(
            rule.id,
            raw.get("id").and_then(|v| v.as_str()).unwrap(),
            "rule order drift"
        );
        for profile in Profile::ALL {
            for (field, name) in [
                (Field::Subject, "subject"),
                (Field::Body, "body"),
                (Field::Trailers, "trailers"),
            ] {
                let got = rule.stance(profile, field);
                let want = expected_stance(raw, profile.as_str(), name);
                assert_eq!(got, want, "{} x {} field {name}", rule.id, profile.as_str());
            }
        }
    }
}

#[test]
fn known_matrix_points() {
    let pkg = policy::load().unwrap();
    let rule = |id: &str| pkg.rule_by_id(id).unwrap();

    // internal-doc exemptions: scrub off, process-facts off.
    assert_eq!(
        rule("SLOP-W001").stance(Profile::InternalDoc, Field::Whole),
        Stance::Off
    );
    assert_eq!(
        rule("SLOP-F001").stance(Profile::InternalDoc, Field::Whole),
        Stance::Off
    );
    assert_eq!(
        rule("SLOP-F002").stance(Profile::InternalDoc, Field::Whole),
        Stance::Off
    );
    // public-bug-report tightenings.
    assert_eq!(
        rule("SLOP-S002").stance(Profile::PublicBugReport, Field::Whole),
        Stance::Apply
    );
    assert_eq!(
        rule("SLOP-S002").stance(Profile::Readme, Field::Whole),
        Stance::Off
    );
    assert_eq!(
        rule("SLOP-I002").stance(Profile::PublicBugReport, Field::Whole),
        Stance::Apply
    );
    assert_eq!(
        rule("SLOP-I002").stance(Profile::Readme, Field::Whole),
        Stance::Relax
    );
    // commit field splits.
    assert_eq!(
        rule("SLOP-M001").stance(Profile::CommitMessage, Field::Trailers),
        Stance::Off
    );
    assert_eq!(
        rule("SLOP-M001").stance(Profile::CommitMessage, Field::Body),
        Stance::Apply
    );
    assert_eq!(
        rule("SLOP-S001").stance(Profile::CommitMessage, Field::Trailers),
        Stance::Off
    );
    // v0.1.5 additions: C007 applies everywhere except api-docs relax; W002
    // follows the scrub family's internal-doc exemption plus api-docs relax.
    assert_eq!(
        rule("SLOP-C007").stance(Profile::Readme, Field::Whole),
        Stance::Apply
    );
    assert_eq!(
        rule("SLOP-C007").stance(Profile::ApiDocs, Field::Whole),
        Stance::Relax
    );
    assert_eq!(
        rule("SLOP-C007").stance(Profile::InternalDoc, Field::Whole),
        Stance::Apply
    );
    assert_eq!(
        rule("SLOP-W002").stance(Profile::Readme, Field::Whole),
        Stance::Apply
    );
    assert_eq!(
        rule("SLOP-W002").stance(Profile::ApiDocs, Field::Whole),
        Stance::Relax
    );
    assert_eq!(
        rule("SLOP-W002").stance(Profile::InternalDoc, Field::Whole),
        Stance::Off
    );
}

#[test]
fn tier_counts_are_pinned() {
    let pkg = policy::load().unwrap();
    assert_eq!(pkg.rules.len(), 75);
    let count = |t: Tier| pkg.rules.iter().filter(|r| r.tier == t).count();
    assert_eq!(count(Tier::Violation), 30);
    assert_eq!(count(Tier::Candidate), 41);
    assert_eq!(count(Tier::CoverageHint), 4);
}
