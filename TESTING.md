# Testing Layout Rules

This file documents the canonical test layout for `canic-core` and the
repository-wide test configuration policy.
Follow these rules during refactors to prevent test sprawl.

## Rules

- Unit tests live next to the code under `crates/canic-core/src/...` with `#[cfg(test)]`.
- Seam/workflow tests that need `crate::` internals live under `crates/canic-core/src/test/`.
- PocketIC/system tests live under `crates/canic-core/tests/*.rs` (top-level only).
- Avoid `#[path = "..."]` in tests; use top-level files in `tests/`.
- Test canister crates are not tests; keep repo-level fixtures under
  `canisters/test/` and audit probes under `canisters/audit/`.
- Tests that need private internals must not be promoted to public API; use `cfg(test)` or feature-gated test exports.

This document is the single source of truth for test configuration policy.

---

## Test Configuration Policy

Tests in `canic-core` MUST follow exactly one configuration strategy.
Mixing configuration mechanisms is forbidden.
Workspace test runs should use single-threaded rust test execution (`-- --test-threads=1`)
to avoid PocketIC startup races under parallel harness execution.

### PocketIC Stability Guard (Required)

- Never run workspace PocketIC tests with parallel rust test threads.
- Use `make test` (or explicitly pass `-- --test-threads=1`).
- Keep a writable temp directory with enough free space. PocketIC allocates runtime
  state under `TMPDIR`; this repo's `make test` sets `TMPDIR=.tmp/test-runtime`.
- If you run tests manually and `/tmp` is near full, set `TMPDIR` yourself to avoid
  startup crashes and state-init panics.
- Known failure signatures when this rule is violated include:
  - `KeyAlreadyExists { key: "nns_subnet_id", version: 2 }`
  - `ERROR: Failed to initialize PocketIC ... connection closed before message completed`
  - `HTTP failure ... hyper::Error(IncompleteMessage)`

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
- Tests that load `.icp`-built or production-layout WASM artifacts.

**Rules**
- MUST live under `crates/canic-core/tests/` or other explicit integration locations.
- MUST NOT use `ConfigTestBuilder`.
- MUST document reliance on embedded config at the top of the test file.

**Notes**
- These tests validate deployment realism, not internal logic.
- Some system-level tests may validate core invariants without embedding config; these still fall under Category C.

---

## Non-Fleet Test And Audit Canisters

- Correctness and integration fixture canisters live under `canisters/test/`.
- Audit and measurement probe canisters live under `canisters/audit/`.
- Manual sandbox canisters live under `canisters/sandbox/`.
- These canisters are not Canic fleets and must not use fleet install logic.
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
