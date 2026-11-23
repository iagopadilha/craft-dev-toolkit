---
name: review-pr
description: >
  Review a pull request (or the current branch) with the quality gate included: standards, bugs and spec adherence in one
  report. Reads only; can post the result as a PR comment after confirmation. Use when the user types /review-pr or asks
  for a review of a PR with the quality check.
disable-model-invocation: true
---

# /review-pr

A PR review that already contains the `quality-gate`. One report, one pass.

## Usage

| Command | Does |
|---|---|
| `/review-pr` | reviews the open PR of the current branch; with no PR, the diff of the branch against `<base>` |
| `/review-pr <number or URL>` | reviews that PR |
| `/review-pr --comment` | after the report, posts it as a PR comment |
| `/review-pr --fix` | after the report, applies the safe fixes on the PR's branch (nothing is committed) |

## Steps

1. Load the `github` skill and read the PR: `gh pr view <n> --json number,title,body,baseRefName,headRefName,files`
   and `gh pr diff <n>`. For a PR from another branch, use `gh pr checkout <n>` only when the tree is clean and
   `--fix` was asked, otherwise review from the diff.
2. Run the `quality-gate` skill on the PR's changed files, in a subagent as it does by default. Pass it the PR diff and the
   base branch.
3. Add what is specific to a PR: does the title and body match the diff, does the PR carry unrelated changes, is a
   migration, config or contract change called out, are secrets or generated files included.
4. Return the single report in the `quality-gate` format, then a one-line verdict: `APPROVE-ready`, `NEEDS CHANGES` or
   `BLOCKED` (a BUG or a failed Done-when blocks).

## Comment and fix

- `--comment` publishes under the user's name, so show the exact text first and send it only after confirmation. Use
  `gh pr comment <n> --body-file <file>`. Never approve or request changes on your own.
- `--fix` follows the `quality-gate` rules: behavior-preserving fixes, left uncommitted in the working tree.

Without those flags this command only reads.
