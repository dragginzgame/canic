# Codebase Hygiene Standard

## Purpose

This directory defines source consistency and readability standards for the
Canic workspace, including `canic`, `canic-core`, `canic-macros`,
`canic-control-plane`, `canic-wasm-store`, `canic-cli`, `canic-host`, and
`canic-backup`.

The goal is to keep the codebase easy to navigate while preserving Canic's
layering:

```text
endpoints -> workflow -> policy -> ops -> model
```

If this directory conflicts with `AGENTS.md`, `AGENTS.md` wins. Command,
versioning, release, and deployment policy remains owned by
[`ci-deployment.md`](../ci-deployment.md). Changelog policy remains owned by
[`changelog.md`](../changelog.md).

This standard is not the module hardening audit. Use
[`module-surface-hardening.md`](../../audits/modular/module-surface-hardening.md)
when the task is to justify retained surface, remove stale complexity, or
evaluate cleanup against runtime shape.

## Example Crate

The `example-crate/` tree is documentation-only Rust that models preferred
Canic crate and module shape. It intentionally has no `Cargo.toml`, is outside
the Cargo workspace, and must not own package metadata or version numbers.

```text
example-crate/
└── src/
    ├── lib.rs
    ├── diagnostic.rs
    ├── project/
    │   ├── admission.rs
    │   ├── mod.rs
    │   ├── snapshot.rs
    │   └── tests.rs
    └── workflow/
        ├── mod.rs
        └── route.rs
```

The example demonstrates module-level ownership headers, top-of-file ordering,
grouped imports, narrow visibility, public item docs, invariant-bearing
constructors, typed diagnostics, leaf-local inline tests, and boundary-level
`tests.rs`.

When copying from it:

1. Copy structure and ordering, not the example domain names.
2. Keep runtime authority in the owning module.
3. Use scoped visibility before widening a symbol to `pub`.
4. Put cross-module tests in the owner boundary instead of burying them in a
   leaf module.
5. Keep examples formatted with `rustfmt`.

## 1. Import Organization

Keep imports grouped, stable, and only at the top of each file.

Preferred grouping:

1. `crate` imports
2. `std` imports
3. external crate imports

Rules:

1. Avoid `super::super::...` paths.
2. Avoid `super::...` outside tests unless narrowly justified.
3. Prefer grouped `crate::{...}` imports over scattered long paths.
4. Group imports by root instead of repeating the same path throughout a file.
5. Keep normal imports, re-exports, and module declarations in their own
   blocks.
6. Keep `#[cfg(...)]` imports in the same conceptual block they would occupy
   without the `cfg`.
7. When deriving or implementing `Display`, prefer
   `use std::fmt::{self, Display};` for consistency.

Required top-of-file sequence for module files:

1. `mod ...;` declarations
2. one blank line
3. `use ...;` imports
4. one blank line
5. re-exports: `pub use ...;`, `pub(crate) use ...;`,
   `pub(in ...) use ...;`
6. one blank line
7. constants, types, functions

`#[cfg(test)] mod tests;` belongs with other `mod` declarations.

## 2. Module Header Comments

Non-trivial modules should begin with a module-level documentation header that
states responsibility and boundary.

Keep the first doc paragraph short. Clippy's
`too_long_first_doc_paragraph` lint treats consecutive `//!` lines as one
paragraph, so put a blank doc line after the one-line module name.

Example:

```rust
//! Module: workflow::project::install
//!
//! Responsibility: orchestrate project canister install steps.
//! Does not own: authorization, stable records, or pure placement policy.
//! Boundary: calls ops and policy after endpoints authenticate input.
```

Use these headers to prevent architectural drift. Keep them current when module
ownership changes.

## 3. Type Documentation

Public structs, enums, and traits should document:

1. what the type represents
2. which layer owns it
3. where it is used

Use the repository's scan-friendly section style above structs, enums, and
traits unless the type is a shipped `CandidType` where rustdoc metadata is not
intended:

```rust
///
/// ProjectInstallRequest
///
/// Boundary DTO accepted by the project install workflow.
///
```

Spacing rule for documented type declarations:

1. Leave one blank line before the doc comment block.
2. Leave one blank line after the doc comment block and before attributes or
   the item declaration.
3. Keep related type, inherent impl, and trait impls together when feasible.

Error enum formatting:

1. Keep one blank line between variant blocks.
2. Prefer alphabetical variant order by variant name when no semantic ordering
   is stronger.
3. Assert typed errors in tests; do not test error strings.

## 4. Function Documentation

Comment intent, invariants, ownership, or non-obvious tradeoffs only.

Use item documentation when a function:

1. is public API
2. enforces invariants
3. crosses a layer boundary
4. can panic through a public contract
5. performs non-obvious orchestration or policy work

Function documentation uses idiomatic Rust rustdoc prose. Do not use the
scan-friendly type documentation block for functions, and do not add a rustdoc
line that only repeats the function name.

Preferred:

```rust
/// Format a byte size using IEC units with two decimal places.
///
/// Examples: `512.00 B`, `720.79 KiB`, `13.61 MiB`.
pub fn byte_size(bytes: u64) -> String {
    // ...
}
```

Avoid:

```rust
/// byte_size
///
/// Format a byte size using IEC units with two decimal places.
///
pub fn byte_size(bytes: u64) -> String {
    // ...
}
```

Avoid comments that restate the next line or describe stale implementation
history. Private helpers do not need comments when the name and local context
are clear.

Ordering rule for documented items with attributes:

1. docs/comments
2. attributes
3. function declaration

Public APIs with reachable panic paths must include a `# Panics` section naming
the condition. Prefer typed errors when callers can recover.

## 4.1 Lint Suppressions

Prefer `#[expect(...)]` over `#[allow(...)]` for lint suppressions so stale
suppressions surface automatically. Use `#[allow(...)]` only for confirmed
false positives where the lint may legitimately stop firing.

## 5. Section Banners

Use section banners when grouping multiple related functions in a large module.
Also use a test section banner in any file that contains both non-test code and
inline tests.

Section banners are navigation comments, not item documentation. Use normal
`//` comments for them, including test section banners; do not use `///`
rustdoc blocks for headings such as `Tests`.

Example:

```rust
// -----------------------------------------------------------------------------
// Validation
// -----------------------------------------------------------------------------
```

Test section example:

```rust
// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    // ...
}
```

Do not add non-test banners to small files where the type and function order is
already obvious.

When a file contains both non-test code and inline tests, keep the test module
at the bottom behind a `// Tests` banner, even if the file is otherwise small.

## 6. Function Ordering

Prefer stable ordering:

1. public API
2. constructors/builders
3. core logic
4. helpers/utilities
5. tests, at the bottom behind a `// Tests` banner when the file also contains
   non-test code

When a type and its impls live in the same file:

1. place the inherent impl below the type when feasible
2. follow with trait impls for that type
3. keep each type family together

## 7. Function Size

Functions longer than roughly 80 lines should be reviewed for decomposition.

Split by semantic phase when possible:

1. validation
2. mapping/conversion
3. decision/policy
4. execution
5. commit/persistence
6. cleanup

Avoid deeply nested logic blocks. Large `match` bodies should dispatch to
helpers.

## 8. Visibility and Layer Boundaries

Minimize visibility by default.

Guidance:

| Layer | Visibility |
| --- | --- |
| `dto` boundary contracts | `pub` only where exported |
| `model` internals | private or `pub(crate)` |
| `ops` internals | `pub(crate)` unless cross-crate API requires `pub` |
| `policy` pure helpers | `pub(crate)` unless intentionally reusable |
| `workflow` orchestration | `pub(crate)` unless exposed through facade/API |
| endpoint macros/facade APIs | `pub` only for consumer-facing surface |

Never widen visibility to bypass layering.

## 9. Invariants and Error Semantics

Production paths should return typed errors, not stringly failures.

Rules:

1. `endpoints` authenticate and marshal input, then delegate.
2. `workflow` orchestrates multi-step behavior and may call `ops` and `policy`.
3. `policy` is pure decision logic: no mutation, async, timers, IC calls, DTOs,
   storage access, or serialization.
4. `ops` owns deterministic state access, conversions, and approved single-step
   platform side effects.
5. `model` owns authoritative state and storage invariants.

Avoid `unwrap()` and `expect()` outside tests and intentional invariant checks.
For replay, lifecycle, authorization, stable-memory, and canister-control paths,
keep failure modes explicit and typed.

Human-readable text is presentation, not a decision contract:

1. Production control flow must not classify Canic-owned errors through
   `Display`, `to_string()`, `message`, `description`, or substring matching.
2. Preserve owner-defined variants or stable machine codes until the final
   public/rendering boundary.
3. When an external tool exposes only text diagnostics, isolate matching in
   one compatibility adapter, keep the original diagnostic, and fail closed
   for unknown wording.
4. Behavioral tests assert variants, codes, or observable state. Tests may
   assert text only when rendering or external diagnostic compatibility is the
   behavior under test.

## 10. Naming Consistency

Use existing Canic vocabulary consistently.

Examples:

1. `request_id`, `ttl_seconds`, `payload_hash`
2. `parent_pid`, `root_pid`, `subnet_pid`
3. `canister_role`, `subnet_role`
4. `Replay`, `Capability`, `Delegation`, `Attestation`
5. `BlobStorage`, `RemoteAsset`, `Cashier`, `GatewayPrincipal`

Do not invent parallel names for existing concepts.

## 11. Data Shape Rules

DTOs are passive boundary data only. Command/request/mutation DTOs must not
implement `Default` unless the default is truly neutral.

Records are persisted storage schema and end in `*Record`. Views are internal
read-only projections and live under `view/`. `export()` and `import()` are
reserved for canonical `*Data` snapshots.

Cross-layer data should use named structs/enums, not boundary type aliases.

## 12. Test Placement and Scope

Placement:

1. unit tests live next to code
2. integration tests live in `tests/`
3. split tests use `mod tests;` at the top with other module declarations
4. inline tests stay at the bottom of the module

Scope:

1. unit tests cover pure logic, conversions, and model/ops invariants
2. PocketIC tests cover canister install, upgrade, inter-canister, lifecycle,
   and IC-call behavior
3. tests should assert typed errors or observable state, not strings
4. do not add production `cfg(test)` branches to fake IC management behavior

Use boundary-level `tests.rs` for behavior crossing sibling modules or shared
fixtures.

## 13. Redundant Code Removal

During hygiene passes, remove:

1. duplicate helpers
2. dead code
3. stale compatibility branches no longer needed
4. outdated comments that describe old behavior
5. duplicate protocol or billing logic once Canic owns the shared surface

Keep removals scoped to the module or concern under review.

## 14. Formatting and Checks

During active development, run:

```text
cargo fmt --all
```

Before merge, use the commands required by
[`ci-deployment.md`](../ci-deployment.md) for the touched surface. Common local
checks include:

```text
make fmt-check
make clippy
make test
```

Do not change Cargo package versions, workspace dependency versions, release
script defaults, or install URLs during ordinary hygiene work.

## Why This Is Valuable

Following this standard improves:

1. architectural clarity
2. review quality
3. refactor safety
4. contributor onboarding speed

## Commit Strategy

For large hygiene sweeps, prefer focused commits by module or concern rather
than one massive commit.

Examples:

1. `cleanup: normalize imports in canic-core workflow`
2. `cleanup: tighten visibility in canic-wasm-store`
3. `cleanup: remove stale protocol helpers from blob storage`
