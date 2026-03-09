# Audit: Capability Pipeline Conformance

## Purpose

Detect drift in the root capability execution pipeline defined by the 0.13
distributed capability model.

Security invariant:

> No capability execution occurs unless envelope validation, proof binding,
> replay checks, and policy checks all pass in the canonical order.

This audit is a correctness and safety audit for capability invocation flow.

## Canonical Contract

Primary references:
- `docs/design/0.13-distributed-capability-invocation.md`
- `docs/status/0.13-distributed-capability-invocation.md`

Required order for root capability endpoint flow:
1. decode envelope and validate envelope headers
2. verify proof mode and capability hash binding
3. project replay metadata
4. run replay guard
5. run capability authorization/policy
6. execute capability effect
7. persist replay record on success only

## Scope

Audit these modules first:
- `crates/canic-core/src/api/rpc/capability/*`
- `crates/canic-core/src/workflow/rpc/request/handler/*`
- `crates/canic-core/src/ops/rpc/mod.rs`
- `crates/canic/src/macros/endpoints.rs`

## Run Context

Record in the result file:
- date
- auditor
- branch
- commit (`git rev-parse --short HEAD`)
- workspace state (`clean` or `dirty`)
- audited paths

## Checklist

Mark each item:
- `[x]` Pass
- `[ ]` Fail
- `[~]` Ambiguous or follow-up needed

### 1. Envelope Validation Is Fail-Closed

Verify that service, capability version, and proof version are checked before
proof verification and execution.

Suggested scans:

```bash
rg -n 'response_capability_v1|validate_root_capability_envelope|unsupported .*version|service must be Root' \
  crates/canic-core/src/api/rpc/capability -g '*.rs'
```

- [ ] Unsupported service is rejected
- [ ] Unsupported capability version is rejected
- [ ] Unsupported proof version is rejected
- [ ] Rejections happen before execution dispatch

Findings:
- (file, line, behavior)

### 2. Capability Hash Binding Is Canonical

Verify canonical hash inputs:
- target canister
- `CapabilityService::Root`
- capability version
- request payload without replay metadata

Suggested scans:

```bash
rg -n 'root_capability_hash|without_metadata|CAPABILITY_HASH_DOMAIN_V1|verify_capability_hash_binding' \
  crates/canic-core/src/{api/rpc/capability,ops/rpc} -g '*.rs'
```

- [ ] Hash excludes replay metadata
- [ ] Role-attestation proof validates hash binding
- [ ] Delegated-grant proof validates hash binding
- [ ] Hash mismatch fails closed

Findings:
- (file, line, behavior)

### 3. Proof Verification Covers All Proof Modes

Verify proof-mode dispatch and invariant checks:
- structural proof constraints
- role attestation verification
- delegated grant claim and signature verification

Suggested scans:

```bash
rg -n 'CapabilityProof::|verify_root_capability_proof|verify_root_structural_proof|verify_role_attestation|verify_root_delegated_grant_proof' \
  crates/canic-core/src/api/rpc/capability -g '*.rs'
```

- [ ] Structural proof is limited to supported capability families
- [ ] Role-attestation proof performs cryptographic and claim checks
- [ ] Delegated-grant proof performs hash, claim, and signature checks
- [ ] Unknown or mismatched proof paths fail closed

Findings:
- (file, line, behavior)

### 4. Replay Metadata Projection Is Safe

Verify replay metadata projection enforces:
- non-zero TTL
- max clock skew
- non-expired metadata
- deterministic replay request id derivation

Suggested scans:

```bash
rg -n 'project_replay_metadata|MAX_CAPABILITY_CLOCK_SKEW_SECONDS|replay_request_id|ttl_seconds' \
  crates/canic-core/src/api/rpc/capability -g '*.rs'
```

- [ ] `ttl_seconds == 0` is rejected
- [ ] future `issued_at` beyond skew is rejected
- [ ] expired metadata is rejected
- [ ] projected metadata is used for workflow replay path

Findings:
- (file, line, behavior)

### 5. Pipeline Ordering Is Preserved

Verify `response_capability_v1` keeps required ordering and dispatches through
replay-first workflow entrypoint.

Suggested scans:

```bash
rg -n 'response_capability_v1|verify_root_capability_proof|project_replay_metadata|response_replay_first' \
  crates/canic-core/src/api/rpc/capability/mod.rs crates/canic-core/src/workflow/rpc/request/handler/mod.rs -g '*.rs'
```

- [ ] Proof verification happens before replay projection
- [ ] Replay check happens before authorization in replay-first path
- [ ] Authorization happens before execution
- [ ] Replay commit happens after successful execution only

Findings:
- (file, line, behavior)

### 6. Policy and Execution Responsibilities Stay Split

Verify policy checks remain in authorization path and side effects remain in
execution path.

Suggested scans:

```bash
rg -n 'authorize_|execute_|deposit_cycles|CanisterLifecycleWorkflow::apply|DelegationWorkflow::provision' \
  crates/canic-core/src/workflow/rpc/request/handler -g '*.rs'
```

- [ ] Authorization path has no side effects
- [ ] Execution path does not bypass authorization
- [ ] No direct model mutation bypasses workflow contracts

Findings:
- (file, line, behavior)

### 7. Test Coverage

Verify coverage exists for:
- capability hash binding success and mismatch
- proof-mode acceptance and rejection paths
- replay duplicate/conflict/expired behavior
- replay-first ordering invariants

Suggested scans:

```bash
rg -n 'capability_hash|proof rejected|replay|duplicate|conflict|expired|response_replay_first' \
  crates/canic-core/src/api/rpc/capability/tests.rs \
  crates/canic-core/src/workflow/rpc/request/handler/tests.rs \
  crates/canic-core/tests/pic_role_attestation.rs \
  crates/canic/tests/root_replay.rs
```

- [ ] Unit coverage present for hash binding
- [ ] Unit coverage present for replay behavior
- [ ] Integration coverage present for capability endpoint proof paths
- [ ] Gaps documented explicitly

Findings:
- (test file, missing case)

## Severity Guide

- Critical: execution possible without proof/replay/policy gate
- High: hash/proof binding bypass or ordering regression
- Medium: replay metadata or clock-skew handling drift
- Low: observability/test gap without direct safety bypass

## Audit Frequency

Run this audit:
- after changes in `api/rpc/capability/*`
- after changes in `workflow/rpc/request/handler/*`
- after proof model updates (`dto::capability`)
- before each release cut
