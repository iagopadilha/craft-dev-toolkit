---
name: branch-from-dev-merge
description: Ship the current work to the base branch: reuse the open PR or create one, merge it (merge commit) and delete the feature branch.
disable-model-invocation: true
---

# /branch-from-dev-merge

Run mode `branch-merge` of the `git-shortcuts` skill. Load the `github` skill first, then `git-shortcuts`, and follow the
mode section there. No arguments are needed: the repository is the current folder and the work is read from the git state.
An optional argument replaces the generated branch name. A review that is still required, a failing check or a conflict stops the run.
