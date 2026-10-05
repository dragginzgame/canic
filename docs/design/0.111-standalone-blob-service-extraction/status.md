# Blob Extraction Implementation Status

Date: 2026-10-04

The 2026-10-05 dependency update selects published ic-blob-storage 0.14.9.
Adapter and both consumer Clippy checks, managed Fast builds and focused
dedicated/embedded PocketIC proofs pass. The latest
[handoff](../../status/current.md#published-blob-dependency-update--2026-10-05)
records artifact hashes and logs; earlier 0.14.6 evidence below is historical.

The maintainer selected the [in-repository hard cut and isolated adapter](0.111-design.md)
after publishing 0.110.52 and selected 0.110.53 for the accumulated changelog draft.
Package versions remain unchanged. This selection does not close FR1 or certify
the preceding minor.

The maintainer subsequently selected optional embedded placement as well as a
separate Component. `mount!` now contributes endpoints and host-selected memory
requests without taking lifecycle or sampler ownership. Public synchronous
`lifecycle::install` / `restore` and `metrics::sample` let the application compose
its own state and reporting; `canister!` assembles the same parts for a dedicated
service. The embedded example owns a persistent counter, application endpoints,
a combined init contract and a combined sampler. This does not add service
machinery or a dependency to Canic's core. Both forms pass managed Fast builds, warning-denied Clippy and focused PocketIC
installation/refusal/restore/recovery proofs. The embedded case also verifies
independent application state and combined metrics. The latest
[handoff](../../status/current.md) records exact artifacts and dependencies; earlier evidence below
retains its recorded source and dependency scope.

The embedded implementation, command group, billing Medic option, stable
allocations and dedicated main-workspace test/CI lane are removed. Generic
feature/descriptor coverage uses maintained Canic fixtures. Active guides point
to the independent service; published historical source links retain their
immutable 0.110.52 snapshot.

The Canic-owned adapter in `integrations/blob-service` is a separate library
workspace selecting published `ic-blob-storage = 0.14.9` and ic-memory 0.25.0.
Its reusable `canister!` macro supplies the managed shell inside the consumer's
own package, App/role metadata and compiled topology. The checked-in `consumer/` example has
its own workspace and Fast profile. The unpublished adapter and consumer must
select the same checkout's Canic facade. Service memory registration belongs to
the canister macro, so importing the re-exported DTOs is passive. The upstream
repository is unchanged and retains no Canic dependency.

[Canic#444](https://github.com/dragginzgame/canic/issues/444) identified why the
previous standalone-only build did not qualify downstream composition. Host now
permits the exact shared facade while preserving identity, feature and protected
internal checks. Managed Fast builds pass for the independent consumer and a
Canic-owned copy of Toko's topology containing a blob Component. The Toko copy
reports 4.38 MiB of Wasm code. This is build evidence, not a live Toko installation.

Before Canic's subsequent ic-memory 0.25 adoption, the focused consumer PocketIC
proof passed actual-Principal installation, exact
plain certificate Candid shape, authorization refusal, malformed/oversized input
refusal without tenant effects, per-canister usage, same-release restoration and
authenticated current-instance recovery. A generic fixture callback encodes
application arguments after allocation without changing protected Canic inputs.
All 36 Host package-contract tests, four managed Component fixture tests and
warning-denied Host/facade/library/consumer Clippy checks pass. Evidence is under
`target/review-validation/`; the [current handoff](../../status/current.md#reusable-blob-composition--2026-10-04)
records fixture qualification and the combined-checkout limitations.

Earlier extraction/reporting evidence includes 30 Core and 48 Host role-contract
tests, 12 facade regressions, 34 Observatory tests, scoped lint, runner barriers,
test inventory, shell and documentation checks. These results qualify their
recorded source states, not the complete current workspace.

The adapter exports upstream aggregate usage through Canic's existing
application metrics sampler. Observatory preserves freshness and redacts public
Principal dimensions. Reporting is per storage canister, not per tenant, and
does not prove provider readiness. Service tests remain upstream; the temporary
consumer proof does not restore a blob suite, runner, test feature or development
dependency to the default Canic graph. Decoder-budget regression feedback remains
[ic-blob-storage#7](https://github.com/dragginzgame/ic-blob-storage/issues/7).

The preceding adapter and consumer qualification selected ic-blob-storage 0.14.6 and
ic-memory 0.25.0, with one memory runtime identity in the consumer Wasm graph.
The main embedded allocation peer is refreshed and its read-only verification
passes on the completed Core graph, including the subsequent memory 0.25.1 patch
refresh (`053-fixture-refresh.log` and `053-fixture-final-verify.log`). The
[current handoff](../../status/current.md#checked-memory-slots--2026-10-04)
records narrow adapter/consumer qualification on this graph. Earlier PocketIC
and Toko-copy results above retain their original 0.24 scope. The focused consumer
PocketIC proof now also passes on the rebuilt service 0.14.6 / memory 0.25.0
artifact, including installation, refusal, metrics, restore and operator recovery.
Evidence: `target/review-validation/blob-consumer-pocketic-025.log` and the
post-fixture-refresh rerun `blob-consumer-pocketic-025-final.log`; the native
proof driver resolves memory 0.25.1 with the current main lockfile.

The production-path audit confirms that Root's top-level installer passes no
application arguments, including for Component Group members. The fixture's
post-allocation encoder is test support only. [Canic#444](https://github.com/dragginzgame/canic/issues/444)
tracks this remaining generic Root/Host delivery requirement: bounded bytes,
exact allocated identity, durable binding before installation, and same-operation
retry. Toko deployment must wait for that path; adding a role or changing
`app.init_mode` does not supply the missing arguments. Live uploads,
reads and provider behavior remain unqualified. The former direct blob-funding
command has no qualified replacement; use the independent service's operator API.
Backup/delegated-auth extraction is separate work. No broad suite, sibling edit,
release transaction, live deployment or Git publication ran.

Provider physical deletion, billing cessation, paid uploads and live retirement
are outside this source task. Keep their retained evidence and obligations intact.
