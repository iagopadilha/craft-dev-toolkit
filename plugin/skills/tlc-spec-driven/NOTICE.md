# Notice

This skill folder is based on the **TLC Spec-Driven** skill (version 2.0.0) by **Felipe Rodrigues** (https://github.com/felipfr), from the **Tech Lead's Club** community (https://github.com/tech-leads-club).

It is licensed under **Creative Commons Attribution 4.0 International** (CC BY 4.0): https://creativecommons.org/licenses/by/4.0/. The original licence applies to the original parts; the changes below are also offered under CC BY 4.0. No endorsement by the original author or by the Tech Lead's Club is implied.

## Changes made in Craft Dev Toolkit

- A short **rule on tests** at the top of `SKILL.md`: spec, design and tasks carry no tests; "Done when" is observable behavior; running tests, curl or E2E is a validation phase of its own, at the end, and only when asked. The passages of `references/tasks.md`, `implement.md`, `validate.md`, `quick-mode.md` and `specify.md` that described test tasks, "tests pass" lines or per-task test runs were rewritten to match.
- Planning documents live in `docs/codebase/` (with a read of the legacy `.specs/codebase/`), with `DEPS`, `PATHS`, `KIND` and `SCOPE` metadata per feature, a scope guard and notes on a `PreToolUse` hook.
- A failure policy, repository hygiene notes, the `tasks.md` format for orchestrated runs, a test-prerequisites section in Specify, and slot-driven loading of the `backend-dev` and `frontend-dev` skills.
- Conformance (STANDARD or LEGACY) gating of refactors, a short orchestrated `PROJECT.md`, `RULES.md` limiting rules, and a handoff note about writing decisions to `STATE.md` and `RULES.md`.
- Command lines of the helper tools point to the `craft` binary of this kit.
