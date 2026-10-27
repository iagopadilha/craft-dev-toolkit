# Standards slots: the contract

The skills of this toolkit are generic. They never name a company, an organization, a design system or an internal app. They know only the **name of a slot**. A project says, in its own `CLAUDE.md`, which skill fills each slot. This is dependency inversion:

- the generic skill depends on the **slot** (an abstraction),
- a company skill **implements** the slot (a detail),
- the project's `CLAUDE.md` **binds** the two (the composition root).

Nothing is bound by default. A project with no binding gets the generic skills' own defaults.

## The `## Standards` section

A level-2 heading named `Standards` in the project's `CLAUDE.md`, followed by one `slot: value` line per bound slot, and nothing else until the next heading:

```markdown
## Standards
git: acme-git-standards
backend: acme:acme-backend-standards
frontend: acme-design-system
app-map: acme-app-map
observability: acme-grafana
```

- **Where it lives.** In a single-project repository, the `CLAUDE.md` at the root. In a monorepo, the `CLAUDE.md` of **each subproject** (the slots usually differ per stack), not the root one.
- **Order and completeness.** Any order. Bind only the slots you need; leave the others out. A slot with an empty value is the same as no binding.
- **Unknown slots** are ignored.
- **Comments.** Do not put comments or prose inside the section: the section is read line by line. A commented-out line (`<!-- git: x -->`) is simply ignored (the parser reports it as a note), and a tool that rewrites the section drops comments.

## How the section is parsed

This is what Claude Maestro's standards service does, and the generic skills read the section the same way. Write the section to fit it:

- The heading is `## Standards` (level 2, any letter case, nothing after it). A heading inside a code fence does not count.
- **A line starting with `#` followed by a space ends the section** (any heading level). Do not put a `# comment` line inside it.
- **Each line is `slot: value`.** No bullets (`- git: x` is an unknown slot), no backticks around the value, no `=` (`git=x` is not read). Blank lines are fine.
- **Slot names are lower-cased** (`Git:` is `git`). The value is kept as written.
- **A duplicate slot: the last line wins.**
- **The plugin prefix is dropped when resolving**: `acme:acme-git-standards` is looked up as `acme-git-standards`, so the prefix does not disambiguate two plugins in that tool.
- A line that is not `slot: value`, an unknown slot, an empty value or an invalid value is ignored and reported as a note, never an error.
- **Tools may rewrite the section** (Maestro's installer does): it keeps the rest of the file but replaces the section, so comments inside it are lost.

## Value syntax

A value is a skill name. It matches:

```
^[A-Za-z0-9_.-]+(:[A-Za-z0-9_.-]+)?$
```

- A bare name: `acme-git-standards`.
- A plugin-qualified name: `acme:acme-git-standards` (plugin `acme`, skill `acme-git-standards`). Claude Maestro drops the prefix when it resolves the skill, so it cannot tell apart two plugins' skills with the same name: keep your skill names unique.

Anything else (spaces, paths, several skills in one line) is not a valid value. One slot binds one skill: if the company wants several rules, they belong in that one skill, which can point to its own references.

## The five slots

| Slot | What the bound skill provides | Read by |
|---|---|---|
| `git` | the organization, the protected branches, the branch naming and the PR flow | `github` |
| `backend` | base URLs, mandatory libraries, real examples, backend rules that win over the generic ones | `backend-dev` |
| `frontend` | the design system: typography, colors, components, ready-made code models | `frontend-dev` |
| `app-map` | which app does what, who calls whom, URLs, local ports and folders | `backend-dev`, `frontend-dev`, `flow-validator` |
| `observability` | where to read the logs and metrics of the running apps | `flow-validator`; an orchestrator's planning and spec agents, such as Claude Maestro's |

How each slot is used, in one line:

- `git`: the `github` skill takes the organization, protected branches and PR flow from it before any git or `gh` action.
- `backend`: `backend-dev` loads it before writing backend code and follows it over its own defaults.
- `frontend`: `frontend-dev` loads it before writing UI code and follows its design system over its own defaults.
- `app-map`: consulted before integrating with another app, and by `flow-validator` to find ports and folders.
- `observability`: `flow-validator` reads logs and metrics through it when a step fails.

`tlc-spec-driven` reads no slot of its own. From Specify onwards it loads the dev skill of the feature's stack (`backend-dev` or `frontend-dev`), and those read the slots.

`playwright-cli`, `ui-component-tests` and `ui-trace-analysis` read none.

## What a generic skill does with a slot

Every skill that reads a slot carries the same short section, "Standards slots". The behavior is always:

1. Read the `## Standards` section of the project's `CLAUDE.md` (in a monorepo, the subproject's).
2. For each slot this skill reads, load the bound skill before working.
3. The bound skill wins where it conflicts with the generic skill.
4. No binding for a slot: use the generic skill's defaults.
5. A bound skill that is not installed: say so in one line, and use the defaults.
6. Never load a company skill that is not bound.

## The rules

1. **No binding, no company rule.** A slot that is missing or empty means the generic skill uses only its own defaults.
2. **The bound skill wins.** Where it conflicts with the generic skill, the bound skill's rule is the one followed.
3. **A standards skill holds only the delta.** It never repeats the generic text. A `git` standards skill can be ten lines: the organization, the protected branches and the PR flow.
4. **A value is a skill name.** A bare name or a plugin-qualified one. A bound skill that is not installed is reported, and the generic default is used.
5. **Generic skills never name a standards skill.** The only places a company name appears are the company's own plugin and the project's `CLAUDE.md`.

## Writing your company's standards plugin

1. **Make your own plugin.** Every standards skill lives in it, never in this toolkit. A plugin is a folder with `.claude-plugin/plugin.json` and `skills/<name>/SKILL.md`; see this repository's `plugin/` for the shape.
2. **Prefix each skill with the company name** (`acme-git-standards`, `acme-backend-standards`, `acme-design-system`, `acme-app-map`, `acme-grafana`), so it is clear what is company-specific and the names never collide with a generic skill.
3. **Write only the delta.** State what differs from the generic skill and nothing else. If you find yourself copying a section of a generic skill, that skill is missing an extension point: open an issue in this repository instead.
4. **Say which slot the skill fills** in its `description`, so it is found for the right task, for example: "Acme's Git rules: organization, protected branches and PR flow. Bound to the `git` slot."
5. **Add the `## Standards` section** to the `CLAUDE.md` template your company uses for new repositories, with the values already filled.
6. **Keep secrets out.** A standards skill is text that gets loaded into context: no tokens, passwords or private URLs that must stay private.

### Example: a `git` standards skill

`acme-git-standards/SKILL.md`:

```markdown
---
name: acme-git-standards
description: Acme's Git rules: the GitHub organization, the protected branches and the PR flow. Bound to the `git` slot, read by the `github` skill.
---

# Acme Git standards

- Organization: `acme-software`. Every repository lives at `github.com/acme-software/<repo>`.
- Protected branches: `main` and `developer`. Never delete them, never push to `main`.
- Feature branches start from `developer`: `feat/<ticket>-<slug>`. PRs target `developer`.
- `developer -> main` only through a release PR titled `release: vX.Y.Z`, merged with `--merge`.
- Every PR title starts with the ticket key (`ACME-123: ...`).
```

The project's `CLAUDE.md`:

```markdown
## Standards
git: acme-git-standards
```

With this, the `github` skill loads `acme-git-standards` before working, uses Acme's organization, protected branches and PR flow, and keeps its own safety rules for everything else.

## Checking a binding

If a slot's skill is not found, the generic skill says so in one line and continues with its defaults. A bound skill is **not installed** when the Skill tool does not find it by that name: check the installed plugins (`/plugin`), the spelling of the value and, for a plugin skill, that the plugin is enabled. A tool that runs the skills for you, such as Claude Maestro, can also report a bound skill that does not resolve.
