# 0.110 B1 Wasm Input Evidence

Date frozen: 2026-09-01
Last updated: 2026-09-23
Design owner: [0.110 Fleet runtime contraction](../../../design/0.110-fleet-runtime-contraction/0.110-design.md)
Frozen predecessor baseline: `3185dc45b` (`v0.109.35`)
Current released baseline: `50f40171d` (`v0.110.5`)

## Authority Boundary

This file retains the B1 review packet and planning inputs. The required
footprint evidence is complete and was accepted by the maintainer on 2026-09-23.
The retained runs reproduce their exact baseline and
controlled ablations from one immutable post-0.109 source, toolchain, feature
set and optimizer configuration.

The human maintainer accepted 0.109 closeout and explicitly promoted B1 on
2026-09-01. Complete B1 acceptance on 2026-09-23 authorizes sequenced B2.

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
risk score of `7/10`. This routes size investigation; it is not a network-limit
violation or correctness failure. It completes the canonical baseline row.

The active [experiment manifest](b1-controlled-ablation-manifest.md) now retains
rows 2–6 and 8–18 with exact controlled inputs and complete repeated
vectors. Canonical and runtime/payload/blob fixture evidence is included where
selected. The five-width cohort retains identical interfaces and named optimized
canonical body evidence from complete reference-preserving traces. Rows 9, 15
and 18 now complete independent retained verification. Row 7's already-
selected source/interface disposition is explicitly accepted, without a
numeric saving or optimized-absence claim. These states supersede
the initial September 3 qualification queue; completed builds need no replay.

The [generated-surface inventory](b1-generated-surface-inventory.md) now ties its
source trace to per-role optimized provider deltas while preserving the distinction
between `.5` and later working overlays. Differential attribution does not prove
safe removal or absence of every unselected implementation. The
[destroyed-state inventory](b1-destroyed-state-inventory.md) covers Canic allocations
and identifies non-reconstructable consumer domains; application reseed remains
outside the Canic release gate. The [historical-family ledger](b1-pool-ledger-recovery-hard-cut.md)
owns the controlled `.2` removal and separately measured helper. None of these
build-only stubs claims instruction, recovery or persistence parity. Complete B1
and its allowances were accepted on 2026-09-23; B2 is now authorized.

## Current absolute-budget review — 2026-09-23

The [row-8 controls](../../reports/2026-09/2026-09-22/b1-row8-measurement/artifact-metrics.tsv)
cover every canonical role and the runtime, payload and blob fixtures. The
[prepared cohort's width-1 control](../../reports/2026-09/2026-09-23/b1-row17-measurement/artifact-metrics.tsv)
supplies the leaf fixture. Each comes from its own verified two-clean-build
capture. These are absolute baseline checks, not a replacement for any
experiment's matched control, and the prepared leaf is not unprepared `.5`.

| Artifact | Code bytes | Code headroom | Defined functions | Function headroom |
| --- | ---: | ---: | ---: | ---: |
| `canonical_app` | 2,650,724 | 7,835,036 | 4,672 | 45,328 |
| `canonical_index_hub` | 2,444,607 | 8,041,153 | 4,346 | 45,654 |
| `canonical_test` | 2,944,773 | 7,540,987 | 5,189 | 44,811 |
| `canonical_user_hub` | 3,072,247 | 7,413,513 | 5,473 | 44,527 |
| `canonical_scale_hub` | 2,997,048 | 7,488,712 | 5,295 | 44,705 |
| `canonical_index_child` | 2,224,620 | 8,261,140 | 3,996 | 46,004 |
| `canonical_user_shard` | 3,001,399 | 7,484,361 | 5,341 | 44,659 |
| `canonical_scale_replica` | 2,660,428 | 7,825,332 | 4,700 | 45,300 |
| `canonical_root` | 6,659,745 | 3,826,015 | 9,596 | 40,404 |
| `canonical_fleet_coordinator` | 3,247,309 | 7,238,451 | 4,463 | 45,537 |
| `canonical_wasm_store` | 2,254,620 | 8,231,140 | 4,244 | 45,756 |
| `runtime_probe` | 2,198,713 | 8,287,047 | 4,192 | 45,808 |
| `payload_limit_probe` | 1,800,078 | 8,685,682 | 3,504 | 46,496 |
| `blob_storage_probe` | 2,092,495 | 8,393,265 | 3,950 | 46,050 |
| `leaf_probe` | 2,242,085 | 8,243,675 | 4,032 | 45,968 |

All fifteen controls exceed the binding reserves and remain below the strict
100 MiB total-module limit. Root is the largest: 7,097,219 total bytes in this
capture, leaving 3,826,015 code bytes and 40,404 defined functions. The one-byte
difference in code from the earlier v6 report is retained literally; neither capture
is substituted into another experiment. Gzip, data, table/element counts,
exports, declarations and hashes remain in the linked vectors.

The design's maximum 1% representative instruction-regression allowance
remains unchanged for later maintained production work. Build-only ablations
have explicit unavailable workload evidence and establish no instruction or
runtime-parity result. Overlapping reachability deltas do not raise this
allowance or forecast a recoverable sum. Unexplained indirect-table growth
also receives no advance approval. The maintainer accepted complete B1 on
2026-09-23; that decision does not waive B4 absence requirements or qualify
subsequent production changes.

### Build-resource evidence disposition — accepted 2026-09-23

The normative B1 vector also asks for clean/warm build time, peak RSS and
process/thread high-water marks. The retained ablation schema records artifact
and optimizer vectors, source/tool identities and clean repetition, but has no
resource columns. Recent progress logs record whole-second build durations;
they are neither controlled warm-build comparisons nor complete process-tree
resource measurements. The earlier v6 footprint evidence likewise supplies no
such resource vector. These missing quantities cannot be recovered from the
artifact bytes or inferred from determinism.

The maintainer explicitly accepts these B1 runs solely as
optimized footprint attribution, explicitly excluding build-speed and resource
claims, without repeating completed ablations to collect unrelated timing.
Any later build-performance claim must have its own controlled cold/warm and
resource qualification. This changes B1's evidence requirement only; it
preserves artifact determinism, absolute reserves, canonical symbol
mapping, explicit runtime-instruction absence, the maximum 1% maintained
workload allowance, and B4 optimized-absence requirements. This disposition
does not accept B1 overall or authorize B2/B3.

## Complete B1 review — accepted 2026-09-23

All eighteen catalog requirements now have their measurement or explicitly
accepted source disposition. The [manifest](b1-controlled-ablation-manifest.md)
binds each experiment to its evidence; no row remains an unqualified switch or
an active measurement. Final runs restore clean source/lockfiles and pass exact
clean-repetition checks. No broad workspace or PocketIC gate was run for this
audit continuation.

| Requirement | Evidence and disposition |
| --- | --- |
| Frozen canonical and capability-fixture baseline; absolute budgets | V6 plus the matched control vectors above cover all eleven roles and four fixtures. Every control exceeds both 5% reserves and satisfies the total-module bound. The prepared leaf is explicitly distinct from unprepared `.5`. |
| Controlled attribution | Rows 2–6 and 8–18 retain complete selected matrices. [Provider results](../../reports/2026-09/2026-09-23/b1-provider-measurements.md), [documentation results](../../reports/2026-09/2026-09-23/b1-prepared-experiments.md#retained-type-documentation-result) and the [historical-family result](b1-pool-ledger-recovery-hard-cut.md#retained-family-only-result--2026-09-23) close the final measurements. Deltas overlap and are not a savings forecast. |
| Exact role expansion | Row 7's source/interface disposition is already accepted; no numeric saving or optimized-absence claim. |
| Five-width generic cohort and named optimized bodies | [Cohort evidence](b1-generic-instantiation-cohort.md) includes exact repeats and canonical function-reference-preserving mappings at all widths. Missing names do not independently prove elimination or folding. |
| Generated surfaces | The [inventory](b1-generated-surface-inventory.md) links source selection to optimized attribution. Final role-inapplicable absence and maintained-behavior proof remain B4 obligations. |
| Destroyed state and reconstruction boundaries | The [allocation inventory](b1-destroyed-state-inventory.md) names non-reconstructable domains and current cycle-conservation preconditions. Historical `.5` inputs promise no predecessor identity, topology or state reuse. Each affected production cut still needs current safety evidence. |
| Allowances and unavailable evidence | Retain the maximum 1% representative instruction-regression allowance for maintained production work. Unexplained indirect-table growth is not approved. Build-only experiments supply no runtime-parity result; the accepted footprint-only disposition supplies no build-speed/resource claim. |

The maintainer explicitly accepted complete B1 and authorized the sequenced B2
work on 2026-09-23. Acceptance covers the immutable footprint attribution and
stated boundaries; role-selected storage is now the active batch.
That future work must measure its current matched workload and optimized
artifacts; these audit stubs are not production implementations. Human B1
acceptance remains distinct from a release instruction and the later human
minor-closeout audit.

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
