# `ai-slop`

Deterministic detector and coverage instrument for generated-text defects in
outbound technical artifacts: public bug reports, commit messages, changelogs,
release notes, readmes, API docs, crate metadata, and internal docs.

The tool detects. It never judges, approves, or edits an artifact. Findings
carry byte spans into the exact payload that was hashed, and every result
binds the artifact hash, the policy digest, and the declared profile.

## Usage

    ai-slop check --profile public-bug-report report.md
    ai-slop check --profile commit-message - < COMMIT_EDITMSG
    ai-slop verify --approval approval.json artifact.md
    ai-slop policy digest
    ai-slop policy snapshot --out SKILL.md

stdout carries one JSON result and nothing else. Diagnostics go to stderr.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | completed with no violation and no unresolved candidate |
| 2 | usage error |
| 10 | violation findings, or a failed verify |
| 20 | unresolved candidate findings |
| 30 | instrumentation error, fail closed |
| 40 | unsupported input, fail closed |

Exit 0 never means clean. Consumers read the coverage block and treat any
unknown result state as fail-closed.

## Profiles

The caller declares one of eight profiles. There is no default and no
detection: `public-bug-report`, `commit-message`, `changelog`,
`release-notes`, `readme`, `api-docs`, `cargo-metadata`, `internal-doc`.

## Segmentation

Tokens quoted in code formatting never fire the prose rules. The fixture
below stays in this readme as a standing check of that contract:

```text
Co-authored-by: Claude <noreply@anthropic.com>
Generated with ChatGPT
utm_source=chatgpt.com
delve seamless leverage
```

## License

Licensed under either of

- Apache License, Version 2.0 (LICENSE-APACHE)
- MIT license (LICENSE-MIT)

at your option.
