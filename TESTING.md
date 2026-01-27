# Testing Layout Rules

This file documents the canonical test layout for `canic-core` and the
repository-wide test configuration policy.
Follow these rules during refactors to prevent test sprawl.

## Rules

- Unit tests live next to the code under `crates/canic-core/src/...` with `#[cfg(test)]`.
- Seam/workflow tests that need `crate::` internals live under `crates/canic-core/src/test/`.
- PocketIC/system tests live under `crates/canic-core/tests/*.rs` (top-level only).
- Avoid `#[path = "..."]` in tests; use top-level files in `tests/`.
- Test canister crates are not tests; keep them outside `tests/` (e.g. `crates/canic-core/test-canisters/`).
- Tests that need private internals must not be promoted to public API; use `cfg(test)` or feature-gated test exports.
This document is the single source of truth for test configuration policy.

---

## Test Configuration Policy

Tests in `canic-core` MUST follow exactly one configuration strategy.
Mixing configuration mechanisms is forbidden.

### Configuration Categories

#### Category A — Internal runtime-configured tests

**Definition**
- Tests that require precise control over topology, roles, or auth.
- Tests that need access to private `canic-core` internals.

**Rules**
- MUST live under `crates/canic-core/src/test/` or as `#[cfg(test)]` unit tests.
- MUST initialize configuration at runtime using `ConfigTestBuilder::install()`.
- MUST NOT rely on embedded config or `canic.toml`.
- MUST NOT use `CANIC_CONFIG_PATH`.

**Notes**
- This is the preferred category for logic, workflow, and auth tests.

---

#### Category B — Host-driven runtime override tests (rare)

**Definition**
- Tests driven by a host harness (e.g. PocketIC) that modify runtime state
  via explicit internal APIs or test hooks after canister installation.

**Rules**
- MUST NOT use `CANIC_CONFIG_PATH`.
- MUST NOT rely on embedded `canic.toml`.
- Runtime mutation MUST occur via explicit APIs, not environment variables.
- If a test cannot satisfy these constraints, it is NOT Category B.

**Notes**
- Category B may currently be empty.
- It exists as a controlled escape hatch for future testkit scenarios.

---

#### Category C — Artifact / embedded-config tests

**Definition**
- Tests that rely on build-time embedded configuration (e.g. `canic.toml`).
- Tests that load `.dfx`-built or production-layout WASM artifacts.

**Rules**
- MUST live under `crates/canic-core/tests/` or other explicit integration locations.
- MUST NOT use `ConfigTestBuilder`.
- MUST document reliance on embedded config at the top of the test file.

**Notes**
- These tests validate deployment realism, not internal logic.
- Some system-level tests may validate core invariants without embedding config; these still fall under Category C.

---

## Test Canister Artifacts

- Test canister crates live under `crates/*/test-canisters/`.
- Their `build.rs` MUST embed static config via `canic::build!` or `canic::build_root!`.
- Test canisters MUST NOT use `ConfigTestBuilder` or private `canic-core` config internals.
- No test canister build script may rely on environment-based config overrides.

---

### Forbidden Patterns (All Categories)

- Using `CANIC_CONFIG_PATH` in tests
- Mixing runtime config with embedded config
- Modifying `build.rs` to support test-only config
- Exposing private config internals to support tests

---

### Required Annotation

Every non-unit test file MUST include a top-of-file comment declaring its category.
Preferred format:

```rust
// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).
```

Legacy format is also acceptable:

```rust
// TEST CONFIG CATEGORY: A (internal runtime-configured)
```
