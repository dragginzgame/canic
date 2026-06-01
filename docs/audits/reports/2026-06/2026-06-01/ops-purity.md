# Ops Purity Audit - 2026-06-01

## Run Context

- Definition: `docs/audits/recurring/system/ops-purity.md`
- Prior retained report: `docs/audits/reports/2026-05/2026-05-16/ops-purity.md`
- Snapshot: `27a00430`
- Worktree: clean before audit slice
- Scope: `crates/canic-core/src/ops/**`, with comparisons against workflow,
  domain policy, access, and endpoint macro code. Host-side evidence,
  provenance, policy-gate, catalog, and packaged proof commands were treated as
  out of scope unless they feed canister runtime ops.

## Executive Summary

Verdict: **PASS**

Initial risk: **3 / 10**.

Post-remediation risk: **2 / 10**.

Ops remains narrow operational code. It owns deterministic state access,
record/view/DTO conversion, platform calls, atomic mutations, runtime metric
stores, and token/proof material verification. No production ops code imports
workflow, public endpoint API, public error DTOs, or generated endpoint macro
semantics.

The audit found two small cleanup opportunities:

- an ops topology mapper lived under an ops-owned `policy` module even though it
  only converts records into policy input views;
- two ops mappers imported `Principal` through `cdk::candid::Principal` instead
  of Canic's runtime type facade.

Both were corrected without changing behavior.

## Audit Definition Refresh

The recurring definition was updated to:

- scope the audit to `canic-core` runtime ops;
- keep host-side evidence/provenance/policy/catalog/release-proof commands out
  of scope unless they feed runtime ops;
- clarify that ops may own conversions into policy-input views;
- allow `PolicyInputMapper` names only for conversion helpers;
- explicitly reject ops-owned `policy` modules or policy decision types.

## Findings

### FIXED - Ops Topology Mapper Path Looked Policy-Owned

Severity: **Low**.

The topology mapper path was:

```text
crates/canic-core/src/ops/topology/policy/mapper.rs
```

The code did not define policy decisions. It only mapped storage registry
records into `TopologyPolicyInput` / `RegistryPolicyInput` views consumed by
domain policy. Still, the path made ops look like it owned a policy module.

Remediation:

```text
crates/canic-core/src/ops/topology/input/mapper.rs
```

Workflow call sites now import:

```rust
ops::topology::input::mapper::RegistryPolicyInputMapper
```

The mapper type names remain unchanged because they accurately describe the
target view shape.

### FIXED - Ops Principal Imports Used Broad Candid Path

Severity: **Low**.

Two ops mapper modules imported `Principal` from `cdk::candid::Principal`.
They now use the runtime type facade:

```rust
crate::cdk::types::Principal
```

Files changed:

```text
crates/canic-core/src/ops/placement/sharding/mapper.rs
crates/canic-core/src/ops/storage/registry/mapper.rs
```

Post-remediation scan:

```bash
rg -n 'cdk::candid::Principal' crates/canic-core/src/ops -g '*.rs'
```

No matches.

### FIXED - Workflow Path Comment Residue

Severity: **Low**.

`ops/storage/children/mod.rs` contained a comment with a concrete workflow path.
That caused recurring scan noise even though there was no code dependency.

The comment now states the invariant without naming a workflow module path.

### PASS - Workflow Dependency Direction

Ops does not call workflow.

Command:

```bash
rg -n 'crate::workflow|workflow::' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

No matches after comment cleanup.

### PASS WITH ACCEPTED HOTSPOTS - Orchestration Drift

Command:

```bash
rg -n 'loop\s*\{|while\s|join_all|sleep|backoff|orchestr|retry|retries' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

Remaining production hits are accepted:

- `ops/storage/registry/subnet.rs` walks a parent chain inside one registry
  record and guards against cycles/length overflow;
- `ops/ic/mod.rs` comments document that IC ops should not own orchestration;
- `ops/replay/mod.rs` uses replay wording in one error message.

No retry loops, sleeps, backoff, joins, or workflow-style orchestration were
found in ops.

### PASS WITH ACCEPTED HOTSPOTS - Policy Ownership

Command:

```bash
rg -n 'struct .*Policy|enum .*Policy|impl .*Policy|mod policy|policy::|/policy/' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
rg --files crates/canic-core/src/ops | rg '/policy/'
```

No ops-owned `policy` module remains.

Accepted hits:

- `PolicyInputMapper` names convert records into policy input views;
- runtime metric modules refer to domain-policy reason/error types for bounded
  metric labeling;
- `ops/runtime/cycles_funding.rs` consumes a domain-policy ledger snapshot.

Pure policy definitions remain under `domain/policy`.

### PASS WITH AUTH HOTSPOT - Endpoint/Auth Semantics

Ops auth still owns token material verification, proof verification, replay
consumption, key/root binding checks, and bounded verifier metrics. Endpoint
subject binding and generated endpoint authorization semantics remain in
access/macros/API guard paths.

Accepted hotspot:

```text
crates/canic-core/src/ops/auth/token.rs
crates/canic-core/src/ops/auth/delegated/**
crates/canic-core/src/ops/auth/verify/**
```

This is security-sensitive runtime material verification, not endpoint
authorization ownership.

### PASS - Public Error Boundary

Ops does not import or return the public DTO error type.

Command:

```bash
rg -n 'crate::dto::error|dto::error::Error|ErrorCode' crates/canic-core/src/ops -g '*.rs' --glob '!**/tests.rs'
```

No matches.

### PASS - Metrics Coordination

Ops runtime metrics remain metric stores and single-operation reporting helpers.
The scan is intentionally broad and returns many metric store/test hits, but no
multi-domain workflow orchestration was found in metrics ops.

## Residual Watchpoints

| Area | Risk | Note |
| --- | --- | --- |
| `ops/rpc/mod.rs` | Medium | Single protocol request/envelope construction is acceptable; keep retries/recovery in workflow. |
| `ops/auth/token.rs` and `ops/auth/delegated/**` | Medium | Token material verification is ops-owned; endpoint subject binding must stay in access/API guard paths. |
| Runtime metrics ops | Low | Metric stores are large and intentionally centralized; avoid moving report orchestration into metrics. |
| Policy input mappers | Low | Mapper names may mention policy input views, but ops must not reintroduce `policy` modules or decision types. |

## Verification Readout

| Check | Result |
| --- | --- |
| Ops workflow dependency scan | PASS |
| Ops orchestration scan | PASS with accepted hotspots |
| Ops policy ownership scan | PASS after mapper path cleanup |
| Ops public error scan | PASS |
| Ops Candid Principal scan | PASS after import cleanup |
| `cargo fmt --all --check` | PASS |
| `cargo test -p canic-core ops --lib --locked` | PASS, 160 tests |
| `cargo test -p canic-core canister_lifecycle --lib --locked` | PASS, compile-only filter, 0 tests matched |
| `cargo test -p canic --test changelog_governance --locked` | PASS |
| `cargo test -p canic --test workspace_manifest --locked` | PASS |
| `cargo clippy -p canic-core --all-targets --locked -- -D warnings` | PASS |
| `git diff --check` | PASS |

## Final Verdict

Pass.

Ops remains narrow operational code. The useful follow-up is to keep auth,
RPC, and metrics as recurring watchpoints because they are expected hot paths,
not because they currently violate the boundary.
