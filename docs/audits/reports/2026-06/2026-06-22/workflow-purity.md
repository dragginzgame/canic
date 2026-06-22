# Workflow Purity Audit - 2026-06-22

## Report Preamble

- Scope: `crates/canic-core/src/workflow/**`, compared with `api::blob_storage`,
  `domain/policy`, `dto`, `model`, `ops`, `storage`, `access`, and endpoint
  macros
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/workflow-purity.md`
- Code snapshot identifier: `5bc5a458` with dirty worktree
- Method tag/version: `workflow-purity-v4`
- Comparability status: partially comparable. The core workflow-purity
  invariant is unchanged, while the definition now includes the blob-storage
  billing boundary.
- Auditor: `codex`

## Audit Definition Update

Before running the audit, `docs/audits/recurring/system/workflow-purity.md` was
reviewed and updated from `workflow-purity-v3` to `workflow-purity-v4`.

Method changes:

- added blob-storage billing boundary coverage;
- added `api::blob_storage`, `ops::blob_storage`, and `ops::cashier` to the
  boundary comparison scope;
- documented that workflow must not own Cashier protocol calls, billing status
  construction, gateway-principal sync storage, project-cycle funding guards,
  or billing config record/view/DTO projection;
- added an explicit blob-storage billing checklist scan.

## Executive Summary

Verdict: **Pass with watchpoints**.

No hard workflow-purity violations were found. Workflow remains orchestration
for the audited root proof and blob-storage billing surfaces:

- root proof batch install requires root, asks `AuthOps` to validate pending
  metadata, broadcasts issuer-local install requests through `CallOps`, and
  records installed outcomes through auth ops;
- delegated-token and role-attestation prepare paths use ops-owned replay
  receipts/codecs and auth ops for proof preparation;
- root RPC, pool create-empty, and ICP refill workflows still sequence
  replay/cost/intent state through ops-owned APIs;
- blob-storage billing has no production ownership under `workflow/`.

No production code changes were made for this audit. The only change was the
audit definition refresh.

## Findings

No hard findings.

### Watchpoint - Blob-Storage Billing API Facade Pressure

The new blob-storage billing scan found no production hits under
`crates/canic-core/src/workflow`. Current ownership is:

- `api::blob_storage` orchestrates endpoint-facing billing status, direct
  Cashier sync, and project-cycle funding;
- `ops::blob_storage` owns stable record projection, gateway registry mutation,
  and transient funding guard helpers;
- `ops::cashier` owns typed Cashier calls and response conversion.

That split is acceptable for the current slice, but `api::blob_storage` remains
a layer watchpoint. If billing orchestration moves into workflow later, the
workflow module should only sequence ops-owned steps.

### Watchpoint - Replay/Hash Helper Pressure

Workflow still contains protocol-visible replay payload hash helpers and replay
decision mapping helpers in:

- `workflow/runtime/auth/prepare/mod.rs`
- `workflow/pool/create_empty.rs`
- `workflow/ic/icp_refill/replay.rs`
- `workflow/rpc/request/handler/nonroot_cycles.rs`
- `workflow/rpc/request/handler/capability.rs`

These remain acceptable because the helpers use ops-owned replay hasher, guard,
receipt, and codec APIs. They should stay under watch because replay hash
semantics are externally visible and should not drift into duplicated local
schemas.

### Watchpoint - Large Workflow Files

Largest production workflow files in this run:

- `workflow/runtime/auth/prepare/mod.rs`: 718 lines
- `workflow/rpc/request/handler/nonroot_cycles.rs`: 717 lines
- `workflow/canister_lifecycle/mod.rs`: 657 lines
- `workflow/rpc/request/handler/replay.rs`: 504 lines
- `workflow/pool/create_empty.rs`: 476 lines

No large-file pressure currently hides a hard ownership violation, but these
remain the priority files for future workflow-purity runs.

## Checklist Results

| Check | Result | Notes |
| --- | --- | --- |
| Storage record / stable access | PASS with watchpoints | Production hits are `ReplayReceipt*` orchestration types, not stable records. No active proof, issuer policy, pending proof batch, ICP refill record, pool record, or direct stable structure access was found in production workflow. |
| Serialization / transport parsing | PASS with watchpoints | Replay encode/decode wrappers delegate to `ops::replay`; call/install Candid bounds remain thin adapters. No workflow root proof assembly, `data_certificate()`, or production `signature_cbor` assembly found. |
| Conversion ownership | PASS with watchpoints | Workflow calls ops/domain mappers and constructs orchestration responses. No blob/record/DTO conversion helper was added under workflow. |
| Platform calls | PASS | Platform effects route through `MgmtOps`, `RequestOps`, `LedgerOps`, `CallOps`, or call workflow adapters. Root proof install broadcasts through `CallOps`; workflow does not retrieve root proofs. |
| Auth semantics | PASS with watchpoints | Workflow applies public self-prepare policy and role-attestation request checks, but does not verify inbound delegated tokens or resolve endpoint identity. Root proof install policy/metadata checks stay in auth ops. |
| Blob-storage billing boundary | PASS with watchpoints | No production workflow hits. Billing sync/status/funding orchestration remains in `api::blob_storage`; stable projection, gateway store mutation, funding guards, and Cashier conversion stay in ops. |
| Policy / persistence policy ownership | PASS | Cost classes, quotas, replay decisions, and auth policy checks are imported from policy/replay/ops layers. No workflow-owned durable policy ledger found. |
| Replay / cost / intent boundary | PASS with watchpoints | Workflow sequences reserve/abort/recover/commit calls; replay, cost guard, and intent state/codecs stay ops-owned. |
| Recovery / idempotence surface | PASS | Pool import/recycle/scheduler and ICP refill still place durable state changes through ops-backed markers before destructive or value-transfer boundaries. |
| Metrics / error mapping | PASS with watchpoints | Metrics use bounded helper surfaces and typed outcomes. No branch on formatted error strings was found in sampled hotspots. |
| Module pressure | PASS with watchpoints | The same high-pressure workflow files remain; no new large blob-storage workflow surface was introduced. |

## Recent-Change Coverage Notes

- Blob-storage billing was covered explicitly. `workflow/` has no production
  `BlobStorage`, `Cashier`, `billing`, `gateway_principal`, or
  `fund_from_project_cycles` hits.
- Current blob-storage billing ownership matches the previous layer and ops
  audit results: API orchestration is a watchpoint, while record projection,
  gateway mutation, funding guards, and Cashier wrappers remain in ops.
- Root proof provisioning still respects the direct-query split: workflow
  install code does not assemble proofs and does not call `data_certificate()`.
- The prior replay/hash helper watchpoint remains unchanged.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
| `rg` storage-record scan from `workflow-purity-v4` | PASS with watchpoints | `ReplayReceipt*` orchestration hits only. |
| `rg` serialization scan from `workflow-purity-v4` | PASS with watchpoints | Replay encode/decode wrappers delegate to ops. |
| `rg` conversion scan from `workflow-purity-v4` | PASS with watchpoints | Mapper calls route through ops/domain modules. |
| `rg` platform-call scan from `workflow-purity-v4` | PASS | Platform effects route through ops/call wrappers. |
| `rg` auth-semantics scan from `workflow-purity-v4` | PASS with watchpoints | No inbound delegated-token verifier ownership. |
| `rg` blob-storage billing scan against `workflow/` | PASS | No production hits. |
| `rg` replay/cost/intent scan from `workflow-purity-v4` | PASS with watchpoints | Sequencing through ops confirmed. |
| `find crates/canic-core/src/workflow -type f -name '*.rs' ! -name tests.rs -exec wc -l {} +` | PASS with watchpoints | Large auth, nonroot-cycle, lifecycle, replay, and pool files remain. |
| `cargo test --locked -p canic-core workflow::runtime::auth --lib -- --nocapture` | PASS | 20 tests passed. |
| `cargo test --locked -p canic-core workflow::rpc --lib -- --nocapture` | PASS | 49 tests passed. |
| `cargo test --locked -p canic-core workflow::pool --lib -- --nocapture` | PASS | 10 tests passed. |
| `cargo test --locked -p canic-core workflow::ic::icp_refill --lib -- --nocapture` | PASS | 41 tests passed. |
| `cargo test --locked -p canic-core blob_storage --lib --features blob-storage-billing -- --nocapture` | PASS | 48 tests passed. |

## Follow-up Actions

| Owner Boundary | Action | Target Run |
| --- | --- | --- |
| `api::blob_storage` / workflow | Keep blob-storage billing sync/funding/status out of workflow unless a deliberate workflow module is designed. If introduced, keep storage, conversion, public error mapping, and Cashier protocol defaults below workflow. | next workflow-purity rerun |
| `ops::replay` / workflow replay helpers | Keep replay codecs in ops. Consider moving protocol-visible replay payload hash construction lower only as a planned boundary cleanup. | next workflow-purity rerun |
| `workflow::runtime::auth::prepare` | Watch delegated-token/role-attestation replay hash and decision mapping pressure. | next workflow-purity rerun |
| `workflow::rpc::request::handler::nonroot_cycles` | Watch large-file pressure around replay, cost guard, authorization, and value-transfer metrics. | next workflow-purity rerun |
| `workflow::canister_lifecycle` | Watch large-file pressure around upgrade preflight, metrics, and propagation orchestration. | next workflow-purity rerun |

## Final Verdict

Pass with watchpoints.

Workflow still owns orchestration, not storage, proof assembly, Cashier protocol
semantics, or blob billing state. The residual risk is file pressure and the
need to keep protocol-visible replay hash helpers from becoming duplicated
lower-layer schemas.
