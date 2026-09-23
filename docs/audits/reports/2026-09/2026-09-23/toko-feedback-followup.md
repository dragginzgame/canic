# Toko Miner feedback follow-up

Date: 2026-09-23. Scope: Canic changes and read-only downstream/source inspection.
No downstream edits, live calls, publication or controlled build measurements.

## Evidence and disposition

Toko Miner's `docs/status/evidence/2026-09-23-staging-0.3.8.md` records successful
matched Canic/CLI 0.110.38 recovery, 50 Applied effects and a zero-new-effect
terminal replay. This supersedes the earlier host-only version-mismatch blocker;
it does not qualify the current dirty Canic candidate. Authenticated gameplay
was not manually tested in that evidence.

Its `docs/status/evidence/2026-09-23-deployment-latency.md` and anchored entries
in `docs/upstream/canic.md` identify four follow-ups:

| Feedback | Current disposition |
| --- | --- |
| CANIC-166: early exact CLI/runtime preflight | Existing locked offline Medic check retained; mismatch advice corrected and pre-qualification/recovery instructions added. |
| CANIC-150: progress and recovery handoff | Existing timing events summarized for humans; completed batch reconciliation refreshes persisted counts. No additional polling, retry deadline inference or JSON schema. |
| CANIC-160: convergence/replay latency | Retained requests attributed below; missing final authority/asset timing coverage added. No live speed-up measured or freshness check removed. |
| CANIC-176: build latency | Controlled cache-hit/miss qualification remains separate work. The reported 1,049.60-second artifact build had all misses and no comparable prior inputs. |

## Replay source trace

The reported 97.872-second replay has 33 `canic_root_command` and 33
`canic_observability` requests. Completed outer spans cover terminal inventory
(31.546 seconds, 35 attempts), planning (21.446 seconds, 22 attempts) and Root
management (2.960 seconds, two attempts). These are partial attribution, not a
complete wall-clock decomposition.

The [retained receipt/plan attribution](toko-terminal-replay-attribution.json)
binds the replay receipt to the exact current plan by operation and plan digest.
It finds nine inspection/reserve pairs inside terminal inventory and 24 pairs
without a parent observation span after Root management completes. That last
stage ends at 57.830 seconds; the final protected inspection completes at
97.871 seconds. Other authority/status reads also occupy this interval, so its
40.041 seconds are not solely the protected inspection cost.

The exact plan contains 24 distinct retained asset Principals under one Root.
The final [authority verifier](../../../../../crates/canic-host/src/fleet_ensure/workflow/reinstall/mod.rs)
iterates those assets through a bounded observation collector, issuing a fresh
reserve preflight and protected inspection for each. This supports one final
inspection per retained asset rather than duplicate traversal. Individual child
correlation remains unavailable: request `target` names the Root endpoint, not
the inspected child. The missing outer timing span, not a demonstrated redundant
call, is the confirmed attribution gap.

This slice now encloses both reinstall authority collection and retained-asset
verification in the existing `RootManagement` timing stage. The original Root
status span remains nested, so summaries exclude it from outer totals. Successful
and failed observations retain their full drained request counts. This changes
attribution only; the retained .38 receipt above is not a measurement of the new
instrumentation.

The maintained [replay owner](../../../../../crates/canic-host/src/fleet_ensure/workflow/continuation/mod.rs)
first collects terminal inventory, then observes/replans against the merged
estate, and finally verifies terminal estate, authority and conservation.
Configured-owner observations already share a planning snapshot, which expires
before terminal authority checks. A new broad replay cache would cross that
deliberate freshness boundary.

[Terminal inventory](../../../../../crates/canic-host/src/fleet_ensure/ops/current_inventory/mod.rs)
validates the Registry, Coordinator operation, Root authority, selected protocol
bindings, workload allocation and actual controlled canisters. Its protected
inspection calls invoke the existing
[reserve preflight](../../../../../crates/canic-host/src/canister_protocol/inspection/mod.rs)
before each Root command. That preflight queries `canic_observability` for the
exact inspection target, validates Root/target identity and available reserve,
and propagates failed reads. This explains why the two request families can
occur in pairs; it does not prove every recorded pair is this caller or redundant.
Paid inspection reads are not new journalled deployment effects.

The next useful measurement is per-target attribution across inventory,
planning and final authority verification, including complete outer spans and
request-parent correlation. Only then compare observation reuse within one
unchanged authority boundary or local contract-parsing costs. Keep exact
controller/module/allocation checks, fresh reserve evidence, conservation and
interrupted-operation recovery. Do not sum concurrent request durations as wall
time or reuse a reserve quote across paid inspections.

The observed gateway failure also reaches `IcpEnsurePlatform::read_status_with`,
which only treats a typed canister-not-found result as absence. Other transport
errors propagate. A retry change needs a typed read-only failure classification
and bounded owner before implementation; neither prose matching nor replay of an
issued mutation is justified by this trace.

## Qualification limits

Focused qualification passes: 21 CLI progress/receipt tests, the locked offline
Medic mismatch regression, five host Store batch/recovery tests, and strict
Clippy for host/CLI targets. Eight focused reinstall inspection/authority tests
also pass, covering fresh reserves, concurrent reads, failure draining and the
new complete outer timing boundary. Formatting, whitespace and report-link checks pass.
The new host regression binds every progress count to the already-written
journal and verifies that observed-complete chunks are never submitted again.
No full workspace or PocketIC suite was run.

This slice changes diagnostics and their propagation, not transport scheduling
or deployment semantics. Controlled build timing still needs clean cold, warm,
relocated, gameplay-change and dependency-change inputs with recorded hit/miss
reasons. The completed B1 footprint evidence supplies no build-speed baseline.
No end-to-end performance improvement is claimed here.
