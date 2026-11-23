---
name: git-shortcuts
description: >
  Shared engine behind the git shortcut commands /branch-from-dev, /branch-from-dev-pr, /branch-from-dev-merge,
  /dev-pr-main, /dev-merge-main, /pr, /sync-dev, /clean-branches and /hotfix. It is loaded by those commands.
  Do not activate it on your own for ordinary git requests: the `github` skill covers those.
---

# Git shortcuts

One engine, many short commands. Each command is a thin skill that names a **mode** and loads this file. The user types
the command with **no arguments** and expects the whole job done: you find the repository, read the work, name the
branch and run the steps. An argument is only an override.

Everything here sits on top of the `github` skill. Load it first and follow its context rules (section 0: `<default>`,
`<integration>`, `<base>`), its `## Standards` slot `git` and its safety rules. This file adds the shortcuts and never
loosens a rule that the `github` skill marks as forbidden.

## What the command itself confirms

Typing a command is an explicit request for the steps listed for that mode. For a `-merge` mode this includes the PR
merge and the deletion of the feature branch (local and remote). It does **not** include:

- deleting a protected branch (never, even on request);
- `--force`, `--admin`, `--no-verify`, `reset --hard`, `branch -D`;
- bypassing a failing check, a missing approval or a conflict.

Any of these stops the run and is reported.

## 1. Read the situation (every mode)

```bash
git rev-parse --show-toplevel
git remote get-url origin
git branch --show-current
git fetch --prune
git status --short
git log --oneline origin/<base>..HEAD
gh auth status
```

- The repository is the current folder's. Never ask for it.
- **Local protected branches are updated at two moments only.**
  1. **Before a branch is created or compared** (`branch`, `branch-pr`, `branch-merge`, `hotfix`, `release-pr`,
     `release-merge`, `sync`): fast-forward the local base so the new branch starts from the remote state.
     `pr` and `clean` do not pull.
  2. **After a merge** (`branch-merge`, `release-merge`): pull what the merge changed, as written in those modes.

  Update without leaving the current branch: `git fetch origin <branch>:<branch>` (fast-forward only) for a branch that
  is not checked out, `git pull --ff-only` for the one that is, when its tree is clean. A branch that cannot
  fast-forward is reported, never reset.
- If the active `gh` account is not the owner of `origin`, say so before any push or PR.
- The base comes from the `github` skill section 0 and the `git` slot. No slot and no integration branch: the base is the
  default branch.

## 2. Detect the work (modes that create or ship a branch)

The work is whatever is **not yet on the base**:

1. uncommitted changes (staged, unstaged, untracked);
2. commits on the current branch that are ahead of `origin/<base>`.

Leave out, and say so in one line, anything that looks like a secret: `.env*`, `*.pem`, `*.key`, `*.pfx`,
`credentials*`, `*secret*`, files under `.specs/` and build output that `.gitignore` already hides.

Nothing to ship: say "nothing to ship" and stop.

Where the work sits decides the branch:

| You are on | Work | Branch to use |
|---|---|---|
| `<base>` (or another protected branch) | uncommitted only | create a new branch from the updated `<base>` and carry the changes |
| `<base>` (protected) | commits ahead of `origin/<base>` | stop and ask: those commits are not on the remote base, and moving them would touch a protected branch |
| a feature branch | any | **keep this branch**. It is the branch the user means. Do not create another one |

To create the branch from the updated base:

```bash
git checkout <base>
git pull --ff-only          # with changes: git stash -u, pull, git stash pop
git checkout -b <type>/<slug>
```

A conflict on `stash pop`, or a pull that is not a fast-forward: stop, report, leave the stash in place.

## 3. Name and classify

The user does not type the name. Read the diff (and commit subjects) and decide:

| Type | Signals |
|---|---|
| `fix` | corrects wrong behavior: a condition, a null case, a wrong value, a failing path; small and targeted |
| `feat` | new files, routes, components, endpoints, options; behavior that did not exist |
| `refactor` | moves, renames, extraction with the same behavior |
| `docs` | only Markdown or comments |
| `chore` | config, dependencies, CI, tooling, formatting |

The slug is two to five lowercase ASCII words joined by `-`, at most 40 characters, from the main subject of the diff:
`fix/status-filter-empty-list`. An argument given by the user replaces the generated name. If the project's `git` slot
names a different pattern, the slot wins.

Print the plan in at most three lines before running, for example:

```
fix/status-filter-empty-list  <-  developer  |  4 files, 1 commit
steps: branch, commit, push
```

Run it right away. Pause only when something is ambiguous (two unrelated changes, work on a protected branch, a mixed
diff you cannot name) or when a rule above stops you.

## 4. Commit

- Stage files by name. Never `git add .` blindly. Never stage the files excluded in section 2.
- One commit per logical change. A diff with two unrelated concerns becomes two commits.
- Conventional Commits: `type(scope): description`, imperative, no trailing period. Add the scope only when the diff
  clearly lives in one area.
- Use the configured git identity. Never change `user.name` or `user.email`.
- If `CONTRIBUTING.md` or the repository requires a sign-off (DCO), commit with `-s`.
- Add the commit attribution lines the session instructs, when it instructs any.
- Do not run the test suite. Typecheck or lint of the touched files is welcome when the project has a cheap script, and
  never blocks.

```bash
git push --set-upstream origin <branch>      # first push
git push                                     # later pushes, no force
```

## 5. Modes

### `branch` (/branch-from-dev)

Sections 1 to 4, then report: branch, short hash, number of files. Stop there.

### `branch-pr` (/branch-from-dev-pr)

`branch`, then open the PR to `<base>`:

```bash
git log origin/<base>..HEAD --oneline
gh pr view --json number,url,state 2>/dev/null     # a PR already open for this branch is reused
gh pr create --base <base> --head <branch> --title "<conventional title>" --body "<body>"
```

- Title: the commit subject when there is one commit, otherwise a Conventional Commits summary of the whole branch.
- Body: the `github` skill template (section 7), filled from the diff. "How to test" lists observable steps to check
  by hand. Do not tick or invent a test checklist.
- A PR already open for the branch: push the new commits, report its URL, do not open another.

### `branch-merge` (/branch-from-dev-merge)

One command, two paths, decided by the current state:

1. **A PR is already open** for the current branch: push any pending commits and use it.
2. **No PR:** run `branch-pr` first (branch, commit, push, open the PR).

Then merge and clean:

```bash
gh pr checks <number>                  # pending: gh pr checks <number> --watch (wait up to ten minutes)
gh pr merge <number> --merge           # merge commit, never --admin, never --auto
git checkout <base>
git pull --ff-only
git branch -d <branch>                 # local
git push origin --delete <branch>      # remote; skip when GitHub already removed it
```

- The merge strategy is **merge commit**.
- Not mergeable (required review missing, a failing check, a conflict, branch protection): **stop and report** the
  reason and the PR URL. If a review is the only thing missing, say who has to approve.
- Never delete a protected branch. The head branch of this mode is a feature branch, so the delete is allowed.
- After the merge, local `<base>` is pulled (`git pull --ff-only`) so it holds the merge commit, and `<default>` is
  updated too when it differs from `<base>`.
- After the cleanup the repository is on `<base>`, up to date, and the feature branch is gone locally and remotely.

### `release-pr` (/dev-pr-main)

Needs an integration branch. Without one the work already targets `<default>`: say so and stop.

```bash
git fetch --prune
git log origin/<default>..origin/<integration> --oneline     # nothing: say "nothing to release" and stop
gh pr view --head <integration> --base <default> 2>/dev/null  # an open PR is reused
gh pr create --base <default> --head <integration> --title "release: <summary>" --body "<commit list>"
```

The title summarizes the commits; the body lists them. Report the URL.

### `release-merge` (/dev-merge-main)

`release-pr`, then checks and `gh pr merge <number> --merge` with the same stop rules as `branch-merge`.
**Never delete `<integration>`** and never pass `--delete-branch`. Afterwards update **both** local branches: `git checkout <default> && git pull --ff-only`, then
`git checkout <integration> && git pull --ff-only`, and finish on `<integration>`.

### `pr` (/pr)

Open the PR of the **current** branch to `<base>`, by the `github` skill section 7: push if needed, check for an open PR,
create it with the title and body rules above. On a protected branch: stop, there is no PR to open from it.

### `sync` (/sync-dev)

Bring `origin/<base>` into the current feature branch.

- On a protected branch: stop.
- Dirty tree: `git stash -u`, sync, `git stash pop`.
- Branch **not shared** (no upstream, or no commits that are not local): `git rebase origin/<base>`.
- Branch already pushed: `git merge origin/<base>`. Never rewrite pushed history, never force-push.
- A conflict: stop, list the conflicted files, do not resolve them yourself unless the user asks.

### `clean` (/clean-branches)

```bash
git fetch --prune
git branch --merged origin/<base>
git branch -vv        # branches whose upstream is [gone]
```

List the candidates (merged into `<base>` or gone from the remote), excluding the current branch and every protected one.
Delete them with `git branch -d` (safe). A branch that `-d` refuses is not merged: list it and ask before any `-D`.
Finish with the count deleted and the count kept.

### `hotfix` (/hotfix)

For an urgent fix that must go to the default branch.

1. Detect the work (section 2) and name it `hotfix/<slug>`.
2. Create the branch from `origin/<default>`, carry the work, commit and push (section 4).
3. A PR into a protected branch affects the production baseline: **show the plan and ask once** before opening it.
4. Open the PR with `--base <default>`.
5. Do not merge in this mode. When the user later asks, merge it as in `branch-merge`, then offer a PR
   `<default> -> <integration>` so the fix also reaches the integration branch.

## 6. Output

End every run with the result only: branch, commit hashes, PR URL, merge state. No narration of the git calls. A stop says
what blocked it and what the user can do next, in one or two lines.
