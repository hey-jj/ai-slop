//! Rule evaluation beyond the word-set and regex-set engines: structural,
//! ratio, and profile-contract rules. One module per rule family that needs
//! code beyond the shared engines.

pub mod coverage;
pub mod density;
pub mod emphasis;
pub mod mechanical;
pub mod profile_contract;
pub mod rendered;
pub mod structural;

use crate::engine::{CompiledPolicy, Hit};
use crate::extract::Doc;
use crate::input::Prepared;
use crate::views::NormView;
use crate::{Config, Field, Stance};

/// Rule IDs served entirely by the shared word-set and regex-set engines.
pub const ENGINE_RULES: &[&str] = &[
    "SLOP-A001",
    "SLOP-A002",
    "SLOP-A003",
    "SLOP-A004",
    "SLOP-P001",
    "SLOP-P002",
    "SLOP-P003",
    "SLOP-P004",
    "SLOP-P005",
    "SLOP-M001",
    "SLOP-M002",
    "SLOP-M003",
    "SLOP-M004",
    "SLOP-M006",
    "SLOP-S001",
    "SLOP-S002",
    "SLOP-S003",
    "SLOP-V001",
    "SLOP-V002",
    "SLOP-V003",
    "SLOP-T001",
    "SLOP-T002",
    "SLOP-T003",
    "SLOP-I001",
    "SLOP-I002",
    "SLOP-I003",
    "SLOP-I004",
    "SLOP-I005",
    "SLOP-C001",
    "SLOP-C002",
    "SLOP-C003",
    "SLOP-C004",
    "SLOP-C005",
    "SLOP-C006",
    "SLOP-Q001",
    "SLOP-E002",
    "SLOP-R001",
    "SLOP-R002",
    "SLOP-F001",
    "SLOP-F002",
    "SLOP-F003",
    "SLOP-O001",
    "SLOP-O002",
    "SLOP-O003",
    "SLOP-O004",
    "SLOP-W001",
    "SLOP-J001",
    "SLOP-G001",
    "SLOP-G002",
    "SLOP-K008",
];

/// Every `(rule id, param key)` the implementation actually reads — or whose
/// behavior it implements with the policy value hardcoded (noted inline).
/// The policy-CI param-coverage gate fails when policy.toml declares a param
/// absent from this list and not explicitly disclosed: a declared-but-dead
/// param is exactly how the H003 unusual-scripts silent false negative
/// once shipped, because the older implemented-symbol check was rule-level
/// only.
pub fn implemented_param_keys() -> &'static [(&'static str, &'static str)] {
    &[
        ("SLOP-M005", "unclosed_fence"),
        ("SLOP-M005", "raw_html_dominance_pct"),
        ("SLOP-M005", "raw_html_dominance_floor_bytes"),
        ("SLOP-E001", "emphasized_words"),
        ("SLOP-E001", "followed_within"),
        ("SLOP-E001", "followed_by"),
        ("SLOP-E003", "list_items_with_leading_bold_label"),
        ("SLOP-D001", "count_rules"),
        ("SLOP-D001", "threshold"),
        ("SLOP-D001", "per_words"),
        ("SLOP-D002", "min_bullets"),
        ("SLOP-D002", "min_link_bullet_pct"),
        ("SLOP-D003", "min_paragraphs"),
        ("SLOP-D003", "max_length_cv_pct"),
        ("SLOP-D004", "count_rules"),
        ("SLOP-D004", "threshold"),
        ("SLOP-D004", "per_document"),
        ("SLOP-X001", "heading_set"),
        ("SLOP-X001", "min_matches"),
        ("SLOP-X002", "min_body_lines"),
        ("SLOP-X002", "min_bullet_line_pct"),
        ("SLOP-X003", "max_words"),
        ("SLOP-X004", "max_words"),
        ("SLOP-X004", "min_headings"),
        ("SLOP-K001", "max_title_chars"),
        ("SLOP-K001", "forbid_title_words"),
        // K002: prefix/period/imperative behavior implemented in
        // profile_contract::evaluate; the booleans are declarations of that
        // hardcoded behavior.
        ("SLOP-K002", "conventional_prefix"),
        ("SLOP-K002", "imperative_lowercase"),
        ("SLOP-K002", "no_trailing_period"),
        ("SLOP-K002", "max_subject_chars"),
        ("SLOP-K003", "section_whitelist"),
        ("SLOP-K003", "dated_heading"), // has_iso_date on version headings
        ("SLOP-K004", "expected_body_from_config"),
        ("SLOP-K004", "max_extra_pointer_lines"),
        ("SLOP-K005", "required_heading"), // hardcoded "license"
        ("SLOP-K005", "expected_wording_from_config"),
        ("SLOP-K006", "description_required"),
        ("SLOP-K006", "max_description_chars"),
        ("SLOP-K006", "keywords_exact"),
        ("SLOP-K006", "categories_min"),
        ("SLOP-K006", "categories_max"),
        ("SLOP-K006", "forbid_brand_prefix"),
        ("SLOP-K007", "expected_version_from_config"),
        // Y001: the three channels are implemented in render::render_invisible
        // (empty-content skip == min 1 char, dropped_html_text, unused refdefs
        // with a title).
        ("SLOP-Y001", "html_comment_min_text_chars"),
        ("SLOP-Y001", "dropped_raw_html_text"),
        ("SLOP-Y001", "unused_link_definitions_with_prose"),
        // Y002: behavior-descriptors of the (narrow, documented) divergence
        // channel in render::render_divergence.
        ("SLOP-Y002", "compare"),
        ("SLOP-Y002", "ignore"),
        ("SLOP-H001", "emit"), // the section map in every coverage block
        ("SLOP-H002", "emit"), // the excluded-bytes map in every coverage block
        ("SLOP-H002", "flag_excluded_pct"),
        ("SLOP-H003", "mixed_line_endings"),
        ("SLOP-H003", "bom_stripped"),
        // Mixed-script token hint implemented in coverage::evaluate; the
        // evasion itself is closed by the norm-view homoglyph fold (A001).
        ("SLOP-H003", "unusual_scripts_in_identifierlike_prose"),
    ]
}

/// Every rule ID with an implementation symbol. The policy CI test checks
/// this list against the package.
pub fn implemented_rule_ids() -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = ENGINE_RULES.to_vec();
    ids.extend(mechanical::HANDLED);
    ids.extend(emphasis::HANDLED);
    ids.extend(density::HANDLED);
    ids.extend(structural::HANDLED);
    ids.extend(profile_contract::HANDLED);
    ids.extend(rendered::HANDLED);
    ids.extend(coverage::HANDLED);
    ids
}

pub(crate) fn rule_idx(cp: &CompiledPolicy, id: &str) -> Option<usize> {
    cp.pkg.rules.iter().position(|r| r.id == id)
}

/// True when the rule is active for the profile (any field) and not
/// deprecated.
pub(crate) fn active(cp: &CompiledPolicy, config: &Config, id: &str) -> Option<usize> {
    let idx = rule_idx(cp, id)?;
    let rule = &cp.pkg.rules[idx];
    if rule.lifecycle == crate::policy::Lifecycle::Deprecated {
        return None;
    }
    if rule.stance(config.profile, Field::Whole) == Stance::Off
        && rule.stance(config.profile, Field::Subject) == Stance::Off
        && rule.stance(config.profile, Field::Body) == Stance::Off
    {
        return None;
    }
    Some(idx)
}

pub(crate) fn param_i64(rule: &crate::policy::Rule, key: &str) -> Option<i64> {
    rule.params.as_table()?.get(key)?.as_integer()
}

pub fn evaluate_structural(
    cp: &CompiledPolicy,
    prepared: &Prepared,
    doc: &Doc,
    norm: &NormView,
    config: &Config,
    hits: &mut Vec<Hit>,
) {
    let _ = norm;
    mechanical::evaluate(cp, prepared, doc, config, hits);
    emphasis::evaluate(cp, prepared, doc, config, hits);
    structural::evaluate(cp, prepared, doc, config, hits);
    profile_contract::evaluate(cp, prepared, doc, config, hits);
    rendered::evaluate(cp, prepared, doc, config, hits);
    coverage::evaluate(cp, prepared, doc, config, hits);
    // Density rules run last: they count resolved hits.
    density::evaluate(cp, prepared, doc, config, hits);
}
