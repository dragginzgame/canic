# CANIC-138 complete local convergence measurement

On 2026-09-06, one controlled five-Workload/five-Ready comparison reduced
apply-to-readiness time from **585.206s to 422.149s (27.86%)**. Apply through
both terminal replay checks fell from **782.796s to 520.756s (33.47%)**.
Both runs completed all 53 effects, exact terminal conservation, original-plan
effect-free replay and a newly generated empty plan's effect-free apply.
This supplies complete local evidence for CANIC-138 criterion 7, replacing the
earlier partial-phase comparison as the current performance evidence.

## Comparison boundary

Both runs execute the existing governed
`literal_zero_fleet_with_initial_children_reaches_effect_free_terminal_replay`
case through the concrete `IcpEnsurePlatform` and real PocketIC control-plane
roles. They retain the same five-plus-five topology, accelerated fixture
observation pacing, lost-response injections, authority checks and assertions.
The fixture's Ledger stub supplies deterministic local creation outcomes;
this is not a mainnet latency or cost benchmark.

The candidate ran first with normal observation reuse. The baseline ran next
with only the observation snapshot disabled. Both used identical temporary
timing spans and the existing progress callback. The
[input record](artifacts/feedback-performance/inputs.json) binds source and
fixture hashes, invocation, execution order and evidence hashes. The
[instrumentation patch](artifacts/feedback-performance/timing-instrumentation.patch)
and [single baseline change](artifacts/feedback-performance/disable-observation-reuse.patch)
make the experimental difference reviewable. The production adapter was
restored byte-for-byte after both runs; no timing helper, cache-disable option
or new production mode remains. Informational fixture progress timestamps remain.

Fresh runs allocate their own operation identities. This comparison does not
claim identical plan, Principal or Wasm identities across runs. It measures
one paired local execution, not a distribution or a universal speedup.

## Whole journey

| Interval | Baseline, reuse disabled | Candidate, normal reuse | Reduction |
| --- | ---: | ---: | ---: |
| First apply event to readiness | 585.206s | 422.149s | 27.86% |
| First apply event through both terminal replays | 782.796s | 520.756s | 33.47% |
| Terminal replay window | 197.590s | 98.607s | 50.10% |
| Complete test, including fixture setup | 826.020s | 564.530s | 31.66% |

The first two intervals exclude artifact setup and are the deployment
comparison. Time outside the apply/replay window was 43.224s baseline and
43.774s candidate; it includes setup, initial planning and cleanup. Cargo test
compilation and runner startup are outside the test duration and are not
included in the claimed reduction. This does not measure unchanged-release
build reuse under CANIC-139.

## Phase and call attribution

Initial apply-to-readiness phase wall times include their observation work:

| Phase | Baseline | Candidate |
| --- | ---: | ---: |
| Infrastructure | 172.024s | 177.641s |
| Import reconciliation | 167.819s | 86.162s |
| Control plane | 63.638s | 57.407s |
| Workload provisioning | 31.480s | 31.048s |
| Pool readiness | 2.277s | 1.943s |
| Terminal verification across phase boundaries | 147.968s | 67.948s |

Temporary spans additionally measure adapter call time across the complete
test, including replay:

| Adapter call category | Calls in each run | Baseline | Candidate |
| --- | ---: | ---: | ---: |
| Create | 13 | 6.950s | 7.233s |
| Import protocol calls | 10 | 14.113s | 13.236s |
| Workload provisioning | 2 | 1.386s | 1.303s |
| Complete Fleet observations | 14 | 339.304s | 98.498s |

Call counts are adapter attempts, not committed effect counts. Import calls
are classified by the active import-reconciliation phase. Complete Fleet
observation spans exclude other status calls elsewhere in the adapter.
Phase wall times and call spans overlap and must not be added together.
Repeated phases are summed. Progress timestamps are checked for monotonic
ordering and against the enclosing test duration.

The unchanged observation-call count with much lower observation time supports
the intended mechanism: reuse avoids repeated status and protected pool-page
queries within an observation. The small infrastructure regression and nearly
unchanged provisioning time also show why the complete measured boundary
matters. This evidence supports retaining the bounded reuse already in Fleet
Ensure; it does not justify new concurrency, caching or authority machinery.

Raw evidence: [candidate log](artifacts/feedback-performance/candidate.log),
[baseline log](artifacts/feedback-performance/baseline.log), and
[structured timings and progress events](artifacts/feedback-performance/results.json).
The remaining batch readiness and Fleet retirement work are tracked in the
[feedback readiness report](fleet-feedback-readiness.md).
