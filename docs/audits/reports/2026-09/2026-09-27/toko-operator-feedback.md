# Toko Miner operator feedback follow-up

Date: 2026-09-27. Published base: Canic 0.110.44. Open patch: 0.110.45.

Scope: CANIC-183, the current CANIC-150 named-progress request, CANIC-160's
transient Root query failure, and CANIC-176's release-build evidence. Toko's
upstream feedback and deployment records were inspected read-only. No live IC effects
or downstream edits are part of this work.

## Operator corrections

CANIC-183 uses the existing bounded receipt owner for completed-source
preparation and completed-estate reset. Selection of the local retained route
precedes measurement. Remote preparation, plan review, publication approval and
execution are inside it. Preparation/publication digests are recorded separately
from execution plan hashes. The CLI finalizes success or failure before rendering;
interrupted and continued invocations retain distinct files. Diagnostic failures
cannot grant execution authority or replace the plan/journal.

CANIC-150 projects each reviewed component occurrence into its deployment,
placement, member path, spec and Root. Exact sequential member cursors supply
state; aggregate Root completion does not. Unavailable member evidence remains
unknown. The live panel prioritizes the current cursor and bounds displayed rows;
plain/JSON milestones and receipts retain all occurrences. Reported retry reasons,
deadlines and observation age retain their existing owners. No extra polling is
introduced.

CANIC-160 connects Fleet apply's Root status observations to the existing native
authenticated query transport. Its finite transient retry envelope preserves
request identity and typed permanent failures. This covers pool reconciliation,
readiness, authority and terminal partition/pool reads. Other external transports
keep their own behavior; no global subprocess retry or mutation replay is added.
This change is not a claim about matched live deployment latency.

## CANIC-176 disposition

The latest Toko 0.3.14 deployment records a 991.33-second application Release
build. Its application/Root batch includes 187.43 seconds of declaration work
and 536.80 seconds of runtime Cargo/link. These nested/overlapping spans cannot
be summed into a wall-time decomposition. The 371.147-second frontend
reinstall/sync belongs to the frontend deployment boundary.

The retained [controlled matrix](../2026-09-24/toko-performance-followup.md)
already qualifies complete-build reuse on a frozen .39 application snapshot:
925.54 seconds for the initial build, 1.86 seconds unchanged, and roughly
460–507 seconds after note, gameplay or dependency changes. Warm repetitions
preserve exact artifact and release identities. The matrix's tool/source/cache
conditions differ from .14; it does not establish a regression or improvement
between these releases.

The known cost remains runtime recompilation after whole-package inputs and the
embedded release identity change. Ignoring Markdown inputs or assigning an old
Wasm to a new release would bypass current input/release authority. The test-speed
session's host-fixture caching and PocketIC concurrency do not qualify application
Release build reuse. No speculative cache contract change belongs in this fix.

Further optimization needs a matched current-release application measurement:

1. Freeze the exact CLI binary, current application source, path dependencies,
   Cargo lockfile, toolchain, profile and relevant build environment in a new
   scratch copy inside Canic. Keep the downstream checkout read-only. Use a
   private target; retain the input inventory and tool hashes.
2. Run the current public build command below once with an empty private target,
   then unchanged. Retain both complete logs, wall time, resource observations,
   selected release manifest and every output hash. Registry and OS caches may
   remain warm; identify that condition explicitly.
3. Measure one controlled application change and unchanged repeat, then a path
   dependency change and unchanged repeat. Restore exact source between cohorts.
   Freeze load/concurrency and distinguish intentional invalidation from a miss
   on unchanged inputs. The older audit harness is immutable historical evidence;
   its hard-coded source edits are not instructions for today's application.

Run from that prepared scratch workspace, with the frozen binary on PATH:

```sh
canic --environment staging build toko_miner --profile release --verbose
```

This is a build-only measurement, not a deployment command. The current batch
does not run a new application Release matrix or claim to close the broader
CANIC-176 performance target. Diagnostic receipt coverage restores the missing
measurement boundary for a future naturally required reset; never repeat a live
hard cut solely to collect timings.

## Qualification

All 368 selected native regressions pass: 67 CLI progress/receipt cases and 301
host observation/protocol/retry cases (14 ignored host cases remain
unselected). Scoped warning-denied host, CLI and internal library/test Clippy
passes. Logs: `.tmp/upstream-native.log` and `.tmp/upstream-feedback-clippy.log`.

The exact completed-reset PocketIC case passes in 139.46 seconds (156-second
test stage including compilation; 157-second runner). It invokes the real CLI in
fresh child processes for preparation, reset review, failed apply, continuation
and plain/JSON terminal replay. Every selected invocation announces and retains
one bounded receipt. Source archives, retained IDs, application state clearing,
cycle conservation, lost-install recovery and no additional terminal effects
remain asserted. Successful CLI reports are captured; failures expose diagnostics.
The ordinary receipt tests retain interruption, omitted-event, timing-pair and
terminal-restoration coverage.

Final log: `.tmp/upstream-reset-pocketic-final.log`; complete test trace:
`target/test-runs/20260927T163714Z-40252.WPsonS/1.log`. An earlier in-process
qualification also passed in 216.19 seconds before the output-capture correction;
these timings have different cache conditions and establish no speedup claim.

This bounded operator correction is ready for review in the existing `.45`
batch alongside the separately qualified test-throughput changes. Versions stay
`.44`, and changes remain uncommitted. No broad validation, release transaction,
Git publication, live IC effect or downstream modification ran. Publication,
Toko adoption and matched live/app-build performance acceptance remain separate.
The operations guide documents current receipt fields, retries and the existing
exact-digest operator commands.
