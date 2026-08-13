# Canic 0.102 Fleet Coordinator Workflow Constructor Leaves

Date: 2026-08-13

## Status

This B1 evidence ledger classifies all 12 direct `InternalError::*` references
in `crates/canic-control-plane/src/workflow/fleet_coordinator/mod.rs`. It
assigns no number and changes no runtime behavior.

Six references are transparent conversions of a protected root's registered
public error. They receive no second Coordinator code. Three broad workflow
invariants are expanded by their exact phase or cursor predicates.

## Endpoint And Phase Admission

| Exact candidate or disposition | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ACCESS_CONTROLLER_REQUIRED` | 1 | Fleet Coordinator initialization caller is not a controller | self; reuses the qualified access identity | Invoke initialization as an admitted controller | public |
| `FLEET_COMPONENT_PROVISIONING_EXPECTED_PHASE_AHEAD` | 1 | Advance command expects a phase later than durable progress; an expected phase behind progress is an exact successful replay | self | Reload status and submit its exact current phase | public |
| `ROOT_DRAINING_RESERVATION_STATUS_CALLER_FORBIDDEN` | 1 | Reservation-status caller is neither a controller nor the exact target root | self | Query as an admitted controller or target root | public |

The three sites produce three exact-label occurrences. One reuses the existing
controller-required identity and two labels are new.

## Scale-Out Service-Publication Fence

The one broad fence constructor merges 17 phase facts. Four facts have the same
meaning in both accepted phases, so they form 13 exact diagnostics rather than
one generic invariant or 17 duplicated labels.

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_SCALE_OUT_PUBLICATION_CURRENT_ROOT_PRESENT` / `FLEET_SCALE_OUT_PUBLICATION_ROOT_EFFECT_IN_FLIGHT` / `FLEET_SCALE_OUT_PUBLICATION_ROOT_COUNT_INCOMPLETE` / `FLEET_SCALE_OUT_PUBLICATION_COMPONENTS_PROVISIONED_TIME_MISSING` | 1 | `ComponentsProvisioned` or `ServiceTopologyPublished` retains a root cursor/effect, incomplete root count or missing immutable completion time | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Preserve status and identify the exact incomplete terminal fact | recent failure |
| `FLEET_SCALE_OUT_COMPONENTS_PROVISIONED_REGISTRY_PRESENT` / `FLEET_SCALE_OUT_COMPONENTS_PROVISIONED_PUBLICATION_TIME_PRESENT` | same site | `ComponentsProvisioned` already contains Registry publication authority or its completion time | `COMPONENT_REGISTRY_STATE_INVALID` for both leaves | Preserve status and reject a partially advanced publication boundary | recent failure |
| `FLEET_SCALE_OUT_SERVICE_REGISTRY_PUBLICATION_MISSING` / `FLEET_SCALE_OUT_SERVICE_PUBLICATION_TIME_MISSING` | same site | `ServiceTopologyPublished` lacks the published Registry or immutable publication time | `COMPONENT_REGISTRY_STATE_INVALID` for both leaves | Preserve status and reconcile the missing publication evidence | recent failure |
| `FLEET_SCALE_OUT_SERVICE_DIRECTORY_CONFIRMATION_BEGUN` / `FLEET_SCALE_OUT_SERVICE_DIRECTORY_SYNCHRONIZATION_PRESENT` / `FLEET_SCALE_OUT_SERVICE_DIRECTORY_PUBLICATION_PRESENT` / `FLEET_SCALE_OUT_SERVICE_RUNTIME_ACTIVATION_BEGUN` / `FLEET_SCALE_OUT_SERVICE_RUNTIME_ACTIVATION_PRESENT` | same site | `ServiceTopologyPublished` already contains later Directory or runtime progress | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Preserve status and reject a crossed phase boundary | recent failure |

The single constructor expands to 13 new exact labels.

## Current-Root Provisioning Cursor

| Exact candidate | Sites | Current meaning | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `FLEET_SCALE_OUT_CURRENT_ROOT_CURSOR_MISSING` | 1 | `RootsAccepted` or `ProvisioningRoots` has no current root cursor | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve status and recover the exact root cursor | recent failure |
| `FLEET_SCALE_OUT_RESERVED_COUNT_EXCEEDS_COMPONENT_COUNT` / `FLEET_SCALE_OUT_CLAIMED_WITH_INCOMPLETE_RESERVATION` / `FLEET_SCALE_OUT_CLAIMED_COUNT_EXCEEDS_COMPONENT_COUNT` / `FLEET_SCALE_OUT_INSTALLED_WITH_INCOMPLETE_CLAIM` / `FLEET_SCALE_OUT_INSTALLED_COUNT_EXCEEDS_COMPONENT_COUNT` / `FLEET_SCALE_OUT_REGISTRY_COMMITTED_WITH_INCOMPLETE_INSTALL` / `FLEET_SCALE_OUT_REGISTRY_COMMITTED_COUNT_EXCEEDS_COMPONENT_COUNT` | 1 | Current-root progress violates one exact reservation, claim, install or Registry-commit prefix relation | `COMPONENT_REGISTRY_STATE_INVALID` for every exact leaf | Preserve the cursor and reconcile the first divergent prefix count | recent failure |

The two sites add eight exact candidates.

## Transparent Root RPC Errors

Six `InternalError::public` references decode and forward an already registered
root error from these exact transports:

- provisioning acceptance;
- provisioning advancement;
- fresh service publication;
- Scale Out service publication;
- Scale Out Component Directory synchronization; and
- runtime activation.

They are transparent propagation sites, not Coordinator diagnostic producers.
B3 must decode the registered compact identity directly, and B4 must preserve
it without allocating an RPC-wrapper code. Request encoding, transport and
response decoding failures remain separately typed by `CallOps` and Candid
conversion before these sites.

## Dynamic Public Context

The initialization denial interpolates only the transport caller principal.
That caller already knows its own identity, and the value changes no action:
discard it and return `ACCESS_CONTROLLER_REQUIRED`. No other maintained
constructor in this module introduces a dynamic public value; remote root error
context belongs to the producing root and its own projection ledger.

## Reconciliation

All 12 source references have one disposition:

- six transparent root-error conversions receive no code;
- the other six constructor sites produce 24 exact-label occurrences;
- one occurrence reuses `ACCESS_CONTROLLER_REQUIRED`; and
- 23 exact meanings are new, with no new safe projection.

## Required Tests

- reject Coordinator initialization from a non-controller without returning
  the caller principal as diagnostic text;
- distinguish an expected phase behind, equal to and ahead of durable progress;
- reject reservation-status access by a foreign caller while admitting the
  exact target root and controllers;
- vary every service-publication phase fact independently and assert its exact
  diagnostic;
- vary every reservation/claim/install/Registry prefix relation independently;
  and
- forward every registered root diagnostic unchanged through all six RPC
  response paths while keeping encoding, transport and decoding causes typed.

## Next Slice

Continue by external-effect and authority risk through Canister pool ops and
workflow, then root Store bootstrap.
