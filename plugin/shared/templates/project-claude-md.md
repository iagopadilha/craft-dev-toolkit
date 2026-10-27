# Standard project CLAUDE.md (template)

Copy to the root of each project as `CLAUDE.md` and fill it in. It is a **pointing summary**, not documentation: it says
where the reusable generic methods/components are and what each structure contains, so the skill (and the agent) can **find
before creating**. Generate the draft of the map with `"${CLAUDE_PLUGIN_ROOT}/bin/craft.exe" map_reuse <root>` and then
curate it by hand (one description line per item). Keep it to about 150 lines.

```markdown
# <Project>

<!-- Optional: when the project keeps .specs/project/RULES.md (permanent limits), add an @-import of that path on its own line
     here, so every session reads it. Without that file, leave this out. -->

<!-- CONFORMANCE:BEGIN -->
## Conformance
Conformance: STANDARD|LEGACY   (do not fill by hand: `"${CLAUDE_PLUGIN_ROOT}/bin/craft.exe" classify_structure <root> --write STANDARD|LEGACY`)
<!-- CONFORMANCE:END -->

## Standards
<!-- Optional. One line per slot you bind: uncomment it (remove the <!-- --> around it) and put your skill name. Leave out the slots you do not use. -->
<!-- git: <skill-name> -->
<!-- backend: <skill-name> -->
<!-- frontend: <skill-name> -->
<!-- app-map: <skill-name> -->
<!-- observability: <skill-name> -->

## How to run and verify
- Build/typecheck/lint/test: `"${CLAUDE_PLUGIN_ROOT}/bin/craft.exe" verify` (or: <project commands>)
- Run locally: .NET with `--launch-profile https` (from `launchSettings.json`), https first, http only if it does not answer; the front end with the scripts of its `package.json`. Do not hand-write a `dotnet run` line without the https profile. (When an orchestrator such as Claude Maestro manages the repo, it writes this as a generated `How to run` block between `RUN:BEGIN` and `RUN:END`.)

## Reuse map (read BEFORE creating a method, function, hook, component or type)
Look here; if you found it, **reuse or generalize**, do not create a twin. Created something reusable? Add the line.

| I need to... | Use | Where | Signature / note |
|---|---|---|---|
| Format a tax ID / document number | `FormatDocument` | `src/Common/Formatters/DocumentFormatter.cs` | `string Format(string, DocumentType)` |
| Call an API with handled errors | `ErrorHandler` | `src/lib/error-handler.ts` | use in the service's `onError` |
| Save several entities at once | `BatchSaver` | `src/Infra/Persistence/` | takes `Add/Update/Remove` |

## Structure (what exists in each folder)
| Folder | What it has | Can receive new code? |
|---|---|---|
| `src/Common/` | generic helpers and extensions | yes, if truly generic |
| `src/Domain/` | entities and rules, no I/O | yes |
| `src/Infra/` | repositories, savers, integrations | yes |
| `<legacy folder>` | <description> | no (only spot fixes) |

## Local conventions (only what differs from the skill)
- <e.g. logs with BeginScope, toasts only via @/components/toast>

## Where the rest is
`.specs/project/RULES.md` (limits, always read) · `.specs/project/STATE.md` (only the now) · `.specs/features/<slug>/` (specs)
```

**The `## Standards` section** binds the standards slots (see `docs/STANDARDS.md` for the exact syntax). Keep it free of bullets, backticks and
prose: it is read line by line, and any line starting with `#` ends it. A tool that rewrites the section drops the comments.

**How the skill uses it:** `backend-dev` and `frontend-dev` (N1) read the "Reuse map" section before naming or
creating; then they confirm with Grep in the code (the map may be stale). Project without the file: the skill offers to generate the
draft with `map_reuse.py`. Owner of the file: the main agent (when an orchestrator such as Claude Maestro runs the
project, only the orchestrator; a subagent suggests a line in `NOTES`).
