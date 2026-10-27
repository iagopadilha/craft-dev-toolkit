---
name: ui-trace-analysis
description: >
  Reading and debugging Playwright traces: open a trace, follow the actions in order, find the failing
  step, compare the DOM snapshots before and after, and read the network and console records around it.
  Use when a Playwright test or flow failed and left a trace, or when the user asks to record or inspect
  a trace. Triggers: "trace", "trace.zip", "show-trace", "why did the test fail", "playwright report",
  "flaky test". Not for writing tests (see ui-component-tests) or running a flow (see flow-validator).
---

# UI trace analysis

Own text for this kit. It describes how to record and read Playwright traces. Option names can change between Playwright versions: when a command here is rejected, check `npx playwright --help` and the official documentation of the installed version.

## Record a trace

- Tests: set `use: { trace: 'on-first-retry' }` in the Playwright config (or `'retain-on-failure'` to keep only failing runs). For a single run: `npx playwright test --trace on`.
- A script that drives a browser by hand: `await context.tracing.start({ screenshots: true, snapshots: true, sources: true })` before the steps and `await context.tracing.stop({ path: 'trace.zip' })` after.
- Keep traces in the project's artifact folder (`test-results/`, ignored by git). A trace holds page content and request data: never commit one and never attach one to a public place without checking it for secrets.

## Open a trace

- `npx playwright show-trace path/to/trace.zip` opens the trace viewer. A report links to its traces: `npx playwright show-report`.
- The viewer has: a timeline of screenshots, the list of actions, and tabs for the call details, the log, the console, the network, the sources and the errors.

## Reading order

1. **Errors tab / the red action.** Find the first failing action. Later failures are usually consequences.
2. **The call details** of that action: the locator used, the wait log ("waiting for locator", "element is not visible", "intercepts pointer events"), the timeout.
3. **Before and after snapshots** of the action: is the element there? Is another element covering it? Is the page still on the previous screen?
4. **Network tab** around the same moment: a request that failed, returned an error status, or never finished explains most "element not found" cases. Look at status, timing and the response body.
5. **Console tab:** uncaught errors and warnings the page logged just before the failure.
6. **Sources tab:** the line of the test that ran the failing action.

## Typical causes

| What the trace shows | Likely cause | Fix direction |
|---|---|---|
| Locator waits until timeout, snapshot shows the element with other text | Text or role changed | Update the locator to the visible contract |
| Click retried, "intercepts pointer events" | Overlay, toast or loading mask on top | Wait for it to go away, or fix the overlay |
| Action passes, next assertion fails, request in network is 4xx/5xx | Backend or data problem, not the UI | Report the request and response; do not patch the test |
| Passes alone, fails in the suite | Shared state between tests | Isolate data and storage per test |
| Different result on retry | Race between render and data load | Wait on a user-visible signal, not a fixed delay |

## Report

Give: the failing step (action, locator, test line), what the snapshot and the network show, the most likely cause, and the smallest fix. If the cause is in the application, say so and stop; do not weaken the test to make it pass.
