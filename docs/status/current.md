# Current handoff — 2026-10-30

## Scope and authority

The maintainer selected the combined 0.113 repairs/adoption batch and requested
all newly published IC dependencies, continuing the pending Persistence adoption.
The maintainer explicitly requested `make release-minor && make publish` for
0.113.0, with in-scope error correction and retries. This supersedes the earlier
source/package publication hold. The selected Make flow owns full qualification,
versioning, commits, atomic branch/tag delivery and crates.io publication. Fleet
deployment is not selected. The preceding minor's closeout is not claimed.
Before the governed bump, package versions remain 0.112.0 and the open 0.113 notes
contain the accepted batch. The pre-release source baseline is
`b9ed7f298bdaf8c9cceac142edf7e56b15d33208`; last published source is
`5c236ab130c0d63892ef281b45bdc3fa353e012f`. Preserve release/recovery state and
artifacts; reconcile uncertain delivery before any retry.

## Current dependency cohort and qualification

The root and both private Blob consumer catalogs/locks select Blob/contracts
0.58.0, Memory/Storage/Metrics Stable 0.56.0, Build/Host/Testkit 0.70.3,
Auth/protocol 0.23.0, Backup/Agent 0.50.2, Query 0.76.2, Platform/Call 0.8.2,
Logging 0.17.2, Metrics 0.17.5, Timers 0.32.1 and Wire/derive 0.7.23.
All 23 direct root IC selections match the latest stable published checksums at
lookup. Each root/consumer lock has one Memory, Storage, Metrics Stable, Platform
and Timers identity. Blob 0.58 resolves the former 0.55/0.56 runtime conflict;
the pending Persistence adoption is now complete. The Rust 1.91 floor and existing
workspace boundaries remain. Local qualification uses the governed Rust 1.99.1.

The exact Testkit server and release-metadata editor are prepared at 0.70.3.
The embedded allocation peer and its source/tool/lock evidence are refreshed.
Shared Tooling remains reviewed 0.27.6 at
`5a72359b5be19d5221334cf2c9b3204621d3f3f6`.
Node/npm/TypeScript remain 24.21.0/12.2.0/6.0.3; Platform SDK 0.1.1 uses Core 6.1.1
and is still the latest published browser selection.

The first 0.113 release attempt passed workspace Clippy and both consumer Clippy
checks, then correctly refused a stale embedded allocation peer after the newer
Tooling/Metrics lock selection. No version/tag/publication effect occurred. The
catalogs now align on Tooling 0.70.3/Metrics 0.17.5; the peer is refreshed and
verified. Commit the fix and retry the requested release.
Exact logs are at `target/release-0113-20261030/attempt-1.log`.

Earlier focused evidence: `target/dependency-refresh-20261030/README.md`; isolated source
and targets: `.tmp/dependency-refresh-20261030`. All 724 selected native cases pass
(Core 485, Contracts 7, Control Plane 66, Host 157, facade 9); five opt-in Host
cases retain their declared ignore conditions. Warning-denied all-target/all-feature
lint passes for Core, Host, Control Plane and Blob. All 15 selected installed PocketIC cases pass: two identical-Wasm Memory upgrades,
dedicated/embedded Blob (five embedded funding scenarios), eleven timer/metrics
cases and the original Host checkpoint workflow. These are local framework proofs,
not live-provider or browser-UI acceptance. The final Backup 0.50.2/Query 0.76.2
host patches repeat Host native/lint and checkpoint proof; unchanged runtime source
and native/Wasm dependency closures retain the other 14 installed cases. Exact
records and reused scopes are in `runtime-reuse.json`.
Nineteen scoped files were integrated under the existing main build locks; original
bytes/hashes are retained, including the concurrent newer Backup/Query lock records,
and the Git index is unchanged. All non-Markdown qualified
source, catalogs, locks and fixtures match the isolated copy.
The earlier `target/dependency-refresh-20261029/` retains its 657 native/17 installed
receipt on Blob 0.57/Persistence 0.55.2/Tooling 0.70.0. The preliminary 0.56 Core
receipts in `target/persistence-metrics-adoption-20261029/` and
`target/persistence-adoption-20261029-primary/` retain their original scopes.

Auth 0.23 consumes admitted session/replay pairs, uses protected upstream
`SessionLimits`/`RequiredScopes` constructors and returns compact admission
receipts. Full records remain in the host store; current-authority authorization,
immutable leases, generation/replay/revocation fences and custody assertions remain.
Memory 0.56 owns the bounded native Wire ledger (`ICML` / `ICMWIRE1`). Metrics
Stable retains `ICMW` rows; Canic-owned values retain `CSW\x01`. No predecessor
reader, migration, automatic ledger reset or unknown-row replacement is provided.
Deploy by clean reinstall; same-release restoration and uncertain-effect recovery
remain required.

## Maintained boundaries

The selected #700 Call hard cut removes generic builders, response wrappers and
workflow/infra forwarding. Framework requests use IC Call directly through stateless
metrics/error instrumentation; counters count polled attempts. Separate evidence
remains at `target/call-cut-20261029/`.
Wire owns owned RPC codecs, stable records and checkpoints. The CBOR/u128 adapter
and decoding tree are removed. ICP-refill amounts preserve upstream BigUint overflow
evidence; replay replies use the lossless Wire projection. Checkpoint/caller/completed-
leaf commitments use independent `.wire.v1` domains. Dedicated Fleet activation
commitments and Host release-build CBOR evidence retain their exact encodings.
`target/storage-wire-20261029/qualification.json` retains the earlier storage-cut
receipt and frozen runtime lock; do not relabel it as the current cohort's proof.

IC Build owns capture/staging/input evidence/traversal/caching; Canic owns recipes,
source/tool custody, transforms and admission. Private staging helpers are removed.
Earlier #529/#530 evidence is at `target/ic-build-completion-20261029/`.

#708 is integrated locally from `86c23f97a6a01c8f3c39dac2be7ff573d3bb813d`.
Fixed provider rosters default to heap; immutable stable overrides share one
upstream Metrics Stable map. Canic owns lifecycle/sampling, attribution,
authorization and bounded selection/history. IC Logging owns bounded heap
recording/filtering/pruning; logs reset on upgrade/reinstall. Stable log checkpoints
remain absent. `canic_logs`/`canic_metrics` require controllers; selected cached Blob
observations use `canic_public_metrics`. Storage grants no publication authority.

Blob stays at `crates/canic-blob-service`; its two independent consumer fixtures stay
under `integrations/blob-service`. Adapters mount explicitly with one host lifecycle
and Memory runtime. IcyDB has no engine dependency; actual consumer composition stays
downstream (#373/#590). Auth's unpublished scaffold still refuses mounting; issuer/
receiver/browser/Root enrollment remains incomplete (#491). The optional Rust-mounted
ICRC adapter retains its required external Candid contracts.

## Backup and remaining acceptance

The approved hard cut removes `canic-backup` and the backup/restore CLI; live
operations remain unavailable. IC Backup/Agent owns the generic engine. Canic retains
Fleet authority, runtime fences and a thin Host checkpoint adapter with mandatory
admission/custody and whole-unit/context commitments. Production transport, durable
ingress/reply custody, Root-paid capture, complete manifests/transfer, stopped restore
and terminal reference release remain unfinished (#513/#394/#550). Do not recreate
a generic operator runner here (IC Backup #75). The authorized 0.112.1 retirement
notice is published; historical evidence and `target/backup-hard-cut/retired-source.tar.gz`
remain retained.

Earlier `target/adoption-qualification-20261029/` records 327 native, 38 installed
cases and four ICRC profiles on its older frozen graph, not the current cohort.
Native Auth allocation qualification does not complete Wasm/all-family heap acceptance
#535. Fresh hosted/macOS, delivered-source/registry-only acceptance and publication
provenance remain separate. Provider linkage #494 / Blob #34 and Platform codec
#78/#79 / Canic #613 retain their external obligations. EX-05 excludes independent
provider credit reconciliation. Jobs/Relay extraction remains unimplemented.

The explicitly selected 0.113 release/publication flow is now active. Its full gate
and release/publication admission own delivery and registry provenance. Remaining
production integrations above retain their separate acceptance boundaries. Do not
start the next minor or claim human closeout from a passing gate. Status remains
descriptive; the release flow owns receipts/versioning. Shared upkeep #319 remains.
Prior handoffs are retained in the refresh/adoption evidence roots; release evidence
stays at `target/session-release-0112/` / `target/session-release-0113/` and
`target/release-0113-20261030/`.
