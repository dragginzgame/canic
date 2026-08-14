# Canic 0.102 Cascade, ICP Refill And Intent-Storage Constructor Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger classifies all thirteen production
`InternalError` constructor references in topology cascade routing, ICP-refill
policy mapping and intent-storage index adapters. It assigns no number and
changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `workflow/cascade/topology.rs` | 4 |
| `workflow/ic/icp_refill/mod.rs` | 4 |
| `ops/storage/intent/mod.rs` | 5 |
| **Total** | **13** |

## Topology Cascade

All four sites preserve already-qualified owners:

| Exact candidate or disposition | Sites | Current meaning | Required hard cut |
| --- | ---: | --- | --- |
| transparent typed Fleet-activation storage cause | 1 | Prepared topology snapshot cannot be bound to exact activation evidence | Propagate the exact storage/activation diagnostic |
| reuse `TOPOLOGY_PARENT_CHAIN_EMPTY` | 1 | Cascade route has no receiver-first path node | Preserve the typed snapshot-validation identity |
| reuse `TOPOLOGY_RECEIVER_MISMATCH` | 1 | First path node is not the exact receiving Canister | Remove the receiver principal from prose; the snapshot and transport target own it |
| reuse `TOPOLOGY_NEXT_HOP_MISSING` | 1 | Requested successor is absent from the branch path | Preserve the existing exact route-repair action |

The workflow performs an additional slice after the complete snapshot has
already passed typed validation. Its empty, receiver and next-hop failures are
the same snapshot authority and remediation as the existing validation
leaves, not a second cascade-specific family.

The child-send and cycle-reconciliation context helpers add no wrapper code.
They must preserve the exact nested transport or cycle diagnostic while
recording route context only in an appropriate typed/log owner.

## ICP-Refill Policy Mapping

The four direct constructors exhaustively map eight already-qualified policy
and build-configuration identities:

| Exact candidate or disposition | Sites | Class/origin | Action and retry |
| --- | ---: | --- | --- |
| reuse `ICP_REFILL_AMOUNT_ZERO` / `ICP_REFILL_AMOUNT_EXCEEDS_LIMIT` | 1 | `InvalidInput` / refill request and ceiling | Correct the amount before retrying |
| reuse `ICP_REFILL_ALREADY_IN_PROGRESS` | 1 | `Conflict` / source-target concurrency | Resume or await the existing operation |
| reuse `ICP_REFILL_CYCLES_FUNDING_DISABLED` / `ICP_REFILL_NOT_CONFIGURED` / `ICP_REFILL_RATE_GATE_DENIED` / `ICP_REFILL_RATE_UNAVAILABLE` | 1 | `Unavailable` / funding and rate policy | Change the named policy state before retrying |
| reuse `ICP_REFILL_BUILD_NETWORK_UNAVAILABLE` | 1 | `Invariant` / build configuration | Rebuild with an exact `ICP_ENVIRONMENT` identity |

`IcpRefillPolicyViolation` remains the decision owner. B4 removes the
intermediate debug-formatted message while retaining the exhaustive class
mapping; neither `PolicyDenied` nor the three class arms receive a wrapper
identity.

The observed conversion rate on a denied request currently has no retrievable
typed owner. That is a dynamic-value ownership gap, not permission to retain
debug prose; it is recorded in the dynamic ledger before B2 can proceed.

## Intent-Storage Index Adapters

The five sites convert typed `IntentStoreOpsError` decisions through the
generic storage string boundary:

| Adapter | Sites | Disposition |
| --- | ---: | --- |
| finite cleanup deadline derivation | 1 | Preserve exact not-found, pending-index or expiry-index identity |
| bounded due-expiry page | 1 | Preserve exact expiry-index identity |
| earliest due-expiry lookup | 1 | Preserve exact expiry-index identity |
| placement-acknowledgement presence | 1 | Preserve exact acknowledgement-index identity |
| placement-acknowledgement page | 1 | Preserve exact acknowledgement/index/primary-record identity |

Every possible leaf is already qualified in
[intent-store-leaves.md](intent-store-leaves.md). The adapter receives no code.
B4 must carry the typed storage cause through `StorageOpsError` without
formatting IDs, deadlines, index keys or primary-record state into a public
message.

## Dynamic Public Context

Fourteen values are classified as `DPC-281` through `DPC-294` in
[dynamic-public-context.md](dynamic-public-context.md): four topology routing
or typed-cause values, five ICP-refill policy values and five intent-storage
adapter values.

The denied observed conversion rate is the only new category-4 value. A
narrow typed ICP-refill policy-preflight result must own the observed rate and
configured minimum for the exact request before the debug message is removed.
It must not become generic diagnostic detail or a global last-error field.

## Reconciliation

All thirteen direct sites now have one disposition. They add no exact meaning,
reuse eleven already-qualified identity occurrences and retain six transparent
typed/context edges. The effective constructor frontier moves from 2,342 to
2,355 classified sites and from 157 to 144 open sites. The qualified semantic
set remains 2,517 exact candidates plus 31 safe projections: 2,548 current
symbolic identities.

The occurrence count is larger than the source-site count because the two
policy class arms exhaustively own two and four distinct typed violations.
Those variants were already deducted in the qualified family ledger.

## Required Tests

- cascade slicing reuses the typed empty-chain, receiver-mismatch and missing-
  next-hop identities;
- nested topology transport and activation-storage failures retain their
  source codes without route wrappers;
- every ICP-refill policy variant maps exhaustively to its existing identity
  and class without debug formatting;
- denied rate evidence is available only through the approved request-scoped
  preflight result;
- each intent expiry/acknowledgement adapter preserves the exact typed storage
  leaf; and
- malformed index identity cannot collapse into a generic storage failure.

## Next Slice

Continue with authority-restore storage, placement allocation and remaining
small runtime/storage adapters.
