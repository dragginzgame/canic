# Workflow Purity Audit - 2026-05-16

## Run Context

- Definition: `docs/audits/recurring/system/workflow-purity.md`
- Related baseline: `docs/audits/reports/2026-05/2026-05-16/layer-boundary.md`
- Snapshot: `92ec102b`
- Branch: `main`
- Worktree: dirty
- Method: V1.0, focused workflow responsibility scan
- Scope: `crates/canic-core/src/workflow/**`, with comparisons against
  `domain/policy`, `ops`, `access`, and endpoint macros

## Executive Summary

Initial risk: **5 / 10**.

Post-remediation risk: **2 / 10**.

The audit found two concrete workflow-purity violations:

- cycles-funding policy and its mutable grant ledger lived under workflow;
- HTTP and management DTO conversion helpers lived under workflow adapters.

Both have been moved to the proper lower layers. Workflow now coordinates the
order of reads, policy evaluation, ops calls, and metric/error handling without
owning the pure policy, grant ledger, or DTO conversion code.

## Findings

### FIXED - Funding Policy And Grant Ledger Lived In Workflow

Severity: **High**.

`workflow/rpc/request/handler/funding.rs` defined `FundingPolicy`,
`FundingDecision`, `FundingPolicyViolation`, and an in-memory funding ledger.
That violated two workflow-purity rules:

- workflow must apply policy, not define policy;
- workflow must coordinate persistence, not own mutable policy ledgers.

Remediation:

- Moved pure cycles-funding policy to
  `domain::policy::cycles_funding`.
- Moved the mutable grant ledger to
  `ops::runtime::cycles_funding::CyclesFundingLedgerOps`.
- Removed the workflow funding module.
- Kept workflow as the coordinator that reads the ledger snapshot, applies the
  policy, executes the authorized transfer, and records the successful grant.
- Added a CI guard that rejects production workflow-defined `*Policy` types.

### FIXED - HTTP DTO Conversion Lived In Workflow

Severity: **Medium**.

`workflow/http/adapter.rs` converted public HTTP DTOs to ops HTTP request
types and converted ops results back to DTOs. That made workflow own boundary
conversion logic.

Remediation:

- Moved HTTP DTO conversion helpers onto `HttpOps`.
- Deleted the workflow HTTP adapter module.
- Kept `HttpWorkflow` as a thin orchestration wrapper around `HttpOps`.

### FIXED - Management Status DTO Conversion Lived In Workflow

Severity: **Medium**.

`workflow/ic/mgmt.rs` contained `MgmtAdapter`, which converted management
status snapshots into public canister-status DTOs.

Remediation:

- Moved `canister_status_to_dto` and supporting conversion helpers to
  `MgmtOps`.
- Kept `MgmtWorkflow::canister_status` as the sequence:
  call management ops, convert through ops, return DTO.

## Checklist Results

### Storage Records / Stable Access

Status: **Pass with test-only residue**.

Command:

```bash
rg -n 'storage::.*Record|stable::|CanisterRecord|EnvRecord|StateRecord|RootReplayRecord' crates/canic-core/src/workflow -g '*.rs' --glob '!**/tests.rs'
```

Remaining match:

- `workflow/rpc/request/handler/replay.rs` has a `#[cfg(test)]` import for
  `ReplaySlotKey`; this is test-only and should be ignored by future guards.

### Serialization / Transport Parsing

Status: **Pass with watchpoints**.

Workflow still has Candid bounds and replay decode wrapper names in:

- `workflow/ic/call.rs`;
- `workflow/runtime/install/mod.rs`;
- `workflow/rpc/request/mod.rs`;
- `workflow/rpc/request/handler/replay.rs`;
- `workflow/rpc/request/handler/nonroot_cycles.rs`.

These are currently boundary adapters or wrappers over ops-owned replay
encoding/decoding, not JSON/YAML/text parsing or storage serialization.

### Conversion Ownership

Status: **Pass after remediation**.

The concrete workflow-owned HTTP and management DTO adapters were moved to ops.
Remaining workflow calls such as `HttpOps::result_to_dto` and
`MgmtOps::canister_status_to_dto` call lower-layer conversion helpers rather
than owning conversion logic.

### Platform Calls

Status: **Pass**.

No direct CDK/infra platform calls were found in workflow. Workflow uses ops
surfaces such as `IcOps`, `MgmtOps`, `RequestOps`, and `HttpOps`.

### Auth Semantics

Status: **Pass**.

No delegated-token verification, endpoint caller verification, update-token
consumption, or authenticated-subject resolution was found in workflow.
Workflow auth hits are runtime setup/key-publication orchestration through
`AuthOps`.

### Policy / Persistence Policy Ownership

Status: **Pass after remediation**.

Workflow no longer defines `*Policy` types or owns the cycles-funding grant
ledger. Remaining workflow `thread_local!` usage is limited to runtime guards
and transition coordination.

## Residual Watchpoints

| Area | Risk | Note |
| --- | --- | --- |
| `workflow/ic/call.rs` | Low | Candid call adapter and intent orchestration live together. |
| `workflow/runtime/install/mod.rs` | Low | Install argument bounds remain workflow-facing adapter glue. |
| `workflow/rpc/request/handler/replay.rs` | Low | Replay decode wrappers delegate actual decoding to ops. |
| `workflow/rpc/request/handler/nonroot_cycles.rs` | Medium | Still centralizes replay, policy application, execution, and metrics. |

## Verification Readout

| Check | Result |
| --- | --- |
| Workflow policy-type scan | PASS |
| Workflow stable-record scan | PASS, test-only residue |
| Workflow direct platform scan | PASS |
| Workflow auth-semantics scan | PASS |
| `cargo fmt --all` | PASS |
| `cargo check -p canic-core -p canic-control-plane -p canic` | PASS |
| `cargo test -p canic-core --lib cycles_funding -- --nocapture` | PASS |
| `cargo test -p canic-core --lib request_cycles -- --nocapture` | PASS |
| `cargo clippy -p canic-core --all-targets -- -D warnings` | PASS |

## Final Verdict

Pass with watchpoints.

Workflow is back to orchestration for the audited areas. The next useful
follow-up is to keep `workflow/rpc/request/handler/nonroot_cycles.rs` small and
make sure any future funding rules land in `domain/policy/cycles_funding`, not
back under workflow.
