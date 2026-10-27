# Environment: find, start, wait, shut down

This skill is the one that starts the applications. Do not assume they are already running: check.

## Step 0: credential gate

**An app with NO authentication needs no credential: skip this gate and go on.** Otherwise, before the first route or screen of the flow (the app must be up, so do it right after "Start" below), prove the credentials work. Everything goes through `testenv exec` (`"${CLAUDE_PLUGIN_ROOT}/bin/craft.exe" testenv exec --project <root> '<command>'`) (on Windows it needs Git Bash; never the WSL `bash.exe` in System32; a command running past 600 s is killed with exit 124). The values come only from the project's `.specs/project/.env.test` (create it with `craft testenv init --project <root>`; templates in `${CLAUDE_PLUGIN_ROOT}/shared/templates/`: `test-environment.md`, `network.example.json`, `env-test.example`), and `testenv` will not give a secret to a host the network list does not allow with `cred`.

1. Read `.specs/project/TEST-ENVIRONMENT.md` for the login route and the names (`$TEST_USER_1_EMAIL`, `$TEST_USER_1_PASSWORD`, the token name).
2. **Preferred:** log in with the test user and keep the token the login returns (save the response body in a git-ignored file under `.playwright-cli/`, read the token from there inside later `testenv exec` commands, never print it).
3. **Fallback:** no user and password, but a stored token (a JWT in `.env.test`): call ONE authenticated read route with it.
4. Any failure (name missing, `testenv` refused with exit 3, login rejected, 401/403, expired token) = the WHOLE flow is `UNVERIFIED` with reason `credential`. Never `FAIL`, never a partial PASS, never another way in. Name the missing name or the refused host in the report, and nothing else about the secret.

Only after the gate passes run the routes and the screens.

## Network

The flow may reach only `localhost`/`127.0.0.1` and the hosts of the project's `.specs/project/NETWORK.json` (when it exists; the machine's `config.json` may add more), each with its own methods and `cred` flag. A host that is not listed is `UNVERIFIED` (name the host) and is never reached another way. The browser is held to the same list by the network filter (see `e2e-front.md`): install it right after opening the session without a URL and before the first navigation, never remove it.

## Where the repos are

Locate the repository (or repositories) of the flow's apps from the working directory and the paths named in the spec or by the user. If the project keeps an app map or a folder index (README, docs, monorepo workspace file), read it; otherwise use `ls` on the workspace root and its backend/frontend/monorepo folders. Ignore archived or deprecated projects unless asked. The disk is the source of truth: if it diverges from a map, tell the user.

A name cited without a prefix: monorepo before a standalone repo; active before archived. Ambiguous between active ones: ask.

## How to find the port and the command

Port and command are not documented in a single place. Read them from the project, in this order:

| What | Where to look |
|---|---|
| .NET backend: URL and port | `Properties/launchSettings.json` of the `*.Api` project (`https` profile) |
| NestJS backend | `package.json` scripts (`start:dev`), `main.ts` (`listen`), `.env` |
| Vite frontend | `vite.config.*` (`server.port`, `server.https`), `package.json` scripts (`dev`) |
| Next frontend | `package.json` scripts |
| Any of them | `README.md` of the repo and the folder, `docker-compose.yml` |
| Backend the frontend calls | the frontend's `.env*` (`VITE_*_URL`) |

The frontend points at the backend through an environment variable. Make sure the backend URL the frontend uses is the one of the backend you started (same port and same scheme). If it does not match, adjust through a process environment variable, **without editing a project file**; if only editing a file solves it, stop and ask.

## HTTPS

- .NET backend without a useful `launchSettings.json`, or when another port is needed: `dotnet build` first and then `dotnet run --no-build --project <csproj> --urls http://127.0.0.1:<port>` (confirmed: it starts and answers 200; shut the process down at the end and check the port was freed).
- .NET backend: use the `https` profile from `launchSettings.json` (`dotnet run --launch-profile https`). If the dev certificate is not trusted, `dotnet dev-certs https --trust` is a user action: warn, do not run it yourself.
- Default scheme, for front and back: try `https://` first. If the host or port does not answer over HTTPS, use `http://` (fallback allowed, without stopping or asking) and record in the report the scheme used and that there was a fallback.
- A frontend (Vite, React, Next) on `localhost`/`127.0.0.1` without `server.https` or a certificate falls into this fallback and starts on `http://localhost:<port>`. If the project already serves the frontend over HTTPS, follow what it does.
- Self-signed cert of the local backend: Playwright with `ignoreHTTPSErrors` (see `e2e-front.md`); curl with `-k` **only** for `https://localhost` and `127.0.0.1`. Remote host with an invalid cert: do not use `-k` or `ignoreHTTPSErrors`; treat it as HTTPS not answering and fall back to HTTP.
- If the user gave a specific URL or scheme, follow theirs.

## One per port

Before starting each app:

1. Check who uses the port. PowerShell: `Get-NetTCPConnection -LocalPort <port> -State Listen -ErrorAction SilentlyContinue | Select-Object OwningProcess`. POSIX: `lsof -i :<port> -sTCP:LISTEN` (or `ss -ltnp 'sport = :<port>'`).
2. Port free: start.
3. Port occupied by a process **this skill started earlier** (recorded in `.playwright-cli/environment.json`): stop it and start again.
4. Port occupied by **another** local process (an app the user left running: IDE, dotnet, Vite, node): **ask the user before killing it**, unless the project's `CLAUDE.md` or the orchestrator prompt says it is pre-authorized. Get the PID and name (PowerShell `Get-Process -Id <pid>`; POSIX `ps -p <pid> -o comm=`). Once authorized: kill it (PowerShell `Stop-Process -Id <pid> -Force`; POSIX `kill <pid>`, then `kill -9 <pid>` only if it does not exit), check the port was freed and start again with the current code. Rebuild allowed. Record in the report which PID/process was terminated. Not authorized and no answer: `UNVERIFIED`, naming the port and PID.

Never take down anything on a remote server or production; this applies only to local processes on this machine.

Never leave two instances on the same port. The same rule applies between different apps competing for the port.

## Start

- Run in the background, with the log in a file (`.playwright-cli/logs/<app>.log`).
- Record in `.playwright-cli/environment.json`: app, command, port, PID. This lets you shut down later and recognize "I started that".
- Wait until healthy: try the `https://` URL and, if the port does not answer over HTTPS, `http://` (fallback), until it answers (limit ~60s, polling every 2s). Prefer a health route if one exists; otherwise the root.
- Did not answer within the limit: `UNVERIFIED` with the last useful log line. Do not retry in a loop.
- Database: the backend starts against the local/DEV configuration the project already has. Never switch the connection string to production.

## Shut down

At the end (and also if the flow aborts): stop only the PIDs you recorded, check the port was freed, delete the record. If the user asked to leave it running, leave it and say so in the report.
