# CANIC-147 public chart history and CANIC-148 qualification boundary

Date: 2026-09-08. Source batch: open 0.110.12 draft on published 0.110.11.

## Accepted scope

The maintainer accepted CANIC-147's five-minute optional sampling task and
24-hour bounded heap history. Canic retains the existing timer authority owner;
application code composes one synchronous measurement participant. IcyDB remains
a provider. Publication is disabled unless public metric families are selected.
No stable history, second scheduler, authentication store or compatibility lane
is introduced. See the maintained [public observability contract](../../../../features/runtime/public-observability.md).

History has 288 fixed slots per admitted series, with a global 256-series and
8 MiB accounted allocation cap. Latest snapshots have independent per-family
bounds. Sampling coalesces source observations in a slot, skips missed slots and
expires old series. Public reads only copy bounded cached data; they cannot
sample, fund, enumerate users or call another canister. Source observation time,
units, counter windows, saturation, coverage and canister version are explicit.
Counter deltas require adjacent comparable points in the same retained series.

A selected application participant runs synchronously within the sampling task.
Its typed failure preserves the prior application snapshot while the other
selected families continue. Application collection remains the application's
responsibility: Canic cannot impose an instruction bound on arbitrary callback
code. Actual producer qualification is therefore a separate adoption requirement.

## Focused evidence

CANIC-147 and the complete accepted Canic source batch are ready for release
approval. The root and detailed 0.110.12 changelog drafts cover the change;
package versions remain unchanged. Final evidence: 19 focused native cases,
eight timer-focused PocketIC cases, canonical Candid equality, the exact native
timer claim check, focused production/test Clippy and document/catalog guards.
The sampling implementation is covered by the passing PocketIC run and final
native regression. The subsequent public-health label correction has separate
focused contract evidence below; the PocketIC run predates that enum change.
CANIC-148's actual Toko Miner producer qualification remains open and must precede enabling its
publication. This downstream gap does not block releasing the opt-in Canic
capability.

## Public health clarification

Public `Health` now returns `PublicHealthStatus::Responding` (`responding` on
the wire). Answering a query does not prove runtime health, authority restoration
or Fleet readiness. Both canonical Fleet Candid interfaces use the dedicated
public enum; protected diagnostic health is unchanged. Downstream bindings and
UI labels must follow the current public contract. This is a direct hard cut.

The two passing focused canonical contract tests cover both Coordinator and Store public
health types and the maintained Store history graph. Their log is
`/tmp/canic147-health-contract.log`; passing warnings-denied focused Clippy is retained at
`/tmp/canic147-health-clippy.log`. No sampling logic changed for this clarification.

## Sampling cost evidence

The first PocketIC run passed seven of eight cases, including the new history,
suspension and restoration journey. The existing cost bound correctly rejected
the first implementation: 46,590,581 instructions on initial sampling and
26,300,512 on repeat sampling. Its 20-million-instruction limit was retained and
extended to cover the initial allocation as well. History now uses a bounded
capacity ring without initializing empty points, one map entry lookup per series,
and once-per-slot expiration. A second measurement reached 19,302,398 initial and 20,118,266 repeat instructions,
so repeat sampling still failed the unchanged bound. The final implementation
also avoids redundant bounded-string copies and merges sorted snapshots in linear
time when checking prior observations.

The final eight-case PocketIC target passes: 120.55 seconds including fixture
builds, 249 seconds for the complete targeted runner including host compilation.
No hour-long Fleet or workspace gate ran. The same instruction ceiling covers
first sampling, repeat sampling and an actual scheduled tick:

| Measurement | Instructions |
| --- | ---: |
| First sample, 256 synthetic checkpoints | 18,881,572 |
| Repeat sample, 4,096 synthetic checkpoints | 17,632,234 |
| Scheduled tick with the populated source | 18,355,413 |

These are neutral fixture measurements, not a universal limit on arbitrary
application providers or Toko Miner qualification. History adds work compared
with snapshot-only publication. The feature remains opt-in.

The PocketIC evidence covers public/observer separation, no timer when disabled,
one sampling claim when enabled and after same-release upgrade, sampling without
readers, cache-only reads, skipped slots, application rejection preserving its
source time, volatile authority suspension and an empty history after lifecycle
upgrade. It also retains the existing cycle-tracking failure-isolation and timer
capacity checks. It is not whole-Fleet or live downstream qualification.

Logs:

- `/tmp/canic147-native.log`: initial selected native tests, exact timer claims
  and current Candid structural equality.
- `/tmp/canic147-native-qualified.log`: final focused history and sampling regression.
- `/tmp/canic147-clippy-final.log`: changed libraries, facade, runtime probe and selected
  protocol/timer integration targets, all features, warnings denied.
- `/tmp/canic147-core-test-clippy.log`: core production/test Clippy.
- `/tmp/canic147-pocketic.log`: first run exposing the cost regression.
- `/tmp/canic147-pocketic-final.log`: intermediate cost measurement.
- `/tmp/canic147-pocketic-qualified.log`: passing final neutral timer qualification.

The existing release-turnaround edits remain a preserved part of the
same open patch. No broad validation, version bump, publication or deployment is
part of this implementation turn.

## Downstream CANIC-148 findings

Toko Miner and IcyDB were inspected read-only. This is not completed downstream
cost qualification and does not authorize enabling publication.

| Required producer/evidence | Current source finding | Required downstream work |
| --- | --- | --- |
| Application aggregate participant | Toko Miner's status still marks richer User/IcyDB producers pending. Its current managed qualification checks disabled publication. | Adopt the new Canic contract and connect one bounded local participant. Keep the selection empty until qualified. |
| Exact users per shard | Game Shard's observability cache already obtains User table counts, including a bounded indexed page-walk fallback when exact count metadata is unavailable. That refresh has its own existing timer. | Supply maintained counts or cached values with their original timestamps. Do not move database/page scans into Canic's sampler or create a second sampling timer. |
| IcyDB execution measurements | `metrics_report()` clones and sorts all observed entity counters; it reports hits, total instructions and a window maximum. Reset identity is currently the window-start millisecond timestamp. | Bound the selected measurement work. Obtain an exact reset identity before publishing cumulative counter deltas; two resets in one millisecond are otherwise ambiguous. A window maximum is not an interval maximum. |
| Actual IC sampling cost | No retained actual-producer sampling measurement was found. Game Shard's native observability test instruction counter returns zero. | Measure the actual Wasm participant and complete Canic sampling tick in PocketIC, with representative application work and the selected maximum cardinality. Native zero-cost counters are not IC evidence. |

Source owners inspected:

- `/home/adam/projects/toko-miner/docs/status/current.md` (CANIC-148 adoption and
  pending producer qualification).
- `/home/adam/projects/toko-miner/apps/toko_miner/game_shard/src/observability/mod.rs`
  (User count, bounded fallback, cached timestamps, existing refresh owner).
- `/home/adam/projects/toko-miner/apps/toko_miner/user_hub/src/qualification.rs`
  (default-disabled public publication proof).
- `/home/adam/projects/icydb/crates/icydb-core/src/metrics/state.rs`
  (report work, instruction semantics and reset identity).

The downstream measurement should retain the exact artifact/release identity,
selected names and entity cardinality, provider-only and complete-tick instruction
counts, cached query cost, retained history bytes, truncation and rejection
behavior. Compare observed User counts with known fixture state. Exercise a
counter reset and rejected provider, confirm cached reads retain source time and
prove protected observer access remains denied to public/player callers.
Instructions must not be relabelled as cycles burned; exact cycle accounting
remains with its existing owner.

Canic's AGENTS.md restricts existing sibling repositories to read-only access.
Completing the missing Toko Miner adapter and its actual-producer qualification
requires explicit authority to edit that repository, in addition to the relevant
IcyDB provider contract being available. Canic's neutral fixtures cannot close
that downstream evidence gap.

## Read-only source provenance

At inspection, Toko Miner HEAD was
`c8d47bfa6c9e9b445f81abbcb76062a6c5d5373f` and IcyDB HEAD was
`bd40327d5657bd5700933d0d7e2dca162b4a4539`. Source findings include working-tree
content; the exact inspected SHA-256 values were:

| Source | SHA-256 |
| --- | --- |
| Toko Game Shard observability | `dedc79ad5f872db3d676659a444de6c4a541605690aea6dc668229adb4cfd73a` |
| Toko User Hub qualification | `f6c33d085bc544d94651a12f3981e45c8aeae935f07a45c834b3401e1a1f8482` |
| IcyDB metrics state | `94905a1158ab2b9f36030434755006d5c00ae37416e3fff9f3e3cede15893b49` |
