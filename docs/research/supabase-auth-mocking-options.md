# Findings: minimal Supabase auth mock for frontend tests

Research date: 2026-08-05

## Context

- Ticket: [Research: Evaluate a minimal Supabase auth mock](https://github.com/dcapal/dcapal/issues/759)
- Parent map: [Portfolio management hub and allocation workflows](https://github.com/dcapal/dcapal/issues/742)

This is a research finding, not a product-code change. It checks the frontend's current Supabase calls and compares maintained or credible test options against the request's constraints: no Supabase server, no production runtime dependency, and only the consumed auth/session surface.

## Executive finding

No small, maintained, Supabase-specific off-the-shelf double was found that satisfies all three constraints. The best fit is a test-only in-repository adapter at the `@app/config` seam. Keep the real `@supabase/supabase-js` client in production and use the existing MSW dependency only for the separate REST API boundary.

The closest ready-made package is `@akirilyuk/supabase-in-memory-server`, but it starts a Node HTTP server and therefore fails the no-server requirement. `supabase-mock-fetch` avoids a server, but it mocks database-shaped fetch calls and does not provide the auth/session methods this frontend consumes. The preliminary note was directionally correct, but its “best fallback” should be described as an integration-test option, not as a candidate for ordinary local or CI frontend tests.

## Consumed client surface

The production client is created with `createClient` in [`dcapal-frontend/src/app/config.js`](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/app/config.js#L1-L19). Direct application callers use:

| Method | Repository use | Minimum double contract |
| --- | --- | --- |
| `auth.getSession()` | App startup, router, portfolio sync, API access-token callback | Resolve `{ data: { session }, error: null }`, including `session: null` |
| `auth.refreshSession()` | API access-token refresh callback | Resolve a configured session or error |
| `auth.getUser()` | Navigation-bar user display | Resolve `{ data: { user }, error: null }` |
| `auth.signOut()` | API auth failure and navigation logout | Clear the fixture session and resolve `{ error: null }` or a configured error |
| `auth.onAuthStateChange(callback)` | Router, portfolio sync, login, and signup | Register callbacks and return `{ data: { subscription: { unsubscribe } } }` |

These calls are visible in [`src/index.js`](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/index.js#L34-L61), [`src/routes/router.js`](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/routes/router.js#L31-L55), [`src/hooks/useSyncPortfolios.tsx`](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/hooks/useSyncPortfolios.tsx#L56-L79), and [`src/components/core/navBar.js`](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/components/core/navBar.js#L135-L167). Login and signup also pass the client to Supabase Auth UI components ([login](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/routes/loginPage.js#L68-L91), [signup](https://github.com/dcapal/dcapal-frontend/src/routes/signUpPage.js#L104-L119)). That is a component integration boundary, not evidence that the application needs to implement every Supabase Auth method.

Supabase documents that `getSession` reads client storage and can return `null`, while `getUser` authenticates by contacting the Auth server. It documents `refreshSession`, `signOut`, and the subscription returned by `onAuthStateChange` separately ([`getSession`](https://supabase.com/docs/reference/javascript/auth-getsession#retrieve-a-session), [`getUser`](https://supabase.com/docs/reference/javascript/auth-getuser), [`refreshSession`](https://supabase.com/docs/reference/javascript/auth-refreshsession), [`signOut`](https://supabase.com/docs/reference/javascript/auth-signout), [`onAuthStateChange`](https://supabase.com/docs/reference/javascript/auth-onauthstatechange)). The double should preserve these distinct call shapes, but it need not reproduce storage, JWT, or network authenticity.

## Options compared

### A. Narrow in-repository adapter — recommended

Create a fresh test-only object with an `auth` property exposing the five methods above. Give it an explicit session fixture, user fixture, configurable errors, and a subscriber set. `setSession` or `emitAuthChange` can drive authenticated and anonymous states; `signOut` should clear the session and notify subscribers; `unsubscribe` should remove exactly one callback.

**Fit:** exact. It has no server, no network request, no new package, and no production bundle impact. It also makes the contract visible: adding a new Supabase call causes a deliberate test-double change.

**Limit:** it tests application session handling, not Supabase protocol behavior, refresh-token rotation, OAuth redirects, JWT validity, or RLS. Those are separate integration concerns.

### B. `supabase-mock-fetch` — not sufficient

The package README describes a Map-backed mock `fetch` implementation for Supabase-style database operations, including REST-like GET/POST/PATCH/DELETE examples ([README](https://github.com/nurulhudaapon/supabase-mock-fetch/blob/main/README.md)). Its published manifest is version `0.0.0-alpha.1` and has no runtime dependencies ([package manifest](https://raw.githubusercontent.com/nurulhudaapon/supabase-mock-fetch/main/package.json)); npm currently lists versions through `0.0.0-alpha.2` ([npm metadata](https://registry.npmjs.org/supabase-mock-fetch)). The repository's latest push was 2025-01-22 ([GitHub repository metadata](https://api.github.com/repos/nurulhudaapon/supabase-mock-fetch)).

**Fit:** no server and could be test-only, but poor API fit. Its documented surface is fetch/database handling; it does not expose `auth.getSession`, `refreshSession`, `getUser`, `signOut`, or `onAuthStateChange`. It would leave the main auth double to implement locally, while adding an alpha package for a different boundary.

### C. `@akirilyuk/supabase-in-memory-server` — integration fallback, not default

This package is a more capable community option. Its README documents an in-memory HTTP server that mimics PostgREST and GoTrue, a helper that creates a real Supabase client, and auth routes for signup, password/refresh-token exchange, logout, and get-user ([README](https://github.com/akirilyuk/supabase-in-memory-server/blob/main/README.md)). Its package manifest declares Node 20.9+/22+, a peer dependency on `@supabase/supabase-js`, and Winston ([package manifest](https://raw.githubusercontent.com/akirilyuk/supabase-in-memory-server/main/package.json)); npm lists version `1.0.1` ([npm metadata](https://registry.npmjs.org/@akirilyuk%2Fsupabase-in-memory-server)).

**Fit:** good when the test goal is exercising the real SDK over HTTP. It fails this ticket's ordinary-test constraint because `createMemorySupabaseServer` starts a Node listener, and the README lists meaningful gaps including OAuth/SSO, OTP, MFA, password recovery, refresh edge cases, RLS, Realtime, Storage, and Edge Functions. Keep it as a possible future integration lane, not as the default auth fixture.

### D. Existing MSW plus the real Supabase client — credible but broader than needed

The frontend already has MSW as a development dependency ([`dcapal-frontend/package.json`](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/package.json#L65-L93)) and starts it for browser journeys ([`src/index.js`](https://github.com/dcapal/dcapal/blob/master/dcapal-frontend/src/index.js#L63-L84)). MSW can intercept browser and Node requests without a separate Supabase service, but using it for Auth would require emulating the GoTrue HTTP protocol and the client’s storage/token lifecycle. It would test more real `supabase-js` behavior than an object adapter, but it is a protocol fixture, not a small client double, and it creates more state and response-shape maintenance than the five-method seam requires.

**Fit:** viable for a later SDK/protocol integration scenario; not the narrowest answer to this issue. Keep MSW focused on the repository’s `/api` boundary unless a future test explicitly needs Supabase HTTP behavior.

### E. Local Supabase CLI stack — out of scope for ordinary tests

Supabase’s official testing guidance uses the local stack for database and integration testing ([testing overview](https://supabase.com/docs/guides/local-development/testing/overview), [database testing](https://supabase.com/docs/guides/database/testing)). This gives the highest service fidelity, but it requires the local Supabase services and is therefore not a no-server mock. It belongs in a separate integration lane for migrations, RLS, and service behavior.

## Decision matrix

| Criterion | In-repository adapter | `supabase-mock-fetch` | In-memory server | MSW + real client | Local Supabase stack |
| --- | --- | --- | --- | --- | --- |
| Exact consumed auth surface | Yes | No | Via HTTP, partial | Via HTTP, if emulated | Yes, real service |
| New production runtime dependency | None | None if dev-only | None if dev-only, but test package | None; already present | CLI/services only |
| Supabase server/listener | No | No | Node listener | No Supabase service, but HTTP handlers | Full local stack |
| Tests real SDK behavior | No | Not for auth | Yes | Yes | Yes |
| Ordinary local/CI fit | Best | Insufficient alone | Heavy | Medium/heavy | Poor |
| Protocol/OAuth/RLS fidelity | Fixture-level | None for auth | Limited/documented gaps | Only what handlers model | Highest |

## Recommendation and boundary

Adopt the narrow in-repository adapter for unit and ordinary frontend CI tests. Keep production importing the real client from `@supabase/supabase-js` with `VITE_SUPABASE_URL` and `VITE_SUPABASE_ANON_KEY`. Select the adapter only through test configuration or dependency injection; do not export it from production config, ship fixture tokens, or use unsigned test tokens in a production bundle.

The implementation should start with `getSession`, `refreshSession`, `getUser`, `signOut`, and `onAuthStateChange`. Add Auth UI methods only when an interaction test requires them. Add one contract test covering the return shapes, unsubscribe behavior, and sign-out notification. Retain a separate real-client/server test only when a change needs Supabase protocol, OAuth, refresh-token, JWT, or RLS confidence.

This resolves the research question without changing product code or GitHub issue state.

## Sources

All external sources above are official Supabase documentation, first-party package metadata, or the source repositories/manifests for the compared packages. Repository citations point to the application code that defines the actual consumed surface.
