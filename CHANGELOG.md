# Changelog

## [0.1.15] - 2026-10-03

### Documentation

- Comments and API docs state parser boundaries, span mapping, quotation
  handling, and repeated-text limits in direct sentences.
- Rule examples in comments name the wording families they exercise.

### Changed

- The generated policy snapshot header tells readers to run the snapshot
  command to regenerate the file.

## [0.1.14] - 2026-09-29

### Added

- SLOP-I006 `hedging-litotes`, a violation in the intensifier family with no
  judge question. It reports the closed set of hedging litotes and stock
  understatements that have no honest reading in confident technical prose:
  the negated adjectives behind `not` (`not inconsiderable`, `not
  unimportant`, `not inconsequential`, `not unfamiliar`), the negated
  difficulty forms (`no small feat`, `no mean feat`, `in no small part`, `was
  no simple task`), the negated privations (`not without its challenges`),
  the partial negations on an ease word (`not exactly trivial`, `isn't
  exactly simple`), the distance forms (`far from trivial`,
  `hardly surprising`, `less than stellar`, `leaves something to be
  desired`), the conditional frames (`it would not be wrong to say`, `it is
  safe to say`, `it is not hard to see`), and the stock understatements (`to
  say the least`, `to put it mildly`, `not rocket science`, `not a walk in
  the park`, `not for the faint of heart`, `not to be underestimated`, a
  comma-led `not to mention`). Every not-led member also takes the
  contractions `isn't`, `wasn't`, `aren't`, and `weren't`. Every adjective is listed and no prefix
  wildcard exists, so `not impossible`, `not unlike`, `not incorrect`, `not
  necessarily`, `not yet`, a bare `far from`, and a bare `hardly` stay
  silent. A clause-initial `No simple task` is a quantifier and stays silent.
  The span is the phrase itself. The rule applies at violation tier in every
  profile, internal-doc included, and quoted text downgrades to candidate.
  `it goes without saying` stays with SLOP-T001.
- SLOP-I007 `understatement-hedge`, a candidate in the intensifier family.
  It reports the understatements that can carry a literal count, rate,
  threshold, rank, or evidence gap: `not uncommon` and eleven other negated
  adjectives, `not unheard of`, `not infrequently`, `not a trivial
  undertaking`, `no small amount`, `not without merit`, `not entirely clear`,
  `not exactly ideal`, `not quite right`, `not particularly`, `not terribly`,
  `far from ideal`, `less than ideal`, `it is not unreasonable to`, `not the best`,
  `could be better`, and `room for improvement`, with the same four
  contractions on every not-led member. The judge question asks for
  the number or the named gap that makes the negation literal. internal-doc
  relaxes to advisory, and quoted text drops. Policy 1.9.0 to 1.10.0.
- The skill gains the shape as hand-read tell 8, a fourth closing question
  for the final reread, and a table of direct rewrites.

### Fixed

- SLOP-F002 `verification-claim` matched `re-verified` as a substring inside
  `signature-verified`, so a README line saying that capture claims are read
  and never signature-verified failed the gate. Every entry now matches on
  word edges, which also silences `unverified` and `unconfirmed`, and a
  hyphen compound ending in `verified` or `confirmed` names a mechanism and
  stays silent. The plain claims (`verified`, `confirmed by`,
  `double-checked`, `was reviewed`) fire as before.
- SLOP-F003 `impact-framing` had the same substring collision: `severe`
  fired inside `persevere`, and the negated compounds `non-urgent`,
  `non-severe`, `unexploitable`, and `non-exploitable` fired on the word they
  negate. Every entry now matches on word edges, the negated compounds and
  the mechanism compounds `self-remediation` and `auto-remediation` carry
  exemptions, and `severely` joins the lexicon so the adverb keeps firing.
- SLOP-K008 `guarantee-claim` fired on the claim a negated compound denies:
  `non-guaranteed`, `non-thread-safe`, `non-lock-free`. Every entry now
  matches on word edges and the negated compounds carry exemptions.
- SLOP-T003 `audience-runway` fired on `mastering` inside `remastering` and
  `re-mastering`. Every entry now matches on word edges, `re-mastering`
  carries an exemption, and `demystifies` and `demystified` join the lexicon
  so the verb keeps firing in every form.

## [0.1.13] - 2026-09-02

### Added

- SLOP-V006 `decision-attribution`, a candidate in the assistant-voice
  family. It reports a decision credited to a role noun in the third
  person, in text the person that noun names is signing. Two closed role
  sets. With an open role (owner, maintainer, author, principal, proxy,
  operator, orchestrator, human, lead, user, reviewer) it reads the
  possessive on a decision noun (`Owner's ruling, 2026-08-20`, `the
  maintainer's call`), a verdict verb (`the owner ruled`, `the proxy signed
  off`), the by-form (`requested by the user`), the hyphen compound
  (`owner-flagged`), `per the owner`, `at the user's request`, and `on the
  principal's instruction`. With a ledger role (owner, maintainer,
  principal, proxy, orchestrator, lead) it also reads the loose verbs (`the
  owner wants`, `the maintainer asked`, `the owner has ruled`), the loose
  possessives (`the owner's request`, `in the owner's stead`), and the
  bare compound (`owner decision`, `owner-proxy ruling`). Three shapes carry no role: a
  decision noun on an ISO date (`ruling (2026-08-20)`), a block-start label
  (`Ruling:`, `Decision:`), and the second person aimed at the signer (`per
  your ruling`, `as you directed`). A trailing ISO date joins the span, so a
  dated stamp reports once. The loose verbs stay off the open roles, so
  `the user asked` and `the author said` stay silent, and the verb sets
  carry verdict verbs only, so `user-defined` and `set by the user` stay
  silent. The roleless verb-plus-date stamp stays with SLOP-V005.
  `api-docs` and `internal-doc` relax to advisory. The specimen shipped in
  an issue after the drafting agent told two reviewers to read the
  paragraph as an attributed decision, and no rule existed for the shape.
  A four-week sweep of session transcripts supplied the spellings. Policy
  1.8.0 to 1.9.0.
- The skill gains the shape as hand-read tell 7 and a third closing
  question for the final reread.

### Changed

- The lazy DFA caches behind the regex-set rules grow from 4 MiB to 8 MiB.
  The sixteen patterns of SLOP-V006 pushed the reverse automaton's minimum
  past the old bound, which failed policy load at exit 30. The cache is an
  upper bound on memory the automaton may use, and nothing else changes.

## [0.1.12] - 2026-09-02

### Changed

- The bare word `impact` leaves the SLOP-F003 lexicon. The rule matches by
  substring, so the entry fired on `impacted`, `impacts`, and every neutral
  sentence that names what a change reaches, which the guard itself files as
  content. The entries that rank a consequence stay, `high-impact` among
  them, and a keep-test reads the lexicon and proves each one still fires.
  The literal `Impact` heading on the bug-report profile is still a SLOP-S002
  verdict heading. Policy 1.7.0 to 1.8.0.

## [0.1.11] - 2026-08-24

### Added

- A `public-comment` profile for public PR and issue comments. It carries
  the outbound discipline of `public-bug-report` without the title
  contract, since a comment has no title and SLOP-K001 reads its first
  line as one. Policy 1.6.0 to 1.7.0.

### Changed

- `Rule::profile_mask` returns `u16` instead of `u8`. The ninth profile
  sits at bit index 8, past the top of a `u8`. The change lands in a
  public signature. No external consumer of it is known.

## [0.1.10] - 2026-08-20

### Changed

- SLOP-C004 tells a concession from a stretch of time. `although` and `though`
  match unqualified, since neither word has a temporal reading. A `while`
  match drops when a participle sits straight after the keyword, so
  `While working on the migration, we found a race` is silent. Eight
  participles are held out because conceding is all they do in that slot:
  `acknowledging`, `recognizing`, `granting`, `accepting`, `conceding`,
  `admitting`, `noting`, and `allowing`. The drop also asks for no finite verb
  between the keyword and the comma, from a closed list of twenty read whole
  and never by suffix, so `While programming language parsers are usually
  written manually` keeps its concession and `While being tested, the parser
  reports` still drops. A `while` match also drops when the clause up to the
  comma carries a progressive, so `While you are working, you might notice
  unexpected changes` is silent. `While the parser is slower, it handles more
  cases` still fires, and so do the shapes the guard records: a durative
  present with no copula (`While the build runs, grab a coffee`) and a copula
  with an adjective, which is inseparable from a real concession.
- SLOP-C004's staged-agreement span opens at the concession word. The match is
  licensed by the terminal punctuation of the sentence before, which across
  two list items sits in an earlier block, and that punctuation is no longer
  part of what the reader is asked to rewrite.
- SLOP-C010 reads a fourth family-1 shape. A segment that opens on `and`, is
  headed by `do not`, `don't`, or `never`, and denies a capability in plain
  form is a statement when an earlier segment of the same sentence already
  named a closed-set subject, so `The rules read text and never detect
  authorship` reports on the adjacency arm with the segment span. `but`, `so`,
  a comma, and a sentence break all leave the imperative reading in place.
- SLOP-C010's command test steps over one `-ly` adverb, so
  `Never actually scores voice` reaches the same verb as `Never scores voice`.
- SLOP-C010's open hedge takes one to three tokens in front of the head noun,
  so `no single early finding is evidence` matches. Four tokens would admit an
  of-phrase and seat the head-noun test on the wrong word, so the wildcard
  stops at three.
- SLOP-C007's comma tail ends at a participle sitting against `not` or
  `never`, so `never judging anyone` is silent. A determiner keeps the tail in
  scope, and `nothing`, `anything`, `something`, `everything`, and `during` are
  named so the four quantifier pronouns and `not during matching` keep firing.
- The published crate carries `tests/**`, `fixtures/**`, and `CHANGELOG.md`
  beside the sources, the policy, and the skill. `cargo package` reports no
  ignored files, and the test suite runs from the unpacked tarball.
- A leading run of emoji, symbols, and whitespace no longer moves a phrase off
  the opening of its line. Every rule that reads a block-start position sees
  what the reader sees, and all five now report behind a decoration the same
  way they report without one: `SLOP-M003` opening `however`, `SLOP-T001`
  opening `overall`, `SLOP-T002` transition openers, `SLOP-S001` signature
  lines, and `SLOP-V002`'s anchored praise. Every one of the five moves the
  same way, from silent to reporting.
- The four bullet glyphs join the decoration a block-start test reads past, so
  a phrase pasted from a rendered list opens its line: U+2022, U+2023, U+2043,
  and U+2219. The middle dot U+00B7 stays out, since it leads a line in fewer
  than one occurrence in twelve and is a letter in Catalan besides. The marker
  set and the decoration set are two named constants now, so a later harvest
  edits both together.
- Plain text and commit bodies drop a leading list, quote, or heading marker
  from the prose they hand the rules, the way markdown already did. A marker
  led line used to push its first word off the opening of the block, which
  left every block-start rule silent on those formats. A marker counts only
  with whitespace after it, so `-3 degrees` and `#4` stay prose. Spans on a
  marker-led line move by the width of the marker, and those bytes count as
  structure now instead of prose. A marker is no longer counted as a word
  either, so a word cap and a per-1000-words rate mean the same thing on plain
  text, a commit body, and markdown.
- SLOP-C004's progressive test reads over one adverb, so
  `While we were already running the tests, ...` is silent the way
  `While we were running the tests, ...` already was. The word qualifies by an
  `-ly` ending or from a closed list, and one is the cap, so
  `While the parser is still half parsing ...` keeps firing.
- `SLOP-D004` reports on decorated and marker-led openers, because it counts
  `SLOP-T002` hits and those openers reach it now. It is the one rule in this
  release that moved without its own text changing.
- `SLOP-V002` anchors eight praise phrases to the opening of a sentence, a
  line, or a list item: `great question`, `good question`,
  `excellent question`, `that's a great question`, `great point`,
  `excellent point`, `you're absolutely right`, and `you are absolutely
  right`. A sentence reporting that someone asked a great question is silent.
  The rest of the lexicon is unanchored, and a rule may now name the anchored
  part of its lexicon in `match.params.block_start_only`.
- `good catch` and `great catch` are out of the assistant-voice lexicon. A
  reviewer who opens a line with either one means it, so the position test
  would have separated nothing. A line that pairs the catch with a second
  praise phrase still reports on the second phrase.
- `fair hit` joins the assistant-voice lexicon, unanchored, since the
  concession reads the same wherever it sits. Two readings come with it and
  both go to the judge: a hit is literal in sport and in games, and the entry
  sits inside `unfair`.

### Fixed

- A document whose first character is more than one byte wide no longer fails
  the run. A rule that reports about the whole document anchors on the first
  character, and the anchor took a single byte, which cut an emoji, an em
  dash, or an accented letter in half and ended the run with an
  instrumentation error and no findings at all. All five anchoring sites take
  the whole character.

### Documentation

- A test reads every rule's guard and judge text and fails on the writing rules
  a machine can settle: em and en dashes, semicolons, contrast scaffolding, and
  the filler words. Scope is those two fields, never the patterns or lexicons a
  rule matches on, and a guard may still quote a term its own rule declares. The
  punctuation classes take no such exemption.
- The C004 and C010 guards state the behavior the code has. C004 no longer
  claims that temporal `while` never fires, and C010 carries the joined-denial
  shape, the instruction the judge settles, and the reason the wildcard stops
  at three.
- The C007 guard gives the measurement behind leaving the `or not` and
  `but not` spellings to the reread: in a 14.7MB corpus pass all three
  `or` hits sat inside `whether or not`, and all four `but` hits were honest
  exclusions.
- Policy 1.6.0. The snapshot reference is regenerated.

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
