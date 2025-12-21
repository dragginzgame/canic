# AGENTS.md

This guide describes how **agents** (contributors, CI, maintainers) should interact with the codebase.
It expands on `README.md` with **workflow rules**, **layering conventions**, and **coding guidelines**.

---

## 📑 Table of Contents
1. [Workflow](#-workflow)
2. [Project Structure](#-project-structure)
3. [Module Layering](#-module-layering)
4. [Coding Style](#-coding-style)
5. [Testing](#-testing)
6. [Security & Auth](#-security--auth)
7. [Design Principles](#-design-principles)
8. [Checklist](#-agent-checklist)

---

## 🚀 Workflow

### Core Commands
- **Format**: `cargo fmt --all` (must run before commit/PR).
- **Lint**: `make clippy` (`cargo clippy --workspace --all-targets --all-features -- -D warnings`).
- **Test**: `make test` (`cargo test --workspace`).
- **Build**: `make build` for release builds.
- **Check**: `make check` for type-check only.

### Build-time Network Requirement
- **Always set `DFX_NETWORK`** to `local` or `ic` for any build/test (enforced by build script).
- For `make`/scripts, `NETWORK=local|mainnet|staging` will map to `DFX_NETWORK=local|ic`.

✅ PRs must pass `make fmt-check`, `make clippy`, and `make test`.

### Versioning & Release
- Scripts in `scripts/ci/` handle bumps and tags.
- Use `make patch|minor|major` → `make release`.
- Tags are immutable. Never alter historical tags.

---

## 📦 Project Structure

```
assets/                 # Shared documentation media (README logo, etc.)
crates/
├─ canic/              # Core library crate (macros, memory/state, ops, auth)
└─ canisters/          # Reference Internet Computer canisters
   ├─ root/            # Orchestrator wiring the full stack
   ├─ app/             # Sample application canister driving end-to-end flows
   ├─ auth/            # Authorization helper canister
   ├─ shard/           # Shard canister implementation
   ├─ shard_hub/       # Shard pool coordinator
   ├─ scale/           # Scaling worker example
   ├─ scale_hub/       # Scaling coordinator example
   └─ blank/           # Minimal test canister
scripts/                # Build, versioning, and environment helpers
.github/workflows/      # CI/CD pipelines
dfx.json                # Local canister topology for dfx
Makefile                # Convenience targets (`make fmt`, `make test`, ...)
target/                 # Build output (ignored)
AGENTS.md, CONFIG.md    # Contributor documentation
```


---

## 🧩 Module Layering

We separate responsibilities into **four main layers**:

### `model/memory/`
- Stable storage across canister upgrades (IC stable memory).
- Includes stable “canister state” such as `AppState` / `SubnetState` (these are persistent).
- Example: `crates/canic-core/src/model/memory/sharding/registry.rs`.

### `model/*` (non-memory)
- Volatile in-process registries/caches (cleared on upgrade).
- Examples: `crates/canic-core/src/model/wasm/wasm_registry.rs`, `crates/canic-core/src/model/metrics/*`.

### `ops/`
- Business logic layer above stable storage (`model/memory`) and runtime registries/caches (`model/*`).
- Responsible for:
  - Applying pool/shard policies.
  - Creating new canisters via management API.
  - Logging, cleanup cadence, authorization.
- Example: `crates/canic-core/src/ops/orchestrator.rs`.

### `endpoints/`
- Public IC endpoints defined via macros (`canic_endpoints_*`).
- Default rule: route mutations through `ops/` so policies stay centralized.
- Temporary exception (target revisit in ~2 weeks): read-only queries may pull directly from `model/memory` or runtime registries (`model/*`) when an ops façade does not yet exist.
- Admin operations are grouped into a single update call per domain (e.g., `shard_admin`).

---

## 🛠️ Coding Style

- **Edition**: Rust 2024.
- **Naming**:
  - `snake_case` for modules/functions.
  - `PascalCase` for types/traits.
  - `SCREAMING_SNAKE_CASE` for constants.
- **Formatting**:
  - Run `cargo fmt --all` before commit.
  - Formatting macros (format!/println!/eprintln!/panic!/log!/etc.):
    - Prefer captured identifiers inside the format string over trailing single args.
      ```rust
      // Preferred
      log!(Topic::Topic, Log::Info, "cleaned up sessions, before: {before}, after: {after}");
      // Avoid mixing styles in the same call
      ```
    - For non-identifier expressions, bind to a local first or use positional formatting.
      ```rust
      let count = items.len();
      log!(Log::Info, "moved {count} items");
      // or
      log!(Log::Info, "moved {} items", items.len());
      ```
  - Comment/layout baseline: use banner separators for major sections.
  - Doc comments on types (`struct`, `enum`, etc.) must be wrapped with empty doc lines for visual padding and stay directly adjacent to the item:
    ```rust
    // -----------------------------------------------------------------------------
    // Section Title
    // -----------------------------------------------------------------------------

    ///
    /// Foo
    /// Describes the Foo type
    ///
    struct Foo;
    ```
- **Linting**: `cargo clippy --workspace -- -D warnings`.

---

## 🧪 Testing

- Unit tests live with modules (`#[cfg(test)]`).
- Integration tests in `tests/` when cross-crate.
- Dummy principals for stability:
  ```rust
  fn p(id: u8) -> Principal {
      Principal::from_slice(&[id; 29])
  }
  ```
 - Test names: snake_case (e.g., `assign_and_get_tenant`, `expired_session_cleanup`).
- Ensure `make test` passes before PR.

---

## 🧭 Design Principles

- Separation of concerns
  - `model/memory/` → stable storage (incl. stable canister state)
  - `model/*` → volatile runtime registries/caches
  - `ops/` → orchestration, policy, logging
  - `endpoints/` → IC boundary
- Predictable lifecycles
  - Shards: register → assign → rebalance → drain → decommission
  - Delegation: register → track → revoke → cleanup
- Minimal public APIs
  - stable storage and registries expose only essentials
  - `ops/` is the sole entrypoint for canister endpoints

---

## ✅ Agent Checklist

Before merging:
- Run `make fmt-check`
- Run `make clippy`
- Run `make test`
- Update `CHANGELOG.md` if user‑facing
- Group admin endpoints under a single `*_admin` update call
- Respect layering: endpoints → ops → model (stable + runtime)
