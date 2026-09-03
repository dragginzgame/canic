# B1 Generic-Instantiation Cohort

## State

The Canic-owned family, representative cohort width and attribution fixture are
selected in the current working-tree overlay. Source validation passes at both
cohort boundaries. One exploratory optimized build at each boundary confirms
that the Candid and export shapes remain fixed, but it was produced from the
mixed dirty tree without determinism or the intermediate widths. It is not an
immutable B1 measurement, so this document does not claim an accepted
inclusive cost, marginal cost or recoverable saving.

The immutable `v0.110.5` `CANIC-WASM-001/v6` baseline remains separate. Its
bounded `twiggy monos` output contains zero rows for every canonical role
because the canonical release artifacts are stripped. Those summaries prove
that the ordinary recurring report cannot supply the required named mapping;
they do not prove that generic instantiations are absent.

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
changing the product protocol or using downstream slopes. B1 remains open on
the five-width immutable artifact vector, cross-width Candid/export identity,
instruction evidence or its explicit absence, and the named post-`-Oz`
mapping. No generic consolidation is authorized from source repetition alone.
