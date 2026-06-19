# Workflow Purity Audit - 2026-06-19

## Report Preamble

- Scope: `crates/canic-core/src/workflow/**`, compared with `domain/policy`,
  `dto`, `model`, `ops`, `replay_policy`, `storage`, `access`, and endpoint
  macros
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-06/workflow-purity.md`
- Code snapshot identifier: `ef55e53c` with dirty worktree
- Method tag/version: `workflow-purity-v3`
- Comparability status: partially comparable. The core workflow-purity
  invariant is unchanged, while the audit definition now covers root proof
  provisioning, active/pending proof records, model/DTO boundary comparison,
  and direct-query root proof retrieval invariants.
- Auditor: `codex`

## Audit Definition Update

Before running the audit, `docs/audits/recurring/system/workflow-purity.md` was
reviewed and updated from `workflow-purity-v2` to `workflow-purity-v3`.

Method changes:

- added root proof provisioning and delegated-auth proof lifecycle coverage;
- added `dto/` and `model/` to the boundary comparison scope;
- extended storage scans to active root-proof policy/proof record names;
- extended platform/auth/replay scans for root proof batch install, direct query
  proof retrieval constraints, and issuer-local active proof install outcomes;
- documented that delegated-token prepare may validate public self-prepare
  policy through `domain::policy`, but must not verify inbound endpoint tokens.

## Executive Summary

Verdict: **Pass with watchpoints**.

The hard findings from the previous run are materially improved:

- workflow no longer imports or carries `IcpRefillRecord` or `CanisterRecord`
  in production paths;
- pool replay response codecs delegate to `ops::replay`;
- ICP refill and delegated-auth replay response codecs delegate to ops;
- root proof provisioning install stays orchestration-only: workflow requires
  root, asks `AuthOps` to validate pending metadata, broadcasts through
  `CallOps`, and records per-issuer outcomes.

One cleanup was made during the audit: the canonical root capability
proof-binding hash was moved from `workflow::rpc::capability::hash` to
`ops::rpc::capability`, because it owns Candid wire encoding and domain
separation. A follow-up cleanup removed stale delegated-token and
role-attestation variants from `RpcWorkflowError`; those variants were no
longer constructed after auth provisioning moved to the current delegated-auth
workflow.

## Findings

No hard workflow-purity violations remain in the audited scope.

### Watchpoint - Workflow Replay/Hash Helper Pressure

Workflow still contains several replay payload hash helpers and replay decision
mapping helpers in:

- `crates/canic-core/src/workflow/runtime/auth/prepare/mod.rs`
- `crates/canic-core/src/workflow/pool/create_empty.rs`
- `crates/canic-core/src/workflow/ic/icp_refill/replay.rs`
- `crates/canic-core/src/workflow/rpc/request/handler/nonroot_cycles.rs`
- `crates/canic-core/src/workflow/rpc/request/handler/capability.rs`

These helpers are currently acceptable because they build operation payload
hashes and sequence ops-owned replay APIs; persistence encoding/decoding routes
through `ops::replay`. They should remain watchpoints because replay payload
hash semantics are protocol-visible and can drift into lower-layer ownership.

### Watchpoint - Large Workflow Orchestration Files

Largest production workflow files in this run:

- `workflow/runtime/auth/prepare/mod.rs`: 718 lines
- `workflow/rpc/request/handler/nonroot_cycles.rs`: 717 lines
- `workflow/canister_lifecycle/mod.rs`: 657 lines
- `workflow/rpc/request/handler/replay.rs`: 504 lines
- `workflow/pool/create_empty.rs`: 476 lines

These are no longer single-file ownership failures, but they remain pressure
areas for future audit runs.

## Code Changes Made

- Added `crates/canic-core/src/ops/rpc/capability.rs` with the canonical root
  capability hash implementation.
- Removed `crates/canic-core/src/workflow/rpc/capability/hash.rs`.
- Updated `workflow::rpc::capability::root_capability_hash` to delegate to the
  ops-owned helper.
- Removed unused delegated-token and role-attestation variants from
  `workflow::rpc::RpcWorkflowError`.
- Updated `docs/audits/recurring/system/workflow-purity.md` to
  `workflow-purity-v3`.

## Checklist Results

| Check | Result | Notes |
| --- | --- | --- |
| Storage record / stable access | PASS | No production `IcpRefillRecord`, `CanisterRecord`, active proof record, issuer policy record, pending proof batch record, or direct stable storage hit found. `ReplayReceipt*` hits are ops replay orchestration types. |
| Serialization / transport parsing | PASS with watchpoints | Workflow replay encode/decode wrappers delegate to `ops::replay`. Capability proof-binding Candid encoding was moved to `ops::rpc::capability`. Remaining Candid bounds are call/install adapters. |
| Conversion ownership | PASS with watchpoints | Workflow calls ops-owned mappers and constructs final response DTOs at orchestration boundaries. No record/DTO mapper ownership leak found. |
| Platform calls | PASS | Platform effects route through `MgmtOps`, `RequestOps`, `LedgerOps`, `CallOps`, or workflow call adapters. Root proof install broadcasts through `CallOps`; workflow does not retrieve or assemble root proofs. |
| Auth semantics | PASS with watchpoints | No delegated-token endpoint verifier ownership found. Delegated-token prepare uses domain policy for public self-prepare validation and `AuthOps` for proof preparation. Root proof install uses `EnvOps::require_root` and `AuthOps` preflight. |
| Policy / persistence policy ownership | PASS | Cost classes and policy decisions come from `replay_policy`, `domain::policy`, or ops. No workflow-owned durable policy ledger found. |
| Replay / cost / intent boundary | PASS with watchpoints | Replay, cost guard, and intent state stay ops-owned. Workflow sequences reserve/abort/recover/commit calls and maps typed outcomes. |
| Recovery / idempotence surface | PASS | Pool import/recycle and ICP refill still stop duplicate/destructive retries through ops-backed state markers. Root proof install reports per-issuer outcomes and marks installed only after successful issuer install. |
| Metrics / error mapping | PASS with watchpoints | Metrics use fixed helpers and typed outcomes. No branch on formatted error strings was found in sampled hotspots. |
| Module pressure | PASS with watchpoints | Large workflow files remain, but the previous `pool/mod.rs` hub was split and no large-file pressure currently hides a hard ownership violation. |

## Recent-Change Coverage Notes

- Root proof provisioning workflow was covered explicitly. The install path does
  not call `data_certificate()`, does not assemble `signature_cbor`, and does
  not attempt root proof retrieval from workflow.
- The direct root query invariant remains outside workflow and is enforced by
  root proof get endpoints/ops, not by issuer wrappers.
- Delegated-token prepare remains issuer-local after active proof installation
  and uses replay receipts before proof preparation.
- The previous ICP refill record-carrier finding is resolved by current
  operation/view types and store ops.
- The previous pool direct `CanisterRecord` finding is resolved by current
  `PoolRegistrationMetadata`/ops surfaces.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg` storage-record scan from `workflow-purity-v3` | PASS | No forbidden production storage record/stable access hit. |
| `rg` serialization scan from `workflow-purity-v3` | PASS | Workflow-owned capability Candid hash removed; replay wrappers delegate to ops. |
| `rg` conversion scan from `workflow-purity-v3` | PASS | Mapper calls route through ops/domain mapper modules. |
| `rg` platform-call scan from `workflow-purity-v3` | PASS | Platform effects route through ops/call wrappers. |
| `rg` auth-semantics scan from `workflow-purity-v3` | PASS | No endpoint token verifier ownership found. |
| `rg` replay/cost/intent scan from `workflow-purity-v3` | PASS | Sequencing through ops confirmed. |
| `find crates/canic-core/src/workflow -type f -name '*.rs' ! -name tests.rs -exec wc -l {} +` | PASS with watchpoints | Large auth/nonroot-cycle/canister-lifecycle files remain. |
| `cargo fmt --all -- --check` | PASS | Formatting check passed. |
| `cargo test --locked -p canic-core workflow::rpc --lib -- --nocapture` | PASS | 49 tests passed. |
| `cargo test --locked -p canic-core workflow::rpc::capability --lib -- --nocapture` | PASS | 15 tests passed. |
| `cargo test --locked -p canic-core workflow::runtime::auth --lib -- --nocapture` | PASS | 20 tests passed. |
| `cargo test --locked -p canic-core workflow::pool --lib -- --nocapture` | PASS | 10 tests passed. |
| `cargo test --locked -p canic-core workflow::ic::icp_refill --lib -- --nocapture` | PASS | 41 tests passed. |

## Follow-up Actions

| Owner Boundary | Action | Target Run |
| --- | --- | --- |
| `ops::replay` / workflow replay helpers | Keep replay response codecs in ops; consider moving protocol-visible replay payload hash construction lower if another workflow file starts duplicating the pattern. | next workflow-purity rerun |
| `workflow::runtime::auth::prepare` | Watch file pressure around delegated-token prepare and role-attestation replay mapping. | next workflow-purity rerun |
| `workflow::rpc::request::handler::nonroot_cycles` | Watch large-file pressure and replay/cost/authorization fan-in. | next workflow-purity rerun |
| `workflow::canister_lifecycle` | Watch large-file pressure around upgrade preflight, metrics, and propagation orchestration. | next workflow-purity rerun |

## Final Verdict

Pass with watchpoints.

Workflow is back to orchestration ownership for this audit. The prior hard
record and Candid-hash leaks are resolved, and the current residual risk is
pressure from large orchestration files and protocol-visible replay hash helper
placement.
