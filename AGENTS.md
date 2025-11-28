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
- **Lint**: `make clippy` (`cargo clippy --workspace -- -D warnings`).
- **Test**: `make test` (`cargo test --workspace`).
- **Build**: `make build` for release builds.
- **Check**: `make check` for type-check only.

✅ PRs must pass `make fmt-check`, `make clippy`, and `make test`.

### Sandboxed builds
- The CLI sandbox can split the workspace across mounts, causing `Invalid cross-device link` during Cargo’s atomic renames.
- When sandboxed, build with a workspace-local target/temp dir to keep all writes on one filesystem:
  - `CARGO_TARGET_DIR=$PWD/target_tmp TMPDIR=$PWD/target_tmp cargo build -p canic --examples`
  - Same pattern for `cargo test`/`make` targets if you hit the error.
- Unsandboxed builds can stick to the default `target/`.
- `target_tmp` is the recommended shared path;

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

We separate responsibilities into **three main layers**:

### `memory/`
- Stable storage across canister upgrades.
- Wraps IC stable memory (`BTreeMap`).
- Example: `memory/ext/sharding/registry.rs` (persistent shard registry).

### `state/`
- Volatile in-memory state (cleared on upgrade).
- WASM registry caching and consent message registries.
- Example: `state/wasm.rs` (tracks registered WASM modules).

### `ops/`
- Business logic layer above `memory/` and `state/`.
- Responsible for:
  - Applying pool/shard policies.
  - Creating new canisters via management API.
  - Logging, cleanup cadence, authorization.
- Example: `ops/ext/sharding.rs` orchestrates shard lifecycle.

### `endpoints/`
- Public IC endpoints defined via macros (`canic_endpoints_*`).
- Default rule: route mutations through `ops/` so policies stay centralized.
- Temporary exception (target revisit in ~2 weeks): read-only queries may pull directly from `memory/` or `state/` when an ops façade does not yet exist.
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
  - `memory/` → stable storage
  - `state/` → volatile runtime state
  - `ops/` → orchestration, policy, logging
  - `endpoints/` → IC boundary
- Predictable lifecycles
  - Shards: register → assign → rebalance → drain → decommission
  - Delegation: register → track → revoke → cleanup
- Minimal public APIs
  - `memory/` and `state/` expose only essentials
  - `ops/` is the sole entrypoint for canister endpoints

---

## ✅ Agent Checklist

Before merging:
- Run `make fmt-check`
- Run `make clippy`
- Run `make test`
- Update `CHANGELOG.md` if user‑facing
- Group admin endpoints under a single `*_admin` update call
- Respect layering: endpoints → ops → state/memory
