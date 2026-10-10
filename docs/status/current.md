# Current handoff — 2026-10-10

The maintainer accepted the exact 0.110 closeout on 2026-10-10 and requested
**0.111.0** live for the complete hard cut. Root package
versions remain **0.110.54** over HEAD `ac55e50334dd6479ec36f404e89e60bcfe9184d6`
(subject `0.110.55`); the implemented family is committed for the authorized release. The root
changelog and detailed 0.111 notes describe one open release batch. GitHub issues
own acceptance and follow-up decisions.

## Settled release graph

The root lock selects Auth/protocol-types 0.3.6, Backup 0.15.0,
Blob/runtime contracts 0.22.4, all four Host crates 0.12.9, Memory 0.35.5,
Metrics 0.5.6, Query 0.54.4, Testkit 0.33.1 and Timers 0.17.6.
Both private Blob consumer locks align their runtime families with this graph.
Selected dependency caches are explicitly prepared; ordinary qualification is
locked and offline. Preserve the graph during qualification.

Shared Tooling **0.3.8** is adopted through committed revision
`67285b28a98b7c4211ad32de726709d4e87edea4`; canonical export and independent
byte/mode checks cover all 108 selected files, including the required advisory
README-freshness task. The new archive guidance separates
archive creation from mandatory independent consumer compilation, retaining
registry admission during publication. The late lock update selected Host 0.12.8 alongside Auth 0.3.5, Blob 0.22.4
and Timers 0.17.5. Published Rust sources for all nine affected upstream crates
are byte-identical to the previously qualified patches; their focused behavior
proofs are reused with their original graph identities. The serde_json 1.0.152
parser patch receives the affected Backup persistence check. Exact latest-graph
inputs are recorded in `latest-graph-inputs.sha256`.

The later release selection adds Host 0.12.9, Memory 0.35.5 and Metrics 0.5.6.
All six affected published Rust sources are byte-identical to the preceding
patches. Release inputs and that comparison are recorded separately in
`release-graph-inputs.sha256` and `upstream-release-patch-equivalence.log`;
earlier archive and runtime proofs retain their original graph identities.
The final selection adds Auth/protocol-types 0.3.6 and Timers 0.17.6;
their published Rust sources are also byte-identical to the preceding patches.
`release-final-graph-inputs.sha256` records the final three-lock selection.

## Implemented family

Canonical wire unions, passive DTOs, IDs, diagnostics and codecs have one
runtime-free owner in `canic-contracts`; storage registration and cycle/template
bytes remain with Core and Control Plane. Endpoint emitters and consumers share
those declarations. The maintainer explicitly allowed the Contracts path cut
under [#354](https://github.com/dragginzgame/canic/issues/354); this is not a
waiver of today's release closeout.

Memory declarations and opens use permanent keys. One sealed host allocation
pool grants explicit framework/application namespaces and diagnostics resolve
observed physical IDs from its protected ledger. Numeric owner partitions and
old macro operands are removed
([#510](https://github.com/dragginzgame/canic/issues/510)). Consumer database
composition remains downstream work; Canic does not acquire IcyDB dependencies.

Auth delegates canonical protocol identities, bounded delegation installation
and batch Merkle construction to published IC Auth. Canic retains issuer policy,
ordering, uniqueness, the 64-issuer budget, root-key deadlines and typed endpoint
refusals. Complete verifier/cache, certification and durable-session adoption
remain under [#491](https://github.com/dragginzgame/canic/issues/491).

Backup delegates checksum/traversal, private staging and verified publication to
IC Backup, and ordinary JSON publication to Host. Canic retains its journal,
create-only conflicts, exact-byte checks and same-operation recovery. Live Fleet
capture/restore remains unavailable pending its authority/consistency orchestration
([#394](https://github.com/dragginzgame/canic/issues/394)).

`canic-blob-service` belongs to the main workspace at its established path,
inherits the root catalog/version/lock and is publishable. Its dedicated and
embedded application consumers remain private independent workspaces at version
0.1.0. The publisher's nine-package order publishes Contracts before consumers
and Blob after the facade. Provider uploads, GC, funding, registry-only
consumption and actual Toko Miner acceptance remain separate from local
composition qualification under [#444](https://github.com/dragginzgame/canic/issues/444).
Production target-bound initialization is already implemented; the managed Blob
helper uses a fixture Root and does not prove the production Root install flow.

Known active-parent child startup exhaustion now schedules the existing retained
operation driver. It preserves the child, original publication/request identity,
retry time and authority expiry; deduplicates drivers; and stops after sixteen
consecutive failures or permanent conflict/refusal. Other activation errors are
not classified as startup-pending. Protected operation status remains protected.
The actual deployed #505 incident is not diagnosed solely by the delayed fixture
([#505](https://github.com/dragginzgame/canic/issues/505)).

## Qualification and delivery boundary

Qualification evidence is retained at
`target/review-validation/release111-settled-20261010/`. Auth (182), Backup
persistence (73), Contracts (341), focused Memory (30), affected-package lint
and shipping Rust 1.91 checks pass. The owning #505 PocketIC case passes
automatic completion, changed-input refusal, restart recovery and exact replay;
its later helper extraction and test-module relocation pass final Clippy without
repeating the same runtime proof. Both complete managed Blob Apps, eight
manifest-bound artifacts, single normal-Wasm identities, 32-method Candid parity
and both dedicated/embedded PocketIC cases pass on the preceding fixed patch
graph. The source-identical upstream patches reuse those proofs with their
original identities; the latest-graph checks below qualify the new selection.

Root and both private Blob consumer locks now project root-owned release
versions and roll back together; external selections and fixture versions stay
unchanged. Stale/tampered lock, committed-view and rollback fixtures pass, as do
three native governed-receipt cases with fake Git. All nine exact 0.111.0 archives
pass independent unpacked CLI/native and
facade/Blob Wasm checks on Rust 1.91, retained external-selection admission and
final byte-integrity checks. The latest normal-Wasm graph has single Memory
0.35.4, Timers 0.17.5, Auth 0.3.5 and Blob/contracts 0.22.4 identities, no
TOML/YAML runtime parser and runtime-free Contracts. The refreshed owning
allocation peer passes read-only verification on the latest graph.
Passing and failed attempts retain separate logs. The
[exact closeout audit](../audits/release-lines/0.110-closeout-audit.md#implemented-family-closeout--2026-10-10)
is PASS WITH LIMITATIONS, accepted by the maintainer on 2026-10-10
([#459](https://github.com/dragginzgame/canic/issues/459)). The implemented batch
and 0.111 notes are ready for the governed release flow. This handoff is not a
release-validation receipt.

The maintainer deferred unfinished FR1/CS1 from this closeout
([#459 decision](https://github.com/dragginzgame/canic/issues/459#issuecomment-6099958187)).
The maintainer also selected current focused proofs for B5; the old two-clean-build
full capability matrix remains historical evidence and no longer gates this
closeout ([decision](https://github.com/dragginzgame/canic/issues/459#issuecomment-6100198166)).
Current release code observes/assesses Fleet evidence and exposes no whole-Fleet
release execution route. Deferral neither completes those batches nor discards
unfinished paid effects; their next implementation batch is unscheduled.

Coherent family publication remains
[#33](https://github.com/dragginzgame/canic/issues/33) and
[#444](https://github.com/dragginzgame/canic/issues/444). The isolated 0.111
version/package preview is review preparation, not a live version mutation or
registry-only acceptance. The governed version transaction requires a committed
qualified source and owns the release-validation receipt. The closeout
acceptance is complete. The maintainer explicitly authorized an exception to
AGENTS.md's agent commit prohibition for the source and governed release commits
required to deliver 0.111.0 only. The selected execution is the normal
`make release-minor && make publish` flow, including its release-owned validation,
tag and atomic push. Preparation and publication retain build artifacts; do not
append cleanup. This scoped authorization does not change the standing policy
for other releases.

The planned 0.112 Blob design retains its next-line identity; no implementation
of that next design is authorized by this handoff. Toko Miner's frozen-input and
actual database/installation acceptance belongs to
[Toko Miner #6](https://github.com/dragginzgame/toko-miner/issues/6), rather than
an additional Canic release gate. #34 is already published; #493 is not required
for its existing top-level Blob Component. Complete funding forecasts and
operator progress remain [#37](https://github.com/dragginzgame/canic/issues/37)
and [#29](https://github.com/dragginzgame/canic/issues/29).

## Retained evidence and prerequisites

Earlier qualification directories retain their original graphs and artifacts:
`contracts-354-current-selection-20261010`, `memory0350-metrics050-20261010`,
`auth-batch-20261010`, `backup-publisher-20261010`,
`shared037-host0126-20261010` and `blob0223-host0125-final-20261010`, all under
`target/review-validation/`. Do not relabel earlier or graph-drift-rejected
results as current qualification. The last Blob directory also retains Binaryen
133 transformations and the successful optimized-component journeys; they are
qualification copies, not published Release-profile manifests.

Testkit setup/admission selects the package directly from Cargo.lock and owns
PocketIC 16.1.0. Local `.tools/ic` selects Binaryen 133; the previously packaged
optimizer under `~/.local/bin` remains 132. Ordinary Release-profile CLI builds
require the governed canonical toolchain setup. No external tool installation,
hook activation, deployment or paid effect is implied. Native macOS and hosted
cancellation execution remain unrun.

Preserve `.canic/local-work/`, `.canic/incident-repairs/canic188/`, old bundles,
archives, failed scratch and historical evidence. Cross-release deployment is a
clean reinstall with cycle custody; predecessor state must not be revived.
Same-release interruption recovery, retry, backup/restore and exact replay remain
required. The prepared source is committed as `afc2521ed`; the first governed
release attempt stopped before compilation on a missing snapshot companion and
a stale chain-key Root crypto inventory. Both corrections retain the selected
upstream revision and runtime graph. The normal release flow is continuing;
no release version, tag, push or registry publication has occurred yet.
Release qualification also corrected stale formatter-root and inherited-offline
publication fixtures. All policy gates then passed. Compilation exposed derive
formatter corruption of `$crate` paths in two exported wire macros; restored
paths are protected with the tool's scoped exclusion and pass Contracts compile
and all 341 owning tests
([Shared Tooling #113](https://github.com/dragginzgame/shared-tooling/issues/113)).
Subsequent release checks passed workspace compile/Clippy, the feature matrix
and embedded-peer preflight. Default-feature cache policy availability and
test-only lint findings are corrected. Ordinary tests exposed stale managed-wire
and upstream checksum assertions plus a selected-directory Backup regression;
the corrections pass all 314 Backup tests, all 11 managed-endpoint tests and
their focused lints. Selected directory aliases resolve through Host while
publication leaves and artifacts retain no-follow admission. The governed
release is continuing; no publication result is claimed by this handoff.
The complete ordinary, runtime and payload suites passed. The ignored PocketIC
tier exposed generated audit-package acquisition races, a rent fixture missing
scheduled charging rounds and a release-binding fixture selecting a stale
canonical optimizer. The focused corrections retain input-drift refusal, actual
bounded debit checks and production tool admission
([#511](https://github.com/dragginzgame/canic/issues/511)). All three owning
regressions, seven optimizer admission/resolution checks, final focused Clippy
and the governed generated-package preflight pass. The next normal release
attempt will qualify the parallel harness; publication remains pending.
