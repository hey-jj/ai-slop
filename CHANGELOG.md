# Changelog

## [0.1.9] - 2026-08-20

### Added

- SLOP-C010 proleptic-capability-denial: a candidate rule for denials of a
  capability nobody claimed, and for the evidential hedges stacked on them.
  The subject opens its clause and comes from a closed set, and a denied
  capability verb is required from a closed list, so denying a function
  (`never fires on irregularity`) reads as the scope fact it is. Two
  qualifying clauses in one block report, and so does a single one standing
  beside a sentence that describes the same subject affirmatively, where
  either a bare pronoun or a second tool noun carries the reference. A third
  spelling carries no subject, so a denial fragment in the middle of a stack
  (`never scores voice`) counts too. Commands are excluded one clause at a
  time: only `do not`, `don't`, and `never` can head one, and only over a
  plain verb, since the finite negations need a subject. A leading coordinator
  is skipped before every test. The affirmative partner is searched in the
  qualifying clause's own sentence first, then the sentence before, then the
  sentence after, and the two sides corefer on a bare pronoun or on the same
  tool-noun lemma. Each qualifying clause reports on its own: a denial with a
  subject reports its coordinator-cut segment, and an evidential hedge reports
  the whole comma-delimited clause. Off nowhere, relaxed on api-docs.
- SLOP-F004 rationale-leak: a candidate rule for sentences that argue for the
  design instead of stating what happens and what to do. Two marker families,
  the bargain behind a choice and an instruction on how to take the text, both
  anchored on a tool noun anywhere in the same sentence, so `This poem should
  be read as an elegy` stays silent. Each marker reports on its own, so a
  sentence carrying two reasons yields two findings. Relaxed on api-docs, off
  on internal-doc. The anchor set is SLOP-C010's, read from
  that block, so the two rules cannot drift apart.
- SLOP-C007 gains the `and not` spelling before a preposition or article. The
  `or not` and `but not` spellings stay on the skill's reread checklist,
  because `whether or not the flag is present` is an honest open condition
  wearing the same letters.

### Changed

- The raw-source guard reads a two-arm line test. Comment and attribute
  openers and punctuation-only lines carry themselves. Every other line has to
  end on a code terminator AND open on a keyword, carry a path or arrow token,
  or have the field-line shape. A definition list writes several words after
  its colon, so it is never a field line. The bar moved to thirty-five
  percent. Measured over this tree: all twenty-two source files are
  rejected, the four prose surfaces score zero, and the lowest source score
  clears the highest prose score by thirty-eight points.
- SLOP-C004's sentence-boundary arm reads a real ordinal test. A digit run
  that opens its line is a list marker and suppresses the match. A digit
  following other text on the line closes a sentence and fires, so
  `version 2. Granted, ...` is a finding where it used to be silent.

### Documentation

- The skill states that the check and the reread are one procedure, adds two
  questions for the final reread, carries the rationale leak as its own
  hand-read entry, carries the proleptic capability-denial stack as the eighth
  contrastive-negation shape with its three hand-read spellings, records the
  conjunction spellings the pattern leaves behind, and lists the dangling
  which-clause and the empty restatement among the patterns no rule will
  catch.
- Policy 1.5.0. The snapshot reference is regenerated.

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
