---
name: flow-validator
description: Validates a FLOW end to end against the application actually running. Two layers - API routes via curl (contract - status, payload, auth) and UI E2E via Playwright (playwright-cli skill). Starts the needed applications (tries HTTPS first and falls back to HTTP if the port does not answer, one per port, closing the previous one if the port collides), runs the script and returns PASS, FAIL or UNVERIFIED per step, with evidence. The flow comes from what was implemented (spec and diff) OR from what the user describes. Use when the user asks to validate a flow, test E2E, test the screen, test the routes, "check this really works", or when the orchestrator asks for the flow validation phase. Do NOT activate for unit tests, build, lint or typecheck (that is the verifier), nor for writing a permanent test suite in the repo. Use ONLY when the user asks (or in the testing phase they call). Never on its own initiative after implementing or fixing something, nor for automatic retesting.
---

# Validate flow

Runs what was built and checks that it works. It does not fix code: it reports.

Two layers, which are not mixed:

| Layer | Tool | Proves |
|---|---|---|
| **Route** | `curl` | API contract: status, body, auth, error. Does not prove the screen works. |
| **E2E** | `playwright-cli` (`playwright-cli` skill) | User flow in the interface, from click to visible result. |

Curl alone **is not E2E**. A report with only curl says "routes validated", never "E2E validated".

## Standards slots

Read the `## Standards` section of the project's `CLAUDE.md` (in a monorepo, the subproject's). For each slot this skill reads (`app-map`, `observability`), load the bound skill before working. It wins where it conflicts with this skill. No binding means this skill's defaults. A bound skill that is not installed: say so in one line and use the defaults. Never load a company skill that is not bound.

## Steps

0. **Credential gate.** An app with NO authentication needs no credential: skip this gate and go on to step 1 (note `auth: none` in the report). Otherwise, before the first route or screen of the flow, prove the credentials work: log in (or call ONE authenticated route with the stored token) through `craft testenv exec`. Prefer the test user (`$TEST_USER_1_EMAIL` + `$TEST_USER_1_PASSWORD`) and use the token the login returns; a stored JWT in `.env.test` is the fallback. It fails (no login, 401/403, expired token, missing name)? The WHOLE flow is `UNVERIFIED` with reason `credential`, never `FAIL`: a credential problem says nothing about the code under test. Do not try another way in. Details in `references/environment.md`.
1. **Flow.** Build or read the script (see Input). Save it in `flows.md`.
2. **Environment.** Find out what must be up, start it, wait for it to become healthy. See `references/environment.md`.
3. **Routes.** If the flow touches an API, validate the routes with curl before the UI. See `references/curl-routes.md`. A broken route makes the corresponding UI step `UNVERIFIED` by dependency, not a UI `FAIL`.
4. **E2E.** If the flow has a screen, run it with Playwright. Load the `playwright-cli` skill first. Open the session without a URL, install the network filter, then navigate (see `references/e2e-front.md`). See that file for the rest.
5. **Cleanup.** Close the browser, shut down what you started, remove the test data you created.
6. **Report.** In the format below.

## Input

Two sources, and they can be combined:

- **From what was implemented.** If the project has `.specs/project/FLOWS.md` with a `## [<slug>]` section for the feature, start from it: its numbered steps are the flow and its `Expected:` lines the expected results. Then read the spec (acceptance criteria, requirement IDs) and the feature's diff, and derive one flow per observable criterion the section does not cover.
- **From what the user asks.** Free text ("log in, open flow X, filter by Y, check column Z"). Turn it into a script.

The script follows `references/flow-format.md`. Save it in `.specs/features/<slug>/flows.md` when there is a feature; without a feature, in `.playwright-cli/flows/<name>.md` or at the path the user gives. If the flow came from the user and is ambiguous (no login data, no success criterion), ask before running. If it came from the spec, proceed without asking.

**Credentials and headers come from `.specs/project/TEST-ENVIRONMENT.md`** (when it exists; template in `${CLAUDE_PLUGIN_ROOT}/shared/templates/test-environment.md`, with `network.example.json` and `env-test.example` beside it; `"${CLAUDE_PLUGIN_ROOT}/bin/craft.exe" testenv init --project <root>` creates the git-ignored `.env.test` with names only and adds it to `.gitignore`). If `${CLAUDE_PLUGIN_ROOT}` is empty or not expanded, use the `TOOLKIT:` path given in the prompt, or locate this plugin's folder (`~/.claude/plugins/cache/**/claude-craft-kit/*/`). The file holds: app and auth URLs, DEV test users, the header of each route group, webhook signature and the list of what must not be exercised. Read it before building the script. The file carries only **NAMES** (`$ADMIN_TOKEN`, `$TEST_USER_1_EMAIL`); values live ONLY in `.specs/project/.env.test` (git-ignored; never read or write that file; no environment variable and no machine-wide `.env` is read at run time). Run every command that needs a credential through `"${CLAUDE_PLUGIN_ROOT}/bin/craft.exe" testenv exec --project <root> '<command with $NAME>'` (single quotes): it fills the values and masks the output. It also refuses (exit 3, nothing runs) to give a secret to a host that the network list does not allow with `cred`. A name missing? `craft testenv check --project <root>` lists it. A secret value never goes into evidence, the report or the summary (write `***`). If the script needs a credential the file lacks (or still says `FILL IN`): do not guess or log in another way; return `UNVERIFIED` with the list of what is missing (the variable names), so the user (or the orchestrator, when one runs you) can fill it in before the next round. Never ask for the value in the chat.

## Hard rules

- **HTTPS first, HTTP as fallback.** For every URL, curl, browser and app, front or back, try `https://` first. If the host or port does not answer over HTTPS, fall back to `http://`: the fallback is allowed and is the default, without stopping or asking. Record in the report which scheme was used and say when there was a fallback. If the user gave a specific URL or scheme, follow theirs. `curl -k` and `ignoreHTTPSErrors` only for `localhost` and `127.0.0.1`.
- **One application per port.** Before starting any app, check whether the port is occupied. If it is, identify the process. A process **this skill started** (recorded in `.playwright-cli/environment.json`): stop it and start again. A process it did **not** start (an app the user left running: IDE, dotnet, Vite, node): **ask the user before killing it**, unless the project's `CLAUDE.md` or the orchestrator prompt says killing local processes on the flow's ports is pre-authorized; then kill it, start again with the current code and record the PID and process in the report (`CLEANUP`/`NOTES`). Not authorized and the user does not answer: `UNVERIFIED` with the port and PID named. Only local processes on this machine: never take down anything on a remote server or production. Never two instances on the same port: it causes CORS issues and answers from the wrong app.
- **Never production.** Run only on local or DEV. Production URL in the script: stop. Production reads only through a read-only database tool when you need to check data, never writes.
- **No PASS without running.** Could not start the app, could not authenticate, browser did not open: `UNVERIFIED`, with the reason.
- **Marked test data.** What you create carries the prefix `zz_validate_` (or the domain's equivalent) and is removed at the end. Do not delete data you did not create.
- **Credentials only through `testenv exec`.** Never in the script, the report, a flag typed with the real value or a file in the repo, and never by reading `.env` files. A token the login returns stays in a git-ignored file under `.playwright-cli/` and is read inside the same `testenv exec` command; do not print it, delete it at cleanup.
- **Hosts only from the network list.** Reach only `localhost`/`127.0.0.1` and the hosts of the project's `.specs/project/NETWORK.json` (when it exists, see `network.example.json`; an orchestrator's `config.json` may add more), with the methods each entry allows, and send credentials only to entries marked `cred`. A host the flow needs that is not listed: `UNVERIFIED` with the host named; never reach it another way (another tool, a proxy, a redirect, a script that builds the URL) and never edit the list yourself: ask the user to add it.
- **Do not edit project code.** Only create flow files and artifacts in `.playwright-cli/`.
- **Do not run write git commands.** Reading `git diff`/`git status` is fine.

## When to open the browser with a window (headed)

The default is **headless**. Use headed only when:

- the user asked to watch;
- manual login, captcha or MFA is needed;
- a step failed in headless and you need to debug what the page shows.

When using headed, warn in one line before opening the window.

## Report

```
FLOW: <name or slug>
VERDICT: PASS|FAIL|UNVERIFIED
LAYERS: route | e2e | route+e2e
ENVIRONMENT: <app>:<port> <https|http>[, fallback from https] ..., or "-"
STEPS:
1. <step> — PASS
2. <step> — FAIL: <expected> x <got> | evidence: <path>
3. <step> — UNVERIFIED: <reason>
EVIDENCE: <paths in .playwright-cli/>
CLEANUP: <what was started and shut down; data removed>
NOTES: up to 3 lines, or "-"
```

Overall `VERDICT`: any `FAIL` → `FAIL`; all `PASS` → `PASS`; some `UNVERIFIED` without `FAIL` → `UNVERIFIED`. Do not paste logs or whole response bodies: only the line that identifies the problem and the path to the evidence.

## References

- `references/flow-format.md` — how to write the script.
- `references/environment.md` — step 0 (credential gate), the network filter, find, start, wait for and shut down the applications; HTTPS with fallback to HTTP; ports.
- `references/curl-routes.md` — validating API routes (credentials only through `testenv exec`, hosts only from the network list).
- `references/e2e-front.md` — Playwright: network filter, session, headless/headed, HTTPS with fallback to HTTP, local dev cert, evidence.
