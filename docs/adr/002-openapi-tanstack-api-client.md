# ADR 002: Use one generated OpenAPI client package with TanStack Query

The frontend needs one stable API boundary while the backend already owns a checked-in OpenAPI document. Create `@dcapal/api-client` as a root-workspace package under `packages/api-client`; generate TanStack Query v5 operations with Orval from `dcapal-backend/docs/openapi.json`, and generate MSW handlers from the same document under the package's `./mocks` export. Use one native `fetch` mutator for the `/api` base path and frontend-provided authentication callbacks. Keep feature-specific response composition in the frontend, not in the generated package.

This supersedes ADR 001's temporary decision not to add a shared Orval client. Generated output is committed so normal frontend builds do not require Rust or Orval, while an explicit generation command keeps the output synchronized with the backend contract. The separate MSW output gives package and application tests deterministic request interception without creating a second runtime client package.

## Considered options

- Keep Axios service wrappers inside `dcapal-frontend`: rejected because it duplicates the backend contract and leaves caching and request lifecycle in feature code.
- Create separate production and test workspace packages: rejected because the reference layout and the required public surface are better represented by one package with `./mocks` and `./model` subpaths.
- Put DcaPal-specific Yahoo and portfolio transformations in the generated package: rejected because generated code should remain a transport contract; those transformations belong to frontend price-provider adapters.
