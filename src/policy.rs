//! Policy package loading, validation, and digest.
//!
//! The canonical package lives in `policy/` and is embedded at build time.
//! This module parses it into typed rules and computes the package digest
//! over a canonical serialization.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const POLICY_TOML: &str = include_str!("../policy/policy.toml");

/// Embedded lexicon files, keyed by their package-relative path.
pub const LEXICONS: &[(&str, &str)] = &[
    (
        "words/assistant-offers.txt",
        include_str!("../policy/words/assistant-offers.txt"),
    ),
    (
        "words/assistant-voice.txt",
        include_str!("../policy/words/assistant-voice.txt"),
    ),
    (
        "words/audience-runway.txt",
        include_str!("../policy/words/audience-runway.txt"),
    ),
    (
        "words/clarity-meta.txt",
        include_str!("../policy/words/clarity-meta.txt"),
    ),
    (
        "words/copula-avoidance.txt",
        include_str!("../policy/words/copula-avoidance.txt"),
    ),
    (
        "words/cutoff-disclaimers.txt",
        include_str!("../policy/words/cutoff-disclaimers.txt"),
    ),
    (
        "words/empty-qualifiers.txt",
        include_str!("../policy/words/empty-qualifiers.txt"),
    ),
    (
        "words/era-overuse.txt",
        include_str!("../policy/words/era-overuse.txt"),
    ),
    (
        "words/filler-meta.txt",
        include_str!("../policy/words/filler-meta.txt"),
    ),
    (
        "words/first-person.txt",
        include_str!("../policy/words/first-person.txt"),
    ),
    (
        "words/hype-adjectives.txt",
        include_str!("../policy/words/hype-adjectives.txt"),
    ),
    (
        "words/impact-framing.txt",
        include_str!("../policy/words/impact-framing.txt"),
    ),
    (
        "words/importance-adjectives.txt",
        include_str!("../policy/words/importance-adjectives.txt"),
    ),
    (
        "words/inflated-diction.txt",
        include_str!("../policy/words/inflated-diction.txt"),
    ),
    (
        "words/injection.txt",
        include_str!("../policy/words/injection.txt"),
    ),
    (
        "words/intensifiers.txt",
        include_str!("../policy/words/intensifiers.txt"),
    ),
    (
        "words/magnitude-claims.txt",
        include_str!("../policy/words/magnitude-claims.txt"),
    ),
    (
        "words/ornamental.txt",
        include_str!("../policy/words/ornamental.txt"),
    ),
    (
        "words/pleasantries.txt",
        include_str!("../policy/words/pleasantries.txt"),
    ),
    (
        "words/provider-artifacts.txt",
        include_str!("../policy/words/provider-artifacts.txt"),
    ),
    (
        "words/provider-attribution.txt",
        include_str!("../policy/words/provider-attribution.txt"),
    ),
    (
        "words/reassurance.txt",
        include_str!("../policy/words/reassurance.txt"),
    ),
    ("words/scrub.txt", include_str!("../policy/words/scrub.txt")),
    (
        "words/signature-lines.txt",
        include_str!("../policy/words/signature-lines.txt"),
    ),
    (
        "words/significance-inflation.txt",
        include_str!("../policy/words/significance-inflation.txt"),
    ),
    (
        "words/stock-openers.txt",
        include_str!("../policy/words/stock-openers.txt"),
    ),
    (
        "words/tracking-params.txt",
        include_str!("../policy/words/tracking-params.txt"),
    ),
    (
        "words/transition-openers.txt",
        include_str!("../policy/words/transition-openers.txt"),
    ),
    (
        "words/vague-attribution.txt",
        include_str!("../policy/words/vague-attribution.txt"),
    ),
    (
        "words/verification-claims.txt",
        include_str!("../policy/words/verification-claims.txt"),
    ),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tier {
    Violation,
    Candidate,
    CoverageHint,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Violation => "violation",
            Tier::Candidate => "candidate",
            Tier::CoverageHint => "coverage_hint",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lifecycle {
    Blocking,
    Advisory,
    Experimental,
    Deprecated,
}

impl Lifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Lifecycle::Blocking => "blocking",
            Lifecycle::Advisory => "advisory",
            Lifecycle::Experimental => "experimental",
            Lifecycle::Deprecated => "deprecated",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Raw,
    Prose,
    Norm,
    Rendered,
}

impl View {
    pub fn as_str(self) -> &'static str {
        match self {
            View::Raw => "raw",
            View::Prose => "prose",
            View::Norm => "norm",
            View::Rendered => "rendered",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MatchKindSpec {
    WordSet,
    RegexSet,
    Structural,
    Ratio,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    None,
    LinkUrl,
    Code,
    Heading,
    Comment,
}

use crate::Stance;

/// Per-profile stance with commit-message field splits. For non-commit
/// profiles the three fields are identical.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FieldStance {
    pub subject: Stance,
    pub body: Stance,
    pub trailers: Stance,
}

impl FieldStance {
    pub fn uniform(s: Stance) -> FieldStance {
        FieldStance {
            subject: s,
            body: s,
            trailers: s,
        }
    }

    pub fn for_field(&self, field: crate::Field) -> Stance {
        match field {
            crate::Field::Whole | crate::Field::Subject => self.subject,
            crate::Field::Body => self.body,
            crate::Field::Trailers => self.trailers,
        }
    }

    pub fn any_active(&self) -> bool {
        self.subject != Stance::Off || self.body != Stance::Off || self.trailers != Stance::Off
    }
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub family: String,
    pub tier: Tier,
    pub lifecycle: Lifecycle,
    pub origin: String,
    pub human_only_waiver: bool,
    pub view: View,
    pub kind: MatchKindSpec,
    pub lexicon: Option<String>,
    /// Resolved literal terms (lexicon entries plus inline words).
    pub terms: Vec<String>,
    pub case_sensitive: bool,
    pub boundary_word: bool,
    pub block_start: bool,
    pub scope: Scope,
    pub params: toml::Value,
    pub patterns: Vec<String>,
    pub guard: String,
    pub judge: Option<String>,
    /// Indexed by profile order in `[semantics].profile_names`.
    pub stances: Vec<FieldStance>,
    /// Exemption collocations. Any phrase covering a match suppresses it.
    pub exemptions: Vec<String>,
}

impl Rule {
    pub fn stance(&self, profile: crate::Profile, field: crate::Field) -> Stance {
        self.stances[profile.index()].for_field(field)
    }

    /// Active-profile bitmask, one bit per profile in package order.
    pub fn profile_mask(&self) -> u8 {
        let mut mask = 0u8;
        for (i, fs) in self.stances.iter().enumerate() {
            if fs.any_active() {
                mask |= 1 << i;
            }
        }
        mask
    }
}

#[derive(Clone, Debug)]
pub struct ProfileDef {
    pub name: String,
    pub format: String,
    pub core_rules: BTreeMap<String, String>,
    pub notes: String,
}

#[derive(Clone, Debug)]
pub struct PolicyPackage {
    pub version: String,
    pub digest: String,
    pub quotation_downgrade: Vec<String>,
    pub profile_names: Vec<String>,
    pub profiles: Vec<ProfileDef>,
    pub rules: Vec<Rule>,
}

impl PolicyPackage {
    pub fn rule_by_id(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id == id)
    }
}

/// Canonical serialization for the digest: the policy.toml body with the
/// digest value emptied and CRLF folded to LF, followed by each lexicon file
/// (path-sorted), each preceded by a NUL-delimited path header.
pub fn canonical_bytes() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"policy.toml\0");
    let toml_lf = POLICY_TOML.replace("\r\n", "\n");
    for line in toml_lf.split_inclusive('\n') {
        if line.trim_start().starts_with("digest = ") {
            out.extend_from_slice(b"digest = \"\"\n");
        } else {
            out.extend_from_slice(line.as_bytes());
        }
    }
    let mut files: Vec<(&str, &str)> = LEXICONS.to_vec();
    files.sort_by_key(|(p, _)| *p);
    for (path, content) in files {
        out.push(0);
        out.extend_from_slice(path.as_bytes());
        out.push(0);
        out.extend_from_slice(content.replace("\r\n", "\n").as_bytes());
    }
    out
}

pub fn compute_digest() -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn lexicon_terms(path: &str) -> Result<Vec<String>, String> {
    let (_, content) = LEXICONS
        .iter()
        .find(|(p, _)| *p == path)
        .ok_or_else(|| format!("lexicon {path} is not embedded"))?;
    let mut terms = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        terms.push(line.to_string());
    }
    if terms.is_empty() {
        return Err(format!("lexicon {path} has no terms"));
    }
    Ok(terms)
}

fn as_str(v: &toml::Value, what: &str) -> Result<String, String> {
    v.as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("{what} must be a string"))
}

fn parse_stance(s: &str, what: &str) -> Result<Stance, String> {
    match s {
        "apply" => Ok(Stance::Apply),
        "relax" => Ok(Stance::Relax),
        "off" => Ok(Stance::Off),
        other => Err(format!("{what}: unknown stance {other}")),
    }
}

fn parse_field_stance(v: &toml::Value, what: &str) -> Result<FieldStance, String> {
    match v {
        toml::Value::String(s) => Ok(FieldStance::uniform(parse_stance(s, what)?)),
        toml::Value::Table(t) => {
            let get = |k: &str| -> Result<Stance, String> {
                match t.get(k) {
                    Some(toml::Value::String(s)) => parse_stance(s, what),
                    Some(_) => Err(format!("{what}.{k} must be a string")),
                    None => Ok(Stance::Apply),
                }
            };
            Ok(FieldStance {
                subject: get("subject")?,
                body: get("body")?,
                trailers: get("trailers")?,
            })
        }
        _ => Err(format!("{what} must be a string or table")),
    }
}

/// Parse the embedded package. Returns an error string on any structural
/// defect. Callers surface this as `instrumentation_error`.
pub fn load() -> Result<PolicyPackage, String> {
    let root: toml::Value =
        toml::from_str(POLICY_TOML).map_err(|e| format!("policy.toml parse: {e}"))?;
    let table = root.as_table().ok_or("policy.toml root is not a table")?;

    let policy_tbl = table
        .get("policy")
        .and_then(|v| v.as_table())
        .ok_or("[policy] missing")?;
    let version = policy_tbl
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or("[policy].version missing")?
        .to_string();

    let semantics = table
        .get("semantics")
        .and_then(|v| v.as_table())
        .ok_or("[semantics] missing")?;
    let quotation_downgrade = semantics
        .get("quotation_downgrade")
        .and_then(|v| v.as_array())
        .ok_or("[semantics].quotation_downgrade missing")?
        .iter()
        .map(|v| as_str(v, "quotation_downgrade entry"))
        .collect::<Result<Vec<_>, _>>()?;
    let profile_names = semantics
        .get("profile_names")
        .and_then(|v| v.as_array())
        .ok_or("[semantics].profile_names missing")?
        .iter()
        .map(|v| as_str(v, "profile_names entry"))
        .collect::<Result<Vec<_>, _>>()?;
    if profile_names.len() != 8 {
        return Err(format!(
            "expected 8 profiles, found {}",
            profile_names.len()
        ));
    }
    for (i, p) in crate::Profile::ALL.iter().enumerate() {
        if profile_names[i] != p.as_str() {
            return Err(format!(
                "profile order mismatch at {i}: package says {}, crate says {}",
                profile_names[i],
                p.as_str()
            ));
        }
    }

    let profile_tbl = table
        .get("profile")
        .and_then(|v| v.as_table())
        .ok_or("[profile.*] missing")?;
    let mut profiles = Vec::new();
    for name in &profile_names {
        let def = profile_tbl
            .get(name)
            .and_then(|v| v.as_table())
            .ok_or_else(|| format!("[profile.{name}] missing"))?;
        let format = def
            .get("format")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("[profile.{name}].format missing"))?
            .to_string();
        let mut core_rules = BTreeMap::new();
        if let Some(cr) = def.get("core_rules").and_then(|v| v.as_table()) {
            for (k, v) in cr {
                core_rules.insert(k.clone(), as_str(v, "core rule stance")?);
            }
        }
        let notes = def
            .get("notes")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        profiles.push(ProfileDef {
            name: name.clone(),
            format,
            core_rules,
            notes,
        });
    }

    let rules_arr = table
        .get("rule")
        .and_then(|v| v.as_array())
        .ok_or("[[rule]] entries missing")?;
    let mut rules = Vec::new();
    let mut lexicon_uses: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for rv in rules_arr {
        let rt = rv.as_table().ok_or("rule entry is not a table")?;
        let id = rt
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("rule missing id")?
            .to_string();
        let get_str = |k: &str| -> Result<String, String> {
            rt.get(k)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| format!("rule {id} missing {k}"))
        };
        let name = get_str("name")?;
        let family = get_str("family")?;
        let tier = match get_str("tier")?.as_str() {
            "violation" => Tier::Violation,
            "candidate" => Tier::Candidate,
            "coverage_hint" => Tier::CoverageHint,
            other => return Err(format!("rule {id}: unknown tier {other}")),
        };
        let lifecycle = match get_str("lifecycle")?.as_str() {
            "blocking" => Lifecycle::Blocking,
            "advisory" => Lifecycle::Advisory,
            "experimental" => Lifecycle::Experimental,
            "deprecated" => Lifecycle::Deprecated,
            other => return Err(format!("rule {id}: unknown lifecycle {other}")),
        };
        let origin = get_str("origin")?;
        let human_only_waiver = rt
            .get("waiver")
            .and_then(|v| v.as_str())
            .map(|s| s == "human-only")
            .unwrap_or(false);
        let view = match get_str("view")?.as_str() {
            "raw" => View::Raw,
            "prose" => View::Prose,
            "norm" => View::Norm,
            "rendered" => View::Rendered,
            other => return Err(format!("rule {id}: unknown view {other}")),
        };
        let match_tbl = rt
            .get("match")
            .and_then(|v| v.as_table())
            .ok_or_else(|| format!("rule {id} missing match"))?;
        let kind = match match_tbl.get("kind").and_then(|v| v.as_str()) {
            Some("word-set") => MatchKindSpec::WordSet,
            Some("regex-set") => MatchKindSpec::RegexSet,
            Some("structural") => MatchKindSpec::Structural,
            Some("ratio") => MatchKindSpec::Ratio,
            other => return Err(format!("rule {id}: bad match kind {other:?}")),
        };
        let lexicon = match_tbl
            .get("lexicon")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let mut terms = Vec::new();
        if let Some(path) = &lexicon {
            lexicon_uses
                .entry(path.clone())
                .or_default()
                .push(id.clone());
            terms.extend(lexicon_terms(path)?);
        }
        if let Some(words) = match_tbl.get("words").and_then(|v| v.as_array()) {
            for w in words {
                terms.push(as_str(w, "match.words entry")?);
            }
        }
        if kind == MatchKindSpec::WordSet && terms.is_empty() {
            return Err(format!("word-set rule {id} has no terms"));
        }
        let case_sensitive = match match_tbl.get("case").and_then(|v| v.as_str()) {
            Some("sensitive") => true,
            Some("insensitive") | None => false,
            Some(other) => return Err(format!("rule {id}: unknown case mode {other}")),
        };
        let boundary_word = match match_tbl.get("boundary").and_then(|v| v.as_str()) {
            Some("word") => true,
            Some("none") | None => false,
            Some(other) => return Err(format!("rule {id}: unknown boundary {other}")),
        };
        let block_start = match match_tbl.get("position").and_then(|v| v.as_str()) {
            Some("block-start") => true,
            None => false,
            Some(other) => return Err(format!("rule {id}: unknown position {other}")),
        };
        let scope = match match_tbl.get("scope").and_then(|v| v.as_str()) {
            Some("link-url") => Scope::LinkUrl,
            Some("code") => Scope::Code,
            Some("heading") => Scope::Heading,
            Some("comment") => Scope::Comment,
            None => Scope::None,
            Some(other) => return Err(format!("rule {id}: unknown scope {other}")),
        };
        let params = match_tbl
            .get("params")
            .cloned()
            .unwrap_or(toml::Value::Table(Default::default()));
        let mut patterns = Vec::new();
        if let Some(pats) = rt.get("patterns").and_then(|v| v.as_array()) {
            for p in pats {
                patterns.push(as_str(p, "patterns entry")?);
            }
        }
        if kind == MatchKindSpec::RegexSet && patterns.is_empty() {
            return Err(format!("regex-set rule {id} has no patterns"));
        }
        let guard = get_str("guard")?;
        let judge = rt
            .get("judge")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if tier == Tier::Candidate && judge.is_none() {
            return Err(format!("candidate rule {id} has no judge question"));
        }

        let profiles_val = rt
            .get("profiles")
            .and_then(|v| v.as_table())
            .ok_or_else(|| format!("rule {id} missing profiles"))?;
        let default_stance = match profiles_val.get("default") {
            Some(v) => parse_field_stance(v, &format!("rule {id} profiles.default"))?,
            None => FieldStance::uniform(Stance::Apply),
        };
        let mut stances = vec![default_stance; 8];
        for (k, v) in profiles_val {
            if k == "default" {
                continue;
            }
            let idx = profile_names
                .iter()
                .position(|n| n == k)
                .ok_or_else(|| format!("rule {id}: unknown profile {k}"))?;
            stances[idx] = parse_field_stance(v, &format!("rule {id} profiles.{k}"))?;
        }

        let mut exemptions = Vec::new();
        if let Some(ex) = rt.get("exemptions").and_then(|v| v.as_table()) {
            for (_, v) in ex {
                if let Some(arr) = v.as_array() {
                    for phrase in arr {
                        exemptions.push(as_str(phrase, "exemption phrase")?.to_lowercase());
                    }
                }
            }
        }

        rules.push(Rule {
            id,
            name,
            family,
            tier,
            lifecycle,
            origin,
            human_only_waiver,
            view,
            kind,
            lexicon,
            terms,
            case_sensitive,
            boundary_word,
            block_start,
            scope,
            params,
            patterns,
            guard,
            judge,
            stances,
            exemptions,
        });
    }

    // Every embedded lexicon must be referenced by exactly one rule.
    for (path, _) in LEXICONS {
        match lexicon_uses.get(*path).map(|v| v.len()).unwrap_or(0) {
            1 => {}
            0 => return Err(format!("lexicon {path} is referenced by no rule")),
            n => return Err(format!("lexicon {path} is referenced by {n} rules")),
        }
    }

    let mut seen = std::collections::BTreeSet::new();
    for r in &rules {
        if !seen.insert(r.id.clone()) {
            return Err(format!("duplicate rule id {}", r.id));
        }
    }

    let digest = compute_digest();
    Ok(PolicyPackage {
        version,
        digest,
        quotation_downgrade,
        profile_names,
        profiles,
        rules,
    })
}
