# Validating routes with curl

Validates the **contract**, not the interface. Try `https://` first; if the port does not answer over HTTPS, use `http://` (fallback) and record in the report the scheme used.

## Before writing the call

Confirm field name, route, verb and type by reading the **source code** (controller/DTO `.cs`, Nest `.ts`), never from documentation or memory. An invented field produces a false FAIL.

## Auth

- Find out how the route authenticates (Bearer JWT, cookie) in the code/config.
- Credentials only through `testenv exec` (`"${CLAUDE_PLUGIN_ROOT}/bin/craft.exe" testenv exec --project <root> '<curl with $NAME>'`, single quotes): the values come only from the project's `.specs/project/.env.test`, the output is masked, and `testenv` refuses (exit 3, nothing runs) to send a secret to a host the network list does not allow with `cred`.
- The token comes from the login of step 0 (test user and password; a stored JWT only as the fallback). A login that fails makes the whole flow `UNVERIFIED` (`credential`), not a route `FAIL`. Do not print the token; keep it in a git-ignored file under `.playwright-cli/` and read it inside the `testenv exec` command.
- Hosts only from the network list: `localhost`/`127.0.0.1` and the entries of the project's `.specs/project/NETWORK.json` (when it exists), with the methods each allows. A route on an unlisted host, or a method the entry does not allow, is `UNVERIFIED` (name it); never reach it another way.
- Also check the no-token case: expect 401.

## What to check per route

| Item | How |
|---|---|
| Status | `-w "%{http_code}"` |
| Body | expected fields present and of the right type (`jq`, if available) |
| Error | invalid input → 400 with a message; no permission → 403; nonexistent → 404 |
| Effect | after POST/PUT/DELETE, a GET confirms the state |
| Idempotency | only if the spec asks |

## Form

```bash
"${CLAUDE_PLUGIN_ROOT}/bin/craft.exe" testenv exec --project <root> \
  'curl -sS -k -o .playwright-cli/routes/<step>.json -w "%{http_code}" \
  -X POST "https://localhost:<port>/api/..." \
  -H "Authorization: Bearer $ADMIN_TOKEN" -H "Content-Type: application/json" \
  -d @.playwright-cli/routes/<step>.body.json'
```

`-k` only for `localhost` and `127.0.0.1`. Save the body to a file and read only what is needed; do not dump the whole response into the report.

## Limits

- Writes only on local/DEV, with `zz_validate_` data, and clean up what you created.
- A route that triggers an external effect (email, payment, real webhook): do not call it. Mark `UNVERIFIED` and give the reason.
- Load and performance are out of scope; this is functional validation.
