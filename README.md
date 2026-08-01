# `ai-slop`

A lint for text you are about to ship: bug reports, commit messages,
changelogs, release notes, READMEs, API docs, crate metadata, and internal
docs. It flags AI slop before the text goes out.

Findings carry byte spans into the exact bytes that were hashed, and every
result records the artifact hash, the policy digest, and the profile, so
the same input and policy always give the same result.

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

Exit 0 on its own does not mean the artifact is clean: check the coverage
block, and treat any result state you do not recognize as a failure.

## Profiles

Each run takes exactly one of eight profiles, and there is no default:
`public-bug-report`, `commit-message`, `changelog`,
`release-notes`, `readme`, `api-docs`, `cargo-metadata`, `internal-doc`.

## Segmentation

Prose rules skip anything in code formatting. The block below stays in
this README as a regression check of that:

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
