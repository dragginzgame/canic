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

### Versioning & Release
- Scripts in `scripts/app/` handle bumps and tags.
- Use `make patch|minor|major` → `make release`.
- Tags are immutable. Never alter historical tags.

---

## 📦 Project Structure

```
crates/
├─ icu/                 # Core library (shared)
└─ canisters/           # Internet Computer canisters
   ├─ root/
   ├─ example/
   ├─ game/
   ├─ instance/
   └─ player_hub/
scripts/                # Build, versioning, env helpers
.github/workflows/      # CI/CD pipelines
target/                 # Build output (ignored)
```


---

## 🧩 Module Layering

We separate responsibilities into **three main layers**:

### `memory/`
- Stable storage across canister upgrades.
- Wraps IC stable memory (`BTreeMap`).
- Example: `memory/shard.rs` (persistent shard registry).

### `state/`
- Volatile in-memory state (cleared on upgrade).
- Caches, delegation sessions, authentication.
- Example: `state/delegation.rs` (ephemeral delegation sessions).

### `ops/`
- Business logic layer above `memory/` and `state/`.
- Responsible for:
  - Applying pool/shard policies.
  - Creating new canisters via management API.
  - Logging, cleanup cadence, authorization.
- Example: `ops/shard.rs` orchestrates shard lifecycle.

### `endpoints/`
- Public IC endpoints defined via macros (`icu_endpoints_*`).
- Must call **`ops/` only**, never touch `memory/` or `state/` directly.
- Admin operations are grouped into a single update call per domain (e.g., `shard_admin`, `delegation_admin`).

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
      log!(Log::Info, "cleaned up sessions, before: {before}, after: {after}");
      // Avoid mixing styles in the same call
      ```
    - For non-identifier expressions, bind to a local first or use positional formatting.
      ```rust
      let count = items.len();
      log!(Log::Info, "moved {count} items");
      // or
      log!(Log::Info, "moved {} items", items.len());
      ```
  - Comment/layout baseline: use banner separators for major sections and keep a blank
    line between doc comments and the item they describe, e.g.
    ```rust
    // -----------------------------------------------------------------------------
    // Section Title
    // -----------------------------------------------------------------------------

    /// Explains what Foo does.
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
