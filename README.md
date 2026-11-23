# Craft Dev Toolkit

**Development standards for AI coding agents, as skills, with a small toolbox behind them.**

Your agent gets reusable, company-free guidance for backend and frontend work, Git and GitHub, spec-driven planning and end-to-end flow checks. Your team's own rules plug in through **standards slots** in each project's `CLAUDE.md`, so the generic skills never need to know your company.

Written in the Claude Code plugin format; works with Claude Code and compatible agent CLIs.

## Skills

| Skill | What it does |
|---|---|
| `backend-dev` | .NET and Node.js (NestJS): layers, naming, SOLID, dependency injection, structured logs |
| `frontend-dev` | React + Vite + TypeScript (and Next.js): components, hooks, queries, services, UI review |
| `github` | Git and GitHub: what needs confirmation, what is never allowed, branches, pull requests |
| `tlc-spec-driven` | Planning in four adaptive phases (Specify, Design, Tasks, Execute) with traceable requirements |
| `flow-validator` | Checks a flow against the running app: API with curl, UI with Playwright; PASS, FAIL or UNVERIFIED |
| `playwright-cli` | Browser automation for tests, E2E checks and screenshots |
| `ui-component-tests` | Component tests in a real browser |
| `ui-trace-analysis` | Reading Playwright traces to find why a test failed |

## Shortcut commands

Type `/` and pick one. Each takes no arguments: the repository is the current folder and the work is read from the git state.

| Command | What it does |
|---|---|
| `/branch-from-dev` | Reads the work, names a `feat/*` or `fix/*` branch from the updated base, commits and pushes |
| `/branch-from-dev-pr` | The same, then opens the pull request |
| `/branch-from-dev-merge` | Reuses the open PR or creates one, merges it (merge commit) and deletes the feature branch |
| `/dev-pr-main`, `/dev-merge-main` | Pushes the work on the integration branch straight to it (no new branch), then opens the PR to the default branch, or merges it. The integration branch is never deleted |
| `/pr`, `/pull`, `/sync-dev`, `/clean-branches`, `/hotfix` | Open the PR of the current branch, pull the repo and current branch (or `/pull <branch>`), bring the base in, delete merged local branches, urgent fix from the default branch |
| `/spec-full`, `/spec-run` | A complete `spec.md`, `design.md` and `tasks.md` from one description; then run the tasks up to the point you name |
| `/quality-gate`, `/review-pr` | Judge finished code against `backend-dev` and `frontend-dev`, hunt bugs, check the spec; the PR review includes the gate |

Branches, the organization and the protected branches come from the `github` skill and the `git` slot below. A merge never
bypasses a failing check, a missing approval or a conflict, and a protected branch is never deleted.

## Standards slots

A generic skill only knows the name of a slot. The project says which skill fills it, in its `CLAUDE.md`:

```markdown
## Standards
git: acme-git-standards
backend: acme-backend-standards
frontend: acme-design-system
```

Slots: `git`, `backend`, `frontend`, `app-map`, `observability`. An empty slot means the skill's own defaults; a bound skill wins where it conflicts. Details in [docs/STANDARDS.md](docs/STANDARDS.md).

## The `craft` binary

One Windows binary the skills call as `bin/craft.exe <tool>`: `verify` (typecheck, lint, build, test), `testenv` (test credentials kept out of the repo, with a network list), `regression_diff`, `map_reuse`, `classify_structure`, `sync_core`. Each documents itself with `--help`.

## Install

```
/plugin marketplace add iagopadilha/craft-dev-toolkit
/plugin install craft-dev-toolkit@craft-dev-toolkit
```

The release archive carries `craft.exe` in the plugin's `bin/` folder. Windows 10 or 11; `testenv exec` needs Git for Windows.

**Installing from a clone or from source:** the repository does not track `plugin/bin/`, so the plugin has no `bin/craft.exe` until you build it. Rolo Loop's `harness doctor` reports this as `lacks bin/craft.exe`, and `/plugin update` does not fix it. You need the Rust toolchain (`winget install Rustlang.Rustup`) and the Visual Studio Build Tools with the C++ workload. Then run, from the repository root:

```
powershell -File tools\install-local.ps1
```

It builds `craft.exe` and copies it to `plugin/bin/` and to every cached version under `~/.claude/plugins/cache/craft-dev-toolkit/`. Run it again after a plugin update or a version bump, because a new cache folder starts without the binary.

## Works with

[Rolo Loop](https://github.com/iagopadilha/rolo-loop) runs spec-driven build loops with these skills and uses `craft` for verification and test environments. The toolkit also works on its own.

## Build from source

```
cargo test --workspace
cargo build --release          # target/release/craft.exe
```

## Licence

MIT, see [LICENSE](LICENSE). `tlc-spec-driven` is based on work by Felipe Rodrigues (Tech Lead's Club, CC BY 4.0) and `playwright-cli` on Microsoft's skill (Apache-2.0), both modified; see [THIRD_PARTY.md](THIRD_PARTY.md). Not affiliated with Anthropic.
