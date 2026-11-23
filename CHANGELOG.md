# Changelog

All notable changes are listed here, newest first. The format follows Keep a Changelog and the versions follow Semantic Versioning.


## [Unreleased]

### Added

- Git shortcut commands on top of the `github` skill: `/branch-from-dev`, `/branch-from-dev-pr`, `/branch-from-dev-merge`, `/dev-pr-main`, `/dev-merge-main`, `/pr`, `/pull`, `/sync-dev`, `/clean-branches`, `/hotfix`. They read the work from the git state, name the branch themselves and need no arguments. Engine in the `git-shortcuts` skill.
- `/spec-full` (spec, design and tasks in one go, name derived, no test tasks) and `/spec-run` (run the pending tasks up to a named point) over `tlc-spec-driven`.
- `/quality-gate` (standards, bug hunt and spec adherence over finished code, in a subagent by default, read-only) and `/review-pr` (a PR review that includes the gate).

## [0.4.0] - 2026-11-21

### Changed

- Renamed to **Craft Dev Toolkit** (repository and plugin id `craft-dev-toolkit`, formerly `claude-craft-kit`): the skills serve AI coding agents in general, written in the Claude Code plugin format. Reinstall with `/plugin marketplace add iagopadilha/craft-dev-toolkit`; Rolo Loop still finds an install made under the old id.
- Claude Maestro is now Rolo Loop (`iagopadilha/rolo-loop`); every mention follows.
- README rewritten short.

### Added

- First version of the Rust rebuild.
- `craft standards check`: runs the deterministic rules of the skills (`rules.json` next to `backend-dev` and `frontend-dev`) on the changed files, judging only new code; the `llm` rules are left to the Agent Maestro `quality-gate` agent.

### Changed

- `tlc-spec-driven` tasks reference: `tasks.md` still describes the parallel tracks it could split into, but the orchestrator's plan decides; the default is one executor walking the tasks in order (Anthropic, "When to fold subagents into your main agent loop", 2026-10-22).
