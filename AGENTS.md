Below is a **clean, consolidated rewrite of `AGENTS.md`** that incorporates the architectural decisions you have made during this refactor cycle: DTO separation, model/view boundaries, lifecycle semantics, timer usage, policy enforcement, and module placement.
This version is opinionated, explicit, and intended to *prevent regression*.

---

# AGENTS.md

This document defines **how agents (contributors, CI, maintainers)** must interact with the codebase.
It supplements `README.md` with **authoritative rules** on workflow, layering, lifecycle behavior, and data ownership.

This file is normative: **code should conform to it**, not the other way around.

---

## 📑 Table of Contents

1. [Workflow](#-workflow)
2. [Project Structure](#-project-structure)
3. [Layering & Data Ownership](#-layering--data-ownership)
4. [Lifecycle Semantics](#-lifecycle-semantics)
5. [Coding Style](#-coding-style)
6. [Testing](#-testing)
7. [Security & Auth](#-security--auth)
8. [Design Principles](#-design-principles)
9. [Agent Checklist](#-agent-checklist)

---

## 🚀 Workflow

### Core Commands

* **Format**: `cargo fmt --all`
* **Lint**: `make clippy`
  (`cargo clippy --workspace --all-targets --all-features -- -D warnings`)
* **Test**: `make test` (`cargo test --workspace`)
* **Build**: `make build`
* **Check only**: `make check`

✅ All PRs **must** pass:

* `make fmt-check`
* `make clippy`
* `make test`

---

### Build-time Network Requirement

* **`DFX_NETWORK` must always be set** (`local` or `ic`).
* For scripts / make:

  * `NETWORK=local|mainnet|staging`
  * maps to `DFX_NETWORK=local|ic`

Build scripts enforce this; missing configuration is fatal.

---

### Versioning & Release

* Versioning handled by scripts in `scripts/ci/`
* Use:

  * `make patch | minor | major`
  * then `make release`
* Tags are immutable. Never rewrite history.

---

### API Stability

* No stability guarantees yet.
* Breaking changes are acceptable while architecture is settling.

---

## 📦 Project Structure

```
assets/                     # Documentation assets
crates/
├─ canic/                   # Core library (macros, lifecycle, ops, model, dto)
└─ canisters/               # Reference canisters
   ├─ root/                 # Root orchestrator
   ├─ app/                  # Example application
   ├─ auth/                 # Auth helper canister
   ├─ shard/                # Shard implementation
   ├─ shard_hub/            # Shard coordinator
   ├─ scale/                # Scaling worker
   ├─ scale_hub/            # Scaling coordinator
   └─ blank/                # Minimal test canister
scripts/                    # CI / release helpers
.github/workflows/          # CI pipelines
dfx.json                    # Local IC topology
Makefile                    # Dev shortcuts
AGENTS.md, CONFIG.md        # Contributor docs
```

---

## 🧩 Layering & Data Ownership

### High-level flow

```
endpoints  →  workflow  →  ops  →  model
                       ↘
                        dto (transfer only)
```

---

### `dto/` — **Data Transfer Objects (pure)**

DTOs represent **boundary payloads only**.

DTOs:

* contain **only data**
* contain **no logic**
* do **not** call ops, workflow, or model
* may reference **other DTOs or public view types**
* are safe to serialize, clone, and expose

Examples:

* init payloads (`CanisterInitPayload`)
* RPC requests / responses
* sync bundles (`StateBundle`, `TopologyBundle`)
* dry-run plans
* endpoint response views

DTOs **must not**:

* define storage bounds
* contain invariants
* assemble themselves from ops

---

### `model/` — **Authoritative domain state**

#### `model/memory/`

* Stable memory (persists across upgrades)
* Authoritative system-of-record state
* Defines storage bounds and invariants

Examples:

* registries
* pools
* cycle tracker
* logs

#### `model/*` (non-memory)

* Volatile runtime state (cleared on upgrade)
* Metrics, caches, registries

Model types:

* may define invariants
* may define storage traits
* are **not** public by default

---

### Model *Views* (important distinction)

Some model-derived types are **read-only snapshots** intended to cross boundaries.

Rules:

* View types are **data-only**
* No mutation
* No storage traits
* Safe to expose
* Used by ops, workflow, DTOs, endpoints

Examples:

* `DirectoryView`
* `RegistryView`
* `CycleTrackerView`
* metric snapshots

DTOs **may depend on view types**, but never on authoritative model types.

If a type is used in DTOs, it must be explicitly treated as a **view**.

---

### `ops/` — **Execution & policy**

Ops:

* enforce policy
* orchestrate storage mutations
* call IC management APIs
* perform logging, cleanup, scheduling

Submodules:

* `ops::storage` — thin façades over model/memory
* `ops::runtime` — execution control (timers, guards, schedulers)
* `ops::ic` — direct IC system interactions

Rules:

* Endpoints must route **mutations** through ops
* Ops may return model views or DTOs
* Ops must not expose raw model internals

---

### `workflow/` — **Coordination**

Workflow:

* sequences multi-step operations
* coordinates ops calls
* assembles DTOs
* owns “how things happen”

Workflow must not:

* define storage schemas
* expose IC endpoints directly

---

### `endpoints/`

* IC boundary only
* Authentication & routing
* Thin delegation into workflow
* No business logic

Admin endpoints:

* grouped by domain (`*_admin`)
* single update call per domain

---

## 🔄 Lifecycle Semantics

### Lifecycle macros

* `canic::start!`
* `canic::start_root!`

Responsibilities:

* define IC hooks (`init`, `post_upgrade`)
* load embedded config
* restore minimal environment
* **schedule async work, never await it**

Macros must:

* remain thin
* contain no async logic
* schedule async hooks using timers (`Duration::ZERO`)

---

### Lifecycle adapters (`core::lifecycle::*`)

Adapters are **synchronous glue**.

Rules:

* restore environment context
* **spawn** async workflow bootstrap
* never `await` workflow
* identical semantics for init and post-upgrade

There must be **no difference** in execution model between init and upgrade.

---

### User lifecycle hooks

User-defined hooks:

* `canic_install`
* `canic_setup`
* `canic_upgrade`

Rules:

* wired by lifecycle macros
* scheduled via `TimerOps::set(Duration::ZERO, …)`
* always run **after** CANIC invariants are restored
* async, non-blocking, idempotent

---

## 🛠️ Coding Style

* **Edition**: Rust 2024
* Naming:

  * `snake_case` modules/functions
  * `PascalCase` types
  * `SCREAMING_SNAKE_CASE` constants

### Formatting

* Always run `cargo fmt --all`
* Prefer captured identifiers in format strings
* Avoid mixing formatting styles

### Doc comments

* Use padded doc comments on types
* Keep comments directly adjacent
* Use section banners for structure

---

## 🧪 Testing

* Unit tests live next to code
* Integration tests in `tests/`
* Dummy principals helper:

```rust
fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}
```

* Test names: `snake_case`
* All tests must pass under `make test`

---

## 🔐 Security & Auth

* Auth is enforced at **endpoints**
* Workflow and ops assume authenticated input
* Never embed auth checks in model or DTOs
* Subnet / parent checks must be explicit and documented

---

## 🧭 Design Principles

* Clear ownership:

  * model = truth
  * ops = execution
  * workflow = coordination
  * dto = transfer
* No hidden side effects
* No cross-layer shortcuts
* Policy must be enforced on execution paths
* Dry-run endpoints must use policy, not ops directly

---

## ✅ Agent Checklist

Before merging:

* [ ] `make fmt-check`
* [ ] `make clippy`
* [ ] `make test`
* [ ] Update `CHANGELOG.md` if user-facing
* [ ] Admin endpoints grouped under `*_admin`
* [ ] Endpoints → workflow → ops → model
* [ ] DTOs are data-only
* [ ] No model internals leaked publicly
* [ ] Lifecycle adapters spawn, never await

---

If code conflicts with this document, **the code is wrong**.
