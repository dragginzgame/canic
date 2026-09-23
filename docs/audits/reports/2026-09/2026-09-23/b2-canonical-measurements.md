# B2 canonical storage checkpoint

Date: 2026-09-23. B2 remains active; this is footprint evidence, not complete
runtime or release qualification.

## Matched inputs and method

The comparison spans the cumulative B2 cuts: lazy stable TLS, direct selected
startup/restoration, and endpoint-selected synchronous Fleet-admission readers.
The baseline is the immediately preceding dirty-source snapshot recorded in the
[B2 working ledger](../../../working/0.110-fleet-runtime-contraction/b2-storage-reachability.md).
Of 1,994 captured files, only 48 B2 source/test files differ. Manifests, lock,
configuration, compiler, tools and the host artifact pipeline match. This is a
fresh current-source comparison; immutable B1 ablations remain attribution only.

Both conditions use the same isolated checkout and target paths. Each starts
with its own fresh Wasm target; later role builds reuse only that condition's
dependencies. Source replacement writes changed bytes with current timestamps.
The actual host Release builder applies shrink, public Candid metadata, Binaryen
132 `-Oz`, gzip and validation. No production optimization is omitted to retain
names. These runs make no build-time, resource or clean-repeat claim.

The [structured measurements](b2-canonical-measurements.json) retain exact
vectors, source/tool identities, selection/interface parity, frozen reserves,
excluded attempts and evidence hashes. The
[compressed records](b2-canonical-records.tar.gz) contain the method, source
manifest, logs, interface/selection records and verified function mappings.
Raw Wasm, full linker maps and source archives remain under
`.tmp/b2-canonical-20260923/`.

## Completed pairs

| Artifact | Code bytes before | Code bytes after | Code delta | Defined-function delta | Table-slot delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| app | 2,724,357 | 2,698,747 | -25,610 | -51 | -22 |
| test | 2,977,943 | 2,981,671 | +3,728 | -15 | -20 |
| root | 7,530,530 | 7,505,625 | -24,905 | -65 | -46 |
| fleet_coordinator | 3,849,246 | 3,821,562 | -27,684 | -53 | -25 |
| wasm_store | 2,655,194 | 2,630,418 | -24,776 | -62 | -32 |
| index_hub | 2,832,155 | 2,807,249 | -24,906 | -48 | -22 |
| index_child | 2,696,309 | 2,699,633 | +3,324 | -16 | -20 |
| user_hub | 2,923,024 | 2,925,919 | +2,895 | -15 | -21 |
| user_shard | 3,170,765 | 3,174,060 | +3,295 | -22 | -20 |
| scale_hub | 2,813,716 | 2,787,750 | -25,966 | -49 | -22 |
| scale_replica | 2,732,573 | 2,706,963 | -25,610 | -51 | -22 |
| runtime_probe | 2,527,494 | 2,504,454 | -23,040 | -54 | -22 |
| payload_limit_probe | 1,909,338 | 1,885,559 | -23,779 | -58 | -22 |
| blob_storage_probe | 2,112,848 | 2,089,125 | -23,723 | -63 | -26 |
| leaf_probe | 2,719,375 | 2,722,688 | +3,313 | -16 | -20 |

These final artifacts preserve byte-identical Candid, export names/kinds and
selected capabilities. Independent section parsing, the frozen repository
replica-local-function counter and the host optimizer's metric agree. Gzip
decompresses to the exact measured Wasm; WABT structural validation and `didc`
checking pass. All fifteen satisfy the frozen code/function reserves and total-module
limit. The eleven canonical roles collectively lose 166,215 code bytes, 447
functions and 272 table slots. The four fixtures separately lose 67,229 bytes,
191 functions and 90 slots. These sums describe separate deployed modules, not
one canister's headroom. Four canonical roles and the leaf grow code by
2,895–3,728 bytes while losing functions and table slots. No table grows.
The earlier mixed-guard diagnostic is not a canonical result.

Root remains the largest artifact: 7,505,625 code bytes, 10,112 local functions
and 7,997,045 total bytes. Its code reserve is 2,980,135 bytes and its function
reserve is 39,888. Both independent frozen reserves pass.

## Optimized correspondence

A linker map binds symbols to raw compiler body offsets, sizes and hashes. A
separate name-preserving pass repeats the actual shrink/Candid/optimizer steps.
The method verifies a complete bijection onto canonical function bodies:
signatures, locals, non-reference instructions, every direct reference, and
import/export/table bindings. Other non-custom sections match exactly, except
type/function indices normalized by that verified mapping. Function mappings
need not be unique where implementations are equivalent.

For large modules, anchor binding and constraint propagation replace exhaustive
enumeration of indistinguishable functions. The independent final assertions
still check every mapped body and reference. Small controls accept equivalent
bodies and reject altered bodies or wrong export bindings. The first exhaustive
Root attempt timed out and is excluded; the corrected method verifies all
10,223 functions including imports in its baseline artifact.

The app goes from eleven named Fleet-admission projection bodies to none. The
admission-enabled test retains fifteen. The app selects only the `false`
evaluator; the test contains both `false` and `true` bodies, explaining the
new generic specialization. Both roles deliberately select automatic top-ups,
so their recovery owners remain required. Stable auth, sharding and
control-plane storage have no named bodies in either role. These are exact
named-body observations linked to canonical output; absence of names alone
does not establish absence of inlined code.

## First-access-inclusive instruction evidence

A measurement-only wrapper surrounds the actual synchronous initialization,
post-upgrade and sampling exports. Every original function body remains
byte-identical, original non-executable custom metadata is restored, and Wasm
validation passes. Two additional globals retain the cost and stage; a diagnostic
query reads them without changing the update heap. There is no product endpoint
or source modification. The same counter/call bookkeeping is included in both
conditions, without subtraction. Deferred timers, later callbacks, compilation
and management-canister install costs are outside this window.

The runtime fixture uses a fixed application-subnet time and fresh canisters at
1, 211 and 256 application metric rows. Each condition covers the first and
second sample after installation, then the first and second sample after
restoring the identical Wasm. Typed results, valid chart values, nonempty bounded
history and exact instrumentation counts pass. All instruction fields repeat
exactly in a second independent run; this is execution reproducibility, not an
independent clean-build repetition.

- Synchronous runtime initialization falls from 11,466,352 to 9,297,924
  instructions, approximately 18.91%.
- Synchronous restoration falls from 12,444,023 to 11,182,463, approximately
  10.14%.
- All twelve complete sampling exports remain within the frozen 1% allowance;
  the largest increase is approximately 0.672%. The internal callback's largest
  increase is approximately 0.734%.
- Two exact chart-query subspans regress: at 256 rows after installation, the
  first query rises from 170,990 to 251,186 instructions, and the second from
  91,139 to 171,097. Other corresponding query states are nearly unchanged or
  cheaper. Per-load maxima across the four observed states remain close, but
  those maxima do not waive the two individual observations.

The chart timer starts inside the handler and excludes preflight, authorization
and response encoding. A separate minimal control in the same pinned PocketIC
engine reports 232 instructions without a heap write, 80,248 with the first
write, and 80,264 with a neighbouring write. Growing zero, one or two Wasm pages
without touching them costs 200 in each control. The results are consistent with
memory-access charging and changed heap placement, rather than growth alone;
they do not identify the actual allocation responsible for the two chart cases.
The upstream [embedding configuration](https://github.com/dfinity/ic/blob/master/rs/config/src/embedders.rs)
documents memory-access/write charges. Its current defaults are not asserted to
be the pinned simulator's configuration. No arbitrary absolute instruction cap
or new CI gate was added, and the accepted allowance was not relaxed.

The structured report retains all raw vectors, exact-repeat checks, wrapper
proofs, pinned simulator hash and the control method. Sampling qualification is
positive evidence; complete runtime qualification remains open.

## Retained-evidence verification

Independent readback verifies all 446 compressed records against their local
sources, all fifteen artifact pairs and trace hashes, frozen reserves, interface
parity and shrinking tables. All 1,994 candidate source hashes match both the
shared checkout and isolated product snapshot. Runtime disposition preserves
the two failing query observations. Report links and whitespace checks pass;
the current-document semantics guard passes with two existing advisory warnings
about the parked metrics-history design file. No product source changed during
this measurement checkpoint, and no broad test suite ran.

## Residual ownership and continuation

Root's remaining three broadly named admission functions construct and hash
child projections. Its admission **storage** bodies are absent; those payload
owners must not be mistaken for an unselected Root-owned projection store.

Root still has named local template chunk-set readers. One retained path is
`source_chunk_set_info_for_manifest` in
`crates/canic-control-plane/src/workflow/runtime/template/publication/release/source.rs`:
the bootstrap-binding branch selects local `TemplateChunkedOps` instead of a
Store call. Another path is `resolved_bootstrap_chunk_set_for_manifest` in the
parent template workflow. Root's approved-manifest mirror is an explicit
maintained owner; do not delete it merely because its storage module also serves
the Wasm Store. Audit the exact local-bootstrap demand before removing those
remaining readers or claiming full role-inapplicable-storage absence.

The leaf comparison uses the maintained default generic width of one, with its
explicit admission predicate as a positive control. Named `Page<T>` body counts
are unchanged at that width. This is not a fresh five-width generic-cohort
qualification. The new `false`/`true` access-evaluator bodies are retained in the
named report; finish the cohort disposition with the residual-owner review.

Decision: continue B2. All canonical roles and affected fixture artifacts now
have matched footprint evidence. Next resolve the reproducible chart-query
allocation effect, audit the Root-local bootstrap readers and finish the
representative workload/generic disposition. B3 has not started. B2 and the
complete release batch are not push-ready. No broad gate, version transaction,
commit, publication or downstream mutation ran.
