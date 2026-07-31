# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before exploring, read these

- **`CONTEXT-MAP.md`** at the repo root — it identifies the relevant app context for the work and points to its `CONTEXT.md`.
- **The relevant app's `CONTEXT.md`** — read the context for `dcapal-frontend`, `dcapal-backend`, or `dcapal-optimizer-wasm` before exploring that app.
- **`docs/adr/`** — read system-wide ADRs that touch the area you're about to work in.
- **The relevant app's `docs/adr/`** — read app-specific ADRs that touch the area you're about to work in.

If any of these files don't exist, proceed silently. Don't flag their absence; don't suggest creating them upfront. The `/domain-modeling` skill (reached via `/grill-with-docs` and `/improve-codebase-architecture`) creates them lazily when terms or decisions actually get resolved.

## File structure

Multi-context repo:

```
/
├── CONTEXT-MAP.md                         ← app boundaries and shared concepts
├── docs/adr/                              ← system-wide decisions
├── dcapal-frontend/
│   ├── CONTEXT.md                         ← frontend glossary and invariants
│   └── docs/adr/                          ← frontend-specific decisions
├── dcapal-backend/
│   ├── CONTEXT.md                         ← backend glossary and invariants
│   └── docs/adr/                          ← backend-specific decisions
└── dcapal-optimizer-wasm/
    ├── CONTEXT.md                         ← optimizer glossary and invariants
    └── docs/adr/                          ← optimizer-specific decisions
```

`CONTEXT-MAP.md` is the navigation point for the three app contexts and records shared DcaPal concepts and cross-app boundaries. Define app-specific terms and invariants in the corresponding `CONTEXT.md`.

## Use the glossary's vocabulary

When your output names a domain concept, use the term as defined in the relevant `CONTEXT.md` or, for shared concepts, `CONTEXT-MAP.md`. Don't drift to synonyms that the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007 (event-sourced orders) — but worth reopening because…_
