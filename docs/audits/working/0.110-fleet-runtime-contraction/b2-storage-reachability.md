# B2 role-selected storage reachability

Date: 2026-09-23. Status: active implementation.

The maintainer accepted complete B1 and explicitly authorized B2. The
[design](../../../design/0.110-fleet-runtime-contraction/0.110-design.md#b2-role-selected-storage-reachability)
and [accepted evidence](b1-input-evidence.md#complete-b1-review--accepted-2026-09-23)
govern this batch. B3, release publication and minor closeout are separate.

## Current source boundary

The captured pre-change macros register stable TLS touch functions with constructors.
`MemoryRegistryOps::bootstrap_registry` drains their global function-pointer
queue, retaining every registered store initializer. Metadata declaration
constructors separately register stable keys, ID reservations and authority
ranges; those declarations must remain available before opening any store.

The first cut uses ordinary lazy thread-local storage and removes Canic's eager
TLS execution registry. Metadata, bootstrap admission, committed allocation
checks and store access owners stay intact. This is the substrate change, not
proof that all role-inapplicable restore/recovery code has disappeared.

The pre-change source is captured locally at
`.tmp/b2-storage-baseline-20260923/source.tar.gz`, SHA-256
`e381097367c5f9298c15553a17c41c58a6b7851b047859d73856abbea583a3fe`.
It contains workspace manifests/lock, toolchain, crates, applications, canisters,
scripts and Makefile, excluding targets and generated `.canic` outputs. It is a
dirty-source snapshot, not a commit, release, or replacement for immutable B1.
Do not unpack it over the shared checkout.

## Completion sequence

1. Remove constructor-retained executable TLS registration. Prove bootstrap
   commits allocation metadata without opening unrelated stores, then prove
   selected access preserves its state and leaves other allocations unopened.
2. Trace and generate direct role/capability initialization, restoration and
   same-release recovery calls. Remove remaining unselected executable roots;
   do not replace the registry with another erased callback collection.
3. Qualify selected lifecycle/admission and same-release recovery paths with
   focused native and PocketIC evidence. Keep synchronous restoration ahead of
   lifecycle participants and deferred work.
4. Retain current matched optimized artifacts and maintained-workload instruction
   evidence. Respect accepted absolute budgets, the maximum 1% instruction
   allowance and no unexplained indirect-table growth. No byte/function saving
   is credited from source inspection or the B1 attribution stubs alone.
5. Propagate current APIs, fixtures, documentation and the open patch changelog.
   B2 is not complete or push-ready until these obligations are covered.

## First implementation cut

Canic-owned stable stores now use ordinary lazy `std::thread_local!` declarations.
The 56 converted eager TLS declarations retain their stable keys; three
scalar occupancy caches use const initializers. The Canic eager macro
wrappers, global executable queue and drain APIs are removed without aliases.
The separate allocation declaration/range constructors and composed bootstrap
admission remain unchanged. `bootstrap_registry` commits the allocation registry
without invoking every store initializer.

The consuming non-root lifecycle macros now call the compile-selected startup
or restoration API directly. Non-root workflow entrypoints also make direct
calls to the selected runtime owner after their shared initialization/restoration;
they no longer pass erased startup function pointers. Managed restoration still
checks the embedded release and Active phase before starting services. Local
restoration retains its explicit standalone mode. Participant ordering and
zero-delay scheduling remain in their existing owners.

The native bootstrap regression failed against the captured eager implementation
because unrelated declarations already had allocated pages. It passes after the
cut: metadata exists before access, only the selected Env allocation opens, its
state remains readable, and diagnostic reads do not advance the ledger generation.
A corresponding control-plane proof selects FixtureStore and verifies the other
allocation extents and every binding stay unchanged.

Focused results so far:

- `cargo test --locked -p canic-core --lib memory`: 30 pass, including admission,
  stable-ID coverage, ledger diagnostics and lazy Env access.
- `cargo test --locked -p canic-control-plane --lib storage::stable`: 35 pass,
  including lazy FixtureStore access and existing storage snapshot/reopen tests.
- Exact baseline failure: `/tmp/canic-b2-lazy-baseline.log`; candidate logs:
  `/tmp/canic-b2-core-memory.log` and `/tmp/canic-b2-control-storage.log`.

## Remaining reachability and qualification

The first cut does not close B2. Subsequent cuts below remove shared lifecycle
Fleet-admission calls and select the access evaluator's admission reader.
Existing auth/sharding selection and control-plane role features also need
current optimized absence evidence. Capability-owned record redesign remains B3,
after B2 remeasurement.

Strict scoped Clippy passes for `canic-core`, `canic-control-plane` and `canic`
with `--all-targets --all-features --keep-going -- -D warnings`; the three scalar
occupancy caches use const initializers. Current document semantics also pass,
with the existing parked-history layout advisories. Isolated Coordinator, Root and Store feature builds also pass. The corrected
Root-only import gate is covered independently of feature unification. Lifecycle
and memory ABI guards pass (12 tests), as does the managed-endpoint guard (10).
The participant ordering check now observes tokenized call identities and order,
without depending on local binding names, comments or whitespace.

The focused `lifecycle_boundary` PocketIC target passes all six cases, including
init/post-upgrade participant rollback and corrected retry, repeated same-release
restoration, release mismatch rejection and malformed lifecycle input. The runner
completed in 260 seconds. The initial sandboxed attempt stopped before test
compilation at the compiler-cache socket; the successful governed retry used an
empty `RUSTC_WRAPPER` and local simulator socket permission. This is focused
lifecycle evidence, not complete workspace or B2 qualification.

The [structured first-cut evidence](../../reports/2026-09/2026-09-23/b2-storage-first-cut.json)
records exact source/log hashes, commands, 87 native tests, isolated role checks,
strict scoped lint and six PocketIC tests. Raw logs remain locally under
`.tmp/b2-storage-baseline-20260923/qualification/`. The matched current optimized
artifact vectors and maintained-workload instruction measurements remain open.
No production footprint or deployment-speed improvement is claimed, and the
complete B2/release batch is not yet push-ready.

Lazy initialization can move work from bootstrap into the first store access.
The maintained workload comparison must cover that access; a lower startup cost
alone cannot qualify instruction parity or the accepted regression allowance.

## Compile-selected Fleet admission lifecycle

The consuming actor now uses its existing
`canic_capability_fleet_admission_projection` flag to select direct init and
restore owners, including all admission/automatic-top-up combinations. Shared
initialization checks compiled selection against configured enrollment and exact
payload presence before stable mutation. Shared restoration checks selection
against configured enrollment; selected owners restore projection authority
synchronously before services, participants or deferred work. Ordinary owners
contain no projection initialization/restoration call. The local and Store
lifecycle surfaces are unchanged by this cut.

Qualification passes 20 native tests: the complete enrollment/payload matrix,
restore selection, tokenized lifecycle selection/ordering and managed endpoint
guards. The guard evaluates cfg expressions instead of requiring a particular
attribute spelling. Scoped core/facade all-target/all-feature Clippy and the
changed integration target's strict Clippy pass. Seven lifecycle PocketIC cases
include missing selected admission rejecting installation without a module,
corrected retry and same-release restoration. The separate active-projection
case preserves generation, digests and admitted/unlisted caller behavior.

The [structured selection evidence](../../reports/2026-09/2026-09-23/b2-admission-selection.json)
retains exact sources, commands and log hashes. Raw qualification logs remain in
`.tmp/b2-admission-selection-20260923/qualification/`. These results qualify this
source checkpoint; earlier first-cut evidence remains immutable historical proof.

## Selected admission reader

The universal `access::expr` interpreter previously retained the admission reader
for every generated guarded endpoint. Endpoint expansion now computes an exact
const selection from its complete guard AST, including `all`, `any` and `not`.
Manual dynamic expressions retain their full evaluator; custom predicates retain
their own dependencies. A reached unselected predicate is a terminal typed denial,
so boolean composition cannot turn a selection error into access.

The first async specialization did not remove the reader: its future resume state
still called `require_fleet_admission`. The final cut makes the local-state
`auth::is_fleet_admitted` helper synchronous. Direct callers remove `.await`;
endpoint predicate syntax and the observed-caller helper remain unchanged. This
is a pre-1.0 hard cut without a compatibility wrapper. Forty-three native tests,
strict scoped lint and the exact active PocketIC projection/recovery case pass.

The [reader report](../../reports/2026-09/2026-09-23/b2-admission-reader.md) and
[structured evidence](../../reports/2026-09/2026-09-23/b2-admission-reader.json)
retain the matched diagnostic vectors and source/log hashes. The parent-only
control loses 23,526 code bytes and 23 functions; named record construction and
validation disappear while allocation metadata remains. Admission-only adds 294
code bytes. Mixed guards add 5,384 code bytes and two slots attributable to the
second evaluator drop/poll pair. Other table entries retain their name multiplicity
when only Binaryen numeric suffixes are ignored.

These isolated probes retain names and omit the product shrink/Candid-finalization
pipeline. They cannot close per-role optimized absence or establish production
size/instruction savings. The audit leaf explicitly requests admission and is a
positive control. The next work is the current canonical role/fixture artifact
matrix and maintained workloads including first access, accounting for mixed-guard
cost, absolute budgets and table attribution. Broader B4 pruning remains sequenced
separately; B2 and the release batch remain incomplete.

## Canonical measurement checkpoint

The [current matched report](../../reports/2026-09/2026-09-23/b2-canonical-measurements.md)
retains cumulative B2 production comparisons against the captured pre-cut source.
All eleven canonical roles and four fixtures now have complete canonical function
mappings, exact interface/selection parity, independent counters and passing
frozen reserves. The canonical modules collectively lose 166,215 code bytes,
447 functions and 272 slots; fixtures separately lose 67,229 bytes, 191 functions
and 90 slots. Four canonical roles and the leaf grow code slightly while every
table shrinks. Do not substitute earlier name-preserving probes for these artifacts.

Repeated execution measurements show synchronous runtime initialization down
18.91% and same-Wasm restoration down 10.14%. All twelve complete sampling
exports remain within 1%. Two chart-query subspans at 256 rows reproducibly
increase by approximately 80,000 instructions. A separate heap-write control
supports investigation of memory-access charging; it does not identify the
application allocation responsible. No allowance or CI gate changed.

Next resolve those exact query cases, audit Root-local template chunk-set readers
against required bootstrap demand, and finish representative workload/generic
qualification. Root's named admission storage is absent; retained projection
builders serve child payloads. The leaf is a positive admission control measured
at width one only. The report retains exclusions, raw paths and evidence hashes;
all measurement builds/traces have finished. B2 remains active, B3 has not started,
and the complete batch is not push-ready. No build-speed claim follows.

## Final ownership qualification — complete

The [storage closeout report](../../reports/2026-09/2026-09-23/b2-storage-closeout.md)
closes B2 with all fifteen refreshed canonical/fixture artifacts, exact source
and interface checks, complete named-body mappings, 92 metrics tests, strict
scoped lint and the passing real Root/Store provisioning/recovery journey.
Root loses its six local chunk-storage bodies while keeping approved manifests.
The temporary query-reader experiments are excluded from product source.

On 2026-09-23 the maintainer explicitly accepted the four measured cold-query
regressions, each adding one first-touch heap charge. Sampling and warmed-query
comparisons remain within 1%; the general 1% rule remains in force elsewhere.
The complete queries repeat exactly. Retain the accepted B1 Page cohort; B2
does not change that family, and the final named generic reports are refreshed.

Decision: stop B2 at push readiness. The accepted release batch and both `.39`
changelog surfaces are complete. B3 requires a separate residual record/codec
benefit decision; B4/B5 and the human minor-closeout gate remain. No broad gate,
version transaction, commit, push, deployment or sibling edit ran.
