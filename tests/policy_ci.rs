//! Policy CI. Digest reproducibility, ID and implementation
//! completeness, guard and judge presence, lexicon reference uniqueness,
//! snapshot generation.

use ai_slop::policy;

#[test]
fn digest_is_reproducible_and_prefixed() {
    let a = policy::compute_digest();
    let b = policy::compute_digest();
    assert_eq!(a, b);
    assert!(a.starts_with("sha256:"));
    assert_eq!(a.len(), "sha256:".len() + 64);
}

#[test]
fn every_rule_resolves_to_an_implementation_symbol() {
    let pkg = policy::load().unwrap();
    let implemented = ai_slop::rules::implemented_rule_ids();
    for rule in &pkg.rules {
        assert!(
            implemented.contains(&rule.id.as_str()),
            "rule {} has no implementation symbol",
            rule.id
        );
    }
    for id in &implemented {
        assert!(
            pkg.rule_by_id(id).is_some(),
            "implementation {id} has no policy rule"
        );
    }
}

#[test]
fn every_rule_has_guard_tier_lifecycle_and_profiles() {
    let pkg = policy::load().unwrap();
    for rule in &pkg.rules {
        assert!(!rule.guard.is_empty(), "{} missing guard", rule.id);
        assert_eq!(rule.stances.len(), 9, "{} stances", rule.id);
        if rule.tier == policy::Tier::Candidate {
            assert!(rule.judge.is_some(), "{} missing judge question", rule.id);
        }
    }
}

#[test]
fn engines_compile_once_and_load() {
    let cp = ai_slop::engine::compiled().expect("policy compiles");
    assert_eq!(cp.pkg.rules.len(), 84);
}

#[test]
fn generated_snapshot_carries_the_policy_digest() {
    let cp = ai_slop::engine::compiled().unwrap();
    let generated = ai_slop::skill::generate(&cp.pkg);
    assert!(
        generated.contains(&cp.pkg.digest),
        "snapshot carries digest"
    );
}

/// Both quotation-semantics lists name real rules, and the suppress list
/// carries the metaphor-reach rule it was introduced for. Suppression is
/// the candidate-tier analog of the downgrade: no rule sits in both lists.
#[test]
fn quotation_semantics_lists_are_valid() {
    let pkg = policy::load().unwrap();
    for id in pkg
        .quotation_downgrade
        .iter()
        .chain(pkg.quotation_suppress.iter())
    {
        assert!(
            pkg.rule_by_id(id).is_some(),
            "quotation semantics list names unknown rule {id}"
        );
    }
    assert!(
        pkg.quotation_suppress.iter().any(|id| id == "SLOP-A005"),
        "SLOP-A005 must be quotation-suppressed"
    );
    for id in &pkg.quotation_suppress {
        assert!(
            !pkg.quotation_downgrade.contains(id),
            "{id} is in both quotation lists"
        );
    }
}

#[test]
fn owner_mandated_sets_are_marked() {
    let pkg = policy::load().unwrap();
    for id in ["SLOP-A001", "SLOP-A002"] {
        let r = pkg.rule_by_id(id).unwrap();
        assert_eq!(r.origin, "owner-mandate");
        assert!(r.human_only_waiver);
    }
    let j = pkg.rule_by_id("SLOP-J001").unwrap();
    assert!(j.human_only_waiver);
}

/// One tool-noun set serves both SLOP-C010 and SLOP-F004. It is declared on
/// C010's block and read there by both rules, so the pin is on the contents
/// and on the absence of a second declaration. An empty or shrunken set
/// leaves F004 unanchored and silent, which is why this is a test and not a
/// comment.
#[test]
fn the_shared_tool_noun_set_is_declared_once_and_carries_both_spellings() {
    let pkg = policy::load().unwrap();
    let c010 = pkg.rule_by_id("SLOP-C010").unwrap();
    let nouns: Vec<String> = c010
        .params
        .as_table()
        .and_then(|t| t.get("tool_nouns"))
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .expect("C010 declares the shared tool-noun set");
    for singular in [
        "tool", "linter", "crate", "rule", "check", "gate", "detector", "guard", "report",
        "finding", "score", "output", "result", "test",
    ] {
        assert!(
            nouns.iter().any(|n| n == singular),
            "shared set is missing {singular}"
        );
        let plural = format!("{singular}s");
        assert!(nouns.contains(&plural), "shared set is missing {plural}");
    }
    assert_eq!(
        nouns.len(),
        28,
        "the set carries 14 nouns in both spellings"
    );
    let f004 = pkg.rule_by_id("SLOP-F004").unwrap();
    assert!(
        f004.params
            .as_table()
            .is_none_or(|t| !t.contains_key("tool_nouns")),
        "F004 redeclares the shared set instead of reading C010's"
    );
}

/// Param-coverage gate: the violations a package's declared params raise
/// against the code-side implemented list plus the explicit disclosures.
/// Factored out so the synthetic-dead-param test can prove the gate fails.
fn param_gate_violations(pkg: &policy::PolicyPackage) -> Vec<String> {
    // (rule id, param key, file that must disclose it) — a declared param may
    // alternatively be explicitly disclosed as unimplemented. Empty today:
    // the dead params were stripped instead.
    const DISCLOSED: &[(&str, &str, &str)] = &[];
    let implemented = ai_slop::rules::implemented_param_keys();
    let mut out = Vec::new();
    for rule in &pkg.rules {
        let Some(table) = rule.params.as_table() else {
            continue;
        };
        for key in table.keys() {
            if implemented
                .iter()
                .any(|(r, k)| *r == rule.id && *k == key.as_str())
            {
                continue;
            }
            if let Some((_, _, file)) = DISCLOSED
                .iter()
                .find(|(r, k, _)| *r == rule.id && *k == key.as_str())
            {
                let text =
                    std::fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), file))
                        .unwrap_or_default();
                if text.contains(key.as_str()) {
                    continue;
                }
                out.push(format!(
                    "rule {} param {key} claims disclosure in {file} but the file does not mention it",
                    rule.id
                ));
                continue;
            }
            out.push(format!(
                "rule {} declares param {key} with no implementation symbol or disclosure",
                rule.id
            ));
        }
    }
    out
}

// A declared-but-dead structural param is how the H003 unusual-scripts
// silent false negative once shipped — the implemented-symbol check was
// rule-level only, so a param with no code behind it passed CI. Every
// declared param needs an implementation mapping or an explicit disclosure.
#[test]
fn every_declared_param_is_implemented_or_disclosed() {
    let pkg = policy::load().unwrap();
    let violations = param_gate_violations(&pkg);
    assert!(
        violations.is_empty(),
        "param-coverage gate:\n{}",
        violations.join("\n")
    );
}

// The gate itself must fail on a synthetic dead param (proves the meta-fix
// catches the next H003-class regression).
#[test]
fn param_gate_flags_a_synthetic_dead_param() {
    let mut pkg = policy::load().unwrap();
    let rule = pkg
        .rules
        .iter_mut()
        .find(|r| r.params.as_table().is_some())
        .expect("a rule with params");
    rule.params
        .as_table_mut()
        .unwrap()
        .insert("synthetic_dead_param".into(), toml::Value::Boolean(true));
    let violations = param_gate_violations(&pkg);
    assert!(
        violations
            .iter()
            .any(|v| v.contains("synthetic_dead_param")),
        "gate must flag the synthetic dead param: {violations:?}"
    );
}
