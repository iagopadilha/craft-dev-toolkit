# Changelog

All notable changes are listed here, newest first. The format follows Keep a Changelog and the versions follow Semantic Versioning.


## [Unreleased]

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
