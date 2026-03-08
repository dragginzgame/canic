# Changelog

All notable, and occasionally less notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [0.14.x] - 2026-03-08 - Next Minor Line

- `0.14.0` opens the next minor line; patch notes will be added as `0.14.x` changes are finalized.

See detailed breakdown:
[docs/changelog/0.14.md](docs/changelog/0.14.md)

---

## [0.13.x] - 2026-03-07 - Distributed Capability Invocation

- `0.13.0` introduced signed capability envelopes for cross-canister root calls, with built-in replay protection and capability hashing to prevent request reuse/tampering.
- `0.13.1` split large RPC/auth workflow files into smaller modules, making the control plane easier to read and change without altering behavior.
- `0.13.2` continued the module split and moved request/auth helpers behind cleaner facades, reducing coupling between high-traffic code paths.
- `0.13.3` finished the auth/control-plane extraction, standardized directory modules with `mod.rs`, and refreshed complexity/velocity audit baselines.
- `0.13.4` simplified proof, replay, and auth internals with pluggable verifiers, a dedicated replay guard path, faster duplicate rejection, and clearer delegated-auth error grouping.
- `0.13.5` further reduced branching pressure by moving replay commit fully into ops, switching built-in access predicates to evaluator-based dispatch, and replacing monolithic root capability metric events with structured `event_type`/`outcome`/`proof_mode` metrics.

See detailed breakdown:
[docs/changelog/0.13.md](docs/changelog/0.13.md)

---

## [0.12.x] - 2026-03-07 - Root Role Attestation Framework

- `0.12.0` adds root-signed role attestations and an attested root dispatch path, so services can authorize callers by signed proof instead of full directory sync.

See detailed breakdown:
[docs/changelog/0.12.md](docs/changelog/0.12.md)

---

## [0.11.x] - 2026-03-07 - Capabilities Arc and Replay Hardening

- `0.11.0` starts the capability-focused auth line with stronger scope checks and safer account/numeric behavior.
- `0.11.1` hardens root capability replay/dispatch behavior and improves auth diagnostics to make failure triage easier.

See detailed breakdown:
[docs/changelog/0.11.md](docs/changelog/0.11.md)

---

## [0.10.5] - 2026-02-25 - HTTP Raw Responses & Leaner Wasm Builds

### ⚠️ Breaking

- `HttpApi::get()` and `HttpApi::get_with_label()` now return `HttpRequestResult` (raw bytes) instead of deserializing JSON into `T`.

### 🩹 Fixed

- Fixed a deferred memory-registration race where first-touch stable-memory reads could run before late registrations were committed, causing missing reserved-range errors in downstream stores (for example IcyDB commit-marker lookups).
- Endpoint dispatch now enforces runtime memory bootstrap on wasm before handler execution, reducing bad-path risk when lifecycle wiring is custom or incomplete.

### 🔧 Changed

- HTTP ops/workflow no longer perform an extra JSON decode pass after outcalls; they validate status and return the raw body.
- `scripts/app/build.sh` now defaults to release builds so local canister wasm artifacts stay smaller by default. Use `RELEASE=0` to force debug.
- `make test-canisters` now explicitly uses `RELEASE=0 dfx build --all` so test-only endpoints guarded by `cfg!(debug_assertions)` continue to behave as expected in local smoke tests.
- Integration tests that shell out to `dfx build --all` now force `RELEASE=0` to keep root hierarchy and delegation test flows consistent with debug-gated test endpoints.
- Added a wasm-size-oriented workspace `release` profile (`opt-level = "z"`, `lto = true`, `codegen-units = 1`, `strip = "symbols"`, `panic = "abort"`).

### 🗑️ Removed

- Removed `serde_json` from the HTTP outcall decode path and dependency surface.
- Removed unused workspace dependency entries for `futures` and `ic-management-canister-types`.

---

## [0.10.2] - 2026-02-25 - Memory Bootstrap Ordering & Guards

### 🩹 Fixed

- Post-upgrade lifecycle now initializes memory bootstrap first, then restores env context, so upgrade paths no longer depend on implicit stable-memory behavior.
- `EnvOps::restore_root()` and `EnvOps::restore_role()` now fail fast when memory registry bootstrap was not completed, making ordering errors explicit.
- `intent_authority` test canister now initializes memory on both `init` and `post_upgrade`, and defensively before the first update read path.

### 🔧 Changed

- Added runtime memory-bootstrap readiness tracking in `MemoryRegistryRuntime` via `is_initialized()`.
- `ic_memory!` now enforces a wasm runtime guard so stable-memory slots are only accessed during eager TLS bootstrap or after registry initialization.

```rust
// post_upgrade ordering
init_eager_tls();
MemoryRegistryRuntime::init(...)?;
EnvOps::restore_role(...)?;
```

---

## [0.10.1] - 2026-02-24

### 🔧 Changed

- `authenticated()` now supports optional scope syntax (`authenticated()` and `authenticated("scope")`).
- Runtime auth currently ignores scope values; delegated verification still enforces signature, root binding, audience, expiry, and `sub == caller`.

---

## [0.10.0] - 2026-02-24 - WIP Direct Delegated Token Model

### ⚠️ Breaking

- Relay-style authenticated RPC envelope support was removed in favor of direct token verification at each endpoint.
- Auth DTOs were reshaped to principal-based audiences and explicit shard/root binding (`root_pid`, `shard_pid`, `aud: Vec<Principal>`).

### 🔧 Changed

- Delegated token verification now enforces subject binding (`token.sub == caller`) and explicit audience binding (`self_pid ∈ token.aud`) before authorization.
- Certificate verification now enforces root authority binding (`cert.root_pid == env.root_pid`) in addition to signature checks.
- User shard token issuance now uses a single atomic sign path and delegation requests now carry shard-scoped audience principals.
- Environment import now treats `root_pid` as write-once: once initialized, later imports must keep the same root principal.

```rust
#[canic_update(requires(auth::authenticated("test:verify")))]
async fn test_verify_delegated_token(token: DelegatedToken) -> Result<(), Error> {
    Ok(())
}
```

### 🗑️ Removed

- Removed `AuthenticatedRequest` / authenticated relay endpoint flow (`canic_response_authenticated`) and related RPC plumbing.
- Removed local delegated-auth bypass path; delegated token checks are no longer skipped on local builds.

### 🧹 Cleanup

- Removed `SignatureInfra` compile-time source scanning from `canic-core/build.rs`; the build script now only validates `DFX_NETWORK`.
- Delegated signing remains single-step and root/shard-scoped; legacy `prepare/get` flows are no longer part of the auth path.
- CI now includes grep guards to fail builds if forbidden ECDSA APIs reappear (`sign_with_ecdsa`, `verify_ecdsa`, `ecdsa_public_key`).

### 🧪 Testing

- Updated delegated auth integration coverage to direct endpoint calls and added subject-mismatch denial cases.
- Updated macro validation tests for `authenticated(...)` predicates with optional scope syntax.

---

## [0.9.26] - 2026-02-22 - Fixes for Toko

### 🔧 Changed

- SubnetRegistryApi is now exported via `canic::api::canister::registry::SubnetRegistryApi`

---

## [0.9.25] - 2026-02-20 - Network & Pool Logging

### 🔧 Changed

- Root init now logs the selected build network (`local` or `ic`) so pool import source selection (`import.local` vs `import.ic`) is visible during bootstrap.
- Pool bootstrap logging now emits import policy context (`minimum_size`, resolved `import.initial`), candidate PID summaries, per-outcome import PID stats (imported/skipped/failed/present), and post-import pool status totals (`ready`, `pending_reset`, `failed`) with minimum-size warnings.

---

## [0.9.24] - 2026-02-20

### 🔧 Changed

- Root cycle top-up RPC now checks available root balance before deposit and returns a clear insufficient-balance workflow error when funds are too low.
- Fresh root init now waits for configured queued pool imports to finish before running `auto_create`, reducing reinstall races that created new canisters instead of reusing imported reserve canisters.

```text
Reinstall behavior now prefers pool reuse first, then creates only what is still missing.
```

---

## [0.9.23] - 2026-02-17

### ⚠️ Breaking

- Renamed `CanisterKind` variants: `Node` → `Singleton`, `Worker` → `Replica`, and added `Tenant`.
- Removed legacy config strings `"node"` and `"worker"`; configs must now use `"singleton"` and `"replica"`.
- Renamed sharding query endpoint `canic_sharding_tenants` to `canic_sharding_partition_keys`.

### 🔧 Changed

- Placement policy is now explicit: `Replica` requires `Singleton + scaling`, `Shard` requires `Singleton + sharding`, and `Tenant` requires a `Singleton` parent.
- Policy and RPC failures now preserve structured error codes/messages instead of collapsing to generic internal errors.
- Sharding identity naming now uses `partition_key` across DTOs, API, workflow, ops, and storage.

### 🧭 Migration Notes

- Update kind names and sharding endpoint names in configs and integrations.

```text
node   -> singleton
worker -> replica
canic_sharding_tenants -> canic_sharding_partition_keys
```

### 🧪 Testing

- Updated seam and sharding tests for new kind names, new policy checks, and `partition_key` terminology.
- Added coverage for duplicate `Singleton` rejection and allowed/blocked child creation paths for `Tenant`, `Replica`, and `Shard`.

---

## [0.9.20] - 2026-02-11

### 🔐 Auth

- Fixed delegated-token ingress decoding for `requires(authenticated())` endpoints with multiple Candid arguments by decoding only argument 0 from `msg_arg_data()`.
- Removed `CANIC_DEV_AUTH` auth bypass and now short-circuit `auth::authenticated()` when `DFX_NETWORK` is unset or `local`.

### 🪑 Env

- Removed `CANIC_ALLOW_INCOMPLETE_ENV`; env bootstrap now always rejects missing required env fields.

---

## [0.9.18] - 2026-02-04

### 🔐 Auth

- Enforce compile-time validation for `requires(authenticated())` endpoints to require a delegated token argument or a lone authenticated request.

### 🧪 Testing

- Added validation tests for authenticated argument rules.

---

## [0.9.17] - 2026-02-02

### 🔐 Auth

- DEV ONLY: move the local auth bypass into delegated token verification so all auth paths respect it; bypass only when `CANIC_DEV_AUTH=1`, returning an inert dev token without reading proofs.

---

## [0.9.16] - 2026-02-01

### 🔐 Auth

- DEV ONLY: allow `auth::authenticated()` to short-circuit when `DFX_NETWORK=local` or `CANIC_DEV_AUTH=1`, leaving production delegation verification unchanged.

---

## [0.9.14] - 2026-01-31 - Shard Lifecycle Cleanup

### 🔐 Auth

- Removed delegation rotation/admin/status surfaces; delegated auth is TTL-bounded and explicitly reprovisioned.

### 🧱 Architecture

- Shard allocation now admits the shard into lifecycle/HRW routing as part of the automated workflow.
- Removed shard rotation targets and unused shard lifecycle state storage.

### 🧭 Docs

- Clarified hub vs shard delegation responsibilities in sample canister docs.
- Clarified `pool` as the renamed config key from legacy `reserve`.
- Documented delegation TTL-boundary and the absence of background rotation or operator-driven shard lifecycle transitions.
- Documented guarded timer slot behavior and removed unused lifecycle timer cancellation.

---

## [0.9.13] - 2026-01-31 - Signer-Initiated Delegation Requests

### 🔐 Auth

- Added a root endpoint for signer-initiated delegation requests.
- user_shard now requests delegation on mint when no proof is stored, then retries minting.
- Root can store the verifier proof locally when requested to support `auth::authenticated()` endpoints.

### 🧱 Architecture

- Shard placement and delegated auth issuance are explicitly decoupled again (no provisioning-time delegation).

### 🧪 Testing

- Delegation flow tests now reflect signer-initiated delegation and root-based verification.

---

## [0.9.12] - 2026-01-27 - Codex Auth Delegation Audit

### 🧱 Architecture

- Moved policy-input/output shapes into `view/` and updated ops/workflow/policy to use them.
- Removed time reads from ops; workflows now pass timestamps into ops boundaries.

### ⚙️ Lifecycle

- Default `DFX_NETWORK` to `local` when unset to avoid init traps in local/dev.
- Introduced an internal bootstrap readiness barrier (`canic_ready`) and moved readiness checks out of public view endpoints and app guards.

### 🔐 Auth

- Restrict delegation provisioning to root callers only.
- Added root-only delegated-auth status query and rotation state tracking for observability.
- Expanded delegated-auth logging across provisioning, rotation, proof storage, and verification failures.

### 🧪 Testing

- Added PocketIC coverage for root-only delegation provisioning and signer proof validation.
- Delegation provisioning tests now use an isolated target dir and a minimal test config to avoid bootstrap drift.
- Aligned test categorization comments and embedded-config usage across test harnesses and test canisters.
- Embedded the delegation signer stub WASM into the root stub to keep topology cascades realistic under PocketIC.

### 🧭 Docs

- Relaxed storage-module rule to allow `serde`/proc-macro derives where needed.
- Consolidated test configuration policy in `TESTING.md`, including annotation format and test-canister rules.

## [0.9.11] - 2026-01-26 - Delegated Authentication (last part)

### 🔐 Auth

- Added auth rejection counters for delegated token failure paths (missing proof, proof mismatch, expired cert) and signer mint-without-proof.
- Counters are emitted only on existing failure paths; auth behavior unchanged.
- Collapsed test/dev auth canisters into `user_hub` + `user_shard` with root push-provisioning and proof-gated signing.
- Added a PocketIC authenticated-RPC test that provisions the root as a verifier and exercises `canic_response_authenticated`.

### 🧭 Docs

- Updated topology and config examples to reflect `user_hub`/`user_shard` and the finalized delegation issuance model.

## [0.9.10] - 2026-01-23

### 🔐 Auth

- Renamed the delegated auth guard to `auth::authenticated()` and added a shortcut to it.
- Added optional `ext: Option<Vec<u8>>` to delegated token claims for application-specific data.
- Exposed `DelegationApi::verify_token_verified` to return verified claims + cert for session construction.

---

## [0.9.7] - 2026-01-23 - IC Call Cleanup

### ⚡ Optimisations

- IC call builders now treat argument encoding as fallible end-to-end and expose raw-arg injection.
- Removed `try_with_*` in favor of a single fallible `with_*` path across infra/ops/workflow/api call layers.
- Intent pending scans now iterate stable maps directly to avoid cloning the full pending index.
- Endpoint metrics snapshots avoid intermediate HashMap clones.
- Env setters avoid redundant clones and writes for unchanged values.

### 🧯 Reliability

- Root directory resolvers now propagate config errors instead of panicking.

### 🧭 Practices

- Tests no longer match errors by string; use typed errors or observable state.

### 🔒 Security

- Removed raw IC signature APIs from the public CanIC surface. Delegated authentication is now the only supported runtime signing mechanism. This prevents misuse of low-level signature primitives and enforces delegated-auth invariants.

---

## [0.9.6] - 2026-01-22 - Lifecycle Hardening

- 🛸 Renamed config: `app_state` → `app`, `app.mode` → `app.init_mode`, and `whitelist` → `app.whitelist`.
- 🥑 Renamed `[delegation]` to `[auth.delegated_tokens]`, default enabled, with clearer disable errors.
- 🧯 Lifecycle init/post-upgrade now surface phase-correct failures; traps are restricted to lifecycle adapters.
- 🧷 Canic built-in endpoints now return `Result` to avoid trapping on access denials.
- 🪑 Env bootstrap defaults are test-only unless `CANIC_ALLOW_INCOMPLETE_ENV=1` is set.
- 🧸 Directory imports validate required roles against config.
- 🪐 Added lifecycle boundary regression test and a trap-usage guard.

---

## [0.9.5] - 2026-01-21 - Access Families + DSL Alignment

- 🧭 Refactored access predicates into explicit families (`app`, `auth`, `env`) with `expr` as internal evaluation only.
- 🔓 Exposed public, composable auth predicates under `canic::access::auth` without duplicating logic.
- 🧹 Removed legacy DSL shims and aligned DSL built-ins with access families (env owns build-network rules).

---

## [0.9.4] - 2026-01-21 - App State Init + Sync Access

- ✅ App init mode is now config-driven (`app.init_mode`) with a default of `enabled`.
- ⚡ Endpoints only become async when explicit access predicates are present; implicit app gating stays sync.
- 🧱 Internal protocol endpoints bypass app-state gating and reject app predicates at compile time.

---

## [0.9.3] - 2026-01-21 - App State Gating Defaults

- ✅ Default app-state gating now applies to all endpoints unless an explicit app predicate is present; app-mode checks use `app::allows_updates()` and `app::is_queryable()`.
- 🧱 Internal protocol endpoints can be marked with `internal` to bypass app-state gating; app predicates are rejected at compile time for these endpoints.
- ⚙️ Added `app.init_mode` configuration for initial app mode (default `enabled`) and apply it during canister init.
- 🧹 Removed `app::is_live` from the DSL, access layer, and docs.

---

## [0.9.2] - 2026-01-20 - Auth Refactor

- 🔐 Auth refactor: Replaced staged access control with a single requires(...) expression model using composable predicates (caller::, app::, self_env::), all evaluated by one async evaluator.

- 🧹 Cleanup: Removed legacy DSL syntax, rule/stage APIs, and error enums; access behavior, metrics, and Unauthorized mapping remain unchanged.

```rust
// Example: complex access expression with composition
#[canic_update(requires(any(
    caller::is_root(),
    all(
        caller::is_controller(),
        not(app::is_readonly()),
        custom(HasPaidAccount),
    ),
)))]
async fn update_critical_settings(
    input: SettingsInput,
) -> Result<(), canic::Error> {
    // …
}
```

This demonstrates, at a glance:

- 🔀 boolean composition (any, all, not)
- 👤 caller-based predicates
- 📦 app state predicates
- 🔧 custom async predicates
- 🔐 a single, readable access surface


Access predicates are grouped explicitly using boolean combinators:

- all(...) — AND: every predicate in the group must pass
- any(...) — OR: at least one predicate in the group must pass
- not(...) — NOT: inverts a single predicate or group

Groups can be nested arbitrarily, so complex policies are expressed declaratively and read top-down. Evaluation short-circuits on the first failure, and only the denying predicate is recorded for metrics and logs.

This keeps access logic local, composable, and auditable without hidden ordering or implicit stages.

---

## [0.9.1] - 2026-01-20 - Consolidation and Consistency Audits

### Added
- Layering guard checks in CI to prevent workflow record usage, public record re-exports, and misuse of "view" naming.
- Formalized layer and naming rules in AGENTS.md (DTO/view/record/ids and mapper naming).

### Changed
- Separated DTO inputs/responses from internal views across core modules and updated mappers accordingly.
- Standardized conversion helper names to avoid "view" outside view projections.
- Reduced storage record exposure by removing public re-exports and routing record access through storage modules.
- Pushed record-to-DTO shaping into ops helpers across env/state/directory/auth/scaling workflows.
- Moved `IntentResourceKey` to ids to keep workflow free of storage schema types.
- Split delegation flow tests so issuance runs only under certified runtime conditions.
- Reworked access control around expression-based predicates (`all`/`any`/`not`/`custom`) and centralized evaluation under `access::expr`.
- Access-denial metrics now record the predicate name (built-in or custom) alongside the coarse kind.
- Delegation auth APIs now expose local sign/verify helpers for proofs and tokens; auth shard/test flows use the unified helpers.

### Broked
- 🚨 Auth is currently broken pending redesign.

---

## [0.9.0] – 2026-01-19 - Delegation's What You Need

This release introduces **delegated signing with local verification**, completes the **root → shard trust model**, and clarifies certified-data requirements for issuance (PocketIC vs replica).

---

### 🔐 Delegation & Trust Model (Core Change)

* Root canister is now the **sole delegation authority** and signs `DelegationCert`s.
* New **auth shard** model:

  * Shards store a `DelegationProof` locally.
  * Shards mint delegated tokens without calling root.
  * Any canister can verify tokens locally against root trust.
* Delegation issuance now uses a **prepare / get** flow to support certified queries correctly.
* Delegation proof storage helpers added to the public delegation API for shard-local handling.
* Auth shard proof updates now accept **any subnet-registered caller** (auth_hub is not a parent).

**Why this matters:**
This unlocks scalable, local verification of delegated authority with no runtime dependencies on root or registries.

---

### 🧪 Delegation Verification

* Delegation and token verification are local-only and validate against the
  stored proof.
* Verification does not require certified data or a query context.
* Issuance still depends on data certificates when retrieving canister
  signatures.
* Test-only partial verification endpoints were removed.

**Why this matters:**
Verification is deterministic and testable without query-time assumptions,
while issuance remains explicit about its certified-data dependency.

---

### 🧱 Topology, Ops, and Layering Corrections

* Directory resolvers and builders moved fully into **ops**.
* Workflow now consumes ops for canonical directory resolution.
* Child-canister resolution centralized in ops:

  * Root uses the subnet registry.
  * Non-root uses cached children.
* Root bootstrap test harness now waits for subnet directory materialization.
* Architecture documentation updated to reflect:

  * auth_shard topology
  * access-stage ordering
  * certified-query requirements

**Why this matters:**
This resolves prior layer leakage and restores a clean dependency direction.

---

### 📊 Metrics, Errors, and Observability

* Endpoint metrics now treat **non-`Result` endpoints** as implicit `ok` completions.
* Delegation token/proof errors now surface **actual failure reasons** instead of generic internal errors.
* Delegation flow test advances PocketIC certified time before signature retrieval to avoid false negatives.

---

### ⚙️ Sharding, Policy, and Intent Handling

* Sharding policy now consumes **policy-scoped placement and assignment views** assembled by workflow.
* Added an **hourly intent cleanup workflow**:

  * Aborts expired pending intents
  * Reconciles capacity totals
* Intent TTL is now enforced logically at read time:

  * Expired intents no longer count as pending or reserving capacity.

---

### 🧭 Environment & API Surface

* Public `api::env::EnvQuery` re-export added for canister-level environment inspection.

---

### 📌 Summary

**0.9.0 completes Canic’s delegated-authority model.**
The root canister now delegates signing power cleanly, shards mint tokens independently, and any canister can verify authority locally—*with certified security on real replicas and explicit, honest behavior under PocketIC*.

This is a **foundational release**; many smaller changes exist, but they all serve this core outcome.


## [0.8.6] - 2026-01-16
### Fixed
- Raised intent pending entry storage bound to accommodate 128-byte intent keys with TTL, plus a regression test.

## [0.8.5] - 2026-01-16 - Intent System
### Added
- Framework-level intent store backed by stable memory, with ops-layer APIs for reserve/commit/abort and upgrade-safe recovery helpers.
- Intent IDs reserved in canic-core stable memory registry to prevent accidental reuse.
- PocketIC contention test canisters + race test to validate intent-based reservation flow under async interleaving.
- Test-only config builder for programmatic config setup in tests.
- canic-memory strict registry enforcement and registry introspection helpers for ranges/IDs.

## [0.8.4] - 2026-01-14 - Cleanup
- Clarified build docs: `DFX_NETWORK` defaults to `local` when unset.
- Collapsed redundant snapshot types in ops/workflow (env/scaling/sharding/pool) and kept canonical `*Data` at boundaries.
- Pool selection now operates directly on `PoolData`; view flattening happens at the DTO boundary.
- Sharding registry exports canonical data; `pool` string conversion happens in the mapper.
- Ops log and memory registry snapshots now return DTO views directly; removed identity mappers.
- Renamed `RootBootstrapSnapshot` to `RootBootstrapContext`.

## [0.8.3] - 2026-01-13
- Added protocol to the public api layer so things like dispatch::Icrc21 can be exported
- Split the DSL surface into `canic-dsl` (symbols) and `canic-dsl-macros` (proc macros), with facade re-exports updated to match.
- Macro guards now use the `app_is_live` DSL symbol instead of reserving `app`.

## [0.8.1] - 2026-01-13
- HttpApi is now exported under api::ic along with call

## [0.8.0] - Public API Consolidation & Error Model Hardening - 2026-01-13

### Breaking (pre-1.0, intentional)
- Formalized the public Canic API surface.
- Introduced a structured `canic::api` module that exposes runtime capabilities by intent (access, calls, canisters, RPC, observability, timers). Direct access to internal `canic_core` modules is now explicitly unsupported.
- Clarified access semantics in the public API by resolving ambiguity between caller-based and self-based checks:
  - `caller_is_*` functions live under `api::access::auth`.
  - `self_is_*` environment predicates live under `api::access::env`.
  - This removes prior naming collisions and makes authorization logic explicit.

### Added
- Stable public data contracts.
  - Exposed `canic::dto`, `canic::ids`, and `canic::protocol` as first-class public modules.
  - These are now the canonical, versioned contracts for Candid, RPC, testing, and tooling.
- Curated public runtime API.
  - `api::access` - authorization, environment predicates, guardrails.
  - `api::call` - inter-canister call primitives.
  - `api::canister` - placement, scaling, sharding, WASM management.
  - `api::rpc` - non-IC RPC abstractions.
  - `api::ops` - observability helpers (logging, perf).
  - `api::timer` - scheduling helpers.
- Opinionated prelude aligned with the new API.
  - Prelude now re-exports only the public API surface (no internal paths, no aliases).
  - Reduces boilerplate while preserving semantic clarity.

### Changed
- Internal error model hardened.
  - Errors now normalize through a single `InternalError` boundary with class + origin metadata.
  - Workflow, ops, and infra layers consistently map into internal errors before DTO conversion.
- Macro expansion safety improved.
  - All DSL and lifecycle macros now rely on the public `api`, `dto`, and `ids` modules.
  - Internal core access is restricted to a hidden `__internal` module used only during macro expansion.
- Removed accidental public exposure of core internals.
  - `canic_core` is no longer re-exported directly.
  - Downstream crates are guided to stable facade APIs instead of internal modules.

### Cleanups
- Removed unused or misleading internal error categories.
- Eliminated workflow-local error enums that existed only as conversion wrappers.
- Reduced layering violations where ops/workflow code previously tagged errors with incorrect origins.

### Migration Notes
- Replace direct references to `canic_core` with `canic::api`, `canic::dto`, or `canic::ids`.
- Update authorization checks:
  - `is_root` (caller) -> `api::access::auth::caller_is_root`.
  - Environment checks -> `api::access::env::*`.
- Macros (`#[canic_query]`, `#[canic_update]`, `start!`, `start_root!`) continue to work unchanged.

### Release Status
- This release completes a major internal refactor to stabilize Canic's public contract ahead of future feature work.
- While pre-1.0, 0.8.0 establishes the intended long-term API shape and significantly reduces the likelihood of breaking changes going forward.

## [0.7.28] - 2026-01-12
- Moved public macro entrypoints (build/start/timer/perf/auth and endpoint bundles) into the `canic` facade crate.

## [0.7.26] - 2026-01-10
- Cleaned up stale documentation and layering inconsistencies across storage, ops, and workflow modules.
- Enforced root canister presence in prime subnet config, requiring `subnets.prime.canisters.root` to be `kind = "root"`.
- Directory rebuilds/imports now reject duplicate roles for app/subnet directories.

## [0.7.23] - 2026-01-09
- Guarded root bootstrap so it fails fast if the embedded WASM registry is uninitialized, preventing auto-create from running before WASMs are loaded.

## [0.7.22] - 2026-01-09
- Unified canister modeling by removing summary/snapshot abstractions and standardizing all internal topology, registry, and children workflows on a single authoritative CanisterRecord, simplifying data flow and eliminating lossy projections.

## [0.7.21] - 2026-01-08
- added with_args, try_with_args to call api workflow
- complete refactor of the IC Call wrapping, on four different layers, fun
- redid the cdk/spec directory with better structure
- bug fixes over multiple versions

## [0.7.15] - 2026-01-08
- refactored the endpoint wrappers - now they convert from canic::Error into the downstream
return error type specified by the developer

## [0.7.14] - 2026-01-08 - Cleanup Complete
- removed DTO usage from ops by introducing ops-local command types and generic cascade/install payloads
- mapped app state commands in workflow before invoking ops
- routed API wrappers through workflow for signature, network, config, wasm, timer, and IC call/http helpers
- moved BuildNetwork into ids:: as it's not really a good fit for the ops/workflow layers
- moved EndpointId/Call, SystemMetricKind and AccessMetricKind into ids::

## [0.7.13] - 2026-01-07
- lots of work on bubbling up errors.  InfraErrors treated differently.  Standardised all errors under ops/
- split the Ic ledger code over api, workflow, ops and infra
- re-wrapped the ic Call type via CallOps, adding metrics
- moved IC management status view adapters into workflow; ops now return internal mgmt status types
- routed eager TLS init through MemoryRegistryOps in workflow runtime
- generally a full day of refactoring but not much of it is interesting enough to mention

## [0.7.12] - 2026-01-06
- updated the signature code to use the HashTree from ic-certified-map.  Basically none of us know how it works so this
is just trial and error.
- added require_tenant_shard to ShardingApi
- normalized infra IC call wrapper + error flow, and made infra signature/NNS errors lossless

## [0.7.11] - 2026-01-05
- Refactored sharding placement into a pure, deterministic policy operating on explicit state snapshots, with all configuration registry access, and side effects moved into query/workflow layers.
- Updated root hierarchy tests to use explicit root install helpers and improved bootstrap reliability.
- Added a global Topology re-entrancy guard just in case (Pocket IC does things a little differently, which is good for
exposing problems you didn't know existing)
- Updated `AGENTS.md` and `ARCHITECTURE.md` and did another full codex layer violation scan
- Moved timestamp minting into workflow; ops now accept explicit `created_at` for pool, sharding, registry, and log writes.
- Policy now depends directly on Config, not ConfigOps

## [0.7.10] - 2026-01-04
- moved api instrumentation to access/
- wrapped most Api, Ops and Workflow functions within a corresponding namespace struct
- added create/install lifecycle logs in MgmtOps for symmetry
- Routed workflow ambient IC calls through `ops::ic::runtime` (time, identity, spawn, trap).
- Replaced query-only API wrappers with re-exported `*Query` types in `api/*`.
- Endpoint macros call `api::*Query` directly, and sharding tenants query now accepts `String`.

## [0.7.9] - 2026-01-04
- mirrored the authentication functions in access/ to api::access for public consumption
- macro access checks now return Error at the endpoint boundary

## [0.7.8] - 2026-01-04
- Nested policy directory/registry under policy::topology to align module structure
- Namespaced pool workflow helpers under PoolWorkflow
- Exposed DFX_NETWORK via network() in api::ic::network
- Namespaced metrics query helpers under MetricsQuery

## [0.7.7] - 2026-01-04
- Split out api/topology and added in missing functions
- Stopped the macros panicking if there was an error with the stable log
- Moved free functions into the ProvisionWorkflow struct

## [0.7.6] - 2026-01-04
### Fixed
- Resync certified_data from the signature map during post-upgrade.

## [0.7.4] - 2026-01-04
- Added new ckTokens to canic-cdk, such as ckUNI and ckWBTC
- CI fixes to make sure that PocketIC doesn't run out of memory

## [0.7.3] - 2026-01-04 - Mostly Done
### Added
- Public API `api::ic::call` wrapper that routes through ops for metrics/logging and maps internal errors to `Error`.
- Ops-level `ops::ic::call::CallOps` helper for typed IC calls with candid encode/decode handling.

### Changed
- `SubnetIdentity::Manual` no longer requires a caller-provided subnet principal; runtime supplies a deterministic placeholder for test/support flows.
- Made Ops:: structs consistent
- flattened the ops/metrics/store structs so there's only one set of MetricsOps structs now
- Preludes cleaned up to reduce redundant imports.

## [0.7.2] - 2026-01-03 - Workflow & Policy Audit
### Changed
- Renamed topology lookup API to `subnet_directory_pid_by_role` to make directory sourcing explicit.
- Registry policy now consumes canister config from workflow to avoid policy → ops config access.
- Subnet registry registration no longer enforces singleton roles; kind checks live in policy.
- Pool selection now deterministically picks the oldest entry with a stable tie-breaker.
- Cycle tracker retention cutoff is now computed in workflow/policy and passed into ops.
- Log retention parameters are derived in workflow/policy and passed into ops.
- Workflow scheduling cadence constants moved out of ops into workflow config.
- Cycles auto-topup eligibility is now decided in policy and executed in workflow.
- Randomness scheduling enablement is now decided in policy and executed in workflow.
- Env fallback vs hard-error policy moved into domain policy and applied in workflow.
- Topology invariant checks now live in domain policy and are invoked by workflow.

## [0.7.1] - 2026-01-03 - Ops Audit
### Highlights
- Major internal refactor to make layer boundaries explicit (api/endpoints/workflow/ops/domain) and reduce cross-layer coupling.
- Endpoint wrappers are now grouped by feature domain, making the codebase easier to navigate and maintain.
- Data crossing boundaries is consistently shaped as DTOs/views instead of leaking internal storage types.

### Added
- Workflow RPC helper for create-canister requests and planning helpers for scaling/sharding.
- DTO types for memory registry endpoints.

### Changed
- IC network calls now flow through `ops::ic` so side effects have a single, explicit home.
- Directory builders return view types; workflow directory sync imports/exports those views.
- Core layering/lifecycle docs aligned with AGENTS guidance.
- Example canisters call workflow helpers instead of policy/ops directly.
- canic-memory startup now goes through `runtime::registry` and README updated.
- Upgrade decisioning moved into policy; ops/infra upgrades are purely mechanical.
- Non-root env defaulting centralized in `EnvOps`.
- Log control ops split from log view ops (`LogOps` vs `LogViewOps`).
- API endpoint wrappers reorganized into domain modules; macro call sites updated.

### Fixed
- Memory registry endpoint returns a proper DTO view without leaking internal types.

## [v0.7.0] — 2025-12-30 - Architecture Consolidation & Runtime Discipline

This release is a structural milestone focused on clarifying responsibility boundaries, eliminating architectural ambiguity, and hardening runtime behavior across the system. While user-visible behavior is largely unchanged, the internal model is now significantly more coherent, testable, and extensible.

### Highlights

* **Strict Layer Separation Enforced**

  * Clear demarcation between **model**, **ops**, **workflow**, and **runtime** concerns.
  * Storage-backed state, runtime orchestration, and view/DTO adaptation are now explicitly separated.
  * Removed implicit cross-layer coupling and eliminated several “gray area” abstractions.

* **Model ↔ View Canonicalization**

  * Systematic `From`/adapter patterns established between model types and DTO/view representations.
  * Storage types no longer leak into API or workflow layers.
  * Enables safer refactors, clearer invariants, and more predictable serialization boundaries.

* **Runtime vs Storage Semantics Clarified**

  * Runtime logic moved out of storage-oriented ops where side effects or scheduling were previously ambiguous.
  * Ops are now narrowly scoped to deterministic state transitions and validation.
  * Workflow owns orchestration, propagation, and cascade semantics.

* **Topology & Cascade Hardening**

  * Topology synchronization rewritten around explicit bundle semantics.
  * Parent/child propagation is now validated hop-by-hop with cycle and termination guarantees.
  * Failures abort cleanly instead of producing partial or inconsistent topology state.

* **Policy-Driven Pool & Lifecycle Logic**

  * Pool admissibility and lifecycle checks are now explicitly policy-based and side-effect free.
  * Local vs network-dependent behavior is isolated and testable.
  * Runtime enforcement no longer conflates eligibility checks with mutation.

* **Metrics & Instrumentation Cleanup**

  * HTTP and runtime metrics normalized behind canonical method/label mapping.
  * DTO conversion paths are explicit and consistent with the broader view strategy.

### Why This Matters

v0.7 dramatically reduces architectural entropy. It makes the system easier to reason about, safer to evolve, and far more resistant to subtle bugs caused by layer leakage or mixed responsibilities. This release lays the foundation for future features without accumulating technical debt.


## [0.6.20] - 2025-12-26
### Added
- Added required `kind = "root" | "singleton" | "worker" | "shard"` to subnet canister configs, with
  validation that directory roles must be `kind = "singleton"`.
- Added typed endpoint identity (`EndpointCall`, `EndpointId`, `EndpointCallKind`) derived by macros and propagated through dispatch
  and metrics (endpoint labels are no longer user-supplied).
- Added `log.max_entries` validation (<= 100,000) to prevent unbounded log retention.
- Added a log readiness gate so logging is a no-op until runtime initialization completes.

### Changed
- App/subnet directories now map roles to a single `Principal`.
- Registry registration now rejects duplicate principals and singleton-role collisions.
- Topology snapshots now use `TopologyDirectChildView` in `children_map` to avoid redundant parent identifiers.
- Pool entry views are assembled from split header/state parts to avoid duplicating identity fields.

### Fixed
- Subnet registry subtree traversal now guards against parent cycles.
- Pool export validates readiness and metadata before removing entries.
- Certified-data signature ops now enforce update-only context to prevent query traps.

## [0.6.19] - Perf Stack
- Endpoint dispatch now records exclusive perf totals via a scoped stack; removed `perf_scope` from the prelude and dropped the `defer` dependency.  This means that endpoints can call each other and the correct performance metrics are logged.

## [0.6.18] - 2025-12-24
### Added
- Added `log.max_entry_bytes` to cap per-entry log message size and truncate oversized entries.
- Pool admin queued imports now return a summary with pool status counts and skip reasons.

### Changed
- `PageRequest` no longer implements `Default`; callers must construct it directly (`PageRequest { limit, offset }`).

### Fixed
- `EnvOps::import` now returns a typed error when required env fields are missing, and non-root init traps with a clear message.
- `Http::get` now treats any 2xx status as success (instead of only 200).
- Shard draining now reassigns tenants off donor shards by planning with donor exclusion.
- Sharding plan `CreateBlocked` now carries structured reasons, and sharding lookup/planning APIs accept `AsRef<str>`.

## [0.6.17] - 2025-12-23
### Added
- Subnet pool bootstrapping now supports `pool.import.local` and `pool.import.ic` to seed the warm pool before root auto-create.

## [0.6.16] - 2025-12-22
### Fixed
- Local pool imports now skip non-routable canister IDs instead of persisting failed entries.
- Pool import immediate now surfaces reset failures to callers.
- Failed installs of pooled canisters now attempt a recycle back into the pool.
- Failed installs of newly created canisters now delete the canister to avoid orphaning.
- Pool import/recycle now remove topology registry entries only after a successful reset.
- App state now cascades during directory syncs so newly created canisters match root mode.

## [0.6.13] - 2025-12-21
  - Env/config accessors are fallible: ConfigOps::current_* and EnvOps::* return `Result`, and callers propagate or
    handle errors; lifecycle entrypoints trap on missing env/config with clear messages.
  - Directory ops hardened: added infallible get accessors, made canic_subnet_directory infallible, and aligned tests/
    endpoints accordingly.
  - Env semantics tightened: import validates required fields; root/non‑root predicates tolerate missing env with safe
    fallbacks; removed unused env helpers; try_* env accessors are test‑only.
  - Bootstrapping + local fallback clarified: get_current_subnet_pid renamed to try_get_current_subnet_pid; local
    non‑root env fallback uses deterministic principals; IC still traps on missing env.
  - Init payload safety: removed CanisterInitPayload::empty and Default; construct via struct literal.
  - Testkit upgrades: non‑root installs now pass deterministic EnvData; optional helper added to install with custom
    directories; directories are empty by default by intent.
  - Docs updated: AGENTS.md + CONFIG.md now explain runtime invariants and local/IC behavior.
  - PocketIC wrapper now has explicit, high‑signal documentation (singleton rationale, assumptions, directory opt‑in,
    fatal install failures).


## [0.6.12] - 2025-12-21
- Enforced build‑time DFX_NETWORK (must be local or ic) across all Cargo builds; scripts/Makefile now map
NETWORK=local|mainnet|staging to DFX_NETWORK=local|ic and fail fast if missing/invalid.

## [0.6.10] - 2025-12-21
- improved rust error handling for ICRC-21, the ? flow is now useable

## [0.6.9] - 2025-12-20
- renamed reserve → pool (config key `pool`)
- pool entries now track status (`PendingReset`, `Ready`, `Failed`) to support background resets
- added `ImportQueued` (batch, background reset) and `ImportImmediate` (synchronous reset) admin commands
- added pool unit tests covering queued imports, requeue scheduling, and metadata preservation

## [0.6.8] - 2025-12-18
- removed Mutex from the rand crate, so no chance of an expect() panic
- rand utils now seed a ChaCha20 PRNG from IC `raw_rand` and reseed on a timer (metrics track the raw_rand call)
- per-canister randomness reseed interval is configurable (default 3600s) and can be disabled
- randomness can seed from time nanos as an alternative to IC `raw_rand` (config uses `source = "ic"` or `"time"`)

## [0.6.7] - 2025-12-18
### Fixed
- `#[canic_query]`/`#[canic_update]` no longer panic on unsupported parameter patterns; now emit proper compile errors.
- Root/non-root runtime startup now traps with a clear message if stable memory registry init fails.
- Lifecycle config load now traps with a clear message (instead of panicking) when embedded config is invalid.
- Sharding registry no longer panics on invalid `(pool, tenant)` inputs; returns a storage error instead.
- ICRC-2 allowance expiry checks now compare against IC nanosecond time (fixes false “expired” errors).

### Added
- `BoundedString::try_new` for fallible bounded-string construction.
- XRC candid bindings under `spec::ic::xrc` and IC-edge wrappers under `ops::ic::{cmc,xrc}`.
- Rust-decimal-backed `Decimal` type under `canic-types` (candid encodes as `text`).
- Canic-specific pricing DTOs under `canic-core::dto::payment`.

### Changed
- Refactored long modules to remove `clippy::too_many_lines` suppressions (SNS env table and lifecycle orchestrator).
- Centralized internal cross-canister RPC method name strings.
- Payment and pricing helpers moved out of `spec/` into `ops::ic/` (spec is now spec-only).

## [0.6.6]
- added back build_network() that reads in option_env!(DFX_NETWORK), and added access policies
- refactored testkit::pic so it uses a static variable for all tests (we were running out of chunks)
- canic-dsl weren't passing through clippy lints
- moved icrc out of ic in ops/ for consistency
- changed canic-dsl so that custom error types can be used as long as they have From<canic::Error>
- made the Call wrapper accept any kind of principal (icydb works)
- set up http_get so it's a namespace struct Http, and also used in the prelude.
- added get_raw and get_raw_with_label to Http

## [0.6.0] - Aquafresh 3-in-1 Endpoint Protection

### Changed
- Major internal refactor: removed the old `ops/` and `model/` interface layer; wrappers were removed or split between crates.
- `canic-dsl` endpoints now support three levels of endpoint security and automatically apply `perf_scope`.
- Reserve subsystem refactor: move reserve orchestration into `ops::reserve` + `ops::service` and consolidate state access via `ops::storage`.

### Added
- Split metrics queries into per-metric endpoints: `canic_metrics_system`, `canic_metrics_icc(page)`, `canic_metrics_http(page)`, `canic_metrics_timer(page)`, `canic_metrics_access(page)`.

### Removed
- Removed the aggregated `canic_metrics` endpoint and `MetricsReport` type.

## [0.5.22] - 2025-12-13
### Added
- CI now builds all canister `.wasm` artifacts (and deterministic `.wasm.gz` via `gzip -n`) into `.dfx/local/canisters/...` before running `fmt`, `clippy`, and tests.
- New `canic-dsl` crate with `#[canic_query]` / `#[canic_update]` proc-macro attributes.
- Centralized endpoint dispatch wrappers (sync + async query/update) to unify perf instrumentation and future endpoint hooks.

### Changed
- Config loading is now unconditional in lifecycle; build scripts always provide `CANIC_CONFIG_PATH`, generating a minimal default config when the repo config file is missing.
- Perf instrumentation switched to call-context instruction counter (`ic0.performance_counter(1)`); perf aggregation is now keyed by kind (`Endpoint(name)` vs `Timer(label)`) to avoid label collisions.
- Whitelist enforcement now always consults `Config` (no longer gated behind `feature = "ic"`).
- Root canister embeds dependent canister `.wasm.gz` on `wasm32` builds (non-wasm builds use empty slices).

### Fixed
- `perf_scope!` now reliably records at scope exit (RAII guard lifetime/shadowing).
- Stable memory range initialization is idempotent when re-registering the same initial range (prevents upgrade traps).

### Removed
- `EnvError`; SNS principals now fail-fast on build if invalid.
- All custom cfg-based CI conditionals (notably `cfg(canic_github_ci)`) and related build-script cfg emissions.
- Dead `DFX_NETWORK` network helper.

## [0.5.21] - Perf & Types Consolidation
- Labeled timer metrics: `TimerMetrics` now records mode, delay, and a caller-provided label so scheduled tasks can be distinguished in metrics; interval timers increment on every tick.
- `canic_perf` diagnostic query and instruction aggregation for timer executions (labels + total instructions) to inspect timer cost without inflating main metrics.
- Added `timer!` and `timer_interval!` macros that auto-label timers with `module_path::function` and route through `TimerOps` for perf recording.
- bumped rust to 1.92.0

## [0.5.17] - 2025-12-11 - HTTP Metrics
### Added
- Ops-level `http_get` helper for JSON GETs that records HTTP outcall metrics alongside the system counters.
- Timer metrics wrapper to record scheduled timers (once + interval) and track their cadence alongside other system metrics.

### Changed
- Metrics reporting now distinguishes HTTP outcalls and the main metrics façade is called `SystemMetrics`.

## [0.5.16] - 2025-12-11 - O(n^2) -> O(n)
### Fixed
- Decode `notify_top_up` responses from the CMC and surface errors instead of treating any reply as success, so failed cycle top-ups no longer appear successful.

### Changed
- Topology sync bundles now carry only the parent chain and per-node direct children (no full subtree), removing the quadratic fanout cost and matching the stored parent/child snapshot.

## [0.5.15] - 2025-12-11 - Canister Lifecycle Orchestrator
- simplified the reserve-pool subsystem to make canister recycling more reliable and easier to maintain.
- A new internal utility (recycle_via_orchestrator) integrates cleanly with the orchestrator so that recycling automatically triggers topology/directory updates when required.
- changed (limit, offset) endpoint arguments to use a unified struct

## [0.5.14] - 2025-12-10 - Icc / System Metrics
- split Metrics into two types, System and Inter-canister calls
- Pagination queries now take a `PageRequest` (with defaults and a 1,000 item cap) instead of raw `offset`/`limit` pairs for logs, directories, cycle tracker, and topology children.

## [0.5.13] - 2025-12-10 - Canic Metrics
- Wrapped cross-canister call construction so `CanisterCall` metrics are recorded centrally without scattered increments.
- Targeted topology cascades now delegate to the first child (letting the branch fan out) to honor parent-only auth and cut hop count.
- Added PocketIC coverage for worker creation ensuring new workers register under `scale_hub` and appear in its child view.

## [0.5.12] - 2025-12-10
- Topology syncs are now branch-targeted when creating canisters: root cascades only the affected subtree, retries once per hop, and falls back to a full cascade on errors. Large cascades log warnings so noisy fan-outs are visible.

## [0.5.10]
- added a wrapper around performance_counter
- added more types to ICRC2 (Allowance, TransferFromArgs, etc.)

## [0.5.8] - 2025-12-09
- Reduced topology cascade complexity: subtree extraction now builds a parent→children index once and reuses it for all child bundles, and registry subtrees walk the stable map directly without repeated scans. This keeps syncs near linear even with hundreds of canisters.
- Added targeted topology cascade from root so creates only cascade the affected branch (root→child→…→leaf), with retries and a safe fallback to full cascade if any hop fails.

## [0.5.7] - 2025-12-08
- Added caller/parent context logs for create_canister_request and the root handler so bootstrap failures during repeated create calls surface clearly.

## [0.5.6] - 2025-12-07
### Added
- One timer service entry point to start all background jobs (logs, cycle tracker, reserve) per canister role.
- Info-level tick logs for retention and cycle tracking so you can see timers firing.

### Fixed
- Root init no longer traps if auto-creating canisters fails; it now logs the error and keeps running.
- Log retention moved to a timer instead of every write, keeping logging cheap while still cleaning up.
- Cycle tracker purge now runs on the timer loop instead of a modulus counter, aligning all cleanup on scheduled ticks.

## [0.5.4] - 2025-12-06
- Hardened reserve imports: uninstall first, reset controllers, then remove from registry and recascade before registering into the reserve pool.
- Added a management delete wrapper and explicit delete path separate from uninstall.
- `impl_storable_*` macros now panic with contextual messages on (de)serialization errors and ship basic round-trip/corrupt-data tests.
- Refreshed `canic-memory` README with simpler “why/how” guidance, boot log example, and clearer eager TLS rationale.

## [0.5.2] - 2025-12-06
- Split stable-memory plumbing into the new `canic-memory` crate (manager, registry, eager TLS, macros) and re-exported its macros/runtime from `canic`; added registry/eager-init tests and ops wrapper for initialization.

## [0.5.1] - 2025-12-05
- Moved general-purpose wrappers (Account, Cycles, BoundedString, WasmModule, ULID) into `canic-core::types` and slimmed `canic::types` to topology roles.

## [0.5.0] - canic-cdk breaking change - 2025-12-05
- Added the `canic-cdk` crate as a curated façade over `ic-cdk`, `candid`, timers, and management canister APIs.
- Introduced `canic-core` as the shared types/utils crate (perf macros, MiniCBOR serializers, bounded strings/ULID/cycles, wasm/time/hash helpers); re-exported via `canic::core` and replaces the old `canic-utils` crate.

## [0.4.12] - 2025-12-04
- Removed the auth-specific `verify_auth_token`; callers now pass the signing domain and seed into `ops::signature::verify` when validating tokens.
- Fixed `canic_subnet_canister_children` on root by rebuilding the view from the registry instead of the empty local snapshot.
- Register canisters in the subnet registry before install so init hooks can see themselves; roll back the entry on install failure to avoid phantom records.

## [0.4.8] - 2025-12-04
- made the memory data structures pub(crate), and removed unused code
- commented more public facing functions

## [0.4.7] - 2025-12-04
- Fixed canister signature verification panic on short (10-byte) canister principals by constructing the DER-encoded public key with the signing seed

## [0.4.6] - 2025-12-03 - e2e Tests
- AppDirectory now rebuilds from the registry on root (not just prime root) while children read their stable snapshot, keeping directory queries consistent everywhere.
- SubnetDirectory resolves from the registry on root and falls back to an empty view instead of erroring during early bootstrap/config gaps.
- Added PocketIC coverage that asserts app/subnet directory views match across root and all children after auto-create.
- fixed missing Ops passthrough functions

## [0.4.1] - 2025-12-01 - Bug Splatting
- Register new canisters in the subnet registry only after a successful install to avoid phantom entries on install failure.
- Post-upgrade now replays memory range/ID registrations so new stable-memory segments are validated after upgrades.
- Failed canister installs recycle the allocated canister into the reserve instead of leaving it orphaned.
- Fix ICP→cycles conversion to use ICP-per-XDR and add coverage for the buffered calculation.
- Sharding planner now skips full shards and requests creation when capacity is exhausted.
- Reserve imports reset controllers to the configured set, and registry records track upgraded module hashes.
- Narrowed internal sharding/pagination helpers to crate scope to shrink the public surface.
- Removed unused shard metrics helpers.

## [0.4.0] - 2025-12-01 - endpoints -> ops -> model
- Endpoints now call a slim ops façade; ops owns orchestration and DTOs; model stays pure storage/registries.
- ICRC helpers added to ops for supported standards and consent messages.
- Sharding, topology, directory, reserve, and env access now flow through ops (no direct model calls).
- State and topology sync now use ops DTOs and cascade helpers; logging writes routed through LogOps.
- Auth, request handling, and canister lifecycle updated to enforce layering while keeping behavior the same.

## [0.3.15] - 2025-11-29
- app and subnet_directory() now are on all canisters, use pagination and a proper DTO return type

## [0.3.0] - 2025-11-15
- Added paginated `canic_subnet_canister_children` via `CanisterChildrenOps::page` and `CanisterChildrenPage` DTO, mirroring CycleTracker paging.
- Introduced global log retention config (`max_entries` ring cap + optional `max_age_secs`) with second-level timestamps and enforced trimming.
- Documented the new log config block and refreshed README layout to match current modules.
- Added notes about the cross-filesystem compilation error for the LLM
- fixed logging so that the message is stored correctly, and made the log! macro more ergonomic and include topic
- moved all the mimic utils into canic-utils so they can be used independently
- added FromStr for Account
- added crate_name to the logs, plus filtering on the front end
- Scaling now uses plan_create_worker so there aren't two parallel paths for checking if a worker can be spawned
- lots of work going through the codebase and moving state and memory into model

## [0.2.x] - 2025-11-10 - PRIME Subnet and Topology Foundations

- `0.2.0` introduced prime-subnet topology foundations, including `SubnetRole`, `Env` identity context, and synchronized state+directory snapshots.
- `0.2.1` shipped early stabilization fixes after the initial topology rollout.
- `0.2.2` removed legacy delegation flow and added `ops::signature` for canister-signature creation/verification.
- `0.2.3` moved app/subnet directory projections to `SubnetCanisterRegistry` and included directory state in canister init payloads.
- `0.2.6` continued layer cleanup by splitting memory/ops responsibilities and moving reserve config to per-subnet settings.
- `0.2.7` moved `xxhash` utilities into `canic` for shared sharding usage.
- `0.2.9` strengthened recursive config validation, including invalid subnet-directory detection.
- `0.2.10` switched sharding structures to string-based IDs and standardized scaling placement on HRW.
- `0.2.17` removed the `icrc-ledger-types` dependency in favor of a local implementation.
- `0.2.21` fixed nested canister-role validation so invalid deep config is detected correctly.
- `0.2.24` added `cfg(test)`-gated PocketIC helper support under `test/`.

See detailed breakdown:
[docs/changelog/0.2.md](docs/changelog/0.2.md)

---

## [0.1.x] - 2025-10-08 - Initial Publish and Early Runtime Foundations

- `0.1.0` published `canic` to crates.io after the final rename from `icu`.
- `0.1.3` refreshed documentation, including a README rewrite and cleanup of outdated docs.
- `0.1.4` added delegation sync helpers and a more ergonomic `debug!` logging macro.
- `0.1.7` added subnet PID capture support with `dfx 0.30.2` for root subnet context tracking.

See detailed breakdown:
[docs/changelog/0.1.md](docs/changelog/0.1.md)
