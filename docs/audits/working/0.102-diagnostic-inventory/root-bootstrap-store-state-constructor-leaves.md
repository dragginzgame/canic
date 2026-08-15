# Canic 0.102 Root Bootstrap And Store-State Constructor Leaves

Date: 2026-08-15

## Status

This B1 evidence ledger classifies three constructors in
`workflow/bootstrap/root.rs` and five in
`ops/storage/state/root_wasm_store.rs`. It assigns no number and changes no
runtime behavior.

## Root Subnet Discovery

| Exact candidate or disposition | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `ACCESS_BUILD_NETWORK_UNAVAILABLE` | 1 | `root_set_subnet_id`; `BuildNetworkOps::build_network` is absent | self; reuses qualified access identity | Rebuild with exact `ICP_ENVIRONMENT` | public |
| `ROOT_SUBNET_DISCOVERY_EMPTY` | 1 | `root_set_subnet_id`; `IcWorkflow::try_get_current_subnet_pid` returns `Ok(None)` | self | Refresh/retry Registry evidence; keep root unready | public |
| transparent: exact typed current-Subnet discovery cause | 1 | Root bootstrap currently stringifies the exact IC/Registry discovery error | preserve the nested registered projection | Remove the text adapter and propagate the typed cause | public or structured owner of nested cause |

The three sites produce two exact occurrences. One reuses an existing identity,
one exact meaning is new and one site is transparent.

## Sibling Wasm Store Adoption State

The first two broad adapters format a closed
`SiblingWasmStoreAdoptionError`. They must map exhaustively rather than retain
the Debug discriminator.

| Exact candidate | Sites | Producer function/branch | Public projection | Action and retry | Observation |
| --- | ---: | --- | --- | --- | --- |
| `WASM_STORE_ADOPTION_AUTHORITY_INVALID` / `WASM_STORE_ADOPTION_AUTHORITY_CONFLICT` / `WASM_STORE_ADOPTION_INVENTORY_ALREADY_POPULATED` | 1 | `RootWasmStoreState::begin_sibling_wasm_store_adoption` exact `SiblingWasmStoreAdoptionError` branches, adapted by `RootWasmStoreStateOps::begin_sibling_wasm_store_adoption` | `COMPONENT_REGISTRY_STATE_INVALID` for malformed/inventory state; self for exact retry conflict | Preserve state and correct the exact authority boundary | public or recent failure as stated |
| `WASM_STORE_ADOPTION_AUTHORITY_INVALID` / `WASM_STORE_ADOPTION_AUTHORITY_CONFLICT` / `WASM_STORE_ADOPTION_INVENTORY_ALREADY_POPULATED` / `WASM_STORE_ADOPTION_INTENT_MISSING` | 1 | `RootWasmStoreState::commit_sibling_wasm_store_adoption` exact `SiblingWasmStoreAdoptionError` branches, adapted by `RootWasmStoreStateOps::commit_sibling_wasm_store_adoption` | `COMPONENT_REGISTRY_STATE_INVALID` for every retained-state contradiction | Preserve adoption state and identify the exact failed predicate | recent failure |
| `WASM_STORE_ADOPTION_NOT_VERIFIED` | 1 | `adoption_response`; phase is not Verified | self | Reconcile/query adoption until terminal verification | public |
| `WASM_STORE_ADOPTION_VERIFIED_TIME_MISSING` | 1 | `adoption_response`; `adopted_at_ns` is absent | `COMPONENT_REGISTRY_STATE_INVALID` | Preserve record and fail closed | recent failure |
| `WASM_STORE_ADOPTION_RECEIPT_AUTHORITY_MISMATCH` | 1 | `validate_adoption_authority`; observed and expected `SiblingWasmStoreAdoptionAuthority` differ | self | Replay/query with the exact protected authority | public |

The five sites produce ten exact-label occurrences and seven new unique
meanings. The first three state-error identities deliberately recur at begin
and commit.

## Dynamic Public Context

The two Store-state adapters interpolate a closed typed enum discriminator. It
is authoritatively typed and becomes the exhaustive exact mapping above; the
Debug text is discarded. Root Subnet discovery interpolates a typed nested
`InternalError`; it must propagate that registered code without copying its
rendered message into a new workflow diagnostic.

## Reconciliation

All eight sites have dispositions. They add eight new exact meanings after the
build-network reuse is deducted, plus one transparent nested cause. No safe
projection is added.

All nine referenced exact identities now have function/branch anchors and a
family-level completeness guard.

## Required Tests

- distinguish absent build network, empty IC Subnet observation and every
  exact typed IC/Registry discovery cause;
- exhaustively map all four adoption-state variants at begin and commit;
- reject nonterminal receipt, missing terminal time and every protected
  adoption authority field independently; and
- prove the typed Store-state and current-Subnet causes cross these wrappers
  without formatted-text matching.

## Next Slice

Continue through Wasm Store lifecycle/publication owners and Fleet Mirror/
Directory synchronization by external-effect and authority risk.
