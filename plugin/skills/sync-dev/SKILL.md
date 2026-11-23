---
name: sync-dev
description: Bring the base branch into the current feature branch (rebase when not shared, merge when pushed). Stops on conflict.
disable-model-invocation: true
---

# /sync-dev

Run mode `sync` of the `git-shortcuts` skill. Load the `github` skill first, then `git-shortcuts`, and follow the
mode section there. No arguments are needed: the repository is the current folder and the work is read from the git state.

