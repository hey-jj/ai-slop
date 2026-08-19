# Changelog

## [0.1.7] - 2026-08-19

### Added

- SLOP-E002 gains `profile_exemptions`, a per-profile table of case-sensitive
  covering literals, seeded with the internal-doc ledger verdict token
  `DO NOT BUILD`. The token stays a candidate on every other profile, and a
  case variation still fires.
- SLOP-X004 gains `exempt_heading_sets`: a public-bug-report document whose
  headings after the title are exactly `Reproducer`, `Observed`, `Expected`,
  `Root cause` in order raises no over-structure candidate. An extra or
  reordered heading still fires.
- Prose-format input that reads as raw Rust source is rejected as unsupported
  (exit 40) at two layers: a `.rs` path under a markdown or text profile, and
  content whose outside-fence lines carry Rust signatures at eight lines and
  thirty percent of non-blank lines. The error names the remedy: extract the
  rustdoc and gate the extract. Fenced Rust in a README is unaffected.

### Changed

- SLOP-W001 treats a hyphenated compound as one token, so `serial-port` and
  `port-forwarding` no longer fire on the embedded scrub word.
- SLOP-C004's concession pattern also matches at sentence starts inside a
  paragraph, covering the `While X, Y` contrast that previously passed
  mid-paragraph. Calibrated over 266 human-authored corpus files: nine new
  candidates, seven genuine concession shapes and two temporal-while
  residuals.
- The skill's contrastive-negation section states the two-part keep test and
  names SLOP-C003 in the shape list with a rather-than keep example.

### Documentation

- Policy 1.4.0. The snapshot reference is regenerated.
