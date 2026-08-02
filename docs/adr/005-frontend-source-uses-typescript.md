# ADR 005: Use TypeScript for new frontend source

Status: accepted

## Context

The frontend is moving from JavaScript to TypeScript while the repository remains an incremental migration. New application and browser-test code written in JavaScript does not receive the same type checking as the existing typed API boundary and makes the migration harder to continue consistently. Converting every existing JavaScript file at once would add broad scope and unnecessary risk.

## Decision

Write new frontend application, test, and browser-support modules in TypeScript. Use `.tsx` when a module contains JSX and `.ts` otherwise.

When an existing frontend JavaScript module is substantially rewritten, convert it as part of that change. The more-than-80-percent changed-content rule used by the 2026 migration is a migration heuristic, not a permanent coverage or lint threshold.

Keep JavaScript or ESM JavaScript for Node-only scripts and build or test configuration when adding TypeScript would require runtime tooling without a meaningful benefit. Leave untouched JavaScript modules for a later, focused migration.

Generated source follows its generator. In particular, Orval output is not manually edited; its types and comments come from the OpenAPI source and generation inputs.

## Consequences

New frontend code receives compiler and editor support and aligns with the typed API-client boundary. The repository will remain mixed during the incremental migration, but the boundary is intentional: product and browser-test source moves towards TypeScript, while tooling and generated output keep their own constraints.

The repository does not need a broad JavaScript-to-TypeScript rewrite or a permanent numeric threshold. A future lint or CI rule may enforce the new-file convention if the migration needs stronger protection.

## Considered options

- Continue writing new frontend source in JavaScript: rejected because it prolongs the migration and bypasses type checking for new behavior.
- Convert every existing frontend JavaScript file immediately: rejected because it expands the change beyond the modules being actively maintained and increases migration risk.
- Convert build and Node-only tooling now: rejected because the language change would add runtime configuration without a clear product or maintenance benefit.
