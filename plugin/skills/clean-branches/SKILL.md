---
name: clean-branches
description: Delete local branches already merged or gone from the remote. Never deletes a protected branch.
disable-model-invocation: true
---

# /clean-branches

Run mode `clean` of the `git-shortcuts` skill. Load the `github` skill first, then `git-shortcuts`, and follow the
mode section there. No arguments are needed: the repository is the current folder and the work is read from the git state.

