# gh CLI: advanced commands

## Authentication
```bash
gh auth login              # interactive login
gh auth status             # current authentication
gh auth logout
gh auth refresh            # refresh the token
```

## Repositories
```bash
gh repo view               # info about the current repo
gh repo view --web         # open in the browser
gh repo clone owner/repo
gh repo fork
```

## Pull requests: full flags

### gh pr create
```bash
gh pr create \
  --title "title" \
  --body "description" \
  --base <base> \            # target branch
  --head feat/my-branch \    # source branch (default: current)
  --draft \
  --reviewer user1,user2 \
  --assignee @me \
  --label "feature,needs-review"
```

### gh pr list
```bash
gh pr list --state open          # default
gh pr list --state closed
gh pr list --state merged
gh pr list --state all
gh pr list --author "@me"
gh pr list --base <base>
gh pr list --label "bug"
gh pr list --limit 20
```

### gh pr view
```bash
gh pr view 42                          # by number
gh pr view feat/branch-name            # by branch
gh pr view --json title,body,reviews   # specific fields
gh pr view --web
```

### gh pr review
```bash
gh pr review 42 --approve
gh pr review 42 --approve --body "LGTM!"
gh pr review 42 --request-changes --body "Needs changes in X"
gh pr review 42 --comment --body "General observation"
```

### gh pr merge (always confirm first)
```bash
gh pr merge 42 --squash
gh pr merge 42 --merge
gh pr merge 42 --rebase
gh pr merge 42 --squash                   # merge; the remote branch is deleted in a separate confirmed step
gh pr merge 42 --auto                     # merge when the checks pass
```

### gh pr checkout / diff / status
```bash
gh pr checkout 42           # check out the PR branch locally
gh pr diff 42
gh pr status                # PRs created by you, assigned to you, review requested
gh pr ready 42              # mark a draft as ready
gh pr close 42              # close without merging (confirm first)
gh pr reopen 42
```

## Issues
```bash
gh issue list
gh issue create --title "Bug: X" --body "Description"
gh issue view 10
gh issue close 10
```

## Releases
```bash
gh release list
gh release create v1.0.0 --title "Release 1.0" --notes "Changelog here"
gh release view v1.0.0
```

## Workflows / Actions
```bash
gh run list                  # CI/CD runs
gh run view [id]
gh run watch                 # follow a run live
gh workflow list
```

## Useful aliases (set once)
```bash
gh alias set prc 'pr create'
gh alias set prm 'pr merge --squash'
gh alias set prl 'pr list'
```
