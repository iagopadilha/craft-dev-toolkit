---
name: ui-component-tests
description: >
  Component tests for React (and Vue) components that run in a real browser with Playwright's component
  testing: set up the runner, mount a component with props, drive it like a user, assert on what is
  rendered, fake callbacks and providers. Use only when the user asks for component tests, or when the
  spec says component tests are part of the delivery. Triggers: "component test", "test this component
  in the browser", "mount the component", "playwright-ct", "ct config". Not for end-to-end flows (use
  flow-validator) and not for unit tests of plain functions.
---

# UI component tests

Own text for this kit. It describes how to write and run component tests with Playwright's component testing package. Check the official Playwright documentation for the current status and exact option names of the package before relying on a detail here: component testing has been marked experimental, and the package name carries that word.

## When to use

- The user asks for component tests, or the spec lists them in the delivery.
- Do not add them on your own: the planning rule of this kit keeps tests out of specs, designs and task lists unless the user asks.
- A flow across several pages is an end-to-end check: use `flow-validator`. A pure function is a plain unit test of the project's own runner.

## Setup (once per project)

1. Install the package for the framework in use: `@playwright/experimental-ct-react` (React) or `@playwright/experimental-ct-vue` (Vue), as a dev dependency, plus the browsers (`npx playwright install chromium`). Headless is the default; keep it.
2. Create `playwright-ct.config.ts` at the project root with `defineConfig` from the package. Set `testDir` to the folder of the component tests, `snapshotDir` next to it, `use.ctPort` to a free port and, only if the app needs it, `use.ctViteConfig` to reuse aliases and plugins of the app's Vite config.
3. Create the mount page the runner serves: `playwright/index.html` with a `<div id="root"></div>` and a script tag that loads `./index.tsx`. In `playwright/index.tsx` import the global styles, fonts and any CSS reset the app loads, so components look the way they do in the app.
4. Add a script, for example `"test:ct": "playwright test -c playwright-ct.config.ts"`.
5. Add `playwright/.cache/` to `.gitignore`.

## Writing a test

- File name: `<Component>.ct.tsx` next to the component or in the configured `testDir`.
- Import `test` and `expect` from the package (not from `@playwright/test`), and the component.
- `mount` is a fixture: `const component = await mount(<Button label="Save" onClick={handler} />);`. Props go in as ordinary JSX props.
- Drive the component with locators the way a user would: `getByRole`, `getByLabel`, `getByText`. Avoid CSS selectors and test ids unless the component has no accessible name.
- Assert on what is rendered: `await expect(component).toContainText('Saved')`, `toBeVisible()`, `toBeDisabled()`, `toHaveValue(...)`.
- Callbacks: pass a function that records the call (`const calls: string[] = []; onChange={v => calls.push(v)}`) and assert on the array after the action. Functions run in the browser and report back; keep their arguments simple and serialisable.
- Update props: `await component.update(<Button label="Saved" />)`. Remove: `await component.unmount()`.
- Providers (router, query client, theme): wrap the component in a small test wrapper component defined in a file next to the test and mount the wrapper. For app-wide providers shared by every test, register them with `beforeMount` in `playwright/index.tsx`, and pass per-test choices through `hooksConfig`.
- Network: the component runs in a browser page, so use `page.route(...)` to fake an API instead of hitting a real service.
- Time and randomness: freeze them in the test (`page.clock`, fixed ids) so the result does not change between runs.

## Rules of thumb

- One behavior per test; name the test by what the user sees ("shows the error under the field").
- Test the component's contract (props in, events and DOM out), not its internal state or hooks.
- Never put real credentials, tokens or customer data in a test; use obviously fake values.
- A failing test: read the error and the trace (see `ui-trace-analysis`), fix the component or the test, and stop. Do not loop on retries.

## Delivery

State which command ran (or that none ran, when nothing was asked to run), how many tests, and what they cover. Say what was left out on purpose.
