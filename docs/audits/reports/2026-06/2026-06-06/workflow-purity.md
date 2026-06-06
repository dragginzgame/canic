# Workflow Purity Audit - 2026-06-06

## Report Preamble

- Scope: `crates/canic-core/src/workflow/**`, compared with `domain/policy`, `ops`, `storage`, `replay_policy`, `access`, and endpoint macros
- Compared baseline report path: `N/A` (first workflow-purity run on 2026-06-06)
- Historical baseline report path: `docs/audits/reports/2026-05/2026-05-16/workflow-purity.md`
- Code snapshot identifier: `36c1a589` with dirty worktree
- Method tag/version: `workflow-purity-v2`
- Comparability status: `non-comparable` (method expanded to cover replay receipts, cost guards, durable intents, management-effect recovery, and module pressure)
- Auditor: `codex`

## Least-Recently-Run Selection

Among current recurring audit definitions, `workflow-purity` was least recently
run. `rg -n "workflow-purity" docs/audits/reports docs/audits/recurring` found
only the retained `2026-05-16` run and its cross-reference from `ops-purity`.

## Audit Definition Update

Before running the audit, `docs/audits/recurring/system/workflow-purity.md` was
audited and upgraded from the older focused scan to `workflow-purity-v2`.

Method changes:

- added explicit replay/cost/intent boundary checks;
- added recovery/idempotence checks for pending-reset and management effects;
- tightened storage-record and stable-storage scans;
- separated allowed Candid call/install adapter bounds from forbidden persisted
  replay/capability codecs;
- added metrics/error-mapping and module-pressure sections;
- added workflow-purity to the system audit README for discoverability.

## Executive Summary

Verdict: **Fail with bounded findings**.

No direct CDK/infra platform bypass was found; workflow still performs platform
effects through ops surfaces such as `MgmtOps`, `RequestOps`, `LedgerOps`, and
`IcpRefillOps`.

The updated audit found three workflow-purity issues:

- production workflow carries persisted storage records as state-machine data;
- workflow owns several Candid codecs and hashes for replay/capability payloads;
- `workflow/pool/mod.rs` has become a high-pressure orchestration hub after the
  recent replay and pending-reset slices.

## Findings

### Medium - Persisted Records Cross Into Workflow State Machines

`workflow/ic/icp_refill/mod.rs` imports
`storage::stable::icp_refill::IcpRefillRecord` and passes it through the refill
state machine in functions such as `transfer_record`, `advance_record`,
`transfer_unless_window_stale`, and `notify_record`.

Evidence:

- `crates/canic-core/src/workflow/ic/icp_refill/mod.rs:32`
- `crates/canic-core/src/workflow/ic/icp_refill/mod.rs:292`
- `crates/canic-core/src/workflow/ic/icp_refill/mod.rs:322`
- `crates/canic-core/src/workflow/ic/icp_refill/mod.rs:355`

Why it matters: ops owns deterministic state access and record transitions.
Workflow should coordinate the refill sequence through ops-owned transition
types or command/view projections, not carry the stable record as its own state.

Recommended owner boundary: `ops::storage::icp_refill` should expose a
transition carrier or step API that lets workflow sequence transfer/notify
without importing the stable record type.

### Medium - Replay And Capability Codecs Are Workflow-Owned

`workflow/pool/mod.rs` encodes and decodes the persisted create-empty replay
response with `candid::encode_one` / `decode_one`. Capability proof and grant
helpers also encode/decode wire blobs and grant hashes inside
`workflow/rpc/capability`.

Evidence:

- `crates/canic-core/src/workflow/pool/mod.rs:56`
- `crates/canic-core/src/workflow/pool/mod.rs:690`
- `crates/canic-core/src/workflow/pool/mod.rs:701`
- `crates/canic-core/src/workflow/rpc/capability/proof.rs:88`
- `crates/canic-core/src/workflow/rpc/capability/proof.rs:122`
- `crates/canic-core/src/workflow/rpc/capability/grant.rs:125`

Contrast: root replay decode wrappers in
`workflow/rpc/request/handler/replay.rs` and
`workflow/rpc/request/handler/nonroot_cycles.rs` delegate actual decode work to
`replay_ops::*`, which is the cleaner pattern.

Recommended owner boundary: move persisted replay response codecs and
capability proof/grant codecs to ops-owned replay/capability codec helpers or a
lower dedicated codec module.

### Low - Pool Recycle Reads A Storage Record Directly

`workflow/pool/mod.rs` takes `&crate::storage::canister::CanisterRecord` in
`mark_pool_recycle_pending` to copy role, parent, and module hash into the pool
pending-reset entry.

Evidence:

- `crates/canic-core/src/workflow/pool/mod.rs:519`

This is a narrow read-only leak, but it still couples workflow to a persisted
storage schema. The metadata projection should be supplied by
`SubnetRegistryOps` or registered into the pool through an ops-owned helper.

### Low - Workflow Pool Module Pressure Is High

`workflow/pool/mod.rs` is now 1159 lines and owns create-empty replay
reservation, cost-guard sequencing, pending-reset import/recycle orchestration,
intent helpers, response codecs, and tests in one module.

Evidence:

- `find crates/canic-core/src/workflow -type f -name '*.rs' -exec wc -l {} +`
- largest production workflow files: `pool/mod.rs` 1159,
  `ic/icp_refill/mod.rs` 705, `rpc/request/handler/nonroot_cycles.rs` 592,
  `canister_lifecycle/mod.rs` 521, `rpc/request/handler/replay.rs` 508.

Recommended next split: move pool replay/cost/intent helper surfaces behind
ops-owned helpers first, then split remaining orchestration by command family
only if the ownership boundary stays clear.

## Checklist Results

| Check | Result | Notes |
| --- | --- | --- |
| Storage record / stable access | FAIL | `IcpRefillRecord` and `CanisterRecord` cross into production workflow. Test-only `IntentStore` match ignored. |
| Serialization / transport parsing | FAIL | Pool replay response and capability proof/grant codecs are workflow-owned. Call/install Candid bounds are accepted adapters. |
| Conversion ownership | PASS with watchpoints | Most conversions call ops/mappers; workflow-local capability `TryFrom` remains part of the codec finding. |
| Platform calls | PASS with watchpoints | No direct CDK/infra calls; management/ledger effects route through ops. |
| Auth semantics | PASS | No delegated-token verification or authenticated identity resolution in workflow. Capability request authorization remains workflow-local for already-authenticated root requests. |
| Policy / persistence policy ownership | PASS with watchpoints | `CostClass` comes from `replay_policy`; `ManualRefillPolicyPreflight` is a local preflight carrier, not a pure policy definition. |
| Replay / cost / intent boundary | PASS with codec findings | Recent root and pool paths reserve/recover/commit through ops; codec ownership remains misplaced in pool. |
| Recovery / idempotence surface | PASS with watchpoints | Pool recycle/import now mark pending before reset and short-circuit duplicates; scheduler recovery stays ops-backed. |
| Metrics / error mapping | PASS with watchpoints | Metrics use fixed helpers; no branch on formatted error strings found in sampled hotspots. |
| Module pressure | FAIL | `workflow/pool/mod.rs` is an active hub and should shed lower-layer helpers. |

## Recent-Change Coverage Notes

- Pool `CreateEmpty` replay/cost-guard sequencing is present and bracketed
  around management create.
- Pool `ImportImmediate`, `ImportQueued`, and `Recycle` now short-circuit ready
  or pending-reset entries before repeated destructive reset.
- Pool recycle records pending-reset state before the reset boundary and
  schedules reset recovery on failure.
- Root auth-material signing paths reserve cost guard permits and mark replay
  recovery-required on uncertain signing/commit failures.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg -n "workflow-purity" docs/audits/reports docs/audits/recurring` | PASS | established least-recently-run current recurring audit |
| storage-record scan from `workflow-purity-v2` | FAIL | `IcpRefillRecord`, `CanisterRecord`, and replay receipt matches found |
| serialization scan from `workflow-purity-v2` | FAIL | workflow-owned `encode_one`/`decode_one` paths found |
| conversion scan from `workflow-purity-v2` | PASS | watchpoints only after folding codec matches into serialization finding |
| platform-call scan from `workflow-purity-v2` | PASS | ops-only platform effects |
| auth-semantics scan from `workflow-purity-v2` | PASS | no endpoint auth verifier ownership found |
| policy/persistence scan from `workflow-purity-v2` | PASS | cost classes imported; no workflow-owned ledger policy found |
| replay/cost/intent scan from `workflow-purity-v2` | PASS | sequencing through ops found, with codec finding noted separately |
| recovery/idempotence scan from `workflow-purity-v2` | PASS | duplicate/pending-reset guard paths present |
| module-pressure scan from `workflow-purity-v2` | FAIL | pool and ICP refill modules exceed pressure threshold |
| `cargo test -p canic-core workflow::pool --lib` | PASS | 10 pool workflow tests passed |
| `cargo test -p canic-core workflow::ic::icp_refill --lib` | PASS | 27 ICP refill workflow tests passed |
| `git diff --check` | PASS | no whitespace errors |

## Follow-up Actions

| Owner Boundary | Action | Target Run |
| --- | --- | --- |
| `ops::storage::icp_refill` / workflow IC refill | Hide `IcpRefillRecord` behind ops-owned transition/view types. | next workflow-purity rerun |
| `ops::storage::pool` / subnet registry ops | Replace workflow `CanisterRecord` dependency with an ops-owned metadata projection or helper. | next workflow-purity rerun |
| `ops::replay` or lower codec module | Move pool replay response encode/decode out of workflow. | next workflow-purity rerun |
| `ops::rpc` or lower codec module | Move capability proof/grant blob codecs and grant hash encoding out of workflow. | next workflow-purity rerun |

## Final Verdict

Fail with bounded findings.

Workflow still mostly orchestrates correctly, but `workflow-purity-v2` found
real lower-layer ownership leakage that should be cleaned before this audit can
return to pass-with-watchpoints.
