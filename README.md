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

## Works with

[Rolo Loop](https://github.com/iagopadilha/rolo-loop) runs spec-driven build loops with these skills and uses `craft` for verification and test environments. The toolkit also works on its own.

## Build from source

```
cargo test --workspace
cargo build --release          # target/release/craft.exe
```

## Licence

MIT, see [LICENSE](LICENSE). `tlc-spec-driven` is based on work by Felipe Rodrigues (Tech Lead's Club, CC BY 4.0) and `playwright-cli` on Microsoft's skill (Apache-2.0), both modified; see [THIRD_PARTY.md](THIRD_PARTY.md). Not affiliated with Anthropic.
