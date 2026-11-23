---
name: dev-pr-main
description: Open the pull request from the integration branch to the default branch. Never touches the integration branch itself.
disable-model-invocation: true
---

# /dev-pr-main

Run mode `release-pr` of the `git-shortcuts` skill. Load the `github` skill first, then `git-shortcuts`, and follow the
mode section there. No arguments are needed: the repository is the current folder and the work is read from the git state.

