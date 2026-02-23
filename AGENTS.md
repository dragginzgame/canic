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

* `DFX_NETWORK` selects the target network (`local` or `ic`).
* If unset, it defaults to `local`.
* For scripts / make:

  * `NETWORK=local|mainnet|staging`
  * maps to `DFX_NETWORK=local|ic`

---

### Versioning & Release

* Versioning handled by scripts in `scripts/ci/`
* Use:

  * `make patch | minor | major`
  * then `make release`
* Tags are immutable. Never rewrite history.

---

### Changelog Rules

* Keep the existing changelog structure and header format.
* Use `## [x.y.z] - YYYY-MM-DD - Short Title` when a title is needed.
* Smaller entries may omit the title segment and use `## [x.y.z] - YYYY-MM-DD`.
* Changelog subsections are optional; include only sections relevant to the release.
* Use this fixed emoji mapping for section headers:
  * `Added=➕`
  * `Changed=🔧`
  * `Fixed=🩹`
  * `Removed=🗑️`
  * `Breaking=⚠️`
  * `Migration Notes=🧭`
  * `Summary=📝`
  * `Cleanup=🧹`
  * `Testing=🧪`
* Before editing `CHANGELOG.md`, read the root `Cargo.toml` `[workspace.package].version` and treat that version as current/live (frozen for changelog edits).
* Never edit the changelog entry matching the current `Cargo.toml` version unless the user explicitly asks for an exception.
* By default, write changes under the next release version (for example, current `0.9.28` -> changelog target `0.9.29`) unless the user explicitly requests a different version.
* If the target version header already exists, append to it; do not create a duplicate header.
* Write in plain, industry-friendly language and lead with user impact.
* Keep wording concise and junior-friendly; avoid jargon.
* Keep bullets short (1–2 sentences), and use inline code for API/type names.
* Prefer explaining why a change matters, not only what changed.
* Include code examples only when they clarify behavior, migration, or usage.
* Include at least one fenced block per changelog page when practical (for example usage, migration snippet, or binary spec).

```md
## [0.0.0] - 2026-02-17 - Example Title

### 🔧 Changed

- Updated `MyApi::call()` error handling so policy failures keep structured messages.
```

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
   ├─ user_hub/             # User signup + provisioning coordinator
   ├─ user_shard/           # Delegated signing shard
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
Instrumentation modules (`perf`/logging/tracing) are layer-neutral and may be
used anywhere.

### Storage Modules (constraint)

Storage-owned types may use `serde` and other proc-macro derives when needed.
Keep dependencies minimal and deterministic; avoid heavyweight runtime
dependencies. Apply this rule narrowly to storage-owned types only; do not
modify non-storage types for consistency.

### Non-Negotiable Invariants

Policy must not depend on storage records. Policy depends only on policy
inputs and value types (IDs/enums), not on storage schema.

Workflow must not construct or mutate *Record types. Workflow orchestrates;
ops owns conversion into persisted schemas.

All DTO↔Record, Record↔View, Policy↔Record conversions live in ops::*.
Workflow adapters may exist only for orchestration glue, not shape conversion.

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

### Platform Ops Exception (Normative)

Ops **may** perform **single-step platform side effects** when acting as the
**approved system façade**, including:

* IC calls (including management/system canisters)
* cycle attachment
* timer scheduling
* runtime metrics and instrumentation

Constraints:

* ops must remain single-step and must not perform multi-step orchestration
* ops must not encode **business meaning**
* ops must not loop in a way that encodes retries, orchestration, or time-dependent behavior.
* ops must not decide *whether* an action should occur

Ops must not:

* define business policy
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
* policy decides, but never acts and never serializes.

Policy must not directly observe runtime state or call storage/registry ops.

Allowed:

* reading config
* evaluating observed state passed in as parameters
* deterministic computation

Forbidden:

* state mutation
* IC calls
* async
* timers
* side effects
* serialization (`CandidType`, `Serialize`, `Deserialize`)
* DTO dependencies

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

### Infra Error Semantics

All fallible infra APIs must return `Result<_, InfraError>`; sub-error types
are internal and must not appear in public signatures.

Infra errors are purely structural:

* wrap source errors losslessly (no stringification or collapsing)
* represent mechanical failures only (IC calls, candid encode/decode, mgmt invariants, env access)
* maintain a single, consistent path for IC call failures via `InfraError`

Conversions into `InfraError` must be direct wrappers only.

---

## 📤 Canonical Data, Views, and DTOs

Type aliases do not constitute boundaries.
Any type that crosses a layer boundary must be a named struct or enum.

### Type Layer Rules (Non-negotiable)

Layer | Directory | Meaning
----- | --------- | -------
DTO | `dto/` | External API contract (inputs + responses)
View | `view/` | Internal read-only projections for policy/workflow
Record | `storage/**` | Persisted schema / stable memory
Ops | `ops/**` | Boundary layer that touches storage/infra
Workflow | `workflow/**` | Orchestration only, no schema ownership
IDs | `ids/` | Identity/value types, stable across layers

Hard rule:
A type’s directory defines its role. Names must not contradict the directory.

### Naming Rules (Locked)

Types:
* DTO inputs: `*Input`, `*Args`, `*Request`
* DTO outputs: `*Response`
* DTOs must not use `View` in type or function names
* Views: no suffix; must live under `view/`; read-only, internal, non-persisted
* Records: persisted aggregates end in `*Record`
* IDs and value types never end in `Record`

Functions:
* `dto_to_record` — DTO → storage
* `record_to_view` — storage → view/ projection
* `record_to_response` — storage → DTO response
* `to_input` / `from_input` — workflow ↔ DTO input
* `*_to_dto` — ops → DTO
* `*_view` — forbidden unless using `view::*`

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

### Views — Read-only projections

* Data-only snapshots
* No invariants, no mutation
* Internal-only; may be wrapped in DTOs when explicitly needed
* Used by ops, workflow, policy

Examples:

* `Directory`
* `Registry`
* `CycleTracker`

DTOs may depend on views, **never on authoritative model types**.

---

### DTOs — Boundary contracts

* API / RPC / ABI shapes
* Versionable and replaceable
* Passive data only

### DTO Defaults

DTOs must not implement `Default` unless the default value represents a valid,
neutral, and intentional payload (e.g. empty bundles, metric snapshots).

Command, request, and mutation DTOs must never implement `Default`.

---

### Export / Import Naming

The names `export()` and `import()` are reserved for **canonical snapshot operations**.

Rules:

* `export()` / `import()` operate on `*Data` types
* Views and DTOs must not use `export()` naming
* Use the function naming rules above for conversions and projections

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
* must not call policy or ops directly.

---

### Lifecycle adapters (`core::lifecycle::*`)

Adapters are **synchronous glue**.

Rules:

* restore environment context
* spawn async workflow bootstrap
* never `await`
* identical semantics for init and post-upgrade

Lifecycle adapters must not import DTOs into model state directly.
All DTO → Data projection must occur via ops adapters.

Init and post-upgrade must follow the same execution structure:

* synchronous environment restoration
* scheduling (never awaiting) async bootstrap work

Init and post-upgrade share the same execution model (synchronous adapter, async bootstrap),
but may differ in validation because init consumes external payloads while post-upgrade restores
trusted stable state.
Differences in state initialization (e.g. payload import on init only) are permitted and must be explicit.
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

### Imports

* Keep `use` statements at the top of the file.
* Do not add `use crate::...` imports in the middle of a file.
* Group imports logically and keep them consolidated instead of scattering them across sections.

### Doc comments

* Struct doc comments must be exactly:
  * `///`
  * `/// StructName`
  * `///`
  * (blank line)
  * then the `struct` definition
* Prefer a blank line after any multi-line `///` doc comment block before the item it documents (in addition to the struct-specific rule above).
* Keep comments directly adjacent
* Use section banners for structure
* Large groups of related functions must be separated with a header divider like:
  * `// --- Removal --------------------------------------------------------`

---

## 🧪 Testing

* Unit tests live next to code
* Integration tests in `tests/`
* Any test that creates/installs/upgrades canisters or exercises inter-canister calls **must** use PocketIC (integration tests), not unit tests.
* Do not add `cfg(test)` stubs, test-only branches, or debug-only logic in production code to fake IC management behavior; use PocketIC instead.
* If a unit test would panic due to `ic0` usage, it is misclassified and must be moved to PocketIC.
* Dummy principals helper:

```rust
fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}
```

* Test names: `snake_case`
* All tests must pass under `make test`
* Never match errors by string; assert on typed errors or observable state instead.

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
* [ ] Update `CHANGELOG.md` if user-facing (follow Changelog Rules)
* [ ] Admin endpoints grouped under `*_admin`
* [ ] Endpoints → workflow → policy → ops → model
* [ ] DTOs are data-only
* [ ] No model internals leaked publicly
* [ ] Lifecycle adapters spawn, never await
* [ ] Ops side effects are single-step

---

**This document is authoritative.**
If implementation and documentation disagree, update the implementation.
