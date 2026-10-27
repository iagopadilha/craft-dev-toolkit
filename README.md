# claude-craft-kit

Engineering-standards skills for Claude Code, with a small Rust toolbox behind them.

> Status: under construction. This is a ground-up Rust rebuild; nothing is released yet.

The kit gives Claude Code reusable guidance for backend and frontend work, Git and GitHub hygiene, spec-driven planning and end-to-end flow checks. Your own team's rules plug in through **standards slots** declared in each project's `CLAUDE.md`, so the generic skills never need to know your company.

## Skills

| Skill | What it does | Reads slot |
|---|---|---|
| `backend-dev` | .NET and Node.js (NestJS): layers, naming, SOLID, dependency injection, structured logs, generic repository, behavior regression | `backend`, `app-map` |
| `frontend-dev` | React + Vite + TypeScript (and Next.js): Component, Hook, Query, Service; forms, toasts, grids, UI review | `frontend`, `app-map` |
| `github` | Git and GitHub (`gh`): what needs confirmation, what is never allowed, feature branches, pull requests, local cleanup | `git` |
| `tlc-spec-driven` | Planning in four adaptive phases (Specify, Design, Tasks, Execute) with requirement traceability | none of its own; it loads `backend-dev` or `frontend-dev`, which read theirs |
| `flow-validator` | Checks a flow against the running app: API routes with curl, UI with Playwright; reports PASS, FAIL or UNVERIFIED per step | `app-map`, `observability` |
| `playwright-cli` | Browser automation for tests, E2E checks and screenshots, headless by default | none |
| `ui-component-tests` | Component tests in a real browser with Playwright's component testing | none |
| `ui-trace-analysis` | Recording and reading Playwright traces to find why a test failed | none |

## Standards slots

A generic skill never names a company. It only knows the **name of a slot**. The project says which skill fills that slot, in its `CLAUDE.md`:

```markdown
## Standards
git: acme-git-standards
backend: acme-backend-standards
frontend: acme-design-system
app-map: acme-app-map
observability: acme-grafana
```

The five slots are `git`, `backend`, `frontend`, `app-map` and `observability`. A missing or empty slot means the generic skill uses its own defaults; a bound skill wins where it conflicts; a standards skill holds only the difference. The exact reading rules are in [docs/STANDARDS.md](docs/STANDARDS.md).

## The `craft` binary

One Windows binary with six tools, called by the skills as `bin/craft.exe <tool>`:

| Tool | What it does |
|---|---|
| `verify` | Detects and runs typecheck, lint, build and test (Node and .NET); prints PASS, FAIL or UNVERIFIED |
| `regression_diff` | Compares two JSON snapshots after normalising ids, dates and GUIDs |
| `map_reuse` | Drafts the "Reuse map" section of a project's `CLAUDE.md` |
| `classify_structure` | Suggests STANDARD or LEGACY for a project and writes the Conformance section |
| `sync_core` | Injects the shared core rules into `backend-dev` and `frontend-dev` |
| `testenv` | Test credentials: values only in a git-ignored `.env.test`, a network list that decides where a secret may go, `exec` with masked output |

Exit codes and output formats are stable; each tool documents them with `--help`.

## Works with

[claude-maestro](https://github.com/iagopadilha/claude-maestro) uses this kit for build verification and test environments. The kit also works on its own. This project is not affiliated with Anthropic; "Claude Code" names the product the skills are written for.

## Install

```
/plugin marketplace add iagopadilha/claude-craft-kit
/plugin install claude-craft-kit@claude-craft-kit
```

The `craft` binary is attached to each release; put `craft.exe` in the plugin's `bin/` folder (the release archive already has it there). Windows 10 or 11 is the supported platform; `testenv exec` needs Git for Windows (Git Bash).

## Build from source

```
cargo build --release          # target/release/craft.exe
cargo test --workspace
python tools/craft_diff.py     # compares craft with the reference scripts (needs the reference clone)
```

## Layout

```
plugin/skills/          the eight skills
plugin/shared/         rules injected into the dev skills, templates
crates/craft/          the binary
crates/egress-policy/  network list grammar and judgement (shared with claude-maestro)
crates/pycompat/       small emulations of Python behaviors the tools match
docs/STANDARDS.md      how the standards section is read
```

## Credits and licence

MIT, see `LICENSE`. `tlc-spec-driven` is based on work by Felipe Rodrigues (Tech Lead's Club) under CC BY 4.0 and `playwright-cli` on Microsoft's skill under Apache-2.0; both are modified here. Details in [THIRD_PARTY.md](THIRD_PARTY.md).
