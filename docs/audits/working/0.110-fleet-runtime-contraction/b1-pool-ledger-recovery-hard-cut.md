# 0.110 B1 Pool Ledger Recovery Hard-Cut Ledger

Date: 2026-09-03
State: source-family absence proved; compatible artifact delta open
Design owner: [0.110 Fleet runtime contraction](../../../design/0.110-fleet-runtime-contraction/0.110-design.md)
Last source containing the family: immutable `v0.110.2`
First source without the family: immutable `v0.110.3`
Current released baseline: immutable `v0.110.5` at
`50f40171d6177c3d1e490b1fdb5f6163323b2cd5`

## Verdict

Published `v0.110.3` removed the temporary pool Ledger recovery family across
all product layers. Current source contains no exact helper role/template,
Root command/status variant, DTO, stable record, workflow, host action, build
role, test or CI identifier. No compatibility reader, alias or fallback is
present.

This proves the source hard cut and closes the B1/B4 source-absence trace. It
does not supply the required marginal Wasm result. `CANIC-WASM-001/v6` is a
first-method baseline with an expanded eleven-role roster, so its v0.110.5
numbers are not a compatible comparison against v0.110.2.

## Removed Family

The v0.110.2 implementation temporarily recovered Cycles Ledger funds held in
one empty pool canister account. Root installed a release-bound helper Wasm on
that same controlled canister, instructed it to call the Ledger `withdraw`
method into its native canister balance, verified the result, uninstalled the
helper and retained an exact replay receipt.

The helper itself was a generated standalone crate with:

- immutable init authority binding amount, canister, timestamp, Ledger,
  operation and Root;
- an `authority` query;
- a Root-only `recover` update;
- duplicate-Ledger-response acceptance by exact block index;
- a workspace-seeded locked dependency graph; and
- governed Wasm/Candid/finalization output under the
  `pool_ledger_recovery` infrastructure role and
  `canic:pool-ledger-recovery` Store template.

It was incident recovery machinery, not a Fleet role or ongoing pool funding
capability.

## Layer-Complete Source Diff

| Layer | v0.110.2 surface | v0.110.3/current result |
| --- | --- | --- |
| generated artifact | host `bootstrap_pool_ledger_recovery` builder, generated canister source, isolated manifest/lock verification and finalization | builder module and generated source deleted |
| CLI/build | special `pool_ledger_recovery` build dispatch and infrastructure build preparation | role dispatch and prebuild removed |
| infrastructure manifest | `CanicInfrastructureRole::PoolLedgerRecovery` plus role/package/template classification | role and manifest entry removed |
| Store publication | post-Root-bootstrap helper artifact qualification, chunk publication and staging outside the application catalog | helper is never built or staged; Store has no recovery template authority |
| Root DTO/Candid | `RecoverPoolLedger` command/response and operation-status payloads | variants and payload DTOs deleted |
| Root stable state | current recovery authority/phase plus last terminal receipt inside `RootCanisterPool` memory ID 25; pool asset `RecoveringLedger` phase | fields, records and lifecycle variant deleted in-place under the reinstall-only schema hard cut |
| Root ops/workflow | prepare, exact replay, phase advancement, install/call/verify/uninstall/complete orchestration and status projection | complete transition and orchestration family deleted |
| Root effect accounting | dedicated helper-install cost guard and Ledger recovery pending-state fences | dedicated effect kind and fences removed |
| host Ensure model | `CurrentFleetProtocolAction::RecoverPoolLedger` and correlated fragments | action and fragments deleted |
| host planning/apply | recovery-step compilation, derived operation ID, Root command/status polling, Store artifact lookup and controlled Ledger-cycle/fee accounting | no recovery step is planned or executed; current pool funding uses native canister balances |
| tests | unit, host, generated-helper PocketIC and Fleet-registry qualification fixtures | helper-specific tests and fixtures removed; maintained current pool behavior tests remain |
| CI/workspace | special PocketIC exclusion/inclusion wiring and dependency edges for the helper proof | dedicated CI hook and helper-driven dependency edges removed |

The broad v0.110.2-to-v0.110.3 changes also include unrelated production-
adapter and CI work. Their aggregate source-line delta must not be attributed
to this family or treated as a size result.

## Current Absence Check

The maintained product tree was searched outside `target/`, Git internals and
historical/design/audit documentation for:

```text
PoolLedgerRecovery
RecoverPoolLedger
pool_ledger_recovery
pool-ledger-recovery
canic:pool-ledger-recovery
CanisterPoolLedgerRecovery
```

No match remains. The general Wasm attribution classifier still contains the
broader token `pool_ledger` so historical symbol reports can group a matching
name. It neither names the removed product identity nor contributes to a
canister artifact.

The hard cut also remains visible in current positive shapes:

- `InstallMode` contains only `Install` and `Reinstall`, so no compatibility
  upgrade lane was added for the stable record change;
- `CanisterPoolStateRecord` retains only current creation and handoff journals;
- `CanisterPoolAssetStatusRecord` has no recovery phase;
- Root's current generated command and status unions contain no recovery
  variant; and
- current infrastructure role selection contains only maintained built-ins.

## Required Compatible Measurement

The previously reported roughly 195 KiB compressed helper size is routing
evidence only. It is not the Root/Store marginal delta and is not a code-section
or replica-function result.

The remaining paired experiment must use one frozen method, toolchain,
optimizer, build path, feature set and role roster for both sides. It must
record:

1. the v0.110.2 helper artifact's code, total, gzip, replica-limited function,
   optimizer-defined cross-check, tables, instructions, Candid and exports;
2. Root with and without the command/status/record/workflow family;
3. Store with and without helper publication reachability;
4. the canonical current roles to confirm no unexpected shared-family shift;
5. exact lock/config/source identities and the intentionally limited retained
   diff; and
6. an optimized absence/symbol result, without adding the three overlapping
   deltas into one savings claim.

Because v6 added `index_hub` and `index_child` after the old evidence and has no
compatible predecessor, comparing its published v0.110.5 Root or Store row to
an older v5/v0.110.2 row would mix method and roster changes. B1 must instead
retain a dedicated compatible paired run or an accepted same-source controlled
ablation.

## B1 Disposition

Source-family absence is complete. The compatible whole-family artifact delta
remains open, and B1 must not claim byte/function savings for this deletion
until that paired evidence is retained. B4 should keep an exact identifier
absence ratchet so later work cannot resurrect the helper.
