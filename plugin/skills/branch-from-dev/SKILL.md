---
name: branch-from-dev
description: Read the work in the current repo, name a feat/fix branch from the updated base, commit and push. No arguments needed.
disable-model-invocation: true
---

# /branch-from-dev

Run mode `branch` of the `git-shortcuts` skill. Load the `github` skill first, then `git-shortcuts`, and follow the
mode section there. No arguments are needed: the repository is the current folder and the work is read from the git state.
An optional argument replaces the generated branch name.
