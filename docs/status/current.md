# Current handoff — 2026-10-28

## Selected release

The maintainer explicitly requested release and package publication of Canic
0.113.0, including necessary repairs/retries. This selects that minor boundary;
it does not claim acceptance of the unfinished historical closeout audit.
The previous release is recorded at `5c236ab130c0d63892ef281b45bdc3fa353e012f`.
Its evidence remains in `target/session-release-0112/result.json` and adjacent
attempt logs. Current release evidence is retained under `target/session-release-0113/`.
The pre-release handoff is preserved there as `status-before-release.md`.

Review and commit the coherent source batch, then use `make release-minor` and
`make publish`. Release preflight owns preparation and complete qualification;
do not pre-run another broad gate. Stop on source drift, failed gates or uncertain
remote effects. Inspect saved phases and refs before exact-attempt
`make release-resume VERSION=0.113.0`; never remove a lock from restricted PID
absence or manually bump again. Preserve artifacts and logs throughout.

`make release-minor` releases source; `make publish` publishes packages.
CLI installation, App build, Fleet deployment and live verification are separate.
No live Fleet deployment was requested or verified.

## Coherent batch

Auth 0.19.5, Backup 0.48.2, Blob/contracts and Memory/Storage 0.53.0,
Timers 0.31.16, Wire/derive 0.7.20, IC Call 0.7.3 and Tooling 0.69.7 are selected.
Query minimum is 0.75.5; the root lock selects 0.75.6 and Platform 0.7.3.
Both private consumer locks align. The metadata editor is exactly 0.69.7.
Shared Tooling 0.27.3 was adopted through its exporter from reviewed revision
`7ee5c83744aa822ecd070703dbf0a7d95076d5f6`; its catalog owns sccache 0.19.0.
Make selects managed tool/output/runtime defaults without manual exports;
qualification enforces offline Cargo and a clean source tree.

The Blob adapter lives at `crates/canic-blob-service`; its two independent
consumer fixtures remain under `integrations/blob-service`. Adapters use `mount!`.
The new publishable IcyDB adapter supplies transport/lifecycle composition and
shares the exact committed host Memory runtime without an IcyDB engine dependency.
The Auth scaffold remains unpublished and refuses mounting at compile time.
Endpoint attributes use `query`/`update` and `raw_query`/`raw_update`, with explicit
owned Wire contracts; required external Candid adapters remain separate.
External management/history, NNS, ledger/archive, CMC, asset-read and ICRC-21
contracts use Platform identities. Removed public DTO/attribute names have no aliases.

Memory fixture backings delegate physical identity; retained handles prevent
reopening until final release. Controller checks, bounded spending, cycle
conservation and same-operation recovery remain required. Host environment
normalization preserves artifact reuse, not test-result caching. Serial test
compilation and inventory share one Cargo listing. Ordinary CI explicitly prepares
the metadata editor; FIFO fixtures use the portable command. Cycle-table rendering
shares columns while preserving output. See the complete 0.113 changelog.

## Evidence and limits before the release gate

`target/tooling-backup-refresh-20261028/` records Host reuse/invalidation,
251 Backup tests, real persistent PocketIC lifecycle, pinned tool admission and
release transaction fixtures. `target/blob053-readiness-20261028/` records
36 Core memory, 36 Control Plane storage, six adapter library and three lifecycle
participant tests, scoped strict lint, 86 ICP-refill, 67 operator-mint, two
reinstall-adoption and package-boundary passes. These precede the adapter move.
The dedicated Blob attempt refused `Build(ChangedInputs)` during concurrent
relocation (386s proof within 467s runner); embedded and exact Root snapshot
cases did not run. This is artifact-preparation failure, not snapshot evidence.

The relocated tree's four peer owner tests and read-only verification pass after
the existing generator refreshed Wasm/provenance. The first release attempt stopped
before version mutation on five redundant fixture visibility declarations; corrected
`runtime_probe` all-target/all-feature strict Clippy passes. Retain both failure
and correction logs in the current release directory. Full release results
must be read from that attempt, not inferred from these earlier focused passes.

Earlier Auth/Wire/Timers, IC Call, CLI and CI-fixture evidence remains in
`target/auth-wire-timers-20261028/`, `target/ic-call-adoption/`,
`target/issue-693/` and `target/ci-blockers-20261028/`. Historical warm checkpoint
62.75s → 18.56s (runner 92s → 33s) is not a measured release speedup.
Recent ICRC-21 native protocol tests, enabled runtime-probe compilation and strict
Contracts/Core/facade lint passed; no installed case established that slice alone.

## Unresolved evidence and follow-up

Fresh hosted/macOS qualification remains unresolved. Published 0.112.0 CI run
40659441082 failed ordinary tests, PocketIC and both macOS Host/CLI compilation
jobs; locally corrected causes are not a fresh hosted pass. The October 27
`ActivationResetAdoptionConflict` remains unexplained; isolated adoption passes
cannot disprove it. Actual managed IcyDB database/application acceptance remains
with downstream applications (Canic #590 / IcyDB #373).

Generic external codecs await Platform #78/#79. Canic-owned retirement-drain
request/reply Candid derives remain under Canic #613; complete codec extraction
is not claimed. Live provider linkage remains Blob #34 / Canic #494;
independent-credit reconciliation remains excluded under EX-05 in the closeout
plan. Preserve these distinctions from completed timer/Blob engine ownership.
Publication provenance hardening and deployment orchestration remain deferred.
Shared Tooling latest-stable maintenance remains #319. No release, hosted, or
live-system success may be inferred solely from this descriptive handoff.
