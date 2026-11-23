---
name: hotfix
description: Urgent fix: branch hotfix/* from the default branch, commit, push and open the PR after one confirmation.
disable-model-invocation: true
---

# /hotfix

Run mode `hotfix` of the `git-shortcuts` skill. Load the `github` skill first, then `git-shortcuts`, and follow the
mode section there. No arguments are needed: the repository is the current folder and the work is read from the git state.
An optional argument replaces the generated branch slug.
