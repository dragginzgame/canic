# CANIC-165 transport and receipt qualification

2026-09-11. Continues the [application commit proof](canic165-import-commit.md)
and [selected provisioning design](../../../working/canic165-fixture-provisioning/design.md).
The maintainer requested continued Canic issue fixes and deferred minor closeout.
This qualifies the FP1 application/transport boundary; CANIC-165 remains open.

## Result

The composed Canic/IcyDB probe can now pull a chunk from an exactly selected
test source. Its durable application checkpoint binds the source alongside the
installed Fleet identity and content. A heap lease admits one outstanding pull;
it does not introduce another durable row cursor. After the await, the adapter
reloads authority and opens a fresh synchronous IcyDB request for the commit.

PocketIC exercises real canister messages and proves:

- wrong source replacement, unauthorized source callers, wrong installation
  grants, rejected delivery and conflicting bytes leave rows/progress unchanged;
- a held response permits observing one outstanding read; another pull returns
  the typed Busy outcome without issuing another source read;
- replacing an empty import while its reply is held fences that old callback,
  leaving the replacement cursor at zero with no rows or receipt;
- actual same-Principal reinstall during a held fetch discards the old call
  context; the fresh installation remains empty and can import under its own binding;
- traps after insertion and after checkpoint persistence in the response
  callback roll back both writes and release the heap lease so retry succeeds;
- discarding a successful caller response and restarting retains the committed
  row/cursor; retry does not fetch or write it again; and
- readiness remains NotReady after runtime activation, import completion and
  the first validation step. Only the durable final validation receipt satisfies
  the readiness observation, including after restart and an unobserved final reply.

The test peer retains one 16-byte response in heap and authenticates its exact
target binding. It is not the production Store and does not qualify durable
source upload, Root-issued grants, retention, cycle allowances or Fleet placement.
The controller-only empty-import invalidation is fault injection, not a product
reset or readiness override. The protected row inspection endpoint remains a
test diagnostic; this is not proof of a shipped application dispatch gate.

## Test mechanism and qualification

The source waits on bounded management raw-randomness calls to permit explicit
consensus rounds before replying. A self-call loop was rejected during test
development because it could drain before the paused state was observable.
The 64-round test envelope is only a fault-injection bound, not a production
retry policy or fixture limit. Production delivery still requires its own
bounded backoff and capacity evidence.

Final checks:

| Check | Result |
| --- | --- |
| Composed lifecycle integration, including application and transport proofs | 3 passed; 34.12s tests, 39s runner |
| Probe Clippy, all targets/features, warnings denied | Passed |
| Integration-target Clippy, all package features, warnings denied | Passed |
| Rust/Cargo/toolchain source snapshot | 1,561 entries unchanged through final checks |

The earlier run encountered a locked Cargo metadata failure while the workspace
lockfile changed concurrently. The toml 1.1.6 and toml_edit 0.25.15 entries were
preserved; final checks use a fresh snapshot containing them. This slice did not
update dependencies or relax locked/offline validation. Published IcyDB 0.257.4
and ic-memory 0.13.2 remain selected. The subsequent successful run's extra cold
compilation is not a fixture-delivery timing measurement.

Exact commands, final logs and source digests are bound by the
[qualification record](canic165-transport-qualification.json).

## Next implementation

FP2 integrates retained opaque Store objects, exact Root-derived target grants,
bounded target delivery and receipt-gated placement with the existing owners.
Its acceptance includes a Prepared Root, initial parent/child dependencies,
later-created Shards after operator exit, outages, funding, retention and recovery.
The combined ordering proof must exercise that actual production integration;
requiring it before those paths exist would make the implementation sequence
circular. No acceptance requirement is dropped or replaced by this test peer.

The .15 operator batch remains separate. CANIC-165 is not a completed or
push-ready provisioning batch. No production API, package version, Git
publication, broad validation, live deployment or sibling edit was introduced.
