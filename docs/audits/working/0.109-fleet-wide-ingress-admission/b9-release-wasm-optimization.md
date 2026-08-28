# 0.109 B9 Governed Release Wasm Optimization

Date: 2026-08-28

## Downstream evidence boundary

Toko Miner supplied the qualifying baseline from Canic `0.109.17` and IcyDB
`0.246.0`. Its release App measured 6,157,093 raw bytes, a 5,619,140-byte code
section and 12,240 defined functions. Binaryen 108 `wasm-opt -Oz` with the
module-required bulk-memory, sign-extension and nontrapping-float-to-int
features produced 5,823,004 raw bytes, a 5,290,184-byte code section and 10,887
defined functions. The executable-code reduction was 328,956 bytes (5.85%).

The equivalently optimized, release-bound diagnostic artifact passed Toko
Miner's reported managed PocketIC journey through install, Canic bootstrap,
IcyDB readiness, admission, player operations, retained state, timer restore,
same-Wasm upgrade and fencing. This record treats those values as supplied
downstream evidence, not as a Canic-owned rerun. The final governed Toko Miner
managed and standalone-local gates remain required against the frozen Canic
candidate. Canic does not modify the downstream repository.

Read-only inspection of Toko Miner `ebc49e022d7c26f6117150d8b9436eeba04d8b58`
found the exact two ignored PocketIC journeys in
`apps/toko_miner/app/src/qualification.rs`. The managed case covers install,
Canic/IcyDB readiness, admission, Robot/Rocket/shop state, timer inventory,
same-Wasm upgrade, state restoration, timer restoration and successor fencing;
the standalone-local case covers install, readiness, caller ownership and local
admission bypass. The current dirty qualification script builds both fixtures
with the fast profile and remains pinned to published Canic `0.109.17`; it is
therefore valid behavioral coverage but not final proof of this release-profile
pipeline. Downstream must bind the managed input to the frozen candidate's
canonical optimized release artifact and retain the standalone-local journey
before the gate can qualify this batch. This is Toko-owned follow-up, not
authority to edit its dirty worktree from Canic.

## Canic hard cut

One release finalizer now owns configured Components, Fleet Coordinator and
Wasm Store. Its order is shrink, optional public-Candid metadata, required
official Binaryen 108 optimization, IC code-section enforcement, then
deterministic gzip. Artifact fingerprints, release-set manifests, Wasm Store
publication and module-hash authority consume only those final bytes. Debug and
fast builds do not request Binaryen; release builds have no unoptimized
fallback or second artifact.

The release transform stages its output and verifies the exact official tool
identity, derives its required features under Canic's admitted IC feature
contract, and compares export names/kinds plus embedded public Candid before
replacement. The `.did`, Candid digest and protocol-profile digest are derived
before the transform and remain the same owners. Provenance records before and
after raw, deterministic-gzip, code-section, data-section and defined-function
measurements.

The recurring `CANIC-WASM-001/v4` method builds every Canic-owned role twice
from isolated clean release targets, compares Wasm, gzip, Candid and transform
metrics byte-for-byte, then builds the debug comparison and retains bounded
structural Twiggy evidence. The separate symbol-preserving diagnostic below
owns source attribution without retaining an unoptimized release artifact.

## Named residual measurement

Canonical release Wasm is intentionally stripped, so its Twiggy output is
structural. A temporary symbol-preserving release diagnostic was built from the
same Canic source and optimized with the same Binaryen 108 flags plus `-g`; it
is not a release artifact and cannot enter any digest or publication owner.
Before `-Oz`, Twiggy estimated 148,314 bytes of monomorphization bloat in the
Canic test App. After `-Oz`, 135,057 bytes remained. Stable-sort machinery was
the only material family at roughly 95 KiB; directly named Canic generic
families were small (about 1.4 KiB for lifecycle timer registration and less
than 100 bytes each for the reported auth/storage helpers).

The residual output identified repeated same-type sort instantiations in
Component topology construction and chain-key batch selection. The bounded
source follow-up now initializes already ordered `BTreeMap` owners directly
instead of routing them through `FromIterator`'s internal stable sort. The
three chain-key selectors share one comparator and use unstable sorting only
because the retained unique batch ID completes a total order, making equal-key
input order unobservable. Runtime metrics and other one-off stable sorts remain
unchanged: their individual residuals were about 1.3--2.0 KiB and do not
justify weakening tie semantics without a larger measured owner.

The bounded follow-up passes 14 Component-topology, 16 Component-deployment
and 22 chain-key batch tests plus warning-denied `canic-core` Clippy. A
subsequent canonical release App measured 3,226,245 raw bytes, a 2,969,428-byte
code section and 5,708 defined functions before Binaryen; the final artifact
measured 3,053,593 raw bytes, a 2,800,001-byte code section and 4,963 defined
functions. Relative to the earlier optimized App baseline, the source follow-up
recovered another 27,665 code-section bytes and 46 defined functions.

## Residual IcyDB feedback

The supplied pre-optimization Toko Twiggy estimate attributes 372,894 bytes to
monomorphization families: 230,926 bytes of Rust sort specializations, 85,514
bytes of drop glue and 56,454 bytes of other generics. These totals are not an
IcyDB-only attribution. A prior controlled Toko framework-stack comparison
estimated approximately 293 KiB of IcyDB-related growth, but a fresh named
post-optimization IcyDB `0.246.0` attribution is still required before changing
source.

Actionable upstream IcyDB follow-up after that measurement:

- benchmark request-execution, query and schema code generation per wrapper to
  detect duplicated codec, validation, error and transaction machinery;
- measure stable ordering in query/schema paths and `icydb_schema::compact_sort`,
  then reuse named comparators or already ordered owners only where semantics
  and size evidence permit;
- inventory retained-kernel scan monomorphizations and share concrete scan
  owners where the result, error and ordering contracts are identical;
- verify that SQL/introspection remains unreachable when the `sql` feature is
  disabled and split optional runtime surfaces behind narrower features; and
- retain a representative benchmark canister in IcyDB CI with code-section and
  function-count deltas per release.

Stable sorting must remain wherever equal-key input order is observable.
Unstable sorting is admissible only for a proven total ordering or
interchangeable ties. None of this feedback authorizes Canic to mutate IcyDB or
to weaken query, persistence, authentication, lifecycle or deterministic
canonicalization semantics.

A temporary symbol-preserving IcyDB `0.246.0` ten-typed-entity, SQL-off fixture
was also passed through the same `-Oz` transform with names retained. Twiggy
reported 240,125 residual bloat bytes: Rust sorting remained dominant, while
the three retained-kernel scan families together accounted for 12,682 bytes
(`execute_retained_kernel_scan`, `read_kernel_scan_row`, and
`try_scan_borrowed_primary_rows`). This is actionable upstream evidence, not a
Canic source-change target. The supplied Toko Miner release files themselves
were stripped, so the supplied 372,894-byte named pre-optimization report
remains their authoritative named baseline until Toko produces its frozen
symbol-preserving diagnostic.
