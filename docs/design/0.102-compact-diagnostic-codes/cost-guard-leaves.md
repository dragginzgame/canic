# Canic 0.102 Cost-Guard Diagnostic Leaves

Date: 2026-08-12

## Status

This provisional B1 ledger expands `CostGuardReserveError`, one of the twelve
partially typed conversion owners. It allocates no numbers. Cost-guard requests
are constructed by admitted workflows from protected configuration and runtime
facts; they are not caller-authored boundary DTOs.

## Exact Leaves

| Candidate label | Typed producer | Class/origin | Public projection | Action and retry |
| --- | --- | --- | --- | --- |
| `COST_GUARD_CLASS_UNCOSTED` | `UncostedClass` | `Invariant` / workflow cost manifest | `COST_GUARD_CONFIGURATION_INVALID` | Correct the command's protected cost class; no unchanged retry |
| `COST_GUARD_QUOTA_WINDOW_INVALID` | `InvalidQuotaWindow` | `Invariant` / cost policy | `COST_GUARD_CONFIGURATION_INVALID` | Configure a positive window; no unchanged retry |
| `COST_GUARD_QUOTA_REJECTS_ALL` | `QuotaRejectsAll` | `Invariant` / cost policy | `COST_GUARD_CONFIGURATION_INVALID` | Configure positive admission capacity; no unchanged retry |
| `COST_GUARD_CYCLE_RESERVATION_OUT_OF_RANGE` | `CycleReservationOverflow` | `Invariant` / cost accounting | `COST_GUARD_CONFIGURATION_INVALID` | Reduce the protected reservation to the supported range |
| `COST_GUARD_QUOTA_EXCEEDED` | `QuotaExceeded` | `ResourceExhausted` / command quota | self | Wait for a later quota window or reduce admitted demand |
| `COST_GUARD_CYCLE_RESERVE_REJECTED` | `CycleReserveRejected` | `ResourceExhausted` / payer cycles reserve | self | Top up the payer or reduce the protected reservation |
| `COST_GUARD_RESOURCE_KEY_TOO_LONG` | `ResourceKeyInvalid(BoundedStringError::TooLong)` | `Invariant` / intent-key construction | `COST_GUARD_CONFIGURATION_INVALID` | Correct bounded key derivation; no unchanged retry |

`QuotaRejectsAll` is not an ordinary capacity hit. A zero protected maximum can
never admit the same request after time passes, so the current
`ResourceExhausted` classification is misleading. It belongs with invalid
protected cost policy.

Command names, quota use, limits, balances and required cycles do not enter the
compact error. They already exist in the manifest, request context and intent
totals used by the owning workflow.

## Cause Carriers And Secondary Failure

Two variants receive no code:

- `Store(InternalError)` preserves the exact storage diagnostic; and
- `ReservationRollback { reservation, rollback }` returns the primary
  reservation diagnostic while recording the rollback diagnostic through the
  existing quota-intent cleanup/recovery owner.

The recursive rollback wrapper must not become one combined code or one
formatted message. It contains two independent failures with different owners.
The primary reservation failure remains the caller result. The secondary
rollback failure needs a numeric structured observation and remains eligible
for the existing bounded cleanup workflow.

The same rule applies to `complete_after_failure` and
`recover_after_failure`: remove `with_diagnostic_context`, return the protected
operation's primary diagnostic unchanged and record completion/recovery failure
separately. Reservation IDs remain typed intent state rather than diagnostic
text.

## Public Classification Hard Cut

`CostGuardReservePublicKind` currently collapses the seven leaves into
`InvalidInput` and `ResourceExhausted`, and
`map_cost_guard_reserve_error` formats the original error into a public message.
Both responsibilities disappear in B4:

- each exact leaf owns its internal and public code directly;
- host-only class and origin come from the native catalog;
- `COST_GUARD_CONFIGURATION_INVALID` is the one safe projection for protected
  manifest/accounting invariants; and
- expected quota and cycle pressure remain exact public leaves.

The public-kind enum is sediment after that conversion and must be deleted, not
retained as a second classification table.

## Current Count

This family adds **seven exact candidate leaves** and one safe projection,
`COST_GUARD_CONFIGURATION_INVALID`. The two cause carriers allocate nothing.

## Required Tests

- exhaustive mapping of all nine `CostGuardReserveError` variants;
- proof that protected-policy failures never appear as caller-invalid input;
- exact quota-versus-cycle-pressure public codes;
- preserved `Store` cause identity;
- rollback tests returning the primary code and observing the secondary numeric
  recovery failure;
- context-removal tests for completion and recovery; and
- residue guards removing `CostGuardReservePublicKind`, formatted mapping and
  cost-guard `with_diagnostic_context` calls.
