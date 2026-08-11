# DcaPal frontend guidance

These rules apply to work inside `dcapal-frontend/`. Read the repository root `AGENTS.md`, `CONTEXT-MAP.md`, this app's `CONTEXT.md`, and any relevant ADRs before changing behavior.

## Start a frontend task

1. Read the ticket's user-visible acceptance criteria and identify the journey that proves them.
2. Read the visual or product references named by the ticket. Treat them as references for the requested surface, not as a reason to copy implementation details.
3. Find the existing API, state, and test seams before adding a new one.
4. Decide which behavior belongs in a custom hook or pure module and which markup belongs in a component.

The start step is complete when the affected route, user journey, state transitions, and highest honest test seam are clear.

## UI work

- Prefer shadcn/ui primitives when they fit the interaction. Build product-specific components as reusable components so the same behavior and accessibility contract can be used by more than one screen.
- Keep the interface mobile-first and accessible. Preserve the shared behavior between mobile cards and desktop tables, clear names and focus order, keyboard support, Escape dismissal, visible validation, and non-colour status cues.

## Component structure

- Keep UI components mostly about markup, composition, and accessible states.
- Put query and mutation orchestration, form state, validation, transitions, and derived view data in custom hooks or pure modules. Keep hooks focused on one behavior and make them reusable when more than one surface needs that behavior.
- Keep feature-specific response composition in the frontend. Use the generated API client for transport types and the shared MSW handlers for browser tests; do not hand-edit generated source.
- Write new frontend product and browser-test source in TypeScript, following the existing ADRs and build configuration.

## Tests

- Prefer Playwright end-to-end journeys at the browser boundary for connected user behavior. Cover the real route, dialogs, forms, loading and error states, persistence-visible results, and return navigation.
- Use MSW fixtures to control backend responses in browser journeys. Do not depend on live market providers in automated tests.
- Use focused unit or integration tests for pure calculations, custom hooks, transport edge cases, cache behavior, and other logic that has no honest browser path. Avoid tests that only fake a component tree or assert implementation details when a user journey can prove the behavior.
- Exercise responsive UI at mobile and desktop viewports when the behavior or layout changes across widths. Assert accessible names, focus behavior, validation messages, and keyboard interaction as user-visible behavior.

## Completion check

Frontend work is complete when the affected Playwright journey passes at each changed viewport, focused tests cover logic with no honest browser path, generated source has not been hand-edited, and the applicable type, lint, build, or test commands from the repository pass. Report any environment-dependent check that could not run.
