# CANIC-165 Root source authority

This checkpoint extends [complete build binding](canic165-build-binding.md) with
protected Root source selection and direct Store publication authority. It is
part of the accepted Canic-only implementation, not a release or closeout audit.

## Implemented contract

The required `fixtures` Root-manifest field binds admitted roles to descriptors
and content IDs. The host validates the complete fixture child, projects only
each Root's admitted Component/child closure and includes deduplicated source
payloads in its Store allowance. Root repeats role/content/capacity validation
against its embedded topology and protected initial release digest. The current
pre-1.0 manifest shape has no default or predecessor fallback.

The controller-authenticated `PrepareStoreFixture` command chooses one role from
that exact staged manifest. Root checks its adopted Store and rechecks installed
authority after awaiting the manifest. Only Root registers the descriptor;
Store retains the upload cursor. The existing exact Root/retained-controller
predicate admits direct verified chunk publication. A retained controller must
still be an observed controller. No publisher gains source registration, grant
creation or target read authority from that permission.

Bootstrap and live status verify completed source identity, chunk count, cursor,
byte count and completion flag. They neither relay nor re-read fixture payloads.
Fixture descriptor/source/grant commands have exact replay inventory entries.

## Qualification

| Check | Result |
| --- | --- |
| Host artifact/projection and deterministic Store compiler | 17 tests pass |
| Root manifest/source/completion validation | 6 tests pass |
| Exact role-command replay inventories | 13 tests pass |
| Host/Core/control-plane/facade/internal fixture library and tests, including governed Root fixture | Clippy passes with warnings denied |
| Changed Store integration target | Clippy passes with warnings denied |
| Actual retained Store plus lifecycle/import/transport composition | 4 tests pass, 102.02s; 177s runner |
| Exact Prepared-Root source registration/bootstrap journey | 1 test passes, 188.06s; 217s runner |

Store evidence covers unknown-source refusal, unauthorized upload, retained
controller removal, a 1 MiB controller upload, fresh-heap restart and exact retry,
while the existing source/grant/read recovery assertions remain. Root evidence
uses actual Root and Store Wasms and covers an unselected role, incorrect
manifest size, repeated registration recovering the same retained cursor,
incomplete bootstrap refusal, direct controller upload, exact upload replay and
successful bootstrap/status while Root remains Prepared.

The Root journey includes a 94-second Store build, 50.80-second Root build and
35.54-second issuer build. The Store suite also rebuilds artifacts. These are
qualification timings, not deployment speed comparisons or production ceiling
measurements. No broad workspace or release gate ran.

[Retained evidence](canic165-root-sources-evidence/) includes command logs and the
1,598-file Rust/config/Candid snapshot, unchanged across runtime qualification.
Afterward, import-only cleanup in the host unit test and internal fixture was
checked with final scoped Clippy and native tests; `source.sha256` records the
final checkpoint separately from `runtime-source.sha256`. The Root module header
was then aligned with its registration/publication boundary; no executable
statement changed after final native and Clippy validation.

## Remaining batch

The host still must journal source preparation/upload, externalize payloads into
its existing plan content store and reconcile lost responses. Root-derived
installed-target grants, registered importer/receipt integration, initial
parent/child readiness, later Shards, funding/backoff and reference/retirement
lifecycle remain. Fixture-bearing Fleet generation stays disabled. Existing
retained fixture state conservatively blocks GC, without a completed retirement
contract. CANIC-165 and the combined worktree are not push-ready.

Package versions, publication state, sibling repositories and live Fleets were
unchanged. The root Unreleased note remains unassigned.
