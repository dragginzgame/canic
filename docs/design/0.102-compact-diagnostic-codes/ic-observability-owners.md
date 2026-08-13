# Canic 0.102 IC Projection Observability Owners

Date: 2026-08-13

## Status

This B1 audit resolves the call-site ownership hidden by the phrase “owning
workflow” in [ic-infrastructure-leaves.md](ic-infrastructure-leaves.md). It
allocates no code and adds no runtime field.

All current generic IC calls enter through `infra::ic::call::Call`; direct
callers use the release-build, Cycles Ledger, ICP-refill, management or NNS
adapters. The adapter must classify the exact typed failure before public
projection. The workflow listed below owns numeric observation; it must not
infer the result from a projected code or reject message.

## Current Call-Site Map

| Call family and current production callers | Existing authority/checkpoint | Required exact numeric owner |
| --- | --- | --- |
| Embedded release-build identity during root and non-root runtime initialization | runtime bootstrap/activation readiness | lifecycle numeric log before readiness, then guarded runtime recent failure |
| NNS `get_subnet_for_canister` for the receiver's live Subnet | runtime environment bootstrap | lifecycle numeric log or guarded runtime recent failure; no external-effect journal |
| NNS `get_subnet_for_canister` before pool import | controller request plus root-local pool policy; no asset is admitted yet | guarded root recent failure; the failed observation must not create a pool record |
| Cycles Ledger `create_canister` and result conversion | durable root Canister-pool creation operation, operation ID, attempt settlement and uncertain-result flag | pool creation record/terminal failure code; recent failure is secondary only |
| ICP Ledger/CMC fee, decimals and rate reads | immutable refill request/policy input | guarded root recent failure; no claim that a paid effect occurred |
| ICP Ledger transfer and CMC `notify_top_up` | durable ICP-refill record, block index, operation status and typed refill error code | refill operation's numeric diagnostic before terminal/retry projection |
| Chain-key public-key and signing calls | durable chain-key signing batch and its protected policy/header identity | batch numeric diagnostic plus typed retryable/terminal disposition |
| `canister_info` for authority snapshot prepare/resume | stable authority-restore fence and exact operation ID/history count | restore-fence record or guarded runtime recent failure before state transition |
| Component install through `install_chunked_code` | Component Registry provisioning/removal operation and protected module plan | exact Component operation journal before retry or terminal projection |
| Pool controller reset, uninstall, cycle observation and handoff | durable pool asset state and pending reset/handoff operation | exact pool asset operation diagnostic; no transition to `Ready` on masked failure |
| Component status, stop and post-order recycling/deletion observation | durable Component Registry draining/removal operation | exact removal operation diagnostic before cursor or membership advancement |
| Wasm Store chunk upload and stored-chunk observation | staged release identity, Store-local chunk state and an internal cost-guard reservation; no current retrievable publication-attempt record | safe exact rejection/protocol diagnostic plus a narrow operation-scoped publication-attempt status before retry or phase advancement; the status binds protected Store, release, surface and optional chunk index |
| Wasm Store chunk clearing | durable Store GC intent and Store-local GC/chunk state | exact GC intent diagnostic before retry or phase advancement |
| Wasm Store stop/delete/cycle-reclamation GC | durable Store publication lifecycle/GC intent | exact GC intent diagnostic before absence, reclamation or deletion conclusion |
| Root/Store draining cycle transfers and controller updates | durable root-removal or Store lifecycle intent | exact draining/removal intent diagnostic before retry or terminal receipt |
| Component-to-child `deposit_cycles` RPC | durable RPC replay/cost-guard intent and exact child grant | replay/intent diagnostic before settlement; caller receives only the safe projection |
| Read-only management status query API | no mutation journal | guarded runtime recent failure before returning a masked response |

Management calls made from template publication, Component Registry, Canister
pool, Store lifecycle and root-removal code are therefore not allowed to fall
back to one process-global “platform failed” observation. Where the table names
an existing same-release journal, that journal is the primary owner. Template
publication is the explicit gap: its cost reservation and Store-local chunk
state do not replace the required narrow publication-attempt record. A recent-
failure entry may aid operators but cannot replace durable evidence at an
external-effect boundary.

For every masked result, the exact internal code must be stored on that same
operation/status record or correlated through the record's existing operation
ID, and the caller or operator must have a maintained way to retrieve it. A
number written only to an uncorrelated log does not satisfy this inventory.
Rows using guarded recent failure are admitted only when the corresponding
guarded status surface returns the bounded observation and no durable operation
authority exists.

## Projection-To-Owner Rules

- `IC_PLATFORM_RESPONSE_INVALID` records the exact adapter leaf at the row that
  owns the response. An out-of-range value never becomes an ordinary remote
  rejection.
- `IC_PLATFORM_PROTOCOL_INVALID` records request encoding before invocation or
  response decoding after invocation in the same workflow. The two phases
  remain distinct exact codes.
- `IC_PLATFORM_EFFECT_FAILED` records the exact typed call/rejection/sign-cost
  leaf. For a mutating call, the durable intent remains pending until the
  workflow re-observes authoritative state; the projection never settles it.
- `IC_CALL_REJECTED_DESTINATION_INVALID` remains exact typed absence evidence
  where the design permits absence handling. It is not projected through
  `IC_PLATFORM_EFFECT_FAILED` and is never reconstructed from text.
- `IC_CALL_REJECTED_SYSTEM_TRANSIENT` remains an exact retryable leaf. Retry is
  still admitted only by the owning journal, not by the code alone.
- `IC_CALL_LIQUID_CYCLES_INSUFFICIENT` remains exact and tells the local payer
  to top up; it does not claim anything about the remote Canister.

## Required B4/B5 Wiring

1. Each adapter returns an exhaustive typed exact diagnostic, never a formatted
   `IcInfraError` chain.
2. Every mutating caller persists its exact code in the existing operation
   authority before returning a masked result or scheduling retry, retaining
   the retrievable operation-ID correlation.
3. Read-only/bootstrap callers write the exact number to the guarded bounded
   observation before projection.
4. Existing record text may remain only when separately justified as bounded
   operational context; it never selects retry, absence or commitment.
5. Tests cover encode-before-call, response-lost, decode-after-call, exact
   typed absence, every non-absence rejection and restart from each durable
   operation family above.

No new generic IC-effect journal is justified. The listed authorities already
own their operations, and adding a parallel journal would create competing
recovery truth.
