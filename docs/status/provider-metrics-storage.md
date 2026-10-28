# Provider metrics storage contribution — 2026-10-28

Owning issue: [Canic #707](https://github.com/dragginzgame/canic/issues/707).
The accepted policy is heap by default, immutable stable overrides per provider,
one governed map per canister, and one recording/projection API. Blob's core
remains independent; its standalone work remains
[Blob #109](https://github.com/dragginzgame/ic-blob-storage/issues/109).

## Source and integration boundary

Prepared on `feat/provider-metrics-storage` in an isolated clone from verified
default main `5c236ab130c0d63892ef281b45bdc3fa353e012f` (0.112.0). The normal
Canic checkout and its pending 0.113.0 metrics/dependency cutover are untouched.
This contribution adds only its own notes to the already selected 0.113.0 batch;
package versions are unchanged. It does not merge, release, publish or deploy.

Published `ic-metrics-stable` **0.51.13** matches this base's Memory/Storage family;
the three locks advance only Memory/Storage 0.51.10 to 0.51.13 and add that store.
Metrics stays 0.16.3, Platform 0.7.0, Tooling/Testkit 0.69.0, Wire 0.7.18 and
Blob/Contracts 0.51.0. No unpublished path override is used. The active owner
must align the store with its final coordinated Persistence family when bringing
this slice into its newer pending graph, and qualify that graph separately.
Keep [#599](https://github.com/dragginzgame/canic/pull/599)'s recorder/reporting
cutover; this slice does not replace it or edit those recorder/DTO implementations.
The adapter path on this base is `integrations/blob-service`; the active owner
has a separate relocation to `crates/canic-blob-service` to preserve at integration.

## Implemented and qualified

The [provider guide](../features/runtime/provider-metrics.md) owns the API,
configuration and durability limitations. Fixed rosters admit 16 providers/256
metrics; fresh stable records are explicit and retained missing/malformed records
refuse. Arithmetic and codecs stay upstream. The Blob adapter supplies admission
and reference RPC observations, replay-aware outcomes and instruction quality.
Current accounting remains authoritative gauges. No new endpoint, background
flush, business journal, recovery controller or runtime storage switch was added.

Focused Linux evidence is retained under `/tmp/canic-metrics-storage-*.log` and
this clone's `target/test-runs/`. Nine provider and four configuration/authority
tests pass. Strict library and affected test-target Clippy pass. Actual Rust
1.91 checks the canister core for wasm32. Metadata and direct Platform boundaries
pass for all three workspaces; deployed consumer resolution has one matching
Memory/Storage/Metrics family and no Host/Testkit packages.

Both governed managed composition cases pass through the normal CLI build path:
dedicated heap and embedded stable. The embedded case covers its five existing
funding/recovery variants. Checks cover exact replay, decoder trap before
recording, units, sample counts, public query caching, stable reopen versus heap
reset, and fresh platform comparison windows after same-release upgrades. Source
manifests and all three locks stayed fixed during those runs. Canonical formatting
afterward sorts a native target manifest block and derive order/whitespace.
Final review moves RPC observation orchestration from ops into workflow; endpoint
macros delegate to that owner while ops retains the fixed registration roster.
Both focused managed cases pass again after this layering correction, with
canonical formatting already applied and all five tracked manifest/lock inputs fixed.

Retained managed build IDs are
`127009de67d1a2f0a93f4b3339cf507c43124d9b2c0398045c93d9ed46c60e64`
(dedicated) and
`c4eff13e87af158de2df8e4eb882bc530dadd970c1b0635281486a6fd04fcea1`
(embedded). These are fast-profile local qualification artifacts, not deployments.
The final-layout builds are
`c7151fbad6b2a2798741c6da5cd8f8099ed0f2139287d085692187b3cef579ac`
(dedicated) and
`ffddf6ba66b9258565ec88db00941aefe96dd0e3ab204fa1d089c5d9b9815c07`
(embedded). Their final logs are `/tmp/canic-metrics-storage-final-dedicated.log`
and `/tmp/canic-metrics-storage-final-embedded.log`; both complete governed cases
pass, including all five embedded funding/recovery variants.
The first managed attempt exposed an ungated native validator in Wasm and failed
before installation; its log remains alongside the corrected successful run.
Earlier compilation/test-fixture failures are retained rather than overwritten.

## Remaining qualification and adoption

No full repository gate, native macOS execution, hosted acceptance, installed
production provider test or final pending-owner dependency graph was qualified.
The configured Rust 1.91 library/Wasm checks pass. An additional attempt to run
Core unit tests on 1.91 stopped on existing `std::assert_matches!` test macros
already present in the base; unit tests use the configured 1.99.1 toolchain.
Full installed snapshot/timer restoration remains separate from the normal
same-release upgrades qualified here; retain Tooling
[#263](https://github.com/dragginzgame/ic-tooling/issues/263)'s owning investigation.
Metrics are non-authoritative diagnostics: old snapshots may restore old totals;
the current platform comparison window does not prove lifetime continuity or
clear business recovery fences. Stable recording pays synchronous per-record
write cost. Existing heap providers are not silently migrated by this API; each
owner must opt into registration and preserve its own outcome/replay semantics.

Keep #707 open until owner integration/default-branch acceptance. Keep Blob #109
open for its standalone/reporting integration; do not add Canic to Blob's core.
