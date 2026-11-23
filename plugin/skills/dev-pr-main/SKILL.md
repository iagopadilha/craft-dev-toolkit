---
name: dev-pr-main
description: Push the work on the integration branch straight to it (no new branch), then open the integration to default pull request. Reuses an open PR.
disable-model-invocation: true
---

# /dev-pr-main

Run mode `release-pr` of the `git-shortcuts` skill. Load the `github` skill first, then `git-shortcuts`, and follow the
mode section there. No arguments are needed: the repository is the current folder and the work is read from the git state.

