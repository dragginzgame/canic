# Canic 0.102 ICP-Refill Replay Constructor Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger classifies all sixteen production `InternalError`
constructor references in ICP-refill replay, its value-transfer permit fence
and the ICP Ledger/CMC ops adapter. It assigns no number and changes no runtime
behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/ic/icp_refill/replay.rs` | 14 |
| `workflow/ic/icp_refill/cost_guard.rs` | 1 |
| `ops/ic/icp_refill.rs` | 1 |
| **Total** | **16** |

## Replay Decisions

The seven reserve/replay sites reuse six generic replay identities and add one
ICP-refill-specific recovery fence:

| Exact candidate or disposition | Sites | Class/origin | Action and retry |
| --- | ---: | --- | --- |
| reuse `REPLAY_OPERATION_IN_PROGRESS` | 1 | `Conflict` / exact replay operation | Retry later with the same operation and payload |
| reuse `REPLAY_ACTOR_MISMATCH` | 1 | `Conflict` / replay actor | Never reuse another actor's operation ID |
| reuse `REPLAY_PAYLOAD_MISMATCH` | 1 | `Conflict` / replay payload | Replay only the original refill request or use a new ID |
| reuse `REPLAY_RECEIPT_EXPIRED` | 1 | `Conflict` / replay retention | Begin a newly admitted refill with a new operation ID |
| `ICP_REFILL_REPLAY_RECOVERY_REASON_INVALID` | 1 | `Conflict` / refill recovery state | Recover only cost-settlement or response-commit failure; inspect every other retained reason |
| reuse `REPLAY_PENDING_ACTOR_CAPACITY` | 1 | `ResourceExhausted` / actor pending receipts | Wait for this actor's pending operations to settle |
| reuse `REPLAY_PENDING_COMMAND_CAPACITY` | 1 | `ResourceExhausted` / refill command receipts | Wait for command-kind capacity before retry |

The six generic decisions have the same replay-receipt owner, exposure and
retry contract as authentication prepare. They deliberately share identities.
The recovery reason does not: ICP refill admits two automatic recovery reasons
and coordinates a value-transfer cost settlement, so it cannot reuse the
authentication-specific recovery fence.

## Response And Receipt Reconstruction

The remaining seven replay constructors reuse the typed generic replay family:

| Exact candidate or disposition | Sites | Required hard cut |
| --- | ---: | --- |
| reuse `REPLAY_RESPONSE_ENCODE_FAILED` | 1 | Preserve the typed response encoder and exact refill schema |
| reuse `REPLAY_RESPONSE_DECODE_FAILED` | 1 | Preserve terminal bytes and typed refill response decoder |
| reuse `REPLAY_RECEIPT_MISSING` | 1 | Treat absence as missing durable state, never a fresh operation |
| reuse `REPLAY_RECEIPT_DECODE_FAILED` | 1 | Preserve malformed receipt bytes and typed decode cause |
| reuse `REPLAY_RECEIPT_TOKEN_MISMATCH` | 1 | Reload exact receipt identity; never commit through a stale token |
| reuse `REPLAY_STAGED_RESPONSE_MISSING` | 1 | Preserve receipt and recover only from exact staged bytes |
| reuse `REPLAY_COST_GUARD_SETTLEMENT_MISSING` | 1 | Stop before settlement/commit without the retained cost identity |

Command kind and response schema remain in the replay receipt. Sharing encoder
and decoder codes does not allow one response type to decode another command's
bytes.

Five secondary recovery-marker or cost-settlement failures are currently
appended with `with_diagnostic_context`. They receive no wrapper code. B4
returns the primary failure unchanged and records the secondary exact code
against the same replay operation, following the cost-guard secondary-failure
contract.

## Cost And IC Adapters

The effect permit fence adds one exact meaning:

| Exact candidate or disposition | Sites | Class/origin | Public projection | Action and retry |
| --- | ---: | --- | --- | --- |
| `ICP_REFILL_COST_PERMIT_REQUIRED` | 1 | `Invariant` / value-transfer effect admission | `COST_GUARD_CONFIGURATION_INVALID` | Stop before Ledger/CMC invocation and restore the exact reserved permit |
| transparent typed ICP Ledger/CMC infra cause | 1 | request, transport, response or system-Canister contract | source projection | Preserve the exact qualified IC-infrastructure diagnostic |

A missing permit is not ordinary quota pressure. It proves an internal effect
crossed its pre-effect guard and must remain distinct from reserve rejection.
The ops facade adds no route code around `IcInfraError`.

## Dynamic Public Context

Twelve values are classified as `DPC-317` through `DPC-328` in
[dynamic-public-context.md](dynamic-public-context.md). Two quota ceilings are
caller-derivable maintained contract. Recovery reasons, codec/store causes,
five secondary failures and the IC-infrastructure cause are typed.

Operational refill logs are outside `Error.message`; they remain guarded
observations and do not justify public principals, subaccounts, amounts or
record IDs.

## Reconciliation

All sixteen direct sites now have one disposition. They add two exact meanings,
reuse thirteen existing replay identities and retain one transparent typed IC
edge. The effective constructor frontier moves from 2,379 to 2,395 classified
sites and from 120 to 104 open sites. The qualified semantic set reaches 2,542
exact candidates plus 31 safe projections: 2,573 current symbolic identities.

## Required Tests

- exhaustive refill mapping for every `ReplayReceiptDecision` variant;
- exact generic replay reuse for progress, actor, payload, expiry and quotas;
- refill-specific rejection of every unsupported recovery reason;
- exact response schema binding despite shared codec identities;
- exhaustive receipt-store error mapping without string causes;
- every secondary recovery/settlement failure recorded numerically against the
  same operation while the primary code remains unchanged;
- no Ledger or CMC effect without the exact cost permit; and
- transparent propagation of every typed ICP Ledger/CMC infra leaf.

## Next Slice

Continue with Component runtime and Component Directory synchronization ops.
