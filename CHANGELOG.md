# Changelog

## [0.1.8] - 2026-08-19

### Added

- SLOP-E002 gains `profile_exemptions`, a per-profile table of case-sensitive
  covering literals, seeded with the internal-doc ledger verdict token
  `DO NOT BUILD`. A literal covers a hit only as a standalone token: its
  edges sit on word boundaries and the label ends at a line end, punctuation,
  or a table-cell bar. `We DO NOT BUILD trust by ...` and the embedded
  spelling in `AVOCADO NOT BUILD LIST` both stay findings, the token stays a
  candidate on every other profile, a case variation still fires, and an
  empty exemption array fails the policy load.
- SLOP-X004 gains `exempt_heading_sets`: a public-bug-report document whose
  headings after the title are exactly `Reproducer`, `Observed`, `Expected`,
  `Root cause` in order raises no over-structure candidate. An extra or
  reordered heading still fires. The table is validated at load: an unknown
  profile key or a non-lowercase heading literal fails the policy load.
- Prose-format input that reads as raw Rust source is rejected as unsupported
  (exit 40) at two layers: a `.rs` path under a markdown or text profile, and
  content whose outside-code lines carry Rust signatures at eight lines and
  thirty percent of non-blank lines. The prose/code split is the extractor's
  own segmentation, so backtick-fenced, tilde-fenced, and 4-space indented
  Rust (a bug report's indented reproducer included) all stay prose. The
  error names the remedy: extract the rustdoc and gate the extract.

### Changed

- SLOP-W001 exempts the technical `port` compounds in hyphen spelling
  (`serial-port`, `port-forwarding`, and the hyphen forms of the listed
  space exemptions) as explicit literals. Hyphenation itself never
  suppresses the rule: `research-backed`, `audit-ready`, `straight-port`,
  and `upstream-first` all fire.
- SLOP-C004's sentence-boundary arm requires a real boundary: a non-digit
  before the punctuation and SLOP-C007's terminal-period test, so a list
  ordinal (`has 2. Granted, ...`) and an abbreviation (`e.g. granted, ...`)
  no longer open a match. The concession pattern itself stays line-anchored:
  a 958-file corpus probe measured a sentence-start widening mostly on
  temporal `while` and legitimate human contrasts, so the mid-paragraph
  `While X, Y` shape went to the skill's hand-read checklist instead
  (contrastive-negation shape 7).
- The skill's contrastive-negation section states the two-part keep test and
  names SLOP-C003 in the shape list with a rather-than keep example.

### Documentation

- Policy 1.4.0. The snapshot reference is regenerated.

## [0.1.7] - 2026-08-18

### Added

- SLOP-V005 ledger-stamp: a candidate rule for orchestration-ledger stamp
  diction, meaning a verdict or measurement verb directly followed by a
  bare ISO date, and the owner-verdict phrase with or without its date.
  The prose form with a preposition and release-date diction stay out of
  scope. The rule is off on the internal-doc profile, where the stamp is
  the documented ledger convention. It exists to keep the stamp from
  leaking outward.

### Documentation

- Policy 1.3.0. The snapshot reference is regenerated.
