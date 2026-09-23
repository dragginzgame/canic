# 0.110 B1 Pool Ledger Recovery Hard-Cut Ledger

Date: 2026-09-03
State: source-family absence proved; controlled repeated artifact delta retained
Design owner: [0.110 Fleet runtime contraction](../../../design/0.110-fleet-runtime-contraction/0.110-design.md)
Last source containing the family: immutable `v0.110.2`
First source without the family: immutable `v0.110.3`
Current released baseline: immutable `v0.110.5` at
`50f40171d6177c3d1e490b1fdb5f6163323b2cd5`

## Controlled preparation — 2026-09-23

The [prepared-source report](../../reports/2026-09/2026-09-23/b1-prepared-experiments.md)
now binds the common eleven-role roster and family-only removal on `.2`.
Existing dependency records remain exact; only the two local index fixtures
are added. Offline locked metadata, patch applicability, wrong-anchor and
preparation-drift rejection, and failed-metadata cleanup pass. Row 18 is
`ready`: [paired qualification](../../reports/2026-09/2026-09-23/b1-row18-qualification/verification.json)
verifies all 23 vectors, including the separately registered baseline-only
helper, and clean source restoration. Its 46-build repeated measurement now
passes complete independent verification. The older review below records why
a raw release comparison was rejected and does not describe current readiness.

## Retained family-only result — 2026-09-23

The [complete vectors](../../reports/2026-09/2026-09-23/b1-row18-measurement/artifact-metrics.tsv)
contain two clean repetitions for both conditions across eleven canonical
roles, plus two family-present helper builds. Independent
[verification](../../reports/2026-09/2026-09-23/b1-row18-measurement/verification.json)
checks every payload hash/length, gzip roundtrip, direct section/function
counts, selected membership, exact repeats, method identities and clean
source/lock restoration. The [metadata](../../reports/2026-09/2026-09-23/b1-row18-measurement/run-metadata.tsv)
binds the corrected common preparation and family-only removal.

| Artifact | Code bytes | Raw bytes | Gzip bytes | Defined functions | Table/element entries | Candid bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Root, variant minus matched control | -95,690 | -99,853 | -30,661 | -84 | -13 / -13 | -1,700 |
| Coordinator, variant minus matched control | 0 | 0 | +5 | 0 | 0 / 0 | 0 |
| Each other canonical role, including Store | 0 | 0 | 0 | 0 | 0 / 0 | 0 |
| Deleted helper, complete control artifact | 418,859 | 497,508 | 201,321 | 1,305 | 397 / 396 | 505 |

Root is the only canonical artifact with a code/function reduction. All exports
remain identical; only Root's Candid changes as the family is removed. Store
and the eight configured non-Root roles are byte-identical across conditions.
Coordinator's raw length and function count are unchanged, but its bytes differ
and gzip grows by five bytes; no executable-size reduction is attributed there.
The helper has only a family-present artifact and retains exactly its required
`authority` and `recover` methods. Its complete footprint is separate from
Root's marginal delta and is never added into one module's headroom.

This isolates the already-published removal under the `.2` graph with common
index fixtures. It does not measure all intervening release changes, host
binary size, runtime instructions, recovery parity or build performance, and
it introduces no predecessor reader or compatibility lane. The final B4
absence and current-release behavior gates remain separate.

## Source-pair review — 2026-09-22

The catalog now names the actual removal boundary, `v0.110.2..v0.110.3`,
instead of the stale `v0.109.35..candidate` placeholder. Row 18 remains
`planned`; these historical anchors do not authorize a release-to-release
attribution or satisfy the current canonical selector.

The [structured source review](../../reports/2026-09/2026-09-22/b1-row18-source-review/source-review.json)
binds commit/tree identities, lock/config/toolchain hashes and twenty relevant
source owners. Its [checksum](../../reports/2026-09/2026-09-22/b1-row18-source-review/sha256.txt)
is retained alongside it. The last release containing the family is
`f9009d5ae7be78d4f9dd746431584368770e8364`; its immediate successor
`d5aa319dc6e6d9af48d8833931e076519b80968a` removes the family. The first
release without it is `938c40b738d55b29fe7457a0475f247102a35fc4`.

Three constraints prevent an uncontrolled comparison:

- The release transition changes 22 external package identities, including
  IcyDB and ic-testkit, as well as ICP CLI and PocketIC pins. Keep one resolved
  graph and toolchain for both conditions; do not attribute these changes to
  pool recovery.
- Both historical configs have seven declared roles plus the generated
  Coordinator and Store. Neither includes `index_hub` or `index_child`.
  The full current eleven-role selector requires identical, separately frozen
  fixture preparation on both conditions. The removed helper is an additional
  baseline artifact, not a replacement for a missing current role.
- Helper publication is host orchestration, with helper-catalog admission in
  Root. The reviewed Store builder, Store workflow, publication adapter and
  endpoint projection are byte-identical across the boundary. Removing host
  publication does not itself establish smaller Store Wasm; shared dependency
  reachability still needs measurement.

Use `.2` as the common product source and extract a complete, reviewed
family-only removal patch, including the host changes needed to build it.
Freeze a compatible method and qualify the complete prepared selector before
retaining repeated measurements. Measure the helper separately. The source
review supplies no artifact vector, instruction result or byte/function saving.

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

## Compatible Measurement And Final Absence Boundary

The previously reported roughly 195 KiB compressed helper size is routing
evidence only. It is not the Root/Store marginal delta and is not a code-section
or replica-function result.

The retained paired experiment above uses one frozen method, toolchain,
optimizer, build path, feature set and role roster for both sides. It records:

1. the v0.110.2 helper artifact's code, total, gzip, replica-limited function,
   optimizer-defined cross-check, tables, explicit instruction-evidence absence,
   Candid and exports;
2. Root with and without the command/status/record/workflow family;
3. Store under the same family-only patch, with host helper publication and
   Root catalog admission identified separately from Store-linked code;
4. the canonical current roles to confirm no unexpected shared-family shift;
5. exact lock/config/source identities and the intentionally limited retained
   diff.

The optimized Root code/function/Candid delta and separate helper artifact
complete historical attribution. The design's B4 current-candidate
absence/symbol result remains a separate obligation; neither source absence nor
these overlapping deltas substitutes for that final proof or combines into
one module's savings claim.

Because v6 added `index_hub` and `index_child` after the old evidence and has no
compatible predecessor, comparing its published v0.110.5 Root or Store row to
an older v5/v0.110.2 row would mix method and roster changes. The dedicated
common-preparation, family-only comparison above resolves that attribution gap.

## B1 Disposition

Source-family absence and the compatible whole-family artifact delta are
retained. Keep their historical attribution separate from B4's final current
artifact absence and behavior evidence. Human acceptance of complete B1 remains
pending; current-behavior tests cover the maintained pool contract.
Do not add an anti-resurrection test for the removed family.
