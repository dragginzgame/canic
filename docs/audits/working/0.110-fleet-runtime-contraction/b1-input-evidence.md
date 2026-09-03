# 0.110 B1 Wasm Input Evidence

Date frozen: 2026-09-01
Last updated: 2026-09-03
Design owner: [0.110 Fleet runtime contraction](../../../design/0.110-fleet-runtime-contraction/0.110-design.md)
Frozen predecessor baseline: `3185dc45b` (`v0.109.35`)
Current released baseline: `50f40171d` (`v0.110.5`)

## Authority Boundary

This file retains planning inputs and the active B1 ledger. It is not B1
completion evidence. B1 must reproduce its accepted baseline and
controlled ablations from one immutable post-0.109 source, toolchain, feature
set and optimizer configuration.

The human maintainer accepted 0.109 closeout and explicitly promoted B1 on
2026-09-01. B2 remains blocked on accepted B1 completion evidence.

## Frozen Network Limits

The dated 2026-09-01 authority is the IC
[resource-limit reference](https://docs.internetcomputer.org/references/resource-limits/)
and its normative
[WebAssembly module requirements](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/#webassembly-module-requirements):

| Limit | Frozen value | Binding 5% reserve |
| --- | ---: | ---: |
| Wasm code section | 10,485,760 bytes | 524,288 bytes |
| Total Wasm module | 104,857,600 bytes | independent strict upper bound |
| Replica-limited defined functions | 50,000 | 2,500 functions |

The frozen function interpretation comes from DFINITY `ic` commit
`2f8dc21e2e5c37a4cae7f65d2a4230ac8f143e5a`. Its
[replica validation source](https://github.com/dfinity/ic/blob/2f8dc21e2e5c37a4cae7f65d2a4230ac8f143e5a/rs/embedders/src/wasm_utils/validation.rs#L1301-L1325)
collects only functions for which `is_local()` is true before applying
`max_functions`; imports are validated separately. Its
[embedder configuration](https://github.com/dfinity/ic/blob/2f8dc21e2e5c37a4cae7f65d2a4230ac8f143e5a/rs/config/src/embedders.rs#L21-L26)
sets the default to 50,000. The binding quantity is therefore the function-
section/code-body count, not `ic-wasm`'s imports-plus-definitions total.

The repository-owned `scripts/ci/wasm-replica-function-count.rs` counter
freezes that interpretation. The ablation runner compiles it into private
invocation scratch, records source and executable hashes plus its exact
identity, runs it only after `wasm-validate`, and rejects disagreement with the
builder's independent optimizer-defined count.

## Baseline Attempts

The first isolated `v0.109.35` run built the canonical App artifact, then
stopped before retaining evidence because the v4 optimizer parser required the
obsolete final `<role>/<role>.wasm` log path. The current builder correctly
reports a path-confined staged `candidate.wasm`. `CANIC-WASM-001/v5` assigns
one log to each role build and binds the exact metric record independently of
that disposable path. No partial run is treated as baseline evidence.

The next complete run proved the role-local parser over all 27 role/profile
builds, but exposed contradictory hard-coded v4 prose in the generated v5
report. That attempt is retained as explicitly invalid evidence. The report
generator and executable method fingerprint were then corrected and the whole
audit was rerun from the same clean immutable source; no build output from the
invalid attempt was reused as artifact evidence.

## Canonical v5 Baseline

The corrected
[CANIC-WASM-001/v5 baseline](../../reports/2026-09/2026-09-01/wasm-footprint-v5-2.md)
passed from `v0.109.35` source `3185dc45b` with executable method fingerprint
`e5fea20658708141f9ec95545536c73306fe725f5410567a045cb8ce5df8cc27`.
It built all nine canonical roles twice from independent clean release targets
and once from a debug target. Exact Wasm, gzip, Candid and optimizer metrics
matched between release builds; all artifacts passed the governed host build,
gzip integrity, `ic-wasm` and bounded `twiggy` analyses.

The largest baseline role is the Fleet Subnet Root:

| Quantity | Observed | Frozen limit | Absolute headroom |
| --- | ---: | ---: | ---: |
| code section | 6,709,592 bytes | 10,485,760 bytes | 3,776,168 bytes |
| total module | 7,149,541 bytes | 104,857,600 bytes | 97,708,059 bytes |
| replica-limited defined functions | 9,633 | 50,000 | 40,367 |
| `ic-wasm` total functions | 9,678 | not the replica limit | N/A |

Every repository-owned canonical role therefore exceeds both binding 5%
reserves at the accepted predecessor. At that predecessor capture, the
repository capability fixtures and controlled ablation, generated-surface,
generic-cohort and destroyed-state evidence were still outstanding. The
baseline risk score was `6/10`, driven by first-method
baseline status, Component spread, Root-to-Component size ratio and the large
retained indirect-call table rather than a limit violation.

The repository-owned predecessor measurement is
[0.109 B9 governed release Wasm optimization](../0.109-fleet-wide-ingress-admission/b9-release-wasm-optimization.md).
That report records the canonical Binaryen finalizer, deterministic artifact
measurements and named residual analysis. It shows that the general optimizer
and small sort follow-up have already been consumed; it does not measure the
storage ablations proposed by 0.110.

## Current v6 Released Baseline

The valid
[CANIC-WASM-001/v6 baseline](../../reports/2026-09/2026-09-03/wasm-footprint-v6.md)
reproduces the current immutable `v0.110.5` source `50f40171d` with the complete
eleven-role roster. V6 retains the governed build authority and two-clean-build
determinism while adding the configured indexed Hub and child roles introduced
by the managed Component-tree qualification surface. Every retained evidence
hash verifies, and the audit-method catalog guard accepts the revised method.

The largest current role remains the Fleet Subnet Root:

| Quantity | Observed | Frozen limit | Absolute headroom |
| --- | ---: | ---: | ---: |
| code section | 6,659,744 bytes | 10,485,760 bytes | 3,826,016 bytes |
| total module | 7,097,112 bytes | 104,857,600 bytes | 97,760,488 bytes |
| replica-limited defined functions | 9,596 | 50,000 | 40,404 |
| `ic-wasm` total functions | 9,641 | not the replica limit | N/A |

The run result is `fail` with valid evidence because the method's first-v6-
baseline, Component-spread, Root-ratio and retained-table inputs produce a
defined risk score of `7/10`. This is size-pressure routing evidence, not a
limit violation or correctness failure. It completes the current canonical-
role baseline row only. The
[generated-surface inventory](b1-generated-surface-inventory.md) now completes
the source trace while keeping the immutable baseline distinct from the
working-tree overlay; optimized-artifact absence remains open. The repository-
owned capability fixture matrix and compatible predecessor comparison remain
open. The machine-checked
[controlled-ablation manifest](b1-controlled-ablation-manifest.md) now freezes
the required rows and runner. Its global-registration patch has complete
eleven-role release-build qualification and awaits immutable measurement. The
inclusive activation-persistence, authorization stable-codec, shared-CBOR-
helper, watchdog-recovery-dispatch, endpoint-declaration-construction,
endpoint-reply-serialization and metrics-provider patches remain specified and
await complete selected-artifact qualification and immutable measurement. The
payload-limited raw-
adapter now has an immutable selected-fixture measurement: it accounts for 967
optimized code-section bytes and zero defined functions, so its independent
canister-origin payload bound is retained. The exact-role expansion, type-
documentation and remaining
provider source switches remain open. The
[destroyed-state inventory](b1-destroyed-state-inventory.md) now covers every
Canic allocation and names the non-reconstructable consumer domains, while
keeping application reseed outside Canic's release gate. Maintainer acceptance
of the hard-cut preconditions remains open. The
[generic-instantiation cohort](b1-generic-instantiation-cohort.md) freezes the
Canic-owned `Page<T>` family at `N = 5` from current generated status demand and
adds the fixed audit-only source fixture; its immutable optimized deltas and
named post-`-Oz` mapping remain open.

## Published Predecessor Measurements

The 0.109 report records these relevant optimized results:

| Artifact/evidence | Code-section bytes | Optimizer-defined functions | Independent counter | Meaning |
| --- | ---: | ---: | ---: | --- |
| supplied Toko baseline after Binaryen 108 | 5,290,184 | 10,887 | not recorded | downstream evidence, not a Canic rerun |
| subsequent canonical Canic release App | 2,800,001 | 4,963 | not recorded | repository-owned final artifact after the bounded sort follow-up |
| managed leaf audit role after Binaryen 132 | 2,316,160 | 4,183 | not recorded | exact role-surface pruning evidence, not a controlled before/after pair |

The named Canic residual report found approximately 135 KiB of
monomorphization bloat after optimization, dominated by stable-sort machinery.
The accepted source follow-up recovered another 27,665 code-section bytes and
46 defined functions. Those savings are predecessor facts, not 0.110 forecasts.

## Non-Binding Downstream Pressure Observation

The 2026-08-31 downstream review supplied a current Toko `project_instance`
observation with:

| Measurement | Supplied value |
| --- | ---: |
| code-section bytes | 10,275,629 |
| retained 10 MiB limit headroom | 210,131 |
| exported Candid methods | 268 |
| reduction required for 512 KiB headroom | 314,157 |

The same review reports a combined surface containing delegated-token and
Root-signature verification, blob storage and billing, extensive IcyDB-
generated entity/query code, lifecycle, status and recovery behavior. This is
materially larger and broader than the earlier 5,290,184-byte downstream row.
It demonstrates consumer pressure, not a Canic-owned protocol requirement.

These supplied values are routing evidence, not a reproducible B1 row. The
[downstream pressure ledger](b1-toko-canary-and-reseed.md) recovers the exact
local artifact and Candid hashes, reconfirms the 10,275,629-byte code section
and 268-method service, and binds the presently correlated dirty snapshot. No
immutable source commit, clean-source assertion, complete optimizer manifest,
replica-validator-equivalent counter or instruction vector accompanies the
artifact.
The row therefore remains exactly what its provenance supports: a hash-bound
observation. B1 does not require a Toko/TokoMiner commit, lockfile or
application discard/reseed decision and will not perform further downstream-
specific work unless a result directly routes a Canic-owned change.

The binding replacement is the repository-owned capability fixture matrix:
`runtime_probe`, `payload_limit_probe`, `blob_storage_probe` and `leaf_probe`
alongside the canonical eleven roles. It covers both Canic authentication
paths, blob economics, lifecycle, status, metrics, timers, recovery, payload
adapters and the generic cohort without importing consumer application or
generated-model machinery.

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

## Deleted Pool Ledger Recovery Routing Evidence

Published `v0.110.3` hard-deletes `pool_ledger_recovery`, which was an incident-
specific helper rather than a Fleet role. The
[pool Ledger recovery hard-cut ledger](b1-pool-ledger-recovery-hard-cut.md)
proves current source-family absence across artifact build, Store publication,
Root state/workflow/status/endpoint, DTO/Candid, host planning/apply, tests and
CI. B1 must still measure the immutable predecessor against a compatible
current candidate and report the actual whole-family marginal artifact delta.

The earlier roughly 195 KiB compressed helper report remains routing evidence
only. It is neither a code-section savings result nor additive with shared
Root, Store, DTO, Candid or host machinery.

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
15. excluded timer/watchdog providers;
16. excluded relevant status projection;
17. a controlled Canic-generated generic cohort with equivalent nominal-type
    instantiations built from `1..=N`, where `N >= 2` is frozen from
    representative Canic demand rather than the downstream cohort; and
18. the immutable predecessor compared with the current candidate after the
    complete pool Ledger recovery family was deleted, covering the helper
    artifact/build role, Store publication, Root state/workflow/status/endpoint,
    DTO/Candid, host planning/apply, tests and CI.

Each row records code-section, total and compressed bytes, the exact local/
defined-function count from the frozen replica-validator-equivalent counter,
the optimizer-defined cross-check, `ic-wasm`'s non-binding total,
table/indirect entries and representative instruction deltas. The predecessor
rows above did not run the independent counter and therefore cannot by
themselves satisfy the B1 function gate, although their optimizer-defined
metrics expose the same quantity for correction and planning. Results are
marginal only against their immediately preceding immutable build. Overlapping
symbol totals and ablation deltas are never summed.

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

If accepted Canic-owned contraction cannot place the canonical roster and
repository-owned fixture matrix above both absolute reserves, the evidence
must stop with an exact Canic-owned residual handoff. Optional immutable
consumer observations may separate framework and application pressure, but
they neither block Canic nor authorize downstream mutation.

## Promotion Condition

The design may promote B2 only after the maintainer accepts:

- exact dated network code-section, total-module and defined-function limits
  plus the frozen replica-validator-equivalent counter and IC source used to
  define the binding quantity;
- the canonical role and repository-owned `runtime_probe`,
  `payload_limit_probe`, `blob_storage_probe` and `leaf_probe` fixture set;
- the complete reproducible baseline and ablation ledger;
- the `1..=N` Canic generic-instantiation cohort and named post-`-Oz`
  monomorphization report;
- the current generated-surface inventory for every canonical role;
- instruction/table allowances;
- destroyed-state and reconstruction inventory; and
- the independent 5% absolute byte and function reserves for every binding
  Canic-owned artifact, including 512 KiB of code-section reserve under the
  retained 10 MiB limit.
