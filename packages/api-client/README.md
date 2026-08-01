# `@dcapal/api-client`

This package is the generated REST client for DcaPal Backend. The backend OpenAPI document is the source of truth.

## Generate

From the repository root, regenerate the backend document and Orval output with:

    pnpm api-client:generate

The generated production operations live under `src/gen`. Generated MSW handlers live under `src/gen-mocks`. Generated files are committed and must not be edited by hand.

## Use

The package root exports generated TanStack Query hooks, query options, request functions, and models. The `@dcapal/api-client/model` subpath exports generated models. The `@dcapal/api-client/mocks` subpath exports generated MSW handlers for tests.

The frontend configures the `/api` base path and authentication callbacks once at application startup. The package does not import Supabase. Feature-specific response composition, such as the Yahoo price-provider adapter, belongs in the frontend.

## Test

Use the generated handlers with MSW for package and application tests. Register application-specific fixtures before the generated handlers when a DcaPal flow requires stable data. Use MSW handler overrides for per-test success and failure cases; do not call a live backend from unit tests.
