# Audit: Workflow Purity

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
- perform serialization;
- own conversions;
- parse transport formats;
- access stable structures directly;
- perform IC/platform calls outside ops;
- own auth semantics;
- implement persistence policy.

## Scope

Primary scope:

- `crates/canic-core/src/workflow/**`

Boundary comparison scope:

- `crates/canic-core/src/domain/policy/**`
- `crates/canic-core/src/ops/**`
- `crates/canic-core/src/access/**`
- `crates/canic-macros/src/endpoint/**`

## Run This Audit After

- adding workflow modules;
- moving logic between workflow, policy, and ops;
- adding retry, replay, funding, lifecycle, or install orchestration;
- changing endpoint macro auth/access lowering.

## Checklist

### 1. Storage Record / Stable Access

Workflow must not construct or depend on storage records or stable structures.

```bash
rg -n 'storage::.*Record|stable::|CanisterRecord|EnvRecord|StateRecord|RootReplayRecord' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- no production storage record imports;
- no direct stable structure access;
- test-only storage fixtures are allowed in `tests.rs`.

### 2. Serialization / Transport Parsing

Workflow must not own persistence serialization or transport parsing.

```bash
rg -n 'serde::|serde_json|candid::|CandidType|ArgumentEncoder|ArgumentDecoder|encode_|decode_|from_str|parse\(|IDLDeserialize|IDLBuilder|to_bytes|from_bytes' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Classify matches:

- Candid call/install argument bounds are allowed only as boundary adapters.
- Replay decode wrappers must delegate actual encoding/decoding to ops.
- Any JSON/text/YAML parsing in workflow is a violation.

### 3. Conversion Ownership

Workflow must not own DTO, record, infra, or view conversion logic.

```bash
rg -n 'struct .*Adapter|from_dto|to_dto|request_args_from_dto|result_to_dto|record_to|to_response|record_to_response|impl From|impl TryFrom' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- conversions live in ops, API, or dedicated mapper modules outside workflow;
- workflow may call conversion helpers owned by other layers.

### 4. Platform Calls

Workflow must call platform effects only through ops.

```bash
rg -n 'ic_cdk|crate::cdk::api|cdk::api|HttpInfra|MgmtInfra|call_raw|set_timer|set_certified_data|data_certificate|sign_with_ecdsa|ecdsa_public_key' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- no direct infra/CDK calls;
- ops calls such as `IcOps`, `MgmtOps`, `RequestOps`, and `HttpOps` are
  allowed.

### 5. Auth Semantics

Workflow must not own endpoint auth semantics.

```bash
rg -n 'verify_caller|DelegatedToken|resolve_authenticated_identity|authenticated_with_scope|consume_update|token_material|verify_delegated_token' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- endpoint auth remains in macros/access/API auth boundaries;
- workflow may orchestrate auth runtime setup or key publication through ops.

### 6. Policy / Persistence Policy Ownership

Workflow may apply policy but must not define pure policy types or own mutable
policy ledgers.

```bash
rg -n 'struct .*Policy|enum .*Policy|impl .*Policy|thread_local!.*FUNDING|static .*LEDGER' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Expected:

- pure policy lives in `domain/policy`;
- runtime/persistence state lives in ops or storage;
- workflow coordinates state reads, policy evaluation, and mutation order.

## Output Requirements

Result reports must include:

- exact scope and commit;
- pass/fail for each checklist item;
- any code changes made;
- residual watchpoints;
- validation commands.

## Final Verdict

Choose one:

- Pass — workflow is pure orchestration;
- Pass with watchpoints — no hard violations remain, but pressure exists;
- Fail — workflow owns forbidden behavior.
