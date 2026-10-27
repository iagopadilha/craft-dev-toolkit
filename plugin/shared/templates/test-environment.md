# Test environment

Copy to `.specs/project/TEST-ENVIRONMENT.md`. Names only: a value never goes in this file. Values live in
`.specs/project/.env.test` (git-ignored; create it with `"${CLAUDE_PLUGIN_ROOT}/bin/craft.exe" testenv init --project <root>`).
Allowed hosts live in `.specs/project/NETWORK.json` (start from `network.example.json`).
An app with no authentication needs no credentials: write `Auth: none` below and skip the credential sections.

## Apps

| App | URL (https first, http fallback) | Start command | Health route |
|---|---|---|---|
| api | https://localhost:5001 | `dotnet run --launch-profile https` | `/health` |
| web | https://localhost:5173 | `npm run dev` | `/` |

## Auth

- Auth: login | token | none
- Login route: `POST https://localhost:5001/auth/login` with `{"email": "$TEST_USER_1_EMAIL", "password": "$TEST_USER_1_PASSWORD"}`; the token is in the response field `token`.
- Fallback: stored token `$ADMIN_TOKEN`, sent as `Authorization: Bearer $ADMIN_TOKEN`.

## Test users

| Name | Role | Variables |
|---|---|---|
| User 1 | customer | `$TEST_USER_1_EMAIL`, `$TEST_USER_1_PASSWORD` |
| User 2 | admin | `$TEST_USER_2_EMAIL`, `$TEST_USER_2_PASSWORD` |

## Headers per route group

| Group | Header |
|---|---|
| `/api/*` | `Authorization: Bearer <token from login>` |
| `/webhooks/*` | `X-Signature: $WEBHOOK_SECRET` |

## Do not exercise

- Routes that send real e-mail, charge money or write to production data.
