---
name: pull
description: Pull the repository and the current branch, or pass a branch (/pull developer) to update the local copy of it and bring it into the current branch. Fast-forward only.
disable-model-invocation: true
---

# /pull

Run mode `pull` of the `git-shortcuts` skill. Load the `github` skill first, then `git-shortcuts`, and follow the
mode section there.

- `/pull`: updates the repository (the local protected branches) and the current branch.
- `/pull <branch>`: updates that branch from the remote and brings it into the current branch.
