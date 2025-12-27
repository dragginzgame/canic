Below is the **complete, consolidated, up-to-date `AGENTS.md`**, incorporating all architectural decisions made during this refactor cycle.
This version resolves prior inconsistencies, formalizes naming and data taxonomy, and is safe to treat as **normative**.

You can drop this in verbatim.

---

# AGENTS.md

This document defines **how agents (contributors, CI, maintainers)** must interact with the codebase.
It supplements `README.md` with **authoritative rules** on workflow, layering, lifecycle behavior, and data ownership.

This file is normative: **code must conform to it**.
If code conflicts with this document, **the code is wrong**.

---

## 📑 Table of Contents

1. [Workflow](#-workflow)
2. [Project Structure](#-project-structure)
3. [Layering & Data Ownership](#-layering--data-ownership)
4. [Canonical Data, Views, and DTOs](#-canonical-data-views-and-dtos)
5. [Lifecycle Semantics](#-lifecycle-semantics)
6. [Coding Style](#-coding-style)
7. [Testing](#-testing)
8. [Security & Auth](#-security--auth)
9. [Design Principles](#-design-principles)
10. [Agent Checklist](#-agent-checklist)

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

### Dependency Direction (strict)

Dependencies must point **downward only**:

```
endpoints
   ↓
workflow
   ↓
policy
   ↓
ops
   ↓
model
```

`dto` is used as a **transfer format** by endpoints, workflow, and ops.
`model` and `policy` must not depend on `dto`.

---

## `dto/` — **Data Transfer Objects (pure)**

DTOs represent **boundary payloads only**.

DTOs:

* contain **only data**
* contain **no logic**
* do **not** call ops, workflow, or model
* may reference **other DTOs or view types**
* are safe to serialize, clone, and expose

Examples:

* init payloads
* RPC requests / responses
* sync bundles
* dry-run plans
* endpoint response shapes
* ABI types (`dto::abi::vX::*`)

DTOs must not:

* define storage bounds
* contain invariants
* assemble themselves from ops

---

## `model/` — **Authoritative domain state**

### `model/memory/`

* Stable memory (persists across upgrades)
* System-of-record state
* Storage layout and bounds
* Local, structural invariants

Examples:

* registries
* pools
* cycle tracker
* logs

### `model/*` (non-memory)

* Volatile runtime state
* Caches, metrics, ephemeral registries

Model types:

* may define **local invariants**
* may define storage traits
* are **not public by default**

Rule:

> `model` defines what *is true* about stored state.
> It does not decide what *should happen next*.

---

## `ops/` — **Application services**

Ops provide **deterministic access** to model state.

Ops:

* expose command and query façades
* read and mutate model state
* enforce application preconditions (existence, ownership, mode)
* adapt domain state into views and DTOs

Includes:

* `ops::storage` — thin façades over `model/memory`
* `ops::adapter` — domain → view / DTO mapping

Ops must not:

* define business policy
* perform IC management calls
* use async
* schedule timers
* coordinate multi-step behavior

Rule:

> `ops` applies state changes safely and deterministically,
> but does not decide whether those changes are desirable.

---

## `policy/` — **Decision making (pure)**

Policy owns **system rules and decisions**.

Policy:

* evaluates eligibility, placement, scaling, sharding
* enforces system-wide invariants
* answers “can we?” / “should we?”

Allowed:

* reading config
* reading state via ops
* deterministic computation

Forbidden:

* state mutation
* IC calls
* async
* timers
* side effects

Rule:

> `policy` decides, but never acts.

---

## `workflow/` — **Coordination & orchestration**

Workflow owns **multi-step behavior over time**.

Workflow:

* sequences ops calls
* performs IC management calls
* schedules timers and retries
* executes cascades and rollbacks
* validates decisions via policy

Workflow must not:

* access model directly
* embed policy logic inline

Rule:

> `workflow` is where things happen.

---

## `endpoints/` / macros — **System boundary**

Endpoints and macros:

* define IC entrypoints
* perform auth and guards
* marshal DTOs
* delegate immediately to workflow or ops

Forbidden:

* business logic
* policy decisions
* orchestration
* direct model access

Rule:

> Endpoints and macros wire requests to the system;
> they do not contain business behavior.

---

## 📤 Canonical Data, Views, and DTOs

The codebase distinguishes **three outward-facing representations**.

### `*Data` — Canonical snapshots

* Represent canonical internal state
* Used for import/export, workflows, cascades
* Detached from storage implementation
* Owned by model or ops

Examples:

* `CanisterChildrenData`
* `TopologyData`
* `AppStateData`

---

### `*View` — Read-only projections

* Data-only snapshots
* No invariants, no mutation
* Safe to expose internally or externally
* Used by ops, workflow, DTOs, endpoints

Examples:

* `DirectoryView`
* `RegistryView`
* `CycleTrackerView`

DTOs may depend on views, **never on authoritative model types**.

---

### DTOs — Boundary contracts

* API / RPC / ABI shapes
* Versionable and replaceable
* Passive data only

---

### Export / Import Naming

The names `export()` and `import()` are reserved for **canonical snapshot operations**.

Rules:

* `export()` / `import()` operate on `*Data` types
* Views and DTOs must not use `export()` naming
* Projection functions should be named:

  * `get_*_view`
  * `snapshot_*`
  * `to_*_view`

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
* spawn async workflow bootstrap
* never `await`
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
* Never embed auth checks in model, policy, or DTOs
* Subnet / parent checks must be explicit and documented

---

## 🧭 Design Principles

* Clear ownership:

  * model = truth
  * ops = execution
  * policy = decision
  * workflow = coordination
  * dto = transfer
* No hidden side effects
* No cross-layer shortcuts
* Prefer splitting responsibilities over collapsing them
* Dry-run endpoints must use policy, not ops directly

---

## ✅ Agent Checklist

Before merging:

* [ ] `make fmt-check`
* [ ] `make clippy`
* [ ] `make test`
* [ ] Update `CHANGELOG.md` if user-facing
* [ ] Admin endpoints grouped under `*_admin`
* [ ] Endpoints → workflow → policy → ops → model
* [ ] DTOs are data-only
* [ ] No model internals leaked publicly
* [ ] Lifecycle adapters spawn, never await

---

**This document is authoritative.**
If implementation and documentation disagree, update the implementation.
