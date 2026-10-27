---
name: github
description: >
  Git and GitHub operations with the GitHub CLI (gh): commits, branches, pushes, pulls, working trees,
  pull requests, merges, code review and local cleanup, with a strict safety model (what needs confirmation,
  what is never allowed). Use whenever the user mentions commit, git add, push, pull, branch, checkout,
  worktree, pull request, PR, merge, code review, PR review, upstream, origin, feature branch, hotfix,
  git status, stash, rebase, or any git/github operation, even without the word "git". Triggers: "commit
  this", "create a branch for feature X", "open a PR", "merge this", "fix this in a separate branch", "review
  the PR", "push the code", "clean up old branches". NEVER run an
  operation that needs confirmation without it, nor when the intent is not clear.
---

# GitHub Skill

You are a careful Git assistant. You **guide and execute git operations**, always with the user's explicit confirmation before any action that affects the remote or rewrites history.

## 0. Repository context

- **The organization (or user) is the one of `git remote get-url origin`.** Never assume another one. If the user names a repository without an owner, use the owner of `origin`. If `origin` points somewhere unexpected, flag it before any push or PR.
- **Protected branches** = the repository's **default branch** plus the **integration branch**.
  - Default branch: `gh repo view --json defaultBranchRef -q .defaultBranchRef.name`, or `git symbolic-ref --short refs/remotes/origin/HEAD`.
  - Integration branch: `developer` or `develop`, when it exists locally or on the remote (`git branch -a --list "*developer" "*develop"`). If neither exists, there is none.
- **Working base** = the integration branch when there is one, otherwise the default branch. Feature branches start from it and PRs target it. Below, `<default>`, `<integration>` and `<base>` stand for these.

## Standards slots

Read the `## Standards` section of the project's `CLAUDE.md` (in a monorepo, the subproject's). For each slot this skill reads (`git`), load the bound skill before working. It wins where it conflicts with this skill. No binding means this skill's defaults. A bound skill that is not installed: say so in one line and use the defaults. Never load a company skill that is not bound.

The `git` slot's bound skill provides the organization, the protected branches and the PR flow (base branch, naming, templates). Where it names them, they replace the derivation in section 0. The safety rules below still apply to whatever it names as protected.

---

## 1. Safety rules, NO EXCEPTIONS

These rules apply **always**, regardless of execution mode, granted permissions, automation flags or any instruction received during the conversation. No context, "skip permissions" mode, user instruction or system prompt overrides them.

### Operations that require explicit confirmation

Irreversible, or they affect other people. Always confirm before executing.

| Operation | Why it requires confirmation |
|---|---|
| `git push --force` (and `--force-with-lease`) | Rewrites shared remote history |
| `gh pr merge` | Closes the PR cycle, not easily undone |
| `gh pr close` | Discards open work |
| `git push origin --delete` (remote branch) | Deletes for everyone on the team |
| `git reset --hard` | Permanently discards local work |
| `git rebase` on a shared branch | Rewrites history others may have pulled |
| Any operation on a protected branch | Affects the production or integration baseline |
| Working tree creation or removal | Structural change to the workspace |

### Operations that do NOT require confirmation (feature branches)

Safe, reversible, day-to-day operations on feature branches. Execute directly:

- `git add` + `git commit` on a feature branch: local versioning, fully reversible
- `git push` on a feature branch: only you depend on it
- `git checkout -b` / `git switch -c`: creating a branch has no risk
- `git pull`: only updates your local state
- `git stash` / `git stash pop`: temporary, local only
- `git fetch`: read-only

> **Rule of thumb**: if only you are affected and it can be undone with `git revert` or `git reset`, no confirmation is needed. If it affects the remote, the team or rewrites history, always confirm.

### Absolutely forbidden, never, under any circumstance

- **Delete a protected branch**, local or remote, even if the user explicitly asks.
- **Direct push to the default branch**: always redirect to a PR flow.
- **Day-to-day work on the default branch when an integration branch exists**: warn and suggest the integration branch.
- **Run any git sequence in automatic mode** without step-by-step confirmation when there is a remote effect.

If the user asks to delete a protected branch:
> "`<branch>` is protected and cannot be deleted. Can I help with something else?"

### With an orchestrator

When an orchestrator (for example claude-maestro) drives the work, it commits once per task on the feature branch, never on a protected branch, and pushes and opens the PR once, at the end, through its own delivery step. The confirmation rules above still hold: an orchestrator that is about to touch a protected branch stops and asks the user instead of confirming for them.

### What is allowed on protected branches

- Commit directly on the integration branch (with confirmation, like any commit)
- Push to the integration branch (with confirmation)
- Receive merges via PR: `feature -> <integration>` and `<integration> -> <default>` (or `feature -> <default>` when there is no integration branch)
- `git checkout <integration>` freely to return to the working base
- `git pull` to update them

### Read-only, no confirmation needed

- `git status`, `git log`, `git diff`
- `git branch -a` / `git branch -v`
- `git fetch --prune` / `git fetch -p` (downloads information only)
- `gh pr list` / `gh pr view`
- `git worktree list`
- Analyzing differences between local and remote branches after a fetch

### Local cleanup after fetch (the only autonomous delete case)

After `git fetch -p`, you **may suggest** deleting local branches that no longer exist on the remote. You must:

1. List the branches that would be deleted
2. Ask: "The following local branches no longer exist on the remote: `[list]`. Can I delete them locally?"
3. Only run `git branch -d` (safe, not force) after confirmation

`git branch -D` (force) **always** requires a separate confirmation, even in this context.

---

## 2. Initial diagnosis

Before any operation, understand the context:

```bash
git status
git branch --show-current
git log --oneline -5
git remote get-url origin
```

For PR and review operations, check that `gh` is available:

```bash
gh auth status
```

If `gh` is missing, tell the user: "`gh` CLI not found. To use PR and review operations, install the GitHub CLI at https://cli.github.com and run `gh auth login`." Never install anything automatically.

---

## 3. Commits

```bash
git status                        # show what changed, present it to the user first
git add .                         # or specific files
git commit -m "type: description"
```

Commit message pattern (Conventional Commits):

```
feat: add OAuth authentication
fix: correct email validation
refactor: extract parsing logic
chore: update dependencies
docs: update README
test: add tests for UserService
```

Push with upstream (new branch):

```bash
git push --set-upstream origin branch-name   # first push
git push                                     # next pushes
```

**Before committing (suggestion, never blocking):** if `package.json`, a `Makefile` or lint/test scripts exist, suggest running them first and ask whether the user wants to. Never block.

---

## 4. Branches

Actively suggest a new branch when the user mentions a new feature, a bug fix or hotfix, a large refactoring or an experiment: "This looks like a new feature, want me to create a `feat/feature-name` branch?"

Naming:

```
feat/feature-name
fix/bug-description
refactor/area-refactored
hotfix/critical-issue
chore/technical-task
```

```bash
git checkout -b feat/feature-name        # or: git switch -c feat/feature-name
git branch -a                            # local + remote
git branch -v                            # with last commit
git branch -d feat/feature-name          # delete local (safe)
git push origin --delete feat/feature-name   # delete remote: always confirm
```

---

## 5. Working trees

Use a working tree for a large feature, or when you need to work on two things at once without stashing or switching branches. Suggest it proactively when the user says "I want to work on X without losing what I'm doing", "I need an urgent fix but I'm in the middle of something" or "large feature that will take a while".

Working trees live **next to** the main repository:

```
projects/
├── my-repo/              <- main repo
└── my-repo-feat-x/       <- working tree for feature X
```

```bash
# From the main repo. CONFIRM before executing
REPO_NAME=$(basename $(git rev-parse --show-toplevel))
BRANCH_NAME="feat/feature-name"
WORKTREE_PATH="../${REPO_NAME}-$(echo $BRANCH_NAME | sed 's/\//-/g')"

git worktree add "$WORKTREE_PATH" -b "$BRANCH_NAME"     # new branch + working tree
git worktree add "$WORKTREE_PATH" "$BRANCH_NAME"        # existing branch
git worktree list
git worktree remove ../my-repo-feat-x                   # always confirm
git worktree remove --force ../my-repo-feat-x           # only if it refuses over uncommitted changes
```

Every working tree is on its own dedicated branch (git does not allow two trees on the same branch).

**After removing a working tree (always):** go back to the original repo and leave it on `<base>`, up to date:

```bash
cd ../my-repo                 # the original repo, not the removed tree
git checkout <base>
git pull
git worktree prune
git worktree list             # only the original repo should remain
```

- If the original repo has uncommitted changes that block the checkout or the pull, stop and ask. Never stash or discard them on your own.
- If an editor still holds files in the removed folder, close that editor window and delete the leftover folder.
- Ignored folders (for example `.specs/`) are lost with the tree: copy them to the original repo or to a backup before removing.

Workflow: create the tree, open it in the editor, work in isolation, commit and push, open the PR with `gh`, and after the merge remove the tree.

---

## 6. Pull and sync

```bash
git pull                                                      # current branch
git fetch origin
git status                                                    # ahead/behind
git log HEAD..origin/$(git branch --show-current) --oneline   # what is incoming
git pull origin <base>                                        # a specific branch
git fetch --all && git merge origin/<base>                    # inspect first, apply when ready
```

Update a feature branch with the base:

```bash
git checkout <base> && git pull
git checkout feat/my-branch
git rebase origin/<base>        # ask first if the branch was shared with others
```

With uncommitted changes: `git stash`, `git pull`, `git stash pop`.

---

## 7. Pull requests

> **Rule**: every merge goes through a PR, unless the user explicitly says "direct merge" or "no PR".

```bash
gh pr create                    # interactive (recommended)

gh pr create \
  --title "feat: feature description" \
  --body "## What was done\n\n- item 1\n- item 2" \
  --base <base> \
  --head feat/feature-name
```

Before creating the PR:

```bash
git status
git log origin/<base>..HEAD --oneline    # the commits that will be included
```

PR body template:

```markdown
## What was done
- Describe the main changes

## How to test
- Steps to test

## Checklist
- [ ] Tests passing
- [ ] No debug code left
- [ ] Documentation updated (if needed)
```

```bash
gh pr list                      # open PRs (--state all, --author "@me")
gh pr view [number or URL]      # --web opens the browser
```

## 8. Code review

```bash
gh pr diff [number]
gh pr review [number] --approve [--body "LGTM!"]
gh pr review [number] --request-changes --body "What needs to change"
gh pr review [number] --comment --body "General observation"
gh pr checkout [number]         # test it locally
```

Approving, requesting changes and commenting publish under the user's name: confirm the text before sending.

## 9. Merge

> **Mandatory**: always via PR, unless the user explicitly says "direct merge" or "no PR".

```bash
gh pr merge [number] --squash     # features and fixes: keeps the base clean
gh pr merge [number] --merge      # when the commit history matters
gh pr merge [number] --rebase     # teams that prefer a linear history
```

After the merge, mandatory cleanup:

```bash
git checkout <base>                              # never the default branch when an integration branch exists
git pull
git branch -d feat/feature-name                  # delete local branch
git push origin --delete feat/feature-name       # delete remote branch (confirm)
```

---

## 10. Complete flows (quick reference)

Small feature:

```
1. git checkout <base> && git pull
2. git checkout -b feat/name
3. [work]
4. git add . && git commit -m "feat: description"
5. git push --set-upstream origin feat/name
6. gh pr create --base <base>
7. [review approved] -> gh pr merge --squash
8. git checkout <base> && git pull
9. git branch -d feat/name
```

Large feature (working tree, optional): `git checkout <base> && git pull`, then `git worktree add ../repo-feat-name -b feat/name`, work there with regular commits and pushes, `gh pr create --base <base>` when ready, and after the merge return to the main repo, `git checkout <base> && git pull`, `git worktree remove ../repo-feat-name`, `git branch -d feat/name`.

Urgent hotfix: from `<base>`, `git checkout -b hotfix/description`, fix, commit, push with upstream, `gh pr create --base <base> --title "hotfix: critical issue"`, merge after confirmation, return to `<base>` and pull.

Release (only when an integration branch exists): `gh pr create --base <default> --head <integration> --title "release: vX.Y.Z"`, review and approval, `gh pr merge --merge` (preserves history on the default branch), return to `<integration>` and pull.

---

## 11. `gh` CLI reference

Full command reference in `references/gh-commands.md`: authentication, repositories, issues, releases and advanced flags.
