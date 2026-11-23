---
name: spec-run
description: >
  Execute the pending tasks of an existing spec, up to the point the user names (next task, until T3, only T2 and T4,
  all), and fold in changes the user adds to the request. Builds on the tlc-spec-driven Execute phase. Use when the user
  types /spec-run or asks to implement the tasks of a spec.
disable-model-invocation: true
---

# /spec-run

Run the Execute phase of `tlc-spec-driven` over `tasks.md`, and stop exactly where the user said.

Load the `tlc-spec-driven` skill and follow `references/implement.md` for each task. Its rule on tests applies: no test
tasks, no test runs unless the spec specifies a validation phase or the user asks for it.

## Which spec

- An argument that matches a folder in `.specs/features/` selects it.
- Otherwise the only feature folder, or the one with pending tasks that changed last.
- Several candidates and no way to tell: ask once.

## How far to go

| User writes | Runs |
|---|---|
| `/spec-run` | the next pending task whose dependencies are done |
| `/spec-run until T3` (or `até T3`) | every pending task up to and including `T3`, in dependency order, then stops |
| `/spec-run only T2 T4` (or `só T2 T4`) | only those tasks (stop and report if a dependency is not done) |
| `/spec-run all` (or `tudo`) | every pending task |
| `/spec-run from T4` | `T4` and everything after it |

When the request is unclear, run the next task only.

## Adding to the request

The user may add a change in the same message, for example "and make T4 also cover X" or "add a task for Y after T2".
Apply it **first**: edit `tasks.md` (and `spec.md` or `design.md` when the change reaches them), keep IDs stable, add
new tasks with the next free ID, then run the part you were asked to run. Say in one line what you changed.

## Per task

1. Branch guard: on a protected branch, stop and tell the user to run `/branch-from-dev`. Work happens on a feature branch.
2. Check the dependencies are `done`. State the plan for the task in two or three lines.
3. Implement it, reusing what `design.md` says to reuse.
4. Check the **Done when** by reading the code against it. Run typecheck and lint of the touched files when the project
   has cheap scripts for it.
5. One atomic commit for the task: `type(<feature>): T3 <title>`, staged by file name, never `.specs/`.
6. Set the task `Status` to `done` in `tasks.md` and note any decision in `STATE.md`.

A failure on a task (typecheck, lint, a Done-when you cannot satisfy): fix it once, and if it still fails mark the task
`failed`, stop and report. Do not move on and do not revalidate on your own.

## Output

At the stop point: tasks done, the commit hashes, the tasks still pending and the next one to run. If the last pending
task is done, suggest `/quality-gate`.
