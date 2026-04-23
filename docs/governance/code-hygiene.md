# Codebase Hygiene Standard

## Purpose

This document defines source consistency and readability standards used across the CANIC workspace (`crates/canic`, `crates/canic-core`, and related canisters).

Its goal is to keep the codebase easy to navigate while preserving architectural boundaries (`endpoints -> workflow -> domain/pure decision helpers -> ops -> storage`).

If this document conflicts with `AGENTS.md`, follow `AGENTS.md`.

## 1. Import Organization

Keep imports grouped, stable, and only at the top of each file.

Preferred grouping:

- `crate` imports
- `std` imports
- external crate imports

Rules:

- Avoid `super::super::...` paths.
- `super::...` is allowed only for local module-relative references.
- Prefer grouped `crate::{...}` imports over scattered long paths.
- Do not introduce new `use crate::...` imports in the middle of the file.
- Keep `use` blocks consolidated instead of scattered.

Required top-of-file sequence for module files:

1. `mod ...;` declarations
2. one blank line
3. `use ...;` imports
4. one blank line
5. re-exports (`pub use ...;`, `pub(crate) use ...;`)
6. one blank line
7. constants, types, functions

`#[cfg(test)] mod tests;` belongs with other `mod` declarations.

## 2. Module Header Comments

For non-trivial modules, add a short module-level header (`//!`) that states:

- responsibility
- ownership boundary
- what the module explicitly does not own

This is especially important in orchestration-heavy areas (`workflow/rpc`, lifecycle, replay).

## 3. Type Documentation

Public types should document:

- what the type represents
- which layer owns it
- where it is used

For structs, follow the repo’s canonical doc block style:

```rust
///
/// StructName
///

pub struct StructName;
```

Keep spacing around doc blocks consistent and scannable.

## 4. Function Documentation

Every function should have a concise intent comment:

- public functions: prefer `///`
- private/internal: `//` is acceptable

For non-trivial functions, split the body into short semantic phases (validation, mapping, execution, commit, cleanup).

When attributes are present, keep order as:

1. docs/comments
2. attributes
3. function declaration

## 5. Section Banners

Use section banners only when they improve navigation across groups of related functions.

Example:

```rust
// --- Validation -----------------------------------------------------
// --- Mapping --------------------------------------------------------
// --- Execution ------------------------------------------------------
```

## 6. Function Ordering

Prefer stable ordering:

- public API
- constructors/builders
- core logic
- helpers/utilities
- tests

When a type and impls share a file, keep inherent impl close to the type and trait impls nearby unless separation is intentional.

## 7. Function Size

Functions much longer than ~80 lines should be reviewed for decomposition.

- split by semantic phase
- reduce nesting
- extract helpers when branches represent distinct responsibilities

## 8. Visibility and Layer Boundaries

Minimize visibility by default.

Guidance:

- `model` internals: private / `pub(crate)`
- `ops` and `workflow` internals: `pub(crate)` unless cross-crate API requires `pub`
- boundary DTO/API types: `pub` only where needed

Never use visibility shortcuts to bypass layering.

## 9. Invariants and Error Semantics

Production paths should return typed errors, not panic.

- prefer `Result<_, InternalError>` through ops/workflow boundaries
- infra-facing APIs must keep `InfraError` semantics intact
- avoid `unwrap()`/`expect()` outside tests and intentional invariants

For replay/idempotency paths, keep failure modes explicit and typed (no string-matching behavior).

## 10. Naming Consistency

Use existing CANIC vocabulary consistently. Prefer established names such as:

- `request_id`, `ttl_seconds`, `payload_hash`
- `parent_pid`, `root_pid`, `subnet_pid`
- `canister_role`, `subnet_role`
- `Replay`, `Capability`, `Delegation`, `Attestation`

Do not invent parallel names for existing concepts.

## 11. Match Expression Hygiene

Large `match` expressions should dispatch to helpers rather than embedding long branch bodies.

This keeps request-family dispatchers and capability handlers readable.

## 12. Test Placement and Scope

Placement:

- split tests: `mod tests;` at top with other `mod` declarations
- inline tests: keep at bottom of module

Scope:

- unit tests for pure logic and mapping
- PocketIC integration tests for canister install/upgrade/inter-canister/lifecycle behavior
- do not add test-only production branches to fake IC behavior

## 13. Redundant Code Removal

During hygiene passes, remove:

- duplicate helpers
- dead code
- stale compatibility branches no longer needed
- outdated comments that describe old behavior

## 14. Formatting and CI Gates

Before merge, code must pass:

- `cargo fmt --all`
- `make fmt-check`
- `make clippy` (`cargo clippy --workspace --all-targets --all-features -- -D warnings`)
- `make test`

## Why This Is Valuable

Following this standard improves:

- architectural clarity
- review quality
- refactor safety
- contributor onboarding speed

## Commit Strategy

For large hygiene sweeps, prefer focused commits by module or concern rather than one massive commit.
