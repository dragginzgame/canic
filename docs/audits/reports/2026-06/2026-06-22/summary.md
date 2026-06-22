# Audit Summary - 2026-06-22

## Run Contexts

| Report | Type | Scope | Status |
| ---- | ---- | ---- | ---- |
| `access-purity.md` | Recurring access-boundary audit | `crates/canic-core/src/access/**`, endpoint macro access lowering | PASS with watchpoints |
| `audience-target-binding.md` | Recurring audience/target invariant audit | delegated-token, active root proof install, root proof provisioning, role-attestation, capability target binding | PASS |
| `canic-host-format-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/format/` | PASS |
| `canic-host-icp-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/icp/` | PASS |
| `canic-host-icp-config-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/icp_config/` | PASS |
| `canic-host-install-root-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/install_root/` | PASS |
| `canic-host-installed-deployment-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/installed_deployment/` | PASS |
| `canic-host-policy-gate-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/policy_gate/` | PASS |
| `canic-host-registry-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/registry/` | PASS |
| `canic-host-release-set-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/release_set/` | PASS |
| `canic-host-replica-query-module-surface-hardening.md` | Modular MSH | `crates/canic-host/src/replica_query/` | PASS |
| `layer-violations.md` | Recurring layer-boundary audit | `canic-core`, `canic-macros`, `canic` macro/facade runtime surfaces | PASS with drift risk after remediation |

## Risk Index Summary

| Report | Risk | Notes |
| ---- | ----: | ---- |
| `access-purity.md` | 2 / 10 | Access remains a thin endpoint boundary; definition now explicitly guards root proof provisioning and root issuer policy mutation out of access. |
| `audience-target-binding.md` | 3 / 10 | No binding break found; definition now covers active proof issuer/root binding and batch install metadata matching. |
| `canic-host-format-module-surface-hardening.md` | 1 / 10 | Shared host/operator formatting is retained with owner; no safe cleanup candidate was found. |
| `canic-host-icp-module-surface-hardening.md` | 4 / 10 | ICP CLI adapter surface is retained with owner; no safe cleanup candidate was found, and snapshot text-id parsing is deferred until all production snapshot create flows rely only on JSON receipts. |
| `canic-host-icp-config-module-surface-hardening.md` | 2 / 10 | ICP project config inspection is retained with owner; no safe cleanup candidate was found. |
| `canic-host-install-root-module-surface-hardening.md` | 3 / 10 | Install/root authority remains high consequence, but facade surface risk was reduced by moving test-only imports plus option, identity, clock, and capability helper ownership out of the module root. |
| `canic-host-installed-deployment-module-surface-hardening.md` | 3 / 10 | Installed-deployment resolution is a live public host facade over persisted install state and root registry observation; no safe cleanup candidate was found. |
| `canic-host-policy-gate-module-surface-hardening.md` | 3 / 10 | Policy gate remains a passive public V1 schema/decision facade; test-only private rule import pressure was moved out of the module root. |
| `canic-host-registry-module-surface-hardening.md` | 4 / 10 | Registry parser surface is compact and retained with owner; malformed-row strictness is deferred because backup/snapshot/deployment-truth compatibility needs proof. |
| `canic-host-release-set-module-surface-hardening.md` | 3 / 10 | Release-set remains high consequence, but facade surface pressure was reduced by moving test-only imports into tests, privatizing `stage`, and removing an unused private clock-helper parameter. |
| `canic-host-replica-query-module-surface-hardening.md` | 3 / 10 | Direct local replica query facade remains live for CLI and host local-network flows; one transport-only endpoint helper was narrowed to private. |
| `layer-violations.md` | 3 / 10 | Fixed a production `api::blob_storage` stable-record construction leak by moving billing config record/view/DTO mapping into ops; remaining risk is blob-storage API facade pressure. |

## Method / Comparability Notes

- `access-purity.md` uses a refreshed recurring access-boundary definition and
  is partially comparable with the 2026-06-19 run because the method now
  includes explicit root proof / root issuer policy boundary checks.
- `audience-target-binding.md` uses a refreshed recurring invariant definition
  and is partially comparable with the 2026-06-19 run because the method now
  includes explicit active proof install issuer/root binding and batch install
  proof/pending metadata matching checks.
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
- `layer-violations.md` uses the refreshed recurring layer-boundary definition
  and is partially comparable with the 2026-06-17 run because the method now
  includes explicit guard-parity checks for access records, record re-exports,
  view naming, and root issuer policy API drift.

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
