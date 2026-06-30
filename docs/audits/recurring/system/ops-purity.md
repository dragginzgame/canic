# Audit: Ops Purity

## Purpose

Verify that ops code remains narrow operational code and does not accumulate
workflow orchestration or domain-policy ownership.

This audit is scoped to `canic-core` runtime ops. Host-side evidence,
provenance, policy-gate, catalog, and release-proof commands are outside this
audit unless they feed canister runtime ops.

## Risk Model / Invariant

Ops may own:

- deterministic state access;
- conversions between records, DTOs, views, policy-input views, and platform
  shapes;
- platform calls;
- atomic mutations;
- narrow operational semantics.
- auth proof material verification, root/issuer canister-signature proof
  preparation/retrieval, and issuer-local active proof state installation when
  each operation is a single bounded step.
- blob-storage stable-record/view/DTO mapping, Cashier response conversion,
  typed single Cashier call wrappers, and transient single-operation guards.

Ops must not own:

- retry loops;
- orchestration branching across multiple workflow steps;
- workflow sequencing;
- policy decisions;
- metrics coordination across domains;
- cross-domain business workflows;
- endpoint semantics;
- auth endpoint decisions;
- business state machines.
- root proof batch broadcast orchestration or external provisioning loops.
- blob-storage billing workflow orchestration, readiness/status construction,
  funding attachment decisions, or public endpoint error mapping.

Ops mapper names may contain `PolicyInputMapper`, `RootIssuerPolicyRecordMapper`,
or root-issuer-policy mapping helpers when they only convert storage records,
boundary DTOs, or request material into pure `domain/policy` input views or
records. Ops must not define a generic `policy` module or policy decision
types.

## Scope

Primary scope:

- `crates/canic-core/src/ops/**`

Boundary comparison scope:

- `crates/canic-core/src/workflow/**`
- `crates/canic-core/src/domain/policy/**`
- `crates/canic-core/src/access/**`
- `crates/canic-macros/src/endpoint/**`

## Run This Audit After

- adding ops modules;
- moving logic from workflow into ops;
- adding retry, replay, funding, lifecycle, RPC, auth, or metrics behavior;
- adding state-machine-like mutation helpers;
- changing endpoint auth/access lowering.
- changing build/evidence/provenance host commands only when those changes
  affect `canic-core/src/ops/**`.
- changing root proof provisioning prepare/get/install, active proof install,
  or root issuer policy mapping.
- changing blob-storage billing config persistence/projection, Cashier client
  wrappers/conversions, gateway-principal sync, project-cycle funding, or
  funding guards.

## Checklist

### 1. Workflow Dependency Direction

Ops must not call workflow.

```bash
rg -n 'crate::workflow|workflow::' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- no production ops imports or calls into workflow;
- comments may mention workflow only as context, but avoid them if they cause
  guard noise.

### 2. Orchestration Drift

Ops must not coordinate workflow sequences or retries.

```bash
rg -n 'retry|retries|loop\s*\{|while\s|join_all|spawn\(|sleep|backoff|orchestr|phase|step|transition' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Classify matches:

- atomic storage transitions are allowed;
- platform spawn primitives are allowed if workflow owns when to call them;
- cross-canister request helpers are allowed only when they execute one
  protocol operation.

### 3. Policy Ownership

Ops may consume domain policy outputs and may map storage records into policy
inputs. Ops should not define domain policy types or named policy modules.

```bash
rg -n 'struct .*Policy|enum .*Policy|impl .*Policy|mod policy|policy::|/policy/' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- pure policy definitions live in `domain/policy`;
- mapper names ending in `PolicyInputMapper` and root issuer policy record/DTO
  mappers are conversion helpers, not policy decision owners;
- ops topology/placement mappers should live under `input` or `mapper` modules,
  not an ops-owned `policy` module;
- local modules named for a boundary shape such as `root_issuer_policy` may
  map or validate boundary DTO shape, but must delegate policy decisions to
  `domain/policy`;
- ops metric labels may reference policy error types only for bounded reporting.

### 4. Endpoint/Auth Semantics

Ops may verify token material, prepare/verify proof material, consume replay
state, and perform key/platform operations. Ops must not own endpoint subject
binding, generated endpoint authorization semantics, or root proof install
broadcast orchestration.

```bash
rg -n 'verify_caller|authenticated_with_scope|requires\(|canic_update|canic_query|endpoint|DelegatedToken|verify_delegated_token|install_active_delegation_proof|start_next_chain_key_root_delegation_batch_install|CallOps::|unbounded_wait' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- endpoint auth semantics stay in access/macros/API guard paths;
- ops auth remains token-material verification, proof material
  prepare/retrieve/verify, key resolution, pending metadata, and bounded
  metrics;
- root proof batch install broadcast stays in workflow;
- domain replay protection belongs to replay receipt ops, not delegated-token
  verification.

### 4a. Root Proof Provisioning Split

```bash
rg -n 'prepare_due_chain_key_root_delegation_batch|sign_next_chain_key_root_delegation_batch|get_or_create_chain_key_delegation_proof|start_next_chain_key_root_delegation_batch_install|install_active_delegation_proof|record_chain_key_root_delegation_install|CallOps::|unbounded_wait|root_issuer_policy|validate_root_delegation_proof_prepare_policy' crates/canic-core/src/ops/auth crates/canic-core/src/workflow/runtime/auth crates/canic-core/src/api/auth -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- ops owns root proof batch metadata validation, root canister-signature proof
  prepare/retrieve, pending metadata mutation, and issuer-local active proof
  verification/storage;
- `workflow/runtime/auth/provisioning` owns cross-canister install broadcast
  and per-signer outcome orchestration;
- `api/auth` owns endpoint-facing guards and public error mapping;
- pure issuer policy decisions stay in `domain/policy/auth`.

### 4b. Public Error Boundary

```bash
rg -n 'crate::dto::error|dto::error::Error|crate::Error|Error::invalid|Error::forbidden|Error::exhausted|InternalError::public|Self::public|root_data_certificate_unavailable' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- ops does not construct public DTO errors directly;
- explicitly typed public `InternalError` constructors are allowed only when
  the operation itself must preserve a public protocol error code, such as
  root data-certificate unavailability;
- RPC ops may preserve a remote canister's wire-level public `Error` through
  `InternalError::public`, but must not invent endpoint/API public errors;
- endpoint/API layers remain responsible for general public error DTO mapping.

### 4c. Blob-Storage Billing Ops Split

```bash
rg -n 'set_billing_config|billing_config_dto|billing_config_record_to_view|billing_config_view_to_dto|record_gateway_principal_sync|replace_gateway_principals|BlobStorageFundingOps|CashierClientOps|CashierConversionOps|sync_gateway_principals_from_cashier|sync_gateway_principals_from_configured_cashier|fund_from_project_cycles|status\(' crates/canic-core/src/ops/blob_storage crates/canic-core/src/ops/cashier crates/canic-core/src/api/blob_storage.rs -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- `ops/blob_storage/lifecycle` may construct stable blob-storage billing
  config records, expose read-only views, project DTOs, and mutate stable
  gateway-principal state as bounded storage operations;
- `ops/blob_storage/funding` may own the transient single-flight funding guard,
  but not cycle math, Cashier calls, public error mapping, or stable billing
  workflow state;
- `ops/cashier` may own typed single Cashier call wrappers and response
  conversion/normalization helpers;
- `api/blob_storage` may currently own endpoint-facing billing config
  validation, status DTO construction, Cashier sync/funding orchestration, and
  public error mapping; this remains a layer/API watchpoint, not an ops-purity
  failure, unless that orchestration moves into ops.

### 5. Metrics Coordination

Ops may record metrics for one operation. Workflow should coordinate
multi-domain metric sequences.

```bash
rg -n 'Metric|Metrics::record|record_.*metric|metrics::' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- single-operation metrics in platform/auth/runtime ops are allowed;
- metrics modules are allowed as aggregation stores;
- ops should not run multi-step workflows just to produce one report.

## Output Requirements

Result reports must include:

- exact scope and commit;
- pass/fail for each checklist item;
- accepted hotspots;
- any code changes made;
- residual watchpoints;
- validation commands.

## Final Verdict

Choose one:

- Pass - ops remains narrow operational code;
- Pass with watchpoints - no hard violations remain, but hotspots exist;
- Fail - ops owns orchestration or policy that should move.
