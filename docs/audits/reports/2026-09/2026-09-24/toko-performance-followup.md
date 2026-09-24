# Toko Miner performance follow-up

Date: 2026-09-24. Published baseline: Canic 0.110.39. Implementation batch: 0.110.40.

## Protected-read attribution (CANIC-160)

Protected inspection and install-history receipts now identify the inspected
child as `request.subject`, independently of the Root transport endpoint in
`request.target`. The existing request owner assigns the same subject to nested
reserve/status transports and retains request/parent IDs and the observation
span. Inclusive `canister_inspection` and `canister_history` boundaries add no
remote requests. Their times include nested transports and must not be added to
them or summed across concurrent workers as critical-path wall time.

Coverage includes ordinary/current inventory, retained asset and reinstall
authority checks, install-history reconciliation and child cycle inspection.
Fresh reserves, bounded concurrency, batch drainage, authority checks and error
ordering remain intact. Context/thread-local nesting prevents independent children
from sharing subjects; failure and unwind restore the parent scope. Incomplete
start/end pairs remain incomplete evidence. No payload bytes or credentials are
recorded, and diagnostics confer no authority to reuse observations.

All 42 selected native tests pass: 33 host and nine CLI cases, including actual
stub-transport failure/retry, concurrent children, unrelated timing contexts,
unwind, receipt serialization and interrupted receipts. Strict host/CLI
all-target/all-feature Clippy, scoped formatting and layering pass. The final
source, logs and hashes are retained in `.tmp/toko-performance-20260924/`.
Earlier .40 retry-deadline and build-lock evidence keeps its original source
boundary; the final receipt tests were rerun with this addition.

This completes local attribution, not deployment-performance qualification.
Matched live apply/replay remains downstream work after publication. Existing
receipts cannot retrospectively identify subjects they never recorded. No
unproven observation cache, transport retry classification or concurrency
increase is introduced.

## Release-build measurement (CANIC-176)

All ten local Release invocations pass. The [structured evidence](toko-performance-followup.json)
retains input/tool identities, resource vectors, selected release records, cache
diagnostics and log hashes. The [executed harness](toko-performance-matrix.sh)
is retained byte-for-byte. Its immutable inputs, full build logs, artifact-hash
inventories and process snapshots live in
`.tmp/toko-performance-20260924/matrix-qualified/`. To repeat it, prepare a new
scratch directory with the verified `source.tar`, extracted `original/`, frozen
`tools/canic` and empty `results/`, then run the harness there. The original
results are immutable evidence, not a directory to overwrite on repetition.

The first attempt used committed Toko source
`a7e7023d478e03fb923879ff6307933b4f44854a` and failed after 322.57 seconds:
Game Shard's executor still referenced the removed `FieldAction::BoardMinion`
variant. That failed run remains under the sibling scratch directory `matrix/`;
it is excluded from successful Release comparisons. No Toko files were changed.

The replacement baseline copies 2,018 source files from Toko's previously
qualified .39 adoption snapshot, whose base is
`0a35352aad9269be050ddd3bbb023834ba50878b`, plus its recorded adoption changes.
Every copied file matches the prior qualification inventory. This is a qualified
source snapshot, not an assertion that the base commit alone contains adoption.
The frozen CLI and runtime both use .39; IcyDB is .261.10. The .40 working-tree
instrumentation is qualified separately and is not credited with build savings.

The harness runs the actual `canic --environment staging build toko_miner
--profile release` command, offline, with four Cargo jobs and compiler wrappers
disabled. Only source copies inside Canic scratch change. Private Cargo targets
prevent sharing mutable objects with either live repository; no deployment,
application test suite or frontend build is invoked. Cold means empty private
Cargo objects/declaration caches, with registry sources and OS page caches warm.
The shared host retains existing PocketIC processes; process observations expose
competing builds but do not establish a dedicated-machine benchmark. GNU time's
maximum process RSS is not an aggregate simultaneous process-tree peak.

The ordered cases are cold/unchanged warm; an added package-local qualification
note/unchanged warm; Game Shard energy-bar price 5 to 6/unchanged warm; the path
dependency's enrollment-ID limit 64 to 65/unchanged warm; and baseline source plus
its complete-build evidence at a different absolute path with an empty Cargo
target/unchanged warm. Each source change is restored before the next case;
compiler objects remain warm in the original checkout between changed cases.
The relocation snapshot is captured immediately after the initial warm run.

No cache fingerprint or output admission rule changes for this experiment.
Input records, release identities and exact output hashes determine reuse;
human log summaries explain phases but are not release-blocking assertions.

| Case | Build seconds | Unchanged repeat seconds | Build max process RSS, MiB |
| --- | ---: | ---: | ---: |
| Initial empty target | 925.54 | 1.86 | 2731.5 |
| Qualification note only | 459.95 | 1.84 | 2729.7 |
| Gameplay constant | 506.42 | 1.90 | 2730.1 |
| Path dependency constant | 507.28 | 1.92 | 2731.0 |
| Relocated baseline, empty target | 914.57 | 1.84 | 2731.2 |

Every warm pair preserves its exact complete-build records, all retained file
hashes and the selected eight-role release identity. Each selected release has
30 verified files. Warm maximum process RSS is 103.4–104.4 MiB; no new runtime
or declaration compilation occurs. Both source copies match all 2,018 original
file hashes after the matrix. The lockfile and tool binaries remain unchanged.

These are five ordered single pairs on a shared host. External Toko Cargo work
was observed in every rebuilding case and two warm cases; existing PocketIC
processes also remain present. The JSON counts are repeated sampled process
observations, not unique builds or a measure of contention duration. This proves
the recorded cache behaviour and exact output preservation. It does not establish
statistical timing variance, an isolated resource-contention effect, a .40 speedup
or a comparison with Toko's older CI runs. No timing or memory threshold is added.

## Measured opportunities and limits

Application runtime Cargo/link is the largest reported span: 515.76 seconds
initially, 327.70 for the note, 367.49 for gameplay, 349.82 for the dependency and
507.57 after relocation. Bootstrap Coordinator/Store compilation is separate;
artifact finalization overlaps other work. Do not sum nested or overlapping
phase diagnostics to manufacture a complete wall-time decomposition. Cargo's
internal compile-versus-link split and exact object-cache hit counts are not
available from these logs.

Declaration reuse already narrows changed-tree work. The note case reuses all
six compiled declaration results; gameplay misses Game Shard only; the dependency
case misses four application declarations while Root and Translation hit.
Declaration spans are 27.76, 33.34 and 38.21 seconds respectively, compared with
175.63 initially and 176.22 at the relocated path. Candid extraction is only
0.07–4.37 seconds. Another extraction cache is not supported by these costs.

The note adds no Rust source change and preserves all eight exact Candid files,
but all eight Wasm hashes change under the new release identity. Source review
explains the mechanism: `reuse::input_snapshot` fingerprints whole package trees,
and `WorkspaceBuildContext::apply_to_command` supplies the newly allocated release
identity to runtime compilation. Narrowing that rebuild requires an explicit
proof of compiler/build-script inputs and exact release binding. Simply ignoring
Markdown or retaining old Wasm under a new release identity is not justified.

Relocation misses all complete-build entries and names source/configuration path
changes plus `CARGO_TARGET_DIR` and `PWD`. Copied finalized evidence therefore
does not establish portable exact-build admission. A future portable-cache change
must prove path/environment independence, preserve declared external inputs and
verify outputs; sharing mutable Cargo targets is not that proof.

The next justified performance work is reducing repeated runtime compilation
while preserving release and input authority. These observations prioritize that
work; they do not implement or qualify a new cache contract. CANIC-160 still needs
matched live requests with the new child attribution before selecting further
observation reuse or transport changes. Both optimization feedback items remain
open at that broader scope.

## Delivery boundary

The .40 batch also contains CANIC-182 readiness, CANIC-150 retry deadlines and
CANIC-176 lock diagnostics. Its accepted local attribution and measurement work
is complete. The complete bounded batch and both changelog surfaces are ready
for the maintainer's release flow under the existing affected-line correction
exception. Versions are unchanged and edits remain uncommitted. No broad gate
was pre-run; published adoption, real launcher acceptance and matched live
deployment/replay remain downstream. No commit, push, release or
external-repository mutation occurred. B3 has not started here.
