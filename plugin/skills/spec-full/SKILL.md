---
name: spec-full
description: >
  Write a complete spec.md, design.md and tasks.md for a feature from one description, reading the code first. The
  feature name is derived, never asked. Builds on the tlc-spec-driven skill and never adds test tasks. Use when the user
  types /spec-full or asks for the full spec, design and tasks of a feature in one go.
disable-model-invocation: true
---

# /spec-full

One description in, three finished documents out: `spec.md`, `design.md` and `tasks.md`.

Load the `tlc-spec-driven` skill and follow its rule on tests and its references: `specify.md`, `design.md`, `tasks.md`
and, for an unmapped codebase, `brownfield-mapping.md`. This command only changes the entry point and the sizing.

## What changes against plain `tlc-spec-driven`

- **All three documents, always.** Auto-sizing does not skip Design or Tasks here. A small feature gets short documents,
  never missing ones.
- **No name to type.** Derive the slug from the description: two to five lowercase ASCII words joined by `-`, at most
  40 characters. Create `.specs/features/<slug>/`. If the folder already exists, ask once whether to continue it or
  use `<slug>-2`.
- **No tests anywhere.** No test task, no "Tests pass", no test command in "Done when" or in a check. "Done when" is
  observable behavior. Typecheck and lint of the touched files stay part of each task's delivery.
- **No questions by default.** Read the code and decide. Anything you had to assume goes in an `Assumptions` list at the
  top of `spec.md`. Ask only when the description is blocked by a missing fact that the code cannot answer, and then ask
  all such questions at once.

## Steps

1. Read the description. Read `.specs/project/RULES.md` and `STATE.md` when they exist.
2. Read the code that the feature touches: the entry points, the layer it lives in, the closest existing feature, and
   anything reusable. In a repository with no `.specs/codebase/`, do a light brownfield mapping of just that area.
3. Read the `## Standards` section of the `CLAUDE.md` (the subproject's, in a monorepo) and load each bound skill that
   applies to the design (`backend`, `frontend`, `app-map`). The generic `backend-dev` and `frontend-dev` skills guide the
   layering of the design.
4. Write `spec.md`: goal, scope and out of scope, numbered requirements with IDs, acceptance as observable behavior,
   edge cases, assumptions.
5. Write `design.md`: the files and layers to touch, components and their responsibilities, data flow, contracts and
   what is reused from the repository, each decision with its reason. Match the existing structure of the project.
6. Write `tasks.md`: atomic tasks `T1..Tn`, each with what, where, depends on, reuses and **Done when**. Keep status
   `pending`. Trace each task to the requirement IDs it covers. Order them so each one leaves the code working.
7. Check that every requirement is covered by a task and every task by a requirement. Fix gaps before finishing.
8. Do not commit `.specs/` and do not execute any task.

## Output

Report the folder path, the requirement count, the task count and the assumptions that matter. Then say how to run it:
`/spec-run` for the next task, `/spec-run until T3` to stop at a given task.
