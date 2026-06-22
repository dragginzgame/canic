# Audit: Workflow Purity

Method: `workflow-purity-v4`

## Purpose

Verify that workflow code remains orchestration code and does not absorb lower
or boundary layer responsibilities.

## Risk Model / Invariant

Workflow may:

- orchestrate;
- branch;
- sequence ops;
- apply policy;
- coordinate retries and transitions.

Workflow must not:

- construct storage records;
- mutate storage records directly;
- expose persisted records as workflow-local state carriers when an ops-owned
  command/view type would keep the boundary cleaner;
- perform serialization;
- own conversions;
- parse transport formats;
- access stable structures directly;
- perform IC/platform calls outside ops;
- own auth semantics;
- implement persistence policy;
- own replay/cost/intent ledger schemas or codecs.

Recent replay-protection and root-proof provisioning work expanded workflow
orchestration around replay receipts, cost guards, pending-reset recovery,
management effects, and delegated-auth proof lifecycle operations. This audit
must therefore distinguish allowed sequencing from ownership leaks:

- Allowed: workflow reserves, aborts, marks, commits, or recovers through
  `ops::*` facades while ordering side effects.
- Not allowed: workflow defines persisted receipt schemas, encodes stored replay
  responses itself, mutates stable records, or implements quota/replay/intent
  policies directly.
- Allowed: workflow orchestrates root proof batch install by asking auth ops to
  validate pending metadata and call ops to broadcast issuer-local installs.
- Not allowed: workflow assembles canister-signature proofs, calls
  `data_certificate()`, verifies delegated-token material, or invents issuer
  policy/retrieval authorization.

The blob-storage billing split adds another current ownership boundary.
Workflow must not quietly become the home for Cashier protocol calls,
blob-storage billing status construction, gateway-principal sync storage,
project-cycle funding guards, or billing config record/view/DTO projection.
Those responsibilities currently belong to `api::blob_storage`,
`ops::blob_storage`, and `ops::cashier`. If a future blob-storage workflow is
introduced, it may only sequence ops/API-owned steps; it must not own storage
records, DTO conversion, public error mapping, Cashier protocol defaults, or
transient funding guard state.

## Scope

Primary scope:

- `crates/canic-core/src/workflow/**`

Boundary comparison scope:

- `crates/canic-core/src/domain/policy/**`
- `crates/canic-core/src/dto/**`
- `crates/canic-core/src/api/blob_storage.rs`
- `crates/canic-core/src/model/**`
- `crates/canic-core/src/ops/blob_storage/**`
- `crates/canic-core/src/ops/cashier/**`
- `crates/canic-core/src/ops/**`
- `crates/canic-core/src/replay_policy/mod.rs`
- `crates/canic-core/src/storage/**`
- `crates/canic-core/src/access/**`
- `crates/canic-macros/src/endpoint/**`

## Run This Audit After

- adding workflow modules;
- moving logic between workflow, policy, and ops;
- adding retry, replay, funding, lifecycle, or install orchestration;
- adding cost guards, durable intents, replay receipts, pending-reset recovery,
  or management-effect recovery;
- changing delegated-token prepare, root proof batch prepare/get/install, active
  proof status, or issuer proof verification flows;
- changing blob-storage billing sync/funding/status, Cashier wrappers,
  project-cycle funding guards, or gateway-principal sync;
- changing persisted record shapes consumed by workflow;
- changing endpoint macro auth/access lowering.

## Checklist

### 1. Storage Record / Stable Access

Workflow must not construct or mutate storage records or stable structures.
Direct production dependence on persisted record types is at least a watchpoint
and is a finding when workflow stores, transforms, or passes the record around
as its own state carrier.

```bash
rg -n 'storage::.*Record|storage::stable|stable::|CanisterRecord|EnvRecord|StateRecord|RootReplayRecord|ReplayReceipt|IcpRefillRecord|PoolRecord|CanisterRecord|ActiveDelegationProofRecord|RootIssuerRecord|PendingDelegationProofBatchRecord' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- no production stable-storage imports;
- no production construction or mutation of `*Record` types;
- no workflow-owned state machines that carry persisted records where ops-owned
  transition DTOs would work;
- no direct stable structure access;
- root-proof workflows use ops-owned pending/active proof command and view
  types, not storage records;
- test-only storage fixtures are allowed in `tests.rs`.

### 2. Serialization / Transport Parsing

Workflow must not own persistence serialization or transport parsing.

```bash
rg -n 'serde::|serde_json|candid::|CandidType|ArgumentEncoder|ArgumentDecoder|encode_one|decode_one|encode_|decode_|from_str|parse\(|IDLDeserialize|IDLBuilder|to_bytes|from_bytes|signature_cbor|data_certificate' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Classify matches:

- Candid call/install argument bounds are allowed only as thin boundary
  adapters around ops calls.
- Replay response, receipt, capability-proof, and durable-intent codecs should
  live in ops or a lower dedicated codec module.
- Workflow wrappers named `encode_*` or `decode_*` are allowed only when they
  delegate actual encoding/decoding to ops.
- Any JSON/text/YAML parsing in workflow is a violation.
- Root proof retrieval and canister-signature proof assembly must stay outside
  workflow update paths; workflow must not call certificate/proof assembly
  helpers that require direct query context.

### 3. Conversion Ownership

Workflow must not own DTO, record, infra, policy-input, or view conversion
logic.

```bash
rg -n 'struct .*Adapter|from_dto|to_dto|request_args_from_dto|result_to_dto|record_to|to_response|record_to_response|record_to_policy_input|impl From|impl TryFrom|mapper::|RootDelegationProofBatch.*Response|InstallActiveDelegationProof.*Response' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- conversions live in ops, API, policy-input mappers, or dedicated mapper
  modules outside workflow;
- workflow may call conversion helpers owned by other layers.
- workflow-local `impl From`/`TryFrom` is allowed only for workflow-local error
  wrapping or workflow-owned internal enums; DTO/proof/blob conversions are
  findings.
- workflow may construct final boundary responses from ops outputs when endpoint
  API ownership already fixes the DTO shape, but record/DTO mapping helpers
  belong outside workflow.

### 4. Platform Calls

Workflow must call platform effects only through ops. Workflow may sequence
ops-owned platform effects, but destructive or expensive effects must be paired
with the appropriate replay, intent, or cost-guard preconditions.

```bash
rg -n 'ic_cdk|crate::cdk::api|cdk::api|HttpInfra|MgmtInfra|call_raw|set_timer|set_certified_data|data_certificate|sign_with_ecdsa|ecdsa_public_key|create_canister|install_code|uninstall_code|update_settings|transfer|notify|unbounded_wait|bounded_wait|CANIC_INSTALL_ACTIVE_DELEGATION_PROOF' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- no direct infra/CDK calls;
- ops calls such as `IcOps`, `MgmtOps`, `RequestOps`, and `HttpOps` are
  allowed.
- root-proof install may broadcast through `CallOps`, but proof retrieval must
  not be attempted from workflow or through nested canister calls.
- management create/install/reset/transfer flows have explicit replay, intent,
  recovery, or cost-guard ordering where duplicate execution would be harmful.

### 5. Auth Semantics

Workflow must not own endpoint auth semantics.

```bash
rg -n 'verify_caller|DelegatedToken|resolve_authenticated_identity|authenticated_with_scope|token_material|verify_delegated_token|subject|scope|audience|issuer_pid|RootIssuerPolicy|RootDelegationProof|InstallActiveDelegationProof' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- endpoint auth remains in macros/access/API auth boundaries;
- workflow may orchestrate auth runtime setup or key publication through ops.
- workflow capability authorization may compare already-authenticated callers
  with request subjects, but it must not verify delegated-token material or
  resolve endpoint identity.
- delegated-token prepare workflow may validate the public self-prepare policy
  through `domain::policy` and request issuer proof material from `AuthOps`.
  It must not verify inbound endpoint tokens.
- root proof provisioning workflow may require root through env ops and ask
  auth ops to validate pending batch metadata; issuer registry policy,
  certificate hash verification, active proof verification, and retrieval ACLs
  remain outside workflow.

### 6. Blob-Storage Billing Boundary

Workflow currently has no production blob-storage billing owner. Billing status,
gateway sync, project-cycle funding, and Cashier interaction should remain in
the API/ops split unless a deliberate workflow module is introduced.

```bash
rg -n 'blob|BlobStorage|cashier|Cashier|billing|gateway_principal|fund_from_project_cycles|BlobStorageFunding|storage_gateway|BlobStorageStatus|ReadinessBlocker' crates/canic-core/src/workflow crates/canic-core/src/api/blob_storage.rs crates/canic-core/src/ops/blob_storage crates/canic-core/src/ops/cashier -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- no production blob-storage billing sync/status/funding ownership in
  `crates/canic-core/src/workflow`;
- `api::blob_storage` may currently orchestrate endpoint-facing billing status,
  direct Cashier sync, and project-cycle funding, but remains a watchpoint;
- `ops::blob_storage` owns bounded storage, billing config projection, gateway
  registry projection, and transient funding guard helpers;
- `ops::cashier` owns typed single-call Cashier wrappers and conversion;
- if future workflow code is added, it may only sequence ops/API-owned steps
  and must not own DTO conversion, stable writes, public error mapping, or
  Cashier protocol defaults.

### 7. Policy / Persistence Policy Ownership

Workflow may apply policy but must not define pure policy types or own mutable
policy ledgers.

```bash
rg -n 'struct .*Policy|enum .*Policy|impl .*Policy|thread_local!.*FUNDING|thread_local!.*LEDGER|static .*LEDGER|Quota|CostClass|ReplayPolicy|IntentPolicy' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- pure policy lives in `domain/policy`;
- runtime/persistence state lives in ops or storage;
- workflow coordinates state reads, policy evaluation, and mutation order.
- cost class, quota, replay, and intent semantics are imported from
  `replay_policy`, `domain/policy`, or `ops`, not invented in workflow.

### 8. Replay / Cost / Intent Boundary

Workflow may orchestrate replay, cost guards, and durable intents, but ownership
of replay/cost/intent state and codec behavior must remain below workflow.

```bash
rg -n 'ReplayReceipt|ReplayReceiptDecision|ReplayReceiptToken|reserve_or_replay|commit_receipt|abort_reserved|mark_recovery_required|CostGuardOps|CostGuardRequest|CostGuardPermit|IntentStoreOps|try_reserve|commit_at|abort\\(|request_id|prepare_delegation_proof_batch|install_delegation_proof_batch|preflight_delegation_proof_batch_install_proof' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- replay reservation happens before a non-idempotent external effect;
- replay commit happens only after the authorized effect and authoritative state
  update succeed;
- replay reservations are aborted or marked recovery-required on failure;
- cost guard reserve/complete/recover paths bracket expensive or quota-bearing
  effects;
- intent store operations are invoked through ops, with workflow limited to
  sequencing and metric/error handling.
- root proof batch prepare idempotency and install metadata validation are
  ops/model responsibilities; workflow only sequences install calls and records
  per-issuer outcomes.

### 9. Recovery / Idempotence Surface

Recent pool and replay work makes recovery ordering a first-class workflow
purity risk. Workflow may decide sequence, but recovery state must be ops-owned
and duplicate requests must stop before repeated destructive effects.

```bash
rg -n 'PendingReset|RecoveryRequired|schedule\\(|AlreadyPresent|mark_pending_reset|mark_ready|mark_failed|register_pending_reset|register_ready|remove\\(&pid\\)|reset_into_pool|reset path|duplicate|idempotent|AlreadyInstalled|ProofMismatch|ExpiredOrSuperseded|RejectedBySigner' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- workflow sets or requests durable recovery markers before crossing
  destructive management boundaries;
- duplicate or retry branches short-circuit before repeating create/reset/install
  effects;
- pending/recovery markers are stored through ops;
- failure branches do not overwrite recovery markers in a way that makes a retry
  repeat the same external effect unsafely.

### 10. Metrics / Error Mapping

Workflow may record metrics and map errors at orchestration boundaries, but
metrics and formatted errors must not become policy inputs.

```bash
rg -n 'MetricEvent|Metrics::record|record_.*metric|format!\\(|to_string\\(\\)|InternalError::workflow|InternalError::public|Error::' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- metric labels are bounded or delegated to metrics helpers;
- error mapping stays at boundary seams and does not replace typed policy or
  ops errors;
- workflow does not branch on formatted error strings.

### 11. Module Pressure

Workflow files that accumulate many responsibilities should be flagged even
when individual responsibilities are delegated correctly.

```bash
wc -l crates/canic-core/src/workflow/**/*.rs
rg -n '^fn |^pub fn |^async fn |^pub async fn |^struct |^enum |^impl ' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- large workflow modules have clear section boundaries and helper ownership;
- high-pressure modules list concrete extraction candidates in the report;
- extraction candidates preserve dependency direction rather than moving code to
  another workflow file by default.

## Output Requirements

Result reports must include:

- exact scope and commit;
- method tag/version and method-drift notes;
- pass/fail for each checklist item;
- any code changes made;
- recent-change coverage notes for replay, cost guard, intents, management
  effects, and recovery;
- residual watchpoints;
- validation commands.

## Final Verdict

Choose one:

- Pass — workflow is pure orchestration;
- Pass with watchpoints — no hard violations remain, but pressure exists;
- Fail — workflow owns forbidden behavior.
