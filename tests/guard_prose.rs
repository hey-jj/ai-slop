//! The guard-prose gate. Guard and judge text is hand-written and ships inside
//! the snapshot, so it holds to the same writing rules as any other outbound
//! prose. This test reads the loaded package and fails on the classes a machine
//! can settle: the punctuation the house style bans, the contrast scaffolding,
//! and the filler words.
//!
//! Scope is two string fields per rule. Patterns, params, and lexicons are data
//! the rules match on and are never read here, since a lexicon of banned words
//! is supposed to contain banned words.
//!
//! A guard has to be able to name the terms its own rule catches, so a word
//! class finding is dropped when the matched text is itself a declared term or
//! sits inside a declared pattern. That exemption never reaches the punctuation
//! classes: an em dash in a guard is an em dash whatever the rule is about.

use std::collections::BTreeSet;

/// One thing the gate objects to.
#[derive(Debug, PartialEq)]
struct Violation {
    rule: String,
    field: &'static str,
    class: &'static str,
    span: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}: {} {:?}",
            self.rule, self.field, self.class, self.span
        )
    }
}

/// Everything the package declares as data, which a guard may quote.
struct Declared {
    terms: BTreeSet<String>,
    patterns: Vec<String>,
}

impl Declared {
    fn covers(&self, span: &str) -> bool {
        let s = span.to_lowercase();
        self.terms.contains(&s) || self.patterns.iter().any(|p| p.contains(&s))
    }
}

const PUNCTUATION: &[(char, &str)] = &[
    ('\u{2014}', "em dash"),
    ('\u{2013}', "en dash"),
    (';', "semicolon"),
];

const FILLER: &[&str] = &[
    "it's important to note",
    "in summary",
    "in conclusion",
    "overall",
    "delve",
    "robust",
    "leverage",
    "elevate",
    "unlock",
    "seamless",
    "game-changer",
];

/// True when `at..end` sits on word edges. A hyphen counts as inside a word, so
/// `game-changer` does not match through `game-changers`.
fn word_bounded(hay: &str, at: usize, end: usize) -> bool {
    let inside = |c: char| c.is_alphanumeric() || c == '-' || c == '\'';
    let before = hay[..at].chars().next_back().map(inside).unwrap_or(false);
    let after = hay[end..].chars().next().map(inside).unwrap_or(false);
    !before && !after
}

/// Word-bounded, case-insensitive occurrences of `needle`, as byte ranges.
fn find_all(hay_lower: &str, needle: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(pos) = hay_lower[at..].find(needle) {
        let s = at + pos;
        let e = s + needle.len();
        if word_bounded(hay_lower, s, e) {
            out.push((s, e));
        }
        at = s + 1;
    }
    out
}

/// The whole gate over one field. Kept separate from the package walk so the
/// negative control can drive it with text of its own.
fn check(rule: &str, field: &'static str, text: &str, declared: &Declared) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut push = |class: &'static str, span: String| {
        out.push(Violation {
            rule: rule.to_string(),
            field,
            class,
            span,
        })
    };
    for (ch, class) in PUNCTUATION {
        if text.contains(*ch) {
            push(class, ch.to_string());
        }
    }
    // The typographic apostrophe folds so one spelling of the phrase list
    // matches both.
    let lower = text.to_lowercase().replace('\u{2019}', "'");
    for word in FILLER {
        for (s, e) in find_all(&lower, word) {
            if !declared.covers(&lower[s..e]) {
                push("filler", lower[s..e].to_string());
            }
        }
    }
    // Contrast scaffolding. The paired forms are read as a first half with its
    // partner following inside one sentence's worth of text.
    for (s, _) in find_all(&lower, "not only") {
        let window = &lower[s..lower.len().min(s + 100)];
        if window.contains("but also") && !declared.covers(window) {
            push("not only X but also", window.to_string());
        }
    }
    for (s, e) in find_all(&lower, "on the one hand") {
        if !declared.covers(&lower[s..e]) {
            push("on the one hand", lower[s..e].to_string());
        }
    }
    // The opener shapes are anchored, since both words are ordinary
    // mid-sentence.
    let head = lower.trim_start();
    if let Some(rest) = head.strip_prefix("while ") {
        if let Some(comma) = rest.find(',') {
            if comma <= 60 {
                push("while X, Y opener", format!("while {}", &rest[..comma]));
            }
        }
    }
    if head.starts_with("however ") || head.starts_with("however,") {
        push("however opener", "however".to_string());
    }
    out
}

fn declared(pkg: &ai_slop::policy::PolicyPackage) -> Declared {
    let mut terms = BTreeSet::new();
    let mut patterns = Vec::new();
    for rule in &pkg.rules {
        for t in &rule.terms {
            terms.insert(t.to_lowercase());
        }
        for p in &rule.patterns {
            patterns.push(p.to_lowercase());
        }
    }
    Declared { terms, patterns }
}

/// Every guard and judge string in the package holds to the writing rules.
#[test]
fn guard_and_judge_prose_holds_the_house_style() {
    let cp = ai_slop::engine::compiled().expect("policy compiles");
    let d = declared(&cp.pkg);
    let mut found: Vec<Violation> = Vec::new();
    for rule in &cp.pkg.rules {
        found.extend(check(&rule.id, "guard", &rule.guard, &d));
        if let Some(judge) = &rule.judge {
            found.extend(check(&rule.id, "judge", judge, &d));
        }
    }
    assert!(
        found.is_empty(),
        "guard prose violations:\n{}",
        found
            .iter()
            .map(|v| format!("  {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The gate reads two fields and no others. A lexicon of banned words is
/// supposed to contain banned words, and a pattern that matches `delve` has to
/// spell it.
#[test]
fn the_gate_reads_only_guard_and_judge() {
    let cp = ai_slop::engine::compiled().expect("policy compiles");
    let ornamental = cp
        .pkg
        .rules
        .iter()
        .find(|r| r.id == "SLOP-A001")
        .expect("A001 is in the package");
    assert!(
        ornamental.terms.iter().any(|t| t == "delve"),
        "the ornamental lexicon carries the word the gate would object to"
    );
    let d = declared(&cp.pkg);
    assert!(
        check("SLOP-A001", "guard", &ornamental.guard, &d).is_empty(),
        "a rule about banned words cannot describe itself"
    );
}

/// Negative control. The same function, fed prose that breaks every class,
/// reports every class. Without this the green run above proves nothing.
#[test]
fn the_gate_catches_a_deliberately_bad_guard() {
    let d = Declared {
        terms: BTreeSet::new(),
        patterns: Vec::new(),
    };
    let bad = "While the rule is narrow, it is robust \u{2014} and seamless; \
               it does not only delve but also unlock the full \u{2013} picture. \
               On the one hand it is a game-changer.";
    let found = check("SLOP-X999", "guard", bad, &d);
    let classes: BTreeSet<&str> = found.iter().map(|v| v.class).collect();
    for want in [
        "em dash",
        "en dash",
        "semicolon",
        "filler",
        "not only X but also",
        "on the one hand",
        "while X, Y opener",
    ] {
        assert!(
            classes.contains(want),
            "the gate missed {want}: {:?}",
            classes
        );
    }
    assert!(
        found
            .iter()
            .all(|v| v.rule == "SLOP-X999" && v.field == "guard"),
        "every finding names its rule and field"
    );
    let opener = check("SLOP-X999", "judge", "However, the rule stays.", &d);
    assert!(
        opener.iter().any(|v| v.class == "however opener"),
        "the opener class needs its own sentence: {opener:?}"
    );
}

/// The mention exemption reaches the word classes and never the punctuation.
#[test]
fn the_mention_exemption_stops_at_punctuation() {
    let d = Declared {
        terms: ["robust".to_string(), "\u{2014}".to_string()]
            .into_iter()
            .collect(),
        patterns: Vec::new(),
    };
    assert!(
        check("SLOP-X999", "guard", "The word robust fires here.", &d).is_empty(),
        "a declared term stays quotable"
    );
    let dashed = check("SLOP-X999", "guard", "The rule fires \u{2014} always.", &d);
    assert_eq!(dashed.len(), 1, "the dash is not exempt: {dashed:?}");
    assert_eq!(dashed[0].class, "em dash");
}
