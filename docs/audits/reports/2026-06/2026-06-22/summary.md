# Audit Summary - 2026-06-22

## Run Contexts

| Report | Type | Scope | Status |
| ---- | ---- | ---- | ---- |
| `access-purity.md` | Recurring access-boundary audit | `crates/canic-core/src/access/**`, endpoint macro access lowering | PASS with watchpoints |
| `auth-abstraction-equivalence.md` | Recurring auth abstraction invariant audit | macro-generated auth, access dispatch, delegated-token verifier parity, root/nonroot/blob endpoint guard bundles | PASS with watchpoints |
| `audience-target-binding.md` | Recurring audience/target invariant audit | delegated-token, active root proof install, root proof provisioning, role-attestation, capability target binding | PASS |
| `bootstrap-lifecycle-symmetry.md` | Recurring lifecycle-boundary audit | `start!` macros, lifecycle adapters, root control-plane lifecycle, lifecycle metrics, embedded root wasm-store registration | PASS with watchpoints |
| `canic-host-build-profile-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/build_profile.rs` | PASS |
| `canic-host-format-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/format/` | PASS |
| `canic-host-icp-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/icp/` | PASS |
| `canic-host-icp-config-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/icp_config/` | PASS |
| `canic-host-install-root-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/install_root/` | PASS |
| `canic-host-installed-deployment-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/installed_deployment/` | PASS |
| `canic-host-policy-gate-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/policy_gate/` | PASS |
| `canic-host-registry-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/registry/` | PASS |
| `canic-host-release-set-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/release_set/` | PASS |
| `canic-host-replica-query-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/replica_query/` | PASS |
| `canic-host-response-parse-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/response_parse/` | PASS |
| `canic-host-subnet-catalog-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/subnet_catalog/` | PASS |
| `canic-host-subnet-registry-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/subnet_registry/` | PASS |
| `canic-host-tests-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/tests/` | PASS |
| `canic-host-workspace-discovery-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/workspace_discovery/` | PASS |
| `layer-violations.md` | Recurring layer-boundary audit | `canic-core`, `canic-macros`, `canic` macro/facade runtime surfaces | PASS with drift risk after remediation |
| `ops-purity.md` | Recurring ops-boundary audit | `canic-core` ops, root proof provisioning split, blob-storage billing ops split, Cashier wrappers, metrics | PASS with watchpoints |
| `workflow-purity.md` | Recurring workflow-boundary audit | `canic-core` workflow orchestration, replay/cost/intent, root proof provisioning, blob-storage billing boundary | PASS with watchpoints |

## Risk Index Summary

| Report | Risk | Notes |
| ---- | ----: | ---- |
| `access-purity.md` | 2 / 10 | Access remains a thin endpoint boundary; definition now explicitly guards root proof provisioning and root issuer policy mutation out of access. |
| `auth-abstraction-equivalence.md` | 3 / 10 | Generated and helper auth paths still converge on the canonical verifier; root/nonroot/blob endpoint bundles preserve their authority boundaries, with passive auth DTO/proof fan-in remaining the main watchpoint. |
| `audience-target-binding.md` | 3 / 10 | No binding break found; definition now covers active proof issuer/root binding and batch install metadata matching. |
| `bootstrap-lifecycle-symmetry.md` | 3 / 10 | Lifecycle hooks remain synchronous restore/schedule adapters; root lifecycle adapter pressure from metrics and embedded wasm-store source registration is bounded but worth watching. |
| `canic-host-build-profile-module-surface-hardening.md` | 2 / 10 | Public canister build profile enum is retained for CLI build/install/deploy parsing and host artifact/profile selection; no stale surface found. |
| `canic-host-format-module-surface-hardening.md` | 1 / 10 | Shared host/operator formatting is retained with owner; no safe cleanup candidate was found. |
| `canic-host-icp-module-surface-hardening.md` | 4 / 10 | ICP CLI adapter surface is retained with owner; no safe cleanup candidate was found, and snapshot text-id parsing is deferred until all production snapshot create flows rely only on JSON receipts. |
| `canic-host-icp-config-module-surface-hardening.md` | 2 / 10 | ICP project config inspection is retained with owner; no safe cleanup candidate was found. |
| `canic-host-install-root-module-surface-hardening.md` | 3 / 10 | Install/root authority remains high consequence, but facade surface risk was reduced by moving test-only imports plus option, identity, clock, and capability helper ownership out of the module root. |
| `canic-host-installed-deployment-module-surface-hardening.md` | 3 / 10 | Installed-deployment resolution is a live public host facade over persisted install state and root registry observation; no safe cleanup candidate was found. |
| `canic-host-policy-gate-module-surface-hardening.md` | 3 / 10 | Policy gate remains a passive public V1 schema/decision facade; test-only private rule import pressure was moved out of the module root. |
| `canic-host-registry-module-surface-hardening.md` | 4 / 10 | Registry parser surface is compact and retained with owner; malformed-row strictness is deferred because backup/snapshot/deployment-truth compatibility needs proof. |
| `canic-host-release-set-module-surface-hardening.md` | 3 / 10 | Release-set remains high consequence, but facade surface pressure was reduced by moving test-only imports into tests, privatizing `stage`, and removing an unused private clock-helper parameter. |
| `canic-host-replica-query-module-surface-hardening.md` | 3 / 10 | Direct local replica query facade remains live for CLI and host local-network flows; one transport-only endpoint helper was narrowed to private. |
| `canic-host-response-parse-module-surface-hardening.md` | 2 / 10 | Shared host/CLI response parser facade remains compact; four host-only helpers were narrowed from public API to crate-visible API. |
| `canic-host-subnet-catalog-module-surface-hardening.md` | 0 / 10 | Empty untracked local directory was removed; no tracked Rust module surface exists. |
| `canic-host-subnet-registry-module-surface-hardening.md` | 3 / 10 | Live subnet-registry query facade is retained; query-source provenance was narrowed to host-crate visibility while registry JSON remains public for CLI consumers. |
| `canic-host-tests-module-surface-hardening.md` | 1 / 10 | Crate-root test module is retained; it only guards local-only Candid artifact export and exposes no production surface. |
| `canic-host-workspace-discovery-module-surface-hardening.md` | 2 / 10 | Private workspace/ICP root discovery helpers are retained with owner; explicit `pub(super)` narrowing was rejected by clippy as redundant in a private module. |
| `layer-violations.md` | 3 / 10 | Fixed a production `api::blob_storage` stable-record construction leak by moving billing config record/view/DTO mapping into ops; remaining risk is blob-storage API facade pressure. |
| `ops-purity.md` | 3 / 10 | Ops remains bounded after the blob-storage billing split; residual risk is API facade pressure and keeping Cashier/funding ops as single-operation helpers. |
| `workflow-purity.md` | 3 / 10 | Workflow remains orchestration-only after the root proof and blob-storage billing work; residual risk is API facade pressure plus large replay-heavy workflow files. |

## Method / Comparability Notes

- `access-purity.md` uses a refreshed recurring access-boundary definition and
  is partially comparable with the 2026-06-19 run because the method now
  includes explicit root proof / root issuer policy boundary checks.
- `auth-abstraction-equivalence.md` uses a refreshed recurring invariant
  definition and is partially comparable with the 2026-06-19 run because the
  method now explicitly covers root/nonroot auth endpoint bundles,
  blob-storage endpoint guard bundles, and passive DTO/protocol fan-in scoring.
- `audience-target-binding.md` uses a refreshed recurring invariant definition
  and is partially comparable with the 2026-06-19 run because the method now
  includes explicit active proof install issuer/root binding and batch install
  proof/pending metadata matching checks.
- `bootstrap-lifecycle-symmetry.md` uses a refreshed recurring lifecycle
  definition and is partially comparable with the 2026-06-19 run because the
  method now explicitly covers synchronous lifecycle metrics, embedded root
  wasm-store bootstrap release-set registration/logging, and post-upgrade
  memory registry restore ordering.
- `canic-host-build-profile-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-host-format-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-host-icp-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-host-icp-config-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-host-install-root-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-host-installed-deployment-module-surface-hardening.md` uses `MSH-2.0`
  and is non-comparable because it is the first targeted MSH run for this
  module.
- `canic-host-policy-gate-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-host-registry-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-host-release-set-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-host-replica-query-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-host-response-parse-module-surface-hardening.md` uses `MSH-2.0` and
  is non-comparable because it is the first targeted MSH run for this module.
- `canic-host-subnet-catalog-module-surface-hardening.md` uses `MSH-2.0` and
  is non-comparable because it is the first targeted MSH run for this path.
- `canic-host-subnet-registry-module-surface-hardening.md` uses `MSH-2.0` and
  is non-comparable because it is the first targeted MSH run for this module.
- `canic-host-tests-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-host-workspace-discovery-module-surface-hardening.md` uses `MSH-2.0`
  and is non-comparable because it is the first targeted MSH run for this
  module.
- `layer-violations.md` uses the refreshed recurring layer-boundary definition
  and is partially comparable with the 2026-06-17 run because the method now
  includes explicit guard-parity checks for access records, record re-exports,
  view naming, and root issuer policy API drift.
- `ops-purity.md` uses a refreshed recurring ops-boundary definition and is
  partially comparable with the 2026-06-19 run because the method now includes
  explicit blob-storage billing config projection, Cashier wrapper/conversion,
  and transient funding guard checks.
- `workflow-purity.md` uses a refreshed recurring workflow-boundary definition
  and is partially comparable with the 2026-06-19 run because the method now
  includes explicit blob-storage billing boundary checks.

## Follow-up

- Continue modular MSH passes through the next host modules in tree order,
  escalating to Tier 2 where deployment-truth, recovery, or authority mutation
  surfaces appear.
- Keep blob-storage stable records out of API. Consider moving blob-storage
  billing sync/funding/status orchestration out of `api::blob_storage` in a
  follow-up boundary cleanup.
- Keep root proof provisioning and root issuer policy mutation out of access;
  access should remain endpoint guard evaluation and authenticated identity
  resolution only.
- Keep active proof install issuer/root binding and root proof batch install
  metadata matching in recurring audience-target coverage.
- Keep blob-storage Cashier wrappers, response conversion, and funding guards
  as bounded ops helpers; do not move billing sync/funding/status orchestration
  into ops.
- Keep blob-storage billing sync/funding/status out of workflow unless a
  deliberate workflow module is designed; if that happens, workflow should only
  sequence ops-owned steps.
- Keep replay codecs in ops and continue watching protocol-visible replay hash
  helpers in the large workflow files.
- Keep generated auth endpoint bundles equivalent to canonical verifier paths:
  root proof provisioning should stay controller/registered-caller scoped, and
  blob-storage gateway protocol checks must not become delegated-token product
  auth shortcuts.
- Keep root lifecycle limited to source registration, metrics, runtime
  restoration delegation, provenance logging, and timer scheduling; do not let
  it call storage-backed template admin helpers inline.
