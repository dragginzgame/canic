# B1 Generic-Instantiation Cohort

## State — 2026-09-23

The retained five-width measurement is complete and independently verified.
Both clean repetitions at every width have identical Wasm, gzip, Candid and
metric vectors. All widths preserve the same declaration and exported-method
identities. The prepared runner is integrated and the main catalog is `ready`.
The named post-`-Oz` report now maps canonical bodies through a complete
reference-preserving linker/optimizer trace at all five widths. Instruction
evidence remains explicitly absent; this result authorizes no production
generic change and does not independently close B1.

## Frozen Fixture Preparation — 2026-09-22

Immutable `v0.110.5` predates the cohort: its `leaf_probe` build script ignores
`CANIC_GENERIC_COHORT_WIDTH`. The cohort first appears in commit
`c563de9b596fd1ff9a7a663836bb1c64360e7d38` (`0.110.6`). No measurement from
that inert width input was accepted. The earlier `ready` catalog state was
corrected before executing the experiment.

The audit-only preparation patch is
`scripts/ci/wasm-ablation-patches/b1-17-page-generic-cohort-fixture.patch`, SHA-256
`d327b086e98166d92e3a44497d05e7af959ab8b5be50b943d9b1ce90d740b8b0`.
It copies only the first cohort fixture's library, build script and already
resolved `serde` dependency edge onto `.5`. No package version/checksum changes;
later frontend-admission and recovery-balance probes are excluded. The exact
same preparation applies at every width, including width 1. Only nominal type
count varies. The original unprepared `.5` leaf is not this experiment's control.

All five widths pass scoped offline locked native Clippy; invalid width inputs
reject. Source checks establish only source qualification. The isolated method
then passes its runner regression, ShellCheck and complete five-width artifact
qualification, including independent payload and cross-width interface checks.
The [qualification evidence](../../reports/2026-09/2026-09-22/b1-row17-qualification/verification.json)
retains that single-pass result separately from repeated measurement.

The qualified runner SHA-256 is
`b58e97cfc3d667dfe1d21871ebc49a973857f31c496575023bd338d591805635`.
It binds preparation by digest, applies it before width 1, checks the exact
source diff after every width, records both lock identities and restores the
checkout. Its [integration patch](../../reports/2026-09/2026-09-22/b1-row17-qualification/method-integration.patch)
was applied on 2026-09-23 after all shared-method owners completed. The focused
runner regression and ShellCheck pass on the integrated method. Frozen method
copies for other experiments remain unchanged.

## Retained footprint — 2026-09-23

The [complete vectors](../../reports/2026-09/2026-09-23/b1-row17-measurement/artifact-metrics.tsv),
[determinism records](../../reports/2026-09/2026-09-23/b1-row17-measurement/determinism.tsv)
and [independent verification](../../reports/2026-09/2026-09-23/b1-row17-measurement/verification.json)
retain all ten builds. Verification covers matrix membership, exact repeated
metrics, all payload hashes/lengths, gzip roundtrips, method/harness hashes,
product and prepared locks, and direct Wasm section/function counts. Every
width retains the same 27,628-byte Candid declaration and thirteen exports,
including `audit_page_generic_cohort`.

| Width | Code bytes | Defined functions | Gzip bytes | Marginal code bytes | Marginal functions |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 2,242,085 | 4,032 | 892,344 | — | — |
| 2 | 2,243,726 | 4,035 | 892,839 | +1,641 | +3 |
| 3 | 2,245,347 | 4,038 | 893,252 | +1,621 | +3 |
| 4 | 2,246,724 | 4,041 | 893,614 | +1,377 | +3 |
| 5 | 2,248,580 | 4,042 | 893,686 | +1,856 | +1 |

Width 5 minus width 1 adds 6,495 code bytes, 7,114 raw Wasm bytes, 1,342 gzip
bytes, 585 data bytes, ten defined functions and twelve table/element entries.
Marginal functions are three, three, three and one. Report these nonlinear
results literally; they do not establish a universal per-type cost, an
application scaling rule or a recoverable production saving.

The retained run is
`.tmp/b1-cohort-method-20260922/retained/b1-17-page-generic-cohort-50f40171d617`.
Its product checkout restores cleanly. The canonical artifacts remain stripped;
zero rows in `twiggy monos` cannot establish absence of generic instantiations.
The canonical symbol trace below supplies verified specialized-body mapping;
the older strip-none diagnostic retains its explicit executable limitation. No PocketIC instruction or runtime-parity result is claimed.
Earlier exploratory measurements below remain historical, unqualified context
and do not supply any value in this retained result.

## Canonical symbol trace — complete, 2026-09-23

The [canonical body map](../../reports/2026-09/2026-09-23/b1-row17-canonical-symbols/canonical-bodies.json)
closes the larger-body gap left by the separate strip-none diagnostic below.
It names 15/18/21/24/25 selected optimized functions at widths 1–5, including
concrete Page type/serializer/drop bodies and their type-identity helpers.
Every entry binds the original canonical function index, body length and hash.
The [verification](../../reports/2026-09/2026-09-23/b1-row17-canonical-symbols/verification.json)
and [trace method](../../reports/2026-09/2026-09-23/b1-row17-canonical-symbols/trace-method.md)
record the complete function-reference witnesses and pinned inputs.

The diagnostic retains ordinary strip and optimization settings and captures
linker symbols from exact raw body offsets. Rebuilt executable sections match
the canonical measurements exactly. Initialized data differs only in fixed-width
private scratch-path characters; no address or other data changes. Carrying
those recovered names through the real finalizer can reorder function/type
indices, so the trace verifies all signatures/instructions and a complete
bijection of 4,071/4,074/4,077/4,080/4,081 functions and imports. Every direct
call and export/table/import binding must agree under that one mapping; all
other sections agree exactly with the diagnostic's canonical build. Names are
then projected onto the original canonical bodies. Equal totals or similar
source text are not used as a substitute for this proof.

At widths 1–4, the canonical nominal Page type and serializer bodies are 935
and 636 bytes. The maintained `Page<LogEntry>` type and serializer bodies are
1,088/848 bytes at widths 1–3 and 1,089/849 at widths 4–5; their drop body is
118 bytes. At width 5 the two separate nominal Page entries are no longer
named, while the fifteen nominal type-identity bodies remain. Missing names
alone do not prove elimination or a particular inlining/folding mechanism.
Use the retained inclusive and nonlinear marginal vectors above; never sum
these body sizes into a savings forecast. No new footprint, determinism,
runtime-instruction or build-performance result is claimed by the symbol trace.

## Named diagnostic — verified, 2026-09-23

The separate audit patch
`scripts/ci/wasm-ablation-patches/b1-17-page-generic-cohort-named-diagnostic.patch`
has SHA-256
`d907594ad04aad680617833523d73a8466b137784e1e552c96ebb350b0a2c095`.
It includes the identical cohort preparation, then preserves Rust symbols,
passes `--keep-name-section` through shrink and Candid metadata transforms,
and asks Binaryen to emit names with `-g`. Rust optimization/LTO/codegen units
and Binaryen 132 `-Oz` remain unchanged. The canonical catalog still selects
the original preparation patch; no ordinary product option is added.

All five diagnostic widths complete. The
[named body map](../../reports/2026-09/2026-09-23/b1-row17-named-diagnostic/named-bodies.json)
records exact diagnostic function indices, fully qualified concrete types,
mangled names, body lengths and SHA-256 digests. Independent
[verification](../../reports/2026-09/2026-09-23/b1-row17-named-diagnostic/verification.json)
checks every payload, method identity, direct function count and restored
checkout, and compares every non-custom section against the canonical width.
This is single-pass diagnostic evidence, not a determinism result.

Name preservation changes executable output: each diagnostic has 71 fewer code
bytes and ten fewer defined functions than its canonical counterpart. Candid
is identical. The width-to-width code and function deltas remain identical,
but the executable sections and function indices do not. Consequently diagnostic indices cannot be copied onto canonical artifacts;
larger canonical-body mapping is supplied only by the later linker trace above.
Do not substitute diagnostic digests or function counts for the canonical run.

A narrower canonical mapping is verified for every selected `TypeId::of` body:
10, 13, 16, 19 and 22 bodies at successive widths, including the other maintained
page instantiations. Each has one unique identical canonical body and the same
`(param i32) -> ()` signature. Its concrete type-name bytes match in initialized
memory, and its stored function-table identity resolves to that exact matched
function in both modules. Thus this mapping checks the embedded data and table
reference as well as instruction bytes. Larger type/serializer bodies are not
covered by that proof.

Within the diagnostic, widths one through four retain named `Page<Nominal1>`
Candid `ty` and serializer bodies of 935 and 636 bytes. Each new nominal type
adds three 26-byte `TypeId::of` bodies, for the element, `Vec<Element>` and
`Page<Element>`. At width five, the fifteen identity bodies remain named but
the two larger `Page<Nominal1>` bodies no longer have separate named entries;
the net defined-function increase is one. The map also identifies maintained
`Page<LogEntry>` type/serializer/drop bodies and identity bodies for
`Page<CycleTrackerEntry>` and `Page<MetricEntry>`. Missing names alone do not
prove elimination, inlining or identical-code folding. The inclusive code
increase includes changes outside these small identity bodies and must not be
reported as their summed body size.

## Selected Family And Width

The selected family is `canic_core::dto::page::Page<T>`, including the Candid
construction and serialization bodies reached when the family appears in a
macro-generated endpoint response. It is Canic-owned boundary data, and it is
reached directly by both generated Component status surfaces in
`crates/canic/src/macros/endpoints/role.rs`.

The largest current single-role demand is five distinct element types:

| Generated status response | Element type | Reachability |
| --- | --- | --- |
| `Children` | `CanisterInfo` | `ChildProvisioning` capability |
| `CycleHistory` | `CycleTrackerEntry` | every generated Component status surface |
| `CycleTopups` | `CycleTopupEvent` | `AutomaticTopup` capability |
| `Logs` | `LogEntry` | every generated Component status surface |
| `Metrics` | `MetricEntry` | every generated Component status surface |

The canonical `user_hub` and `scale_hub` roles have both gated capabilities and
therefore reach all five. B1 freezes `N = 5` from that current Canic demand.
The downstream 11-entity observation did not select or scale this cohort.

`Page<T>` is preferable to an arbitrary storage or RPC helper for this
experiment: the family is visibly repeated in one canonical generated surface,
its element type is the isolated variable, and its post-optimization folding is
exactly the behavior B1 needs to observe. The experiment does not imply that
pagination is sediment or that the five production element types have
equivalent semantics.

## Fixed Attribution Fixture

The unpublished `canisters/audit/leaf_probe` package now contains the
`audit_page_generic_cohort` query. It remains an audit-only local canister and
does not enter `apps/test/canic.toml`, the eleven-role release roster, a Store
publication manifest or a product role.

The fixture defines up to five nominal record types. Each has exactly the same
Rust and Candid shape:

```text
record { value : nat64 }
```

It also retains exactly five response variants, five constructors, five match
branches and one exported query at every cohort width. The build input
`CANIC_GENERIC_COHORT_WIDTH` accepts only `1..=5` and changes cumulative type
aliases as follows:

| Width | Distinct nominal element types | Remaining slots |
| ---: | --- | --- |
| 1 | slot 1 | alias nominal type 1 |
| 2 | slots 1-2 | alias nominal type 1 |
| 3 | slots 1-3 | alias nominal type 1 |
| 4 | slots 1-4 | alias nominal type 1 |
| 5 | slots 1-5 | none |

Thus the endpoint name, argument shape, response variant inventory, control
flow, constructor count and wire field shape remain fixed. Moving from `k - 1`
to `k` makes only nominal type `k` and its `Page<T>` instantiation reachable.
All five source declarations are present in the same tracked fixture; types
above the selected width are excluded before Rust reachability analysis.

The width is a measurement switch, not a Cargo feature, runtime option or
published capability. The default is width 1. Invalid, zero and above-five
values reject in the package build script.

## Frozen Measurement Protocol

The retained measurement must use one clean disposable linked worktree at an
exact candidate commit and one fixed absolute worktree and target path. For
each `k` in `1..=5`, it must:

1. remove and recreate the same isolated target and `.icp` output directories;
2. build the audit App through the authoritative `canic-host` artifact builder
   with `canisters/audit/leaf_probe/canic.toml`, release profile, offline Cargo,
   disabled incremental compilation and `CANIC_GENERIC_COHORT_WIDTH=k`;
3. repeat the clean build and require exact optimized Wasm, gzip, Candid and
   optimizer-metric identity;
4. require the same export inventory and Candid hash at all five widths; and
5. record release Wasm, gzip, code-section and data-section bytes, the replica-
   limited defined-function count, optimizer-defined cross-check,
   table/indirect entries and the inclusive and `k - 1` to `k` marginal
   deltas.

Any cross-width export or Candid difference is a confound and blocks the
cohort; it is not attributed to generic cost. Shared code, folding, zero or
negative marginal deltas and nonlinear changes are reported literally. The
deltas are never averaged or multiplied into a role or downstream saving.

The generated query has not yet been installed and executed under PocketIC.
Until a representative workload is added, every cohort row must record
instruction evidence as absent and must not claim instruction parity or
runtime savings.

Canonical artifacts remain stripped. A separate transient symbol-preserving
release diagnostic must therefore use the same release code generation and
Binaryen 132 `-Oz` transform, adding debug names only for attribution. Its
retained report must map surviving Canic-owned `Page<T>`/generated Candid
bodies to fully qualified functions, concrete type arguments and specialized
machine-code bodies. That diagnostic cannot replace, publish or supply a
digest for the canonical artifact.

## Source Validation

The current working-tree overlay passes:

```text
cargo check --locked -p leaf_probe
CANIC_GENERIC_COHORT_WIDTH=5 cargo check --locked -p leaf_probe
```

These checks prove only that the collapsed and fully distinct source surfaces
compile. They are not optimized measurements and do not complete B1.

An additional one-off build of each boundary used the authoritative host
artifact builder and release finalizer on the current working-tree overlay:

| Final metric | Width 1 | Width 5 | Exploratory delta |
| --- | ---: | ---: | ---: |
| Wasm bytes | 2,463,055 | 2,470,169 | +7,114 |
| gzip bytes | 892,334 | 893,689 | +1,355 |
| code-section bytes | 2,242,085 | 2,248,580 | +6,495 |
| optimizer-reported data-section bytes | 185,509 | 186,094 | +585 |
| `ic-wasm` functions | 4,071 | 4,081 | +10 |
| defined functions | 4,032 | 4,042 | +10 |
| table minimum | 853 | 865 | +12 |
| element entries | 852 | 864 | +12 |
| Candid bytes | 27,628 | 27,628 | 0 |
| Candid service methods | 7 | 7 | 0 |
| Wasm export-section entries | 13 | 13 | 0 |

Both Candid files have SHA-256
`36bfe246c42512b0b06d3956c32524a32b85a0598ef804d835132e66655ed982`.
This confirms the fixture's boundary wire-shape invariant. The width-1 Wasm
hash was `f03e47b37d23dbc000c3be5d4afc382d4959893f89fac8cfb6b68e0214e9f3eb`;
the width-5 hash was
`bb8d54d9220175c48b80ea093d368fe82f80fc8a682935d9a4549af30a1adcce`.

These numbers are deliberately non-acceptance evidence: the builds used the
dirty development tree, one warm shared target, one build per boundary, no
frozen replica-validator-equivalent counter, no representative execution and
no widths 2-4.
They validate the harness and indicate that the family survives `-Oz`; they
cannot supply per-instantiation deltas or a savings forecast.

## Decision

Keep the cohort. It isolates a real five-type Canic-generated family without
changing the product protocol or using downstream slopes. All five widths now
have retained exact repetitions, invariant Candid/exports and canonical named
post-`-Oz` body mappings. Runtime instruction evidence is explicitly absent.
Complete B1 awaits human acceptance; no generic consolidation is authorized
from source repetition alone. A production cut that changes this family must
refresh the mapping and explicitly retain, retarget or stop the cohort.
