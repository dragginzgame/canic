# Layer Violations Audit - 2026-06-17

## Run Context

- Definition: `docs/audits/recurring/system/layer-violations.md`
- Compared baseline report: `docs/audits/reports/2026-06/2026-06-01/layer-violations.md`
- Snapshot: `dc2db2e9`
- Branch: `main`
- Worktree: dirty
- Scope: canister runtime layering in `canic-core`, `canic-macros`, and the
  `canic` macro/facade code.
- Method tag/version: recurring layer-boundary audit, 2026-06 post-0.67
  pre-0.68 rerun plus same-slice remediation.
- Comparability: partial. The workspace contains unrelated backup/changelog
  work plus in-progress `canic-core` remediation; findings below separate
  current hard violations from residual drift risks.

## Executive Summary

Verdict: **PASS after remediation**.

Initial rerun risk: **4 / 10**.

Post-remediation risk: **2 / 10**.

The high earlier risk score was justified by multiple independent ownership
breaks appearing in one runtime slice:

- workflow was close to stable record and conversion details;
- storage accepted DTO enum shapes;
- ops constructed public DTO errors;
- stable replay storage depended upward on `ops::replay::model`;
- ICP refill replay/cost-guard/auth-adjacent code had a broad cross-layer
  blast radius.

Those combine badly because they spread one value-transfer operation across
endpoint, workflow, ops, storage, DTO, and replay ownership boundaries. The
problem was not one isolated grep hit; it was several boundaries becoming
porous at the same time.

The current rerun is materially better:

- CI layering guard passes.
- Storage/model no longer import ops.
- Domain/storage/model no longer import DTOs.
- Production workflow no longer touches storage record types.
- Ops no longer creates public DTO errors in non-test code.
- Policy purity scans are clean.
- Auth prepare replay orchestration no longer lives in `api/auth/mod.rs`.

The remaining risk is watchpoint-level: workflow still owns a small number of
established Candid adapter seams. Shared replay response projection for auth,
pool create-empty, and ICP refill now lives in `ops::replay`. No hard
layer-boundary violation remains in this rerun.

## Findings

### Fixed - API Auth Replay Orchestration Moved To Workflow

File: `crates/canic-core/src/api/auth/mod.rs`

Initial evidence:

```text
15:    ops::{
24:            model::{CommandKind, OperationId, RecoveryReason, ReplayActor, ReplayPayloadHasher},
26:                ReplayReceiptDecision, ReplayReceiptReserveInput, ReplayReceiptStoreError,
27:                ReplayReceiptToken, abort_reserved_receipt, commit_receipt_response,
28:                mark_recovery_required, reserve_or_replay_receipt,
35:use candid::{decode_one, encode_one};
134:        let replay_input = ReplayReceiptReserveInput::new(
143:        let token = match reserve_or_replay_receipt(replay_input)
654:        decision: ReplayReceiptDecision,
710:        encode_one(response).map_err(|err| {
733:        decode_one(response_bytes).map_err(|err| {
```

Why this was invalid:

- `api/**` is the endpoint/API boundary.
- The allowed matrix says endpoints/API should depend on workflow, DTO, and
  access/auth guard boundaries.
- This module owns replay mechanics directly: operation IDs, replay actor,
  payload hash construction, reservation, commit/recovery, replay decision
  mapping, and Candid replay response encoding.
- This is orchestration and replay projection, not endpoint marshalling.

Impact:

- Auth prepare flows are harder to audit against the same replay semantics used
  by workflow-owned RPC/ICP refill paths.
- Public error mapping, replay storage behavior, and auth proof preparation are
  co-located in a large API module.
- Future replay policy changes can accidentally diverge between API-auth and
  workflow-owned command paths.

Remediation:

- Added `crates/canic-core/src/workflow/runtime/auth/prepare/mod.rs`.
- Moved delegated-token prepare, root delegation prepare, and role-attestation
  prepare replay orchestration into `RuntimeAuthWorkflow`.
- Kept `AuthApi` as the public DTO facade for these prepare calls.
- Moved replay metadata validation, payload hashing, replay reservation,
  replay decision mapping, response encode/decode, and commit/recovery handling
  out of the API layer.
- Moved shared replay response schema checks and Candid response encode/decode
  for auth prepare, pool create-empty, and ICP refill into `ops::replay`.
- Moved the matching helper tests from `api::auth` to
  `workflow::runtime::auth::prepare`.
- Added a CI layering guard that rejects shared replay receipt orchestration
  terms in `crates/canic-core/src/api`.

Post-remediation scan:

```bash
rg -n "ReplayReceipt|ReplayPayloadHasher|reserve_or_replay_receipt|commit_receipt_response|decode_one|encode_one|ops::replay|OperationId|CommandKind|ReplayActor" crates/canic-core/src/api/auth/mod.rs
```

Result:

```text
no matches
```

The prepare-flow replay orchestration terms no longer appear in the API auth
module.

### Low - Workflow Serialization Watchpoints Remain

Current scan:

```text
crates/canic-core/src/workflow/mod.rs:33:            candid::CandidType,
crates/canic-core/src/workflow/runtime/install/mod.rs:4:        candid::{CandidType, utils::ArgumentEncoder},
crates/canic-core/src/workflow/runtime/install/mod.rs:22:    pub async fn install_with_payload_with_permit<P: CandidType>(
crates/canic-core/src/workflow/runtime/install/mod.rs:60:    pub async fn install_code_with_permit<T: ArgumentEncoder>(
crates/canic-core/src/workflow/rpc/capability/hash.rs:5:use candid::encode_one;
crates/canic-core/src/workflow/rpc/capability/hash.rs:14:    let payload = encode_one(&(
crates/canic-core/src/workflow/rpc/request/mod.rs:25:        A: CandidType + Send + Sync,
crates/canic-core/src/workflow/ic/call.rs:17:use candid::utils::{ArgumentDecoder, ArgumentEncoder};
crates/canic-core/src/workflow/ic/call.rs:88:        A: CandidType,
crates/canic-core/src/workflow/ic/call.rs:99:        A: ArgumentEncoder,
crates/canic-core/src/workflow/ic/call.rs:346:        R: CandidType + DeserializeOwned,
crates/canic-core/src/workflow/ic/call.rs:353:        R: for<'de> ArgumentDecoder<'de>,
```

Classification:

- Not a hard failure in this rerun.
- The remaining workflow Candid sites are established hash, type-bound, and
  generic adapter seams.
- Auth prepare, pool create-empty, and ICP refill replay response encode/decode
  now delegate to `ops::replay`.
- Keep the remaining Candid sites as watchpoints; do not let reusable response
  encoding or replay projection spread back into workflow modules.

### Pass - Lower-Layer Direction

Commands:

```bash
bash scripts/ci/run-layering-guards.sh
rg -n "use crate::api|crate::api::" crates/canic-core/src/workflow crates/canic-core/src/ops crates/canic-core/src/storage crates/canic-core/src/domain crates/canic-core/src/model -g '*.rs'
rg -n "use crate::workflow|crate::workflow::" crates/canic-core/src/ops crates/canic-core/src/storage crates/canic-core/src/domain crates/canic-core/src/model -g '*.rs'
rg -n "use crate::domain::policy|crate::domain::policy::" crates/canic-core/src/ops crates/canic-core/src/storage crates/canic-core/src/model -g '*.rs'
rg -n "use crate::ops|crate::ops::" crates/canic-core/src/storage crates/canic-core/src/model -g '*.rs'
```

Result:

- Guard passed.
- No lower-layer upward imports found.
- Broad `api::` token scans only found `cdk::api` platform calls inside ops,
  which are allowed side effects at the ops layer.

### Pass - DTO Boundary Purity

Command:

```bash
rg -n "crate::dto::|use crate::dto|\\bdto::" crates/canic-core/src/domain crates/canic-core/src/storage crates/canic-core/src/model -g '*.rs'
```

Result: no matches.

### Pass - Ops Public Error Boundary

Command:

```bash
rg -n "dto::error::Error|crate::dto::error|InternalError::public\\(" crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Result: no matches.

### Pass - Workflow Storage Boundary

Command:

```bash
rg -n "storage::.*Record|stable::|IcpRefillRecord|CanisterRecord|RecordOps" crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Result: no matches.

### Pass - Policy Purity

Command:

```bash
rg -n "ic_cdk|crate::ops|crate::workflow|crate::api|serde::|candid::|async fn|\\.await|storage::" crates/canic-core/src/domain/policy -g '*.rs'
```

Result: no matches.

## Verification

| Check | Result |
| --- | --- |
| `bash scripts/ci/run-layering-guards.sh` | PASS, now includes API shared-replay guard |
| lower-layer upward import scans | PASS |
| DTO leakage scan | PASS |
| ops public-error scan | PASS |
| workflow storage-record scan | PASS |
| policy purity scan | PASS |
| API auth replay scan | PASS |
| workflow serialization scan | PASS with watchpoints |
| `cargo test --locked -p canic-core api::auth --lib -- --nocapture` | PASS, 1 test |
| `cargo test --locked -p canic-core workflow::runtime::auth --lib -- --nocapture` | PASS, 19 tests |
| `cargo test --locked -p canic-core workflow::pool::create_empty --lib -- --nocapture` | PASS, 4 tests |
| `cargo test --locked -p canic-core workflow::ic::icp_refill --lib -- --nocapture` | PASS, 41 tests |
| `cargo test --locked -p canic-core ops::replay --lib -- --nocapture` | PASS, 21 tests |
| `cargo fmt -p canic-core -- --check` | PASS |
| `cargo check --locked -p canic-core` | PASS |
| `cargo clippy --locked -p canic-core --all-targets -- -D warnings` | PASS |
| `git diff --check -- crates/canic-core` | PASS |

## Follow-Up

1. Keep the current lower-layer remediations: replay model under `model`,
   storage-owned record enums, workflow-facing ICP refill views, and
   workflow-owned cost-guard public error mapping.
2. Continue classifying the remaining workflow Candid hash/adapter seams before
   broadening guards; no shared replay response encode/decode remains in the
   cleaned auth, pool create-empty, or ICP refill workflow paths.
