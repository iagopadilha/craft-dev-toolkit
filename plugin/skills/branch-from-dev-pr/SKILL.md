---
name: branch-from-dev-pr
description: Same as /branch-from-dev, then open the pull request to the base branch.
disable-model-invocation: true
---

# /branch-from-dev-pr

Run mode `branch-pr` of the `git-shortcuts` skill. Load the `github` skill first, then `git-shortcuts`, and follow the
mode section there. No arguments are needed: the repository is the current folder and the work is read from the git state.
An optional argument replaces the generated branch name.
