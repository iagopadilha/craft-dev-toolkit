# Third-party material

claude-craft-kit is MIT licensed (see `LICENSE`). Two skills are based on third-party work and keep their own licences. Everything else in the repository is original.

## `tlc-spec-driven` (skill folder `plugin/skills/tlc-spec-driven/`)

- Based on the TLC Spec-Driven skill by Felipe Rodrigues (https://github.com/felipfr) of the Tech Lead's Club community (https://github.com/tech-leads-club), version 2.0.0.
- Licence: Creative Commons Attribution 4.0 International (CC BY 4.0), https://creativecommons.org/licenses/by/4.0/.
- Modified here. The list of changes is in `plugin/skills/tlc-spec-driven/NOTICE.md`.

## `playwright-cli` (skill folder `plugin/skills/playwright-cli/`)

- Based on the `playwright-cli` skill of https://github.com/microsoft/playwright-cli, copyright Microsoft Corporation.
- Licence: Apache License 2.0; the licence text is `plugin/skills/playwright-cli/LICENSE`.
- Modified here. The list of changes is in `plugin/skills/playwright-cli/NOTICE.md`.

## Skills written for this kit

`ui-component-tests` and `ui-trace-analysis` are original text. They cover the same subjects as Playwright's component testing and trace viewer, and point to the official documentation for details; no upstream text is reproduced.

## Tools and conventions cited, not copied

Playwright, the GitHub CLI (`gh`), Conventional Commits 1.0.0, TanStack Query, shadcn/ui, AG Grid, Flurl, Newtonsoft.Json and FluentValidation appear by name in skills and examples. Their names belong to their owners; nothing of theirs is bundled.

## Rust crates in the `craft` binary

The crates the binary links, with their licences, are listed by `cargo deny list` (policy in `deny.toml`). A full notice file is generated at release time.
