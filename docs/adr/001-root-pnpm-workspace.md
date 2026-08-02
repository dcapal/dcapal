# ADR 001: Use a root pnpm workspace for frontend dependencies

## Status

Accepted. The temporary decision not to add an Orval client is superseded by
[ADR 002](./002-openapi-tanstack-api-client.md).

## Context

DcaPal currently has one JavaScript application in `dcapal-frontend`. Its npm manifest and lockfile live inside that directory, so package installation, local commands, CI, deployment workflows, and Dependabot all need frontend-specific paths.

The repository is expected to grow shared frontend packages. A workspace boundary and one dependency policy should exist before those packages are introduced.

## Decision

Create a private root pnpm package named `dcapal` and make `dcapal-frontend` the only workspace member for now. Name the application package `@dcapal/frontend` and expose root `frontend:*` scripts that delegate to it with pnpm filters.

Keep all frontend third-party version ranges in the root pnpm catalog. Use one root `pnpm-lock.yaml` and require frozen installs in automation. Keep the generated `dcapal-optimizer-wasm/pkg` dependency as a local file dependency, and build or download it before installing the JavaScript workspace.

Keep the current Webpack configuration, frontend directory, development port, environment variables, generated `dist` directory, routes, and product behavior unchanged. Do not create an empty shared-package directory or add an Orval client in this migration. A later change may add real packages and extend the workspace.

## Consequences

Contributors can install and run frontend commands from the repository root, while package-local pnpm commands remain available. CI and deployment use the same root lockfile and command surface as local development. Dependency versions can be reused by future packages through the catalog.

The migration adds a root JavaScript dependency boundary but does not introduce shared code or change application behavior. Dependabot continues to use its npm ecosystem integration, pointed at the root workspace, because that integration supports the pnpm manifest and lockfile used here.
