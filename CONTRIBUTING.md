# Contributing to claude-craft-kit

Thanks for helping. This repository holds the claude-craft-kit plugin (skills, shared rules, templates) and the `craft` binary (testenv, verify, regression_diff, map_reuse, classify_structure, sync_core).

## Ground rules

- Windows x64 is the supported platform for now.
- Conventional Commits (`feat(scope): ...`, `fix: ...`, `docs: ...`). One logical change per commit.
- Sign every commit off (`git commit -s`): the sign-off states the Developer Certificate of Origin (https://developercertificate.org/) for your change. Pull requests without it are not merged.
- No names, data or hosts of any real company, customer or person in code, examples, fixtures, issues or commit messages. Use `example.com` and synthetic data.
- Never commit a secret. Never print a secret value in a log or a test.
- Every Rust file starts with `// SPDX-License-Identifier: MIT`.
- The `craft` tools are pinned to a reference implementation by `tools/craft_diff.py`. A change that alters their output or exit codes needs its own entry in `CHANGELOG.md` and an explanation in the PR.
- Skills never name a company. A company rule belongs in the company's own plugin and is bound in the project's `CLAUDE.md` through a standards slot (see `docs/STANDARDS.md`).

## Before you open a pull request

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Fill in the pull request template. Small pull requests are reviewed faster.

## Reporting problems

Use the issue templates. For a security problem do not open an issue: follow `SECURITY.md`.
