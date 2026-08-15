# Canic 0.102 RPC Workflow Error Leaves

Date: 2026-08-15

## Status

This evidence-only B1 ledger closes all nineteen declared
`RpcWorkflowError` variants, every current production construction and the
source-specific replay decoder failures currently flattened through
`ReplayDecodeFailed(String)`. It assigns no number and changes no runtime
behavior.

Two declared variants have no production constructor. Eleven of the seventeen
live variants reduce to twelve new exact meanings, four reuse existing replay
decision identities and two are source-selected codec wrappers. The decoder
expansion adds seven more exact meanings beneath the broad wrapper, for
nineteen additions in total.

## Child And Cycles-Funding Authority

| Exact candidate or disposition | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `RPC_CHILD_NOT_FOUND` | `ChildNotFound` | `NotFound` / protected child lookup | self | Reload the exact target or caller membership; do not infer a child from a presented principal |
| `RPC_CHILD_NOT_DIRECT` | `NotChildOfCaller` | `Forbidden` / immediate-parent authority | self | Invoke through the exact registered immediate parent |
| `RPC_CYCLES_FUNDING_BALANCE_INSUFFICIENT` | `InsufficientFundingCycles` | `ResourceExhausted` / funding effect capacity | self | Request the approved amount only after the funding authority has sufficient cycles |
| `RPC_CYCLES_FUNDING_DISABLED` | `CyclesFundingDisabled` | `Unavailable` / child-funding policy | self | Enable child cycles funding before retrying |
| `RPC_CYCLES_FUNDING_CHILD_BUDGET_EXHAUSTED` | `FundingRequestExceedsChildBudget` | `ResourceExhausted` / per-child funding budget | self | Wait for or change the exact child budget; lowering only the submitted request may still be insufficient |
| `RPC_CYCLES_FUNDING_COOLDOWN_ACTIVE` | `FundingCooldownActive` | `ResourceExhausted` / per-child funding window | self | Retry only after the typed preflight cooldown expires |
| `RPC_CYCLES_FUNDING_OPERATION_IN_PROGRESS` | `FundingOperationInProgress` | `Conflict` / per-child funding effect | self | Reconcile the retained pending funding operation before starting another |

`RPC_CYCLES_FUNDING_DISABLED` does not reuse
`ICP_REFILL_CYCLES_FUNDING_DISABLED`. The former rejects a registered child's
direct `RequestCycles` capability; the latter rejects the operator ICP-refill
workflow before Ledger/CMC activity. They have different actors, effects and
retry authorities even though both ultimately require enabling funding.

The four caller-required values identified in the dynamic-context ledger form
one request-scoped `CyclesFundingPreflightResponse`: approved amount, remaining
child budget, maximum child budget and retry delay. The live root/parent cycle
balance remains controller-authenticated operator evidence and must never be
added to that child-facing response.

## Replay Admission And Decisions

| Exact candidate or disposition | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `RPC_REPLAY_OPERATION_ID_REQUIRED` | `MissingReplayMetadata` | `InvalidInput` / root-capability replay identity | self | Supply the capability's exact nonzero operation identity and replay metadata |
| `RPC_REPLAY_TTL_ZERO` | zero-valued `InvalidReplayTtl` | `InvalidInput` / replay retention | self | Supply a positive TTL |
| `RPC_REPLAY_TTL_EXCEEDED` | over-ceiling `InvalidReplayTtl` | `InvalidInput` / replay retention | self | Reduce the TTL to the maintained root-capability ceiling |
| `RPC_REPLAY_TIME_RANGE_UNSUPPORTED` | `ReplayTtlOverflow` | `Invariant` / receiver clock range | self | Stop before reservation; the receiver clock cannot represent any positive expiry |
| reuse `REPLAY_RECEIPT_EXPIRED` | `ReplayExpired` | `Conflict` / retained replay receipt | self | Begin a newly admitted operation with a new identity |
| reuse `REPLAY_PAYLOAD_MISMATCH` | `ReplayConflict` | `Conflict` / replay payload identity | self | Replay only the original payload or use a new identity |
| reuse `REPLAY_OPERATION_IN_PROGRESS` | `ReplayDuplicateSame` | `Conflict` / replay operation | self | Retry the same request after its retained operation settles |
| `RPC_REPLAY_GLOBAL_CAPACITY_EXHAUSTED` | `ReplayStoreCapacityReached` | `ResourceExhausted` / root replay store | self | Await expiry/settlement or perform bounded operator cleanup before retry |
| reuse `REPLAY_PENDING_ACTOR_CAPACITY` | `ReplayStoreCallerCapacityReached` | `ResourceExhausted` / replay actor | self | Await this actor's pending operations before retry |

`InvalidReplayTtl` must be split before compact mapping: its zero and
over-ceiling predicates have different repair instructions. Conversely,
`ReplayTtlOverflow` is not an invalid request. The TTL has already passed the
positive bounded check; overflow occurs only when adding it to the receiver's
nanosecond clock. Reducing a valid TTL cannot make a saturated clock usable.

Root-wide replay capacity does not reuse actor or command capacity. It counts
the complete bounded root replay store, while the reused actor identity is
owned by one transport caller's active receipts.

## Replay Response And Receipt State

The two broad string variants are not allocation owners:

| Wrapper disposition | Current producer | Required hard cut |
| --- | --- | --- |
| source selects `REPLAY_RESPONSE_ENCODE_FAILED` | `ReplayEncodeFailed(String)` | Preserve the typed root response encoder and operation-bound receipt; discard dependency prose |
| source selects an exact decoder/receipt code | `ReplayDecodeFailed(String)` | Replace the free-form bucket with finite typed causes before compact mapping |

The root workflow reuses these already-qualified exact meanings:

- `REPLAY_RESPONSE_ENCODE_FAILED`;
- `REPLAY_RESPONSE_DECODE_FAILED`;
- `REPLAY_RECEIPT_MISSING`;
- `REPLAY_RECEIPT_DECODE_FAILED`;
- `REPLAY_RECEIPT_TOKEN_MISMATCH`;
- `REPLAY_STAGED_RESPONSE_MISSING`; and
- `REPLAY_COST_GUARD_SETTLEMENT_MISSING`.

The current compact-root decoder and shared committed-response helper add the
following exact source meanings:

| Exact candidate | Producer function/branch | Source predicate | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `RPC_REPLAY_COMPACT_VARIANT_TAG_MISSING` | `try_decode_compact_root_replay_response` missing-tag branch | compact prefix has no following variant tag | self | Preserve the terminal bytes; repair the exact compact encoder/decoder contract |
| `RPC_REPLAY_COMPACT_VARIANT_INVALID` | `try_decode_compact_root_replay_response` unknown-tag branch | retained compact tag is unknown | self | Preserve the terminal bytes; never reinterpret an unknown variant |
| `RPC_REPLAY_COMPACT_CYCLES_VALUE_TRUNCATED` | `decode_u128` through `take_exact` | cycles `u128` field has fewer than sixteen bytes | self | Preserve the terminal bytes; fail closed rather than defaulting or padding |
| `RPC_REPLAY_COMPACT_CYCLES_PAYLOAD_TRAILING_BYTES` | `try_decode_compact_root_replay_response` trailing-bytes branch | decoded cycles value leaves bytes behind | self | Preserve the terminal bytes; reject noncanonical payloads |
| `REPLAY_RESPONSE_SCHEMA_VERSION_MISSING` | `committed_response_bytes` missing-version branch | committed response lacks its schema version | self | Preserve the receipt and repair missing terminal schema authority |
| `REPLAY_RESPONSE_SCHEMA_VERSION_UNSUPPORTED` | `committed_response_bytes` version predicate | committed response names another schema version | self | Preserve the receipt; never decode through a compatibility fallback |
| `REPLAY_TERMINAL_RESPONSE_MISSING` | `committed_response_bytes` missing-bytes branch | committed/recovered receipt has no terminal response bytes | self | Preserve the receipt and reconcile its terminal state; never fabricate a response |

The last three meanings are shared by delegated-token, role-attestation,
ICP-refill and root response recovery wherever the same receipt predicate is
reachable. `REPLAY_TERMINAL_RESPONSE_MISSING` remains distinct from
`REPLAY_STAGED_RESPONSE_MISSING`: one contradicts a committed or recovered
terminal receipt, while the other blocks the transition that would commit a
missing staged response.

## Unproduced Sediment

`CanisterRoleNotFound` and `ParentNotFound` have no production constructor in
the current workspace. B4 deletes both variants. They receive no code, no
anti-resurrection compatibility test and no current-ledger row.

## Dynamic Public Context

Slices 62 and 63 of
[dynamic-public-context.md](dynamic-public-context.md) classify all live
fields and expanded codec sources. Caller-owned principals, capability labels,
limits and TTL values disappear from the compact error. Protected receipt
bytes, receiver time and root balance remain guarded. Four funding-preflight
values gain the one narrow typed owner described above. Secondary replay
cleanup failures retain their own numeric identity against the same operation
without replacing or concatenating the primary failure.

## Reconciliation

The nineteen declared variants have complete dispositions: seventeen are live
and two are sediment. Semantic expansion adds twelve exact meanings from eleven
live variants and seven exact decoder-source meanings. Four live variants reuse
existing replay-decision identities, while the two codec wrappers select
source codes and receive no wrapper identity.

The qualified semantic set moves from 2,727 to 2,746 exact candidates. The 31
safe projections are unchanged, producing 2,777 current symbolic identities.

## Required Tests

- exhaustive mapping over the retained `RpcWorkflowError` shape until B4
  replaces the two codec strings with finite typed causes;
- absence of production construction for both sediment variants before their
  deletion;
- separate missing-child and wrong-immediate-parent rejection;
- all five child-funding policy/effect decisions, including a guarded typed
  preflight that never exposes the funding authority's raw balance;
- distinct root-capability metadata, zero-TTL and over-ceiling-TTL rejection;
- valid-TTL receiver-clock overflow before replay reservation;
- exact generic replay reuse for receipt expiry, payload mismatch, in-progress
  operation and actor capacity;
- independent global replay-store capacity;
- source-exhaustive root receipt/store/response decoder mapping;
- compact missing tag, unknown tag, truncated field and trailing-byte
  rejection with terminal bytes preserved; and
- shared schema-missing, schema-unsupported and terminal-bytes-missing mapping
  without staged/terminal state collapse.

## Next Slice

Continue the transitive formatter frontier with the remaining terminal typed
owner that still flattens more than one source action or retry disposition.
