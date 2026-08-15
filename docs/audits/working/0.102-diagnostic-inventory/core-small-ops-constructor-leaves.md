# Canic 0.102 Core Small Ops Constructor Leaves

Date: 2026-08-14

## Status

This evidence-only B1 ledger classifies ten production `InternalError`
constructor references in five small core ops owners. It assigns no number and
changes no runtime behavior.

| Production owner | Sites |
| --- | ---: |
| `ops/component_provisioning_receipt/mod.rs` | 2 |
| `ops/config/mod.rs` | 2 |
| `ops/rpc/mod.rs` | 2 |
| `ops/rpc/request/dispatch.rs` | 2 |
| `ops/runtime/init_payload.rs` | 2 |
| **Total** | **10** |

## Canonical Component Provisioning Receipts

The shared receipt hasher reuses two already-qualified identities:

| Existing exact identity | Sites | Producer function/hard cut |
| --- | ---: | --- |
| `COMPONENT_PROVISIONING_RECEIPT_ENCODE_FAILED` | 1 | `receipt_content_hash`; preserve a finite typed Candid encoder cause and never hash fallback bytes |
| `COMPONENT_PROVISIONING_RECEIPT_BYTE_COUNT_EXCEEDED` | 1 | `receipt_content_hash`; reject an authority whose canonical encoding cannot fit the frozen `u64` byte-count prefix |

The static method label chooses no second diagnostic family. Every caller uses
the same frozen domain-plus-length-plus-bytes construction; receipt kind and
authority remain available from the exact operation/DTO.

## Configuration Adapters

Both configuration constructors are transparent typed edges:

| Adapter | Sites | Disposition |
| --- | ---: | --- |
| compiled Component Topology | 1 | Preserve the exact reachable `ConfigError`/topology diagnostic |
| protected Component deployment validation | 1 | Preserve the exact one of ten qualified deployment-context diagnostics and its approved projection |

The second adapter currently prefixes a formatted typed error. That prefix adds
no owner, action or retry decision and receives no wrapper identity. B4 removes
it and propagates the registered source code.

## RPC Capability Transport

The two direct transport branches add exact meanings:

| Exact candidate | Sites | Producer function/branch | Action and retry |
| --- | ---: | --- | --- |
| `RPC_NONROOT_STRUCTURAL_REQUEST_INVALID` | 1 | `RpcOps::call_response_capability_v1_structural`; a non-root structural capability target receives a request other than cycles funding | Route root-only requests to the exact root; never reinterpret the request |
| `RPC_NON_STRUCTURAL_CAPABILITY_UNSUPPORTED` | 1 | `non_structural_capability_proof_error`; an internal root RPC requests a parent/proof shape not admitted by the structural path | Use the structural parent shape or the delegated-token endpoint |

These are local transport-contract failures, not authorization denials from the
receiver. They must not become `AUTH_PROOF_INVALID` or prove anything about the
remote Canister.

## Request Payload Encoding

The two request adapters add distinct codec meanings:

| Exact candidate | Sites | Producer function/branch | Action and retry |
| --- | ---: | --- | --- |
| `RPC_CREATE_EXTRA_ARG_ENCODE_FAILED` | 1 | `RequestDispatchOps::create_canister_with_metadata`; generic child-creation extra argument cannot be encoded | Fix the admitted application payload/type before retry |
| `RPC_PLACEMENT_EXTRA_ARG_ENCODE_FAILED` | 1 | `RequestDispatchOps::allocate_placement_child_with_metadata`; placement-child extra argument cannot be encoded | Fix the admitted placement payload/type before retry |

The routes remain distinct because they enter different lifecycle/admission
journeys even though both use Candid. Neither formatter text survives B4.

## Wasm Store Initialization Payload

The two target checks have these dispositions:

| Exact candidate or reuse | Sites | Producer function/branch | Action and retry |
| --- | ---: | --- | --- |
| `WASM_STORE_INIT_TARGET_ANONYMOUS` | 1 | `wasm_store_init_args`; host/root requests an initialization payload for the anonymous principal | Supply the real planned sibling Store principal |
| reuse `FLEET_ACTIVATION_WASM_STORE_AUTHORITY_MISMATCH` | 1 | `wasm_store_init_args`; requested sibling Store differs from protected activation authority | Preserve the protected authority and use its exact Store |

The target mismatch is the same exact authority predicate already qualified by
Fleet activation; a payload-builder wrapper would duplicate it.

## Dynamic Public Context

Rows `DPC-338` through `DPC-344` in
[dynamic-public-context.md](dynamic-public-context.md) classify the receipt
label/codec, protected-deployment cause, request-variant label and two Candid
extra-argument causes. All have exact request, DTO, configuration or codec
owners; no free-form value remains in the compact error.

## Reconciliation

All ten sites have one disposition. They add five exact meanings, reuse three
existing identities and retain two transparent typed configuration edges. With
the preceding Fleet-activation/scaling slice, the effective constructor
frontier moves from 2,452 to 2,468 classified sites and from 47 to 31 open
sites. The qualified semantic set reaches 2,671 exact candidates plus 31 safe
projections: 2,702 current symbolic identities.

## Required Tests

- canonical receipt vectors preserve the two existing receipt identities;
- every reachable configuration/topology cause propagates without a wrapper;
- reject non-cycles requests to a non-root structural target;
- reject every unsupported non-structural root request shape;
- distinguish generic-create from placement extra-argument encoding failure;
- reject anonymous and changed Wasm Store initialization targets; and
- prove remote wire errors remain transparent and never become these local
  transport identities.

## Next Slice

Continue with the five remaining two-site workflow owners.
