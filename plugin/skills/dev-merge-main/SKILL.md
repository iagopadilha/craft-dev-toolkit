---
name: dev-merge-main
description: Same as /dev-pr-main, then merge the PR with a merge commit. The integration branch is never deleted.
disable-model-invocation: true
---

# /dev-merge-main

Run mode `release-merge` of the `git-shortcuts` skill. Load the `github` skill first, then `git-shortcuts`, and follow the
mode section there. No arguments are needed: the repository is the current folder and the work is read from the git state.

