# CANIC-165 application commit qualification

2026-09-11. Partial qualification of the
[fixture provisioning design](../../../working/canic165-fixture-provisioning/design.md).
This proves a persistence composition, not a shipped provisioning API or a
complete CANIC-165 implementation. Work stays in Canic; IcyDB remains the
published 0.257.4 dependency and ic-memory remains 0.13.2.

## Result

The managed Canic/IcyDB probe now stores deterministic application rows in
IcyDB and its import checkpoint in a separate application-owned stable cell.
Both writes can share the IC message rollback boundary. No await separates
them. After row insertion succeeds, failure before checkpoint completion must
trap; simply returning a typed error is insufficient.

PocketIC confirms the negative control: an application error returned after
insertion leaves the row committed while the cursor remains zero. Both survive
a same-release fresh-heap restart. This is expected IC commit behavior, not an
IcyDB defect. The qualification deliberately exposes that unsafe path only in
the controller-protected test canister.

The maintained candidate path passes:

- Prepared endpoint fencing and rejection of an unauthorized controller caller;
- typed failure before writes, and traps before rows, after rows and after
  checkpoint persistence, with no retained row or advanced cursor after traps;
- exact retry after restart, conflicting bytes, wrong installation, oversized
  chunks and skipped sequence rejection;
- stored-row validation in primary-key order, one row per invocation, with a
  separate bounded end-of-data check and restart after the first validation step;
- persisted receipt replay after restart without repeating the database scan;
  and
- actual same-Principal reinstall: rows and receipt disappear, old installation
  requests reject, and the new installation can begin its own import.

Begin captures Canic's actual installed Fleet activation identity, including
Fleet, install operation and release build. Every commit/validation compares
that identity again. The application cursor is the sole row-progress owner.
This tests stale requests at the commit boundary; it does **not** claim a
paused Store fetch/callback race has already been qualified.

## Bounded validation and measurements

The fixture has eight rows, each encoded as two fixed-width `u64` values in a
16-byte chunk. Ordered SHA-256 digests bind the selected content. The test-only
descriptor admission ceiling of 256 rows is not a qualified production limit.
No arbitrary 128-row or 20M-instruction product threshold was introduced.

| Measured step | Instructions to the probe's return snapshot |
| --- | ---: |
| Maximum commit | 19,499,486 |
| Maximum one-row validation | 20,704,750 |
| Completed receipt replay | 13,658,619 |

These include the probe's authorization/identity and checkpoint work up to its
counter read, exclude later response serialization and source transport, and
do not establish worst-case production bounds. The validation measurement
exceeds the historical 20M sampling reference without failing an arbitrary gate.
No Wasm-size or end-to-end deployment-speed improvement is claimed.

The runtime test caught an incorrect initial use of `DynamicQuery::limit(1)`:
that caps the complete query rather than selecting a reusable one-row page.
The fixed validator starts each bounded query at its durable next primary key,
checks exact row identity/content, and explicitly proves exhaustion. A typed
failure leaves the previous durable validation checkpoint intact. Completion
compares the accumulated count and creates the receipt without rescanning.

## Parent/child ordering boundary

The existing exact initial-Shard regression passes: a Prepared Root provisions
the configured Hub/Shard tree, reaches terminal membership and replays without
another allocation. This is baseline bootstrap evidence; it has no fixture
source or application-data gate and does not qualify their future composition.

Root Fleet activation and individual Component runtime activation are distinct.
A Root can remain Prepared while a child Component runtime is Active. The
source-grant path must therefore admit an exactly authorized installed child
without waiting for Root Active, parent data readiness or final parent
membership. The target's own Prepared endpoint fence remains intact.

The current sharding allocator registers the child and sets its pool lifecycle
Active after allocation. The bootstrap loop counts retained pool entries.
Receipt-dependent application eligibility cannot be added as an unconditional
wait to that bootstrap completion path: the combined grant, parent bootstrap,
child import and membership ordering needs an explicit runtime proof. Retain
the same target while loading and gate both assignment and target application
access; do not invent another allocation or independently editable ready flag.

## Evidence and remaining work

The two-test IcyDB integration target passes in 30.33s (38s governed runner).
The exact initial-Shard regression passes in 431.01s (444s runner, including
artifact builds). Focused Clippy passes with warnings denied. The same 1,559
Rust/Cargo/toolchain source files remained unchanged across both final runs.
Exact commands, measurements and evidence hashes are in
[the qualification record](canic165-qualification.json).

CANIC-165 remains open. Combined receipt-gated parent/child ordering, paused
transport callbacks, retained Store upload/read grants, source retention,
later Shards, funding/outages and application facade/config/protocol delivery
remain unimplemented or unqualified. Relationship-heavy application imports
and their production ceilings remain downstream qualification, not a conclusion
from this two-field fixture.

The .15 operator batch remains separate. The CANIC-165 proof has no allocated
release/minor and is not a complete push-ready provisioning batch. No broad
gate, package-version change, Git publication or staging effect ran.
