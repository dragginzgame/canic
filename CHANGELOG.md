# Changelog

All notable, and occasionally less notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [0.19.x] - 2026-03-30 - Library Lane Cleanup and Reference Install

- `0.19.1` finishes the library/reference split by moving template/store and sharding implementation lanes out of the default `canic` path, compiling `canic.toml` into the canister instead of parsing TOML at runtime, keeping `canic-utils` off the public facade, standardizing debug-only Candid export on `canic::cdk::export_candid_debug!()`, and hardening the staged `wasm_store`/`root` reference install flow behind `make demo-install` once `dfx` is already running.
- `0.19.0` starts the `0.19` line with a clean post-`0.18` audit baseline, recording the release wasm footprint (`minimal`/`app`/`scale`/`shard` at `2489858` bytes, `root` at `3730865`, `wasm_store` at `2823075`) and the refreshed capability-surface baseline before the next reduction pass begins.

```bash
# terminal 1
scripts/app/dfx_start.sh

# terminal 2
make demo-install
```

See detailed breakdown:
[docs/changelog/0.19.md](docs/changelog/0.19.md)

---

## [0.18.x] - 2026-03-27 - Template Store and Chunked Install Cutover

- `0.18.7` stops stale non-root canisters from spamming root with failed attestation-key refreshes after they fall out of the subnet registry, fixes cached `.did` invalidation so per-canister release builds stop retriggering whole-workspace rebuilds during `dfx build --all`, and compacts shared capability-proof wire payloads behind `CapabilityProofBlob` so non-root interfaces carry less proof-shape fan-out.
- `0.18.6` removes the remaining env-driven eager-init build split, keeps release builds single-pass while caching `.did` files independently of release wasm, stages the full config-defined release set into `root` before local smoke/bootstrap flows continue, adds root-owned bootstrap debug visibility with human-readable wasm sizes, and fixes the local smoke path so it calls the `test` canister that `root` actually created and registered.
- `0.18.5` keeps `ICRC-21` behind role-scoped compile-time gating, trims the shared generated surface by making `canic_app_state` and `canic_subnet_state` root-only, removes embedded release payloads from both `root` and `wasm_store`, and hardens bundle builds so profile-mismatched `.dfx/local` artifacts are no longer silently reused when the AA pipeline stages releases through `root`.
- `0.18.4` gives `root` a single controller-facing `canic_wasm_store_overview` read endpoint built entirely from root-owned state so operators can inspect all tracked wasm stores without direct store queries, consolidates the older split wasm-store status queries into that overview surface, and tightens the local release flow so `make patch` / `make minor` skip PocketIC-heavy tests, rely on an already-running `dfx`, and stop failing plain Cargo/clippy builds when `.dfx` release artifacts have not been generated yet.
- `0.18.3` makes `root` bootstrap its first `wasm_store` automatically again, updates the `canic-memory` eager-init contract so `canic::start!` consumes it seamlessly without extra user wiring, and hardens local `dfx` test flows by starting clean replicas and removing the now-stale manual bootstrap staging step from `make test` and `make patch`.
- `0.18.2` makes the `root` and `wasm_store` release flow fully config-driven from `canic.toml`, moves live wasm-store inventory into runtime subnet state so `root` can create and promote stores dynamically instead of relying on static bindings, and standardizes debug-only Candid export behind `canic::cdk::export_candid!()`.
- `0.18.1` completes the staged `wasm_store` bootstrap follow-up by fixing local `dfx` installs to stage the bootstrap payload before root becomes ready, restoring local compact-config compatibility, and trimming release-only exports so the raw `root` artifact drops further to `3554964` bytes.
- `0.18.0` starts the wasm-store cutover by moving ordinary child payload ownership out of `root`, requiring store-backed chunked install for every role except bootstrap `wasm_store`, reducing the raw release `root` artifact to `4151294` bytes (`delta -10366542` vs `0.17.3`), simplifying setup with one implicit per-subnet `wasm_store` on a fixed 40 MB / 4 MB IC preset, and refreshing the workspace baseline to Rust `1.94.1` with `ctor 0.8` and `sha2 0.11`.

```toml
[subnets.prime]
auto_create = ["app", "user_hub", "scale_hub", "shard_hub"]

[subnets.prime.canisters.app]
kind = "singleton"
```

See detailed breakdown:
[docs/changelog/0.18.md](docs/changelog/0.18.md)

---

## [0.17.x] - 2026-03-25 - Wasm Audit and Endpoint Surface Reduction

- `0.17.3` continues the wasm audit line by tightening `canic_metrics` and `canic_log`, completing the `0.17` root decomposition handoff to `0.18`, and reducing the `minimal` raw release artifact to `2433930` bytes (`delta -26446` vs `0.17.2`).
- `0.17.2` continues the wasm audit line by slimming shared runtime, metrics, and observability paths, bringing the `minimal` raw release artifact down to `2460376` bytes (`delta -100624` vs `0.17.1`) while keeping the intended operator-facing feature set intact.
- `0.17.1` cuts the shared wasm floor again by separating root-only capability verification from the non-root cycles path and by removing the old Canic standards canister-status endpoint, bringing the `minimal` raw release artifact down to `2561000` bytes while keeping the intended runtime feature set intact.
- `0.17.0` starts the wasm audit line with a measured per-canister footprint baseline, renames the canonical baseline canister from `blank` to `minimal`, and trims optional scaling, sharding, delegated-auth, and `ICRC-21` endpoint exports behind compile-time config so disabled features stop inflating every build.

See detailed breakdown:
[docs/changelog/0.17.md](docs/changelog/0.17.md)

---

## [0.16.x] - 2026-03-16 - Delegation Proof Evolution

- `0.16.2` hardens delegated-auth token handling by rejecting malformed or unusable lifetimes at both issuance and verification, making the zero-skew policy explicit, restoring ops-owned proof boundaries, and closing the `0.16` auth/proof line with remaining root/template architecture work handed off to `0.17` and `0.18`.
- `0.16.1` hardens delegated-auth audience binding so verifier proof installs and delegated-session bootstrap reject out-of-scope audiences, while typed auth rollout metrics make prewarm/repair failures easier to track during the `0.16` auth refactor.
- `0.16.0` is reserved as a placeholder minor-line entry for delegation proof evolution follow-up work (deferred from `0.15` Phase 3), with implementation details tracked in the `0.16` design docs.

See detailed breakdown:
[docs/changelog/0.16.md](docs/changelog/0.16.md)

---

## [0.15.x] - 2026-03-12 - Unified Auth Identity Foundation

- `0.15.6` bumps `pocket-ic` to `13.0`, refreshes supporting IC/Rust dependencies, and advances the workspace to `0.15.6` so local and integration tooling stay aligned with the current dependency baseline.
- `0.15.5` fixes CI flakiness in delegation/role-attestation integration builds by making cfg-gated test-material compilation reliably rebuild when `CANIC_TEST_DELEGATION_MATERIAL` changes between runs.
- `0.15.4` completes Tier 1 delegation provisioning guarantees by requiring required verifier fanout success at issuance, adding root-side verifier-target validation and role-labeled provisioning metrics, and validating issuance -> verifier verify -> bootstrap -> authenticated guard success end to end; Phase 3 follow-ups are explicitly deferred to the `0.16` design track.
- `0.15.3` removes unused legacy compatibility shims/fallbacks and records a follow-up `layer-violations` rerun (`3/10`, no hard layer violations).
- `0.15.2` fixes shard token issuance regression by routing non-root delegation requests to root over RPC, so shard-initiated proof refresh works again while root-only authorization stays enforced.
- `0.15.1` finalizes 0.15 release governance docs by recording explicit security sign-off scope/residual risks, freezing the auth-semantic boundary for 0.15, and clarifying canonical release-boundary tracking.
- `0.15.0` hardens delegated-caller behavior into token-gated delegated-session semantics with strict subject binding, TTL clamp, replay/session-binding controls, and auth observability, while keeping raw-caller infrastructure predicates unchanged.

```rust
DelegationApi::set_delegated_session_subject(delegated_subject, bootstrap_token, Some(300))?;
```

See detailed breakdown:
[docs/changelog/0.15.md](docs/changelog/0.15.md)

---

## [0.14.x] - 2026-03-09 - Parent-Funded Cycles Control Plane

- `0.14.4` upgrades recurring architecture/auth audits with normalized risk scoring, structural hotspot tracing, early-warning/fan-in detection, and stronger layer-drift checks so risks are easier to spot before regressions ship.
- `0.14.3` standardizes delegated-token issuance naming on `issue`, adds `DelegationApi::issue_token` as the single app-facing issuance path, and removes legacy `mint` naming from delegation endpoints and metrics labels.
- `0.14.2` consolidates metrics queries under `canic_metrics` (`MetricsRequest`/`MetricsResponse`) and removes the per-metric `canic_metrics_*` endpoint variants.
- `0.14.1` removes `funding_policy` config fields and keeps `topup_policy` as the only cycles config surface, while restoring unbounded request evaluation so oversized requests fail on actual parent balance checks instead of being clamped by config.
- `0.14.0` makes subtree funding parent-only with replay-safe RPC execution, adds an app-level global funding kill switch, and ships parent-emitted cycles funding metrics (totals, per-child, and denial reasons).

```text
canic_metrics(record { kind = variant { RootCapability }; page = record { limit = 100; offset = 0 } })
```

See detailed breakdown:
[docs/changelog/0.14.md](docs/changelog/0.14.md)

---

## [0.13.x] - 2026-03-07 - Distributed Capability Invocation

- `0.13.8` hardens cycles top-up safety validation with stronger config tests, restructures design/audit documentation layout for maintainability, and adds the `0.14` parent-funded cycles control-plane design/status documentation.
- `0.13.7` completed lifecycle boundary follow-up coverage (non-root repeated post-upgrade readiness plus non-root post-upgrade failure-phase checks), tightened root capability metric internals, refreshed replay/audit run guidance for constrained local environments, and fixed intent concurrency capacity checks so `max_in_flight` counts only pending reservations (preventing committed claim intents from permanently blocking later claims for the same caller-scoped key).
- `0.13.6` expanded auth/replay/capability test coverage and aligned root replay integration tests with current duplicate handling, while making the shared root test harness recover cleanly after a failed test.
- `0.13.5` further reduced branching pressure by moving replay commit fully into ops, switching built-in access predicates to evaluator-based dispatch, and replacing monolithic root capability metric events with structured `event_type`/`outcome`/`proof_mode` metrics.
- `0.13.4` simplified proof, replay, and auth internals with pluggable verifiers, a dedicated replay guard path, faster duplicate rejection, and clearer delegated-auth error grouping.
- `0.13.3` finished the auth/control-plane extraction, standardized directory modules with `mod.rs`, and refreshed complexity/velocity audit baselines.
- `0.13.2` continued the module split and moved request/auth helpers behind cleaner facades, reducing coupling between high-traffic code paths.
- `0.13.1` split large RPC/auth workflow files into smaller modules, making the control plane easier to read and change without altering behavior.
- `0.13.0` introduced signed capability envelopes for cross-canister root calls, with built-in replay protection and capability hashing to prevent request reuse/tampering.

```text
same request_id + same payload -> ReplayDuplicateSame (rejected)
same request_id + different payload -> ReplayDuplicateConflict (rejected)
```

See detailed breakdown:
[docs/changelog/0.13.md](docs/changelog/0.13.md)

---

## [0.12.x] - 2026-03-07 - Root Role Attestation Framework

- `0.12.0` adds root-signed role attestations and an attested root dispatch path, so services can authorize callers by signed proof instead of full directory sync.

See detailed breakdown:
[docs/changelog/0.12.md](docs/changelog/0.12.md)

---

## [0.11.x] - 2026-03-07 - Capabilities Arc and Replay Hardening

- `0.11.1` hardens root capability replay/dispatch behavior and improves auth diagnostics to make failure triage easier.
- `0.11.0` starts the capability-focused auth line with stronger scope checks and safer account/numeric behavior.

See detailed breakdown:
[docs/changelog/0.11.md](docs/changelog/0.11.md)

---

## [0.10.x] - 2026-02-24 - Delegated Auth Tightening and Runtime Guardrails

- `0.10.5` switched HTTP outcall APIs to raw response bytes, tightened memory-bootstrap safety, and reduced default wasm artifact size.
- `0.10.2` fixed lifecycle ordering so memory bootstrap is guaranteed before env restoration and runtime stable-memory access.
- `0.10.1` added optional scope syntax to `authenticated(...)` while preserving delegated-token verification semantics.
- `0.10.0` moved authenticated endpoints to direct delegated-token verification with explicit root/shard/audience binding and removed relay-style auth envelopes.

```rust
let raw: HttpRequestResult = HttpApi::get(url).await?;
```

See detailed breakdown:
[docs/changelog/0.10.md](docs/changelog/0.10.md)

---

## [0.9.x] - 2026-01-19 - Delegated Auth and Access Hardening

- `0.9.26` exported `SubnetRegistryApi` at the stable public path.
- `0.9.25` expanded network/pool bootstrap logging for clearer operational diagnostics.
- `0.9.24` added root top-up balance checks and safer pool-import bootstrap ordering.
- `0.9.23` renamed canister kinds and sharding query terminology to the current contract.
- `0.9.20` fixed multi-argument delegated-token ingress decoding and removed legacy dev bypass behavior.
- `0.9.18` enforced compile-time validation rules for authenticated endpoint argument shapes.
- `0.9.17` moved local bypass handling into delegated verification so auth paths stay consistent.
- `0.9.16` added a local/dev short-circuit path for delegated auth under controlled conditions.
- `0.9.14` removed delegation rotation/admin/status surfaces as part of shard lifecycle cleanup.
- `0.9.13` added signer-initiated delegation request support through root.
- `0.9.12` completed auth delegation audit follow-up and strengthened view-boundary usage.
- `0.9.11` added delegated-auth rejection counters for better operational visibility.
- `0.9.10` standardized the delegated-auth guard surface as `auth::authenticated()`.
- `0.9.7` cleaned up IC call builders so argument encoding/injection is consistently fallible and explicit.
- `0.9.6` hardened lifecycle/config semantics and normalized app config naming.
- `0.9.5` aligned access predicates into explicit families (`app`, `auth`, `env`) with a cleaner DSL surface.
- `0.9.4` made app init mode config-driven and aligned sync access behavior.
- `0.9.3` made app-state gating default-on for endpoints unless explicitly overridden.
- `0.9.2` moved endpoint authorization to a single `requires(...)` expression model with composable predicates.
- `0.9.1` ran consolidation audits to tighten layering boundaries and consistency rules.
- `0.9.0` established the delegated-auth baseline and runtime architecture for proof-driven endpoint authorization.

See detailed breakdown:
[docs/changelog/0.9.md](docs/changelog/0.9.md)

---

## [0.8.x] - 2026-01-13 - Intent System and API Consolidation

- `0.8.6` raised intent pending-entry storage bounds to safely handle large keys.
- `0.8.5` introduced the stable-memory intent system with reserve/commit/abort flows and contention coverage.
- `0.8.4` cleaned up docs and reduced redundant snapshot/view conversions.
- `0.8.3` exposed protocol surfaces through the public API layer.
- `0.8.1` exported `HttpApi` under `api::ic` alongside call utilities.
- `0.8.0` consolidated the public API surface and hardened error-model consistency.

See detailed breakdown:
[docs/changelog/0.8.md](docs/changelog/0.8.md)

---

## [0.7.x] - 2025-12-30 - Architecture Consolidation and Boundary Cleanup

- `0.7.28` moved macro entrypoints into the `canic` facade crate.
- `0.7.26` cleaned up stale docs and layering inconsistencies.
- `0.7.23` added a fail-fast root bootstrap guard for uninitialized embedded wasm registries.
- `0.7.22` unified internal topology state on authoritative `CanisterRecord`.
- `0.7.21` expanded IC call workflow helpers with argument-aware variants.
- `0.7.15` standardized endpoint-wrapper error conversion into downstream error types.
- `0.7.14` removed DTO usage from ops via ops-local command types.
- `0.7.13` standardized infra error bubbling and structure under ops.
- `0.7.12` switched signature internals to the `ic-certified-map` hash tree path.
- `0.7.11` moved sharding placement to a pure deterministic policy model.
- `0.7.10` moved API instrumentation ownership into `access`.
- `0.7.9` mirrored authentication helpers into `api::access`.
- `0.7.8` aligned topology policy modules under `policy::topology`.
- `0.7.7` split `api/topology` and filled missing surface functions.
- `0.7.6` resynced certified data from the signature map during post-upgrade.
- `0.7.4` expanded `canic-cdk` with additional ckToken support.
- `0.7.3` added a public `api::ic::call` wrapper routed through ops instrumentation.
- `0.7.2` tightened workflow/policy naming and topology lookup contracts.
- `0.7.1` tightened ops-layer boundaries through an explicit audit pass.
- `0.7.0` consolidated architecture/runtime discipline and clarified boundary ownership.

See detailed breakdown:
[docs/changelog/0.7.md](docs/changelog/0.7.md)

---

## [0.6.x] - 2025-12-18 - Runtime Hardening and Pool Evolution

- `0.6.20` added stricter canister-kind validation, typed endpoint identity, and registry/pool hardening.
- `0.6.19` switched endpoint perf accounting to an exclusive scoped stack model.
- `0.6.18` added log entry byte caps and fixed several lifecycle/http/sharding edge cases.
- `0.6.17` added bootstrap-time pool import support (`pool.import.local` / `pool.import.ic`).
- `0.6.16` hardened pool import/recycle/install failure handling and state cascade behavior.
- `0.6.13` made env/config access fallible with clearer lifecycle failure behavior and stronger directory/env semantics.
- `0.6.12` enforced build-time `DFX_NETWORK` validation across scripts and Cargo workflows.
- `0.6.10` improved ICRC-21 error propagation for idiomatic `?` handling.
- `0.6.9` renamed reserve configuration to pool and introduced status-aware import modes.
- `0.6.8` removed mutex-based randomness plumbing and introduced configurable reseed behavior.
- `0.6.7` replaced macro panics with compile errors for unsupported endpoint parameter patterns.
- `0.6.6` restored build-network access and aligned access-policy/runtime wrappers.
- `0.6.0` introduced a major endpoint-protection/runtime refactor and split metrics endpoints.

See detailed breakdown:
[docs/changelog/0.6.md](docs/changelog/0.6.md)

---

## [0.5.x] - 2025-12-05 - Metrics, Lifecycle, and Memory Foundations

- `0.5.22` aligned CI to build deterministic wasm artifacts before lint/test gates.
- `0.5.21` consolidated perf/type paths and improved timer metric labeling.
- `0.5.17` added ops-level HTTP metrics support.
- `0.5.16` fixed CMC top-up reply handling so failed top-ups are not reported as success.
- `0.5.15` simplified reserve-pool lifecycle orchestration.
- `0.5.14` split metrics into ICC and system categories.
- `0.5.13` centralized canister call metric recording through wrapped cross-canister construction.
- `0.5.12` made topology sync branch-targeted with safer fallback behavior.
- `0.5.10` added a wrapper around `performance_counter`.
- `0.5.8` reduced cascade complexity toward near-linear sync behavior.
- `0.5.7` improved create-flow bootstrap diagnostics with caller/parent context logs.
- `0.5.6` unified background timer startup through a single role-aware service entrypoint.
- `0.5.4` hardened reserve import/recycle sequencing and cascade safety.
- `0.5.2` split stable-memory infrastructure into `canic-memory` and re-exported runtime/macro support.
- `0.5.1` moved shared wrappers into `canic-core::types` and slimmed public type exports.
- `0.5.0` introduced the `canic-cdk` facade and stabilized a curated IC integration surface.

See detailed breakdown:
[docs/changelog/0.5.md](docs/changelog/0.5.md)

---

## [0.4.x] - 2025-12-01 - Registry and Signature Stability Passes

- `0.4.12` unified signature verification entrypoints and fixed root child-directory rebuild behavior.
- `0.4.8` tightened memory visibility and removed unused internals.
- `0.4.7` fixed signature verification panic behavior for short principal forms.
- `0.4.6` aligned directory rebuild behavior and added end-to-end consistency coverage.
- `0.4.1` fixed canister registration ordering to avoid phantom entries on install failure.
- `0.4.0` formalized the `endpoints -> ops -> model` layering contract.

See detailed breakdown:
[docs/changelog/0.4.md](docs/changelog/0.4.md)

---

## [0.3.x] - 2025-11-15 - Pagination and Logging Foundations

- `0.3.15` expanded app/subnet directory access across canisters with paginated DTO responses.
- `0.3.0` added paginated subnet-children APIs and introduced configurable bounded log retention.

See detailed breakdown:
[docs/changelog/0.3.md](docs/changelog/0.3.md)

---
## [0.2.x] - 2025-11-10 - PRIME Subnet and Topology Foundations

- `0.2.24` added `cfg(test)`-gated PocketIC helper support under `test/`.
- `0.2.21` fixed nested canister-role validation so invalid deep config is detected correctly.
- `0.2.17` removed the `icrc-ledger-types` dependency in favor of a local implementation.
- `0.2.10` switched sharding structures to string-based IDs and standardized scaling placement on HRW.
- `0.2.9` strengthened recursive config validation, including invalid subnet-directory detection.
- `0.2.7` moved `xxhash` utilities into `canic` for shared sharding usage.
- `0.2.6` continued layer cleanup by splitting memory/ops responsibilities and moving reserve config to per-subnet settings.
- `0.2.3` moved app/subnet directory projections to `SubnetCanisterRegistry` and included directory state in canister init payloads.
- `0.2.2` removed legacy delegation flow and added `ops::signature` for canister-signature creation/verification.
- `0.2.1` shipped early stabilization fixes after the initial topology rollout.
- `0.2.0` introduced prime-subnet topology foundations, including `SubnetRole`, `Env` identity context, and synchronized state+directory snapshots.

See detailed breakdown:
[docs/changelog/0.2.md](docs/changelog/0.2.md)

---

## [0.1.x] - 2025-10-08 - Initial Publish and Early Runtime Foundations

- `0.1.7` added subnet PID capture support with `dfx 0.30.2` for root subnet context tracking.
- `0.1.4` added delegation sync helpers and a more ergonomic `debug!` logging macro.
- `0.1.3` refreshed documentation, including a README rewrite and cleanup of outdated docs.
- `0.1.0` published `canic` to crates.io after the final rename from `icu`.

See detailed breakdown:
[docs/changelog/0.1.md](docs/changelog/0.1.md)
