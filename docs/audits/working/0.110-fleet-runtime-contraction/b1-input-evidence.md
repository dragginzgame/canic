# 0.110 B1 Wasm Input Evidence

Date frozen: 2026-08-31
Design owner: [0.110 Fleet runtime contraction](../../../design/0.110-fleet-runtime-contraction/0.110-design.md)
Current Canic source baseline: `936043db9` (`v0.109.30`)

## Authority Boundary

This file retains planning inputs. It is not B1 completion evidence and does
not authorize implementation. B1 must reproduce its accepted baseline and
controlled ablations from one immutable post-0.109 source, toolchain, feature
set and optimizer configuration.

The repository-owned predecessor measurement is
[0.109 B9 governed release Wasm optimization](../0.109-fleet-wide-ingress-admission/b9-release-wasm-optimization.md).
That report records the canonical Binaryen finalizer, deterministic artifact
measurements and named residual analysis. It shows that the general optimizer
and small sort follow-up have already been consumed; it does not measure the
storage ablations proposed by 0.110.

## Published Predecessor Measurements

The 0.109 report records these relevant optimized results:

| Artifact/evidence | Code-section bytes | Defined functions | Validator function count | Meaning |
| --- | ---: | ---: | ---: | --- |
| supplied Toko baseline after Binaryen 108 | 5,290,184 | 10,887 | not recorded | downstream evidence, not a Canic rerun |
| subsequent canonical Canic release App | 2,800,001 | 4,963 | not recorded | repository-owned final artifact after the bounded sort follow-up |
| managed leaf audit role after Binaryen 132 | 2,316,160 | 4,183 | not recorded | exact role-surface pruning evidence, not a controlled before/after pair |

The named Canic residual report found approximately 135 KiB of
monomorphization bloat after optimization, dominated by stable-sort machinery.
The accepted source follow-up recovered another 27,665 code-section bytes and
46 defined functions. Those savings are predecessor facts, not 0.110 forecasts.

## Downstream Generic-Instantiation Routing Evidence

A later read-only Toko/IcyDB generator experiment compared a generated
11-entity cohort. It attributed approximately 49.7-54 KiB directly to the
generator cohort and approximately 112-128 KiB to the wider surrounding
specialized-code neighborhood.

These ranges are downstream routing evidence only. The wider neighborhood may
overlap the direct cohort and other shared machinery, and neither range is a
Canic savings forecast. In particular, B1 must not assume that a downstream
4.5-12 KiB-per-entity slope applies to any Canic type or multiply it across a
Canic role. No downstream generator or data-access implementation
recommendation enters 0.110 scope.

## Temporary Pool Ledger Recovery Routing Evidence

The supplied live-recovery review identifies `pool_ledger_recovery` as a
temporary Canic helper, not a permanent Fleet role. It is installed briefly on
an empty pool canister to withdraw cycles held in that canister's own Cycles
Ledger account, then uninstalled after the Root verifies the withdrawal block,
zero Ledger balance and resulting native-cycle balance.

The currently reported compressed helper is roughly 195 KiB. That number is
only routing evidence for a controlled full-family ablation. It is neither a
code-section savings forecast nor part of the unrelated reported frontend
upload. Shared Root, Store, DTO, Candid and host machinery may have a different
marginal cost.

Deletion remains gated on immutable live evidence that both recoveries have
terminal Root receipts, both Ledger balances are zero, both helpers are absent,
both pool assets are terminal with conservation closed, the official Root is
restored and an immediate terminal Fleet replay has zero effect. Until then,
the same-operation recovery surface remains required even if the helper is
already absent from one pool canister.

## Post-v0.109.28 Wasm Routing Evidence

A read-only endpoint-heavy report against Canic source
`9d4a6339cfd57c7c468462b031eae70d31992218` and a slightly older retained
downstream artifact supplied the following overlapping shallow attribution:

| Overlapping family | Approximate bytes |
| --- | ---: |
| Ciborium involving Canic types | 725 KiB |
| Fleet activation CBOR family | 197 KiB |
| authorization CBOR family | 187 KiB |
| replay family | 64 KiB |
| intent family | 61 KiB |
| other Canic stable records | 186 KiB |

Secondary overlapping attribution reported approximately 203 KiB of
Canic-owned Candid work. Within that broad family, later routing separated
approximately 155 KiB around type construction/documentation from the
serialization family and roughly 28 KiB of Canic-owned type-documentation
functions. Payload-limited async adapters contributed another overlapping
roughly 33 KiB. Within the associated CBOR deserialization, map paths and
identifier/field visitors were also material and overlapping.

These figures are retained solely to route B1 experiments. The raw downstream
report is not a repository-owned reproducible artifact, and shared
Serde/Ciborium machinery may remain reachable after one family is removed.
No listed total is additive or recoverable by assumption.

## Required B1 Reproduction

B1 must replace routing evidence with controlled builds for:

1. the current accepted baseline;
2. disabled global storage registration;
3. excluded activation records/codecs;
4. excluded authorization records/codecs;
5. a bounded relevant-CBOR measurement stub;
6. excluded unconditional recovery dispatch;
7. current macro expansion compared with exact role/capability expansion;
8. excluded endpoint Candid type construction;
9. excluded Candid type-documentation generation;
10. excluded Candid serialization/newtype adapters;
11. excluded payload-limited async adapters;
12. independently excluded metrics providers;
13. independently excluded configuration/provisioning providers;
14. independently excluded command providers;
15. excluded timer/watchdog providers; and
16. excluded relevant status projection; and
17. a controlled Canic-generated generic cohort with equivalent nominal-type
    instantiations built from `1..=N`, where `N >= 2` is frozen from
    representative Canic demand rather than the downstream cohort; and
18. the complete temporary pool Ledger recovery family excluded as one paired
    projection, covering the helper artifact/build role, Store publication,
    Root state/workflow/status/endpoint, DTO/Candid and host planning/apply
    surfaces.

Each row records code-section, total and compressed bytes, the exact function
count from the frozen replica-equivalent validator, defined functions,
table/indirect entries and representative instruction deltas. The predecessor
rows above did not record the validator count and therefore cannot satisfy the
B1 function gate. Results are marginal only against their immediately
preceding immutable build. Overlapping symbol totals and ablation deltas are
never summed.

The generic cohort records the complete vector at every cohort size and the
incremental code-section, validator-function, defined-function and
table/indirect-entry delta for every additional instantiation. Its named
post-`-Oz` report maps remaining Canic-owned generic families to fully
qualified functions, concrete type arguments and specialized bodies. Shared
or folded bodies and nonlinear deltas remain explicit; no average slope is
extrapolated.

B1 also retains a generated-surface inventory for every canonical actor and
every public or internal macro that contributes runtime reachability. Each row
names the macro, validated role/capability input and expanded endpoint,
function, type, static, provider, dispatcher, serializer, timer and recovery
roots. Importing or splitting a macro is not savings evidence; paired builds
and optimized-artifact absence own that conclusion.

## Promotion Condition

The design may promote B2 only after the maintainer accepts:

- exact dated network code-section, total-module and declared-function limits
  plus the frozen replica-equivalent validator used to enforce them;
- the canonical role and endpoint-heavy fixture set;
- the complete reproducible baseline and ablation ledger;
- the `1..=N` Canic generic-instantiation cohort and named post-`-Oz`
  monomorphization report;
- the current generated-surface inventory for every canonical role;
- instruction/table allowances;
- destroyed-state and reconstruction inventory; and
- the independent absolute byte and function reserves.
