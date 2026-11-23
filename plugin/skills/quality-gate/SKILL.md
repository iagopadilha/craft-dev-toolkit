---
name: quality-gate
description: >
  Judge the quality of code that is already implemented: structure and good practice by the backend-dev and
  frontend-dev skills (layers, hook apart from component, naming, SOLID, reuse, logs), a hunt for bugs, and adherence
  to the spec when one exists. Runs in a subagent by default and only reads. Use when the user types /quality-gate or
  asks to validate a finished feature against the standards.
disable-model-invocation: true
---

# /quality-gate

A review of finished work against the toolkit's own standards. It adds no rules of its own: the checklist is what the
`backend-dev` and `frontend-dev` skills (and the skills bound in the project's `## Standards` section) already say.

Natural place in the flow: after `/spec-run` has implemented a feature, before the PR.

## Usage

| Command | Runs |
|---|---|
| `/quality-gate` | in a **subagent** (default), over the diff of the current branch against `<base>` |
| `/quality-gate --inline` | in the main agent |
| `/quality-gate <path>` | over a folder or file instead of the diff |
| `/quality-gate --fix` | after the report, applies the safe fixes (without the flag it only reads) |

## Subagent mode

Start one `general-purpose` agent with a self-contained brief: the scope (file list and base branch), the steps below
and the output format. It returns only the report. Do the review yourself only with `--inline` or when no agent tool
exists.

## Steps

1. **Scope.** The files changed on the current branch against `origin/<base>` (the `github` skill, section 0, names the
   base), or the path given. Skip generated files, lockfiles, build output and `.specs/`.
2. **Stack per file.** React, Vite, TSX and frontend TS go to `frontend-dev`. `.cs` and NestJS TypeScript go to
   `backend-dev`. Load the skill and use its text as the checklist. Load each skill bound in the project's
   `## Standards` section (`backend`, `frontend`) and let it win where it conflicts.
3. **Deterministic rules first.** Run `bin/craft.exe standards check --scope <repo> --files <changed files>` (the
   `bin` folder of this plugin). It runs the `rules.json` rules of both skills, judges only new code and does not edit.
   A tool that is missing, or exit code 3: say it ran UNVERIFIED and go on with the manual review.
4. **Structure and good practice.** For the changed code, judge the `llm` rules in each `rules.json` and the skill text.
   Typical points:
   - frontend: hook apart from the component, no business rule in a component, service and query apart, componentization
     and single responsibility, TanStack Query used as the skill says, forms and toasts in the pattern, visual
     consistency with the bound frontend standard;
   - backend: thin controller, rule in the service, data access in the repository, dependency injection, SOLID,
     descriptive names with no abbreviations, structured logs without secrets, error handling, reuse of what exists.
5. **Bug hunt.** Read the changed code for defects: wrong conditions, null and empty cases, off-by-one, race conditions,
   leaked resources, swallowed errors, an unhandled failure path, a changed contract with an unchanged caller. Read the
   callers of anything whose behavior changed.
6. **Spec adherence.** When `.specs/features/<name>/` matches the branch, check each **Done when** against the code.
   Report the ones the code does not satisfy.
7. **Hygiene.** Duplication, a file or function that grew too large, dead code, leftover debug output, typecheck and lint
   of the touched files when the project has cheap scripts. Do not run the test suite unless the user asks.

## Output

One line per finding, no prose around it:

```
FAIL  src/pages/Orders.tsx:48        fetch inside the component -> move it to a useOrders hook
FAIL  Services/OrderService.cs:112   abbreviation "cfg" -> name it "configuration"
BUG   Services/PaymentService.cs:73  amount can be null when the order has no lines -> guard before Sum
SPEC  T4                             Done when "filter keeps the selected status" is not met: reset on rerender
PASS  hook/component separation in src/hooks/*
Summary: 3 FAIL, 1 BUG, 1 SPEC, 14 checked
```

Order: BUG, SPEC, FAIL, PASS. A finding needs a file, a line and the fix in one line. Never report a style
preference as a defect. No finding, and say what was checked: "PASS, 14 checked".

## `--fix`

Only after the report. Apply fixes that keep behavior: moves, renames, extraction, removal of dead code or debug output.
Never touch behavior to satisfy a style rule. Never commit: leave the changes in the working tree and list them.

## Limits

Read-only without `--fix`. Never push, never open a PR, never run the test suite on its own. A missing rule file or an
unknown stack is reported as UNVERIFIED for that part, not as a pass.
