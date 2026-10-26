# claude-craft-kit

A set of engineering-standards skills for Claude Code, with a small Rust toolbox behind them.

> Status: under construction. Ground-up Rust rebuild; nothing is released yet.

The kit gives Claude Code opinionated, reusable guidance for backend and frontend work, Git/GitHub hygiene, spec-driven planning and end-to-end flow checks. Your own team rules plug in through "standards slots" declared in each project's `CLAUDE.md`, so the generic skills never need to know your company.

## What is in the box

- Skills: `backend-dev`, `frontend-dev`, `github`, `flow-validator`, `tlc-spec-driven`, `playwright-cli`, `ui-component-tests`, `ui-trace-analysis`.
- `craft`, a single Windows binary: `verify`, `regression_diff`, `map_reuse`, `classify_structure`, `sync_core`, `testenv`.
- Templates and shared rules injected into the backend and frontend skills.

## Works with

`claude-maestro` uses this kit for build verification and test environments. The kit also works on its own.

## License

MIT for original content, see `LICENSE`. Some skills are derived from third-party work under their own licenses; see `THIRD_PARTY.md` and `NOTICE`.
