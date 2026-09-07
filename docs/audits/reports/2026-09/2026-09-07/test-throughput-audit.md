# Application-neutral test throughput audit

Date: 2026-09-07

Baseline: published `v0.110.8`, commit
`3e8c84cbd4e271d0d97ed2669524b008fcac7c73`.

## Scope and findings

The maintainer requested less redundant, faster deployment validation and
application-neutral fixtures. This batch changes the Canic test harness,
fixtures and runner. It does not change Fleet runtime authority, production
pacing, release validation requirements or application repositories.

1. The complete generated journey deployed nineteen Workloads and five Ready
   assets. Its essential shape is one standalone Component, one scaling Hub
   and one sharding Hub. One initial child per Hub and one Ready asset preserve
   that mixed topology with five Workloads and six total pool assets. The exact
   downstream deployment cardinality belongs in application qualification.
   The separate direct mixed-topology activation journey is removed: the
   generated journey owns mixed-topology readiness, while the focused
   initial-Shard case retains provisioning receipt replay. Supply arithmetic
   uses small synthetic single-child and multi-child cases independently.
2. The private host lost-Create proof repeated nine application-shaped roles
   and eleven synthetic protocol steps. Four neutral roles and three protocol
   steps retain real management effects, reconstruction after a lost Create
   response, conservation and effect-free replay. This remains distinct from
   the production-adapter journey's lost controller and reset responses.
3. Failed-import and generated-reinstall tests repeated complete fresh-estate
   recovery before reaching their own failure boundary. They now prepare real
   infrastructure through the existing production creation/initialization
   adapters. Public Root commands establish import state. No successful host
   journals are manufactured. Reinstall still first establishes and verifies a
   working Fleet, then exercises generated reviewed reinstall and recovery.
4. The Ledger stub accepted its topology and funding authority through init
   arguments but was built against an unrelated application-shaped config.
   That config and its special path helper are removed; the boundary stub uses
   the existing generic fixture config.
5. A separate release runner invocation selected only an ignored instruction
   audit and executed no tests. Its compilation now shares the runtime
   integration invocation. The explicit instruction-audit runner still owns
   execution of that audit. Every classified integration target remains in the
   complete test plan exactly once.
6. Exact Rust-path selectors could previously succeed with zero matching tests.
   The targeted runner now lists the exact selection and requires one matching
   identity before execution. A typo cannot produce successful test evidence.
7. Required-case and ordering assertions lived in an ordinary catalogue test,
   while the release harness called only the uniqueness check. The harness and
   focused catalogue test now call the same complete inventory assertion.
8. The blob-storage inventory guard required a Toko interoperability section
   and four regressions enforced that downstream dependency. The requirement,
   its test fixtures and its four tests are removed. The positive inventory
   fixture contains only gateway evidence. Canonical gateway/Cashier protocol
   surface, DTO and runtime tests remain. Historical external protocol
   provenance is retained in the inventory; it is not application qualification
   required of Canic. Nine Fleet unit-test names also lose their application
   prefix without changing their generic assertions.

## Coverage ownership

| Contract | Retained owner |
| --- | --- |
| Complete generated supply, mixed Component kinds, bounded successor phases, lost controller/reset responses, conservation and both terminal replays | Synthetic generated Fleet journey |
| Initial-child runtime activation and exact provisioning receipt replay | Focused initial-Shard activation fixture |
| First host Create lost response and host reconstruction | Four-canister synthetic host PocketIC proof |
| Generated changed-release reinstall, lost install response, working Fleet and replay | Generated reinstall journey |
| Underfunded import repair, lost Ledger withdrawal and reset responses | Failed-import journey |
| Initial estate funding before provisioning | Single-Workload estate funding journey |
| Reserve replenishment after Workloads consume Ready supply | Four-Workload refill journey |
| Full-capacity Failed reserve repaired without new creation | Four-Workload Failed-reserve journey |
| Complete supply arithmetic and insufficient-capacity rejection | Host generator unit test and generated journey preflight |

The funding cases have different starting states and decision paths. Their
common fee, lost-response and conservation checks are retained. The shared
PocketIC process, baseline cache and exact sealed-artifact cache remain in use.
Independent security, caller, controller, lifecycle and recovery cases are not
removed because their names resemble another case.

## Evidence

Focused results on this batch:

| Check | Result |
| --- | --- |
| Synthetic generated mixed topology, recovery, conservation and both effect-free replays | PASS, 540.83s case / 558s runner; includes 55.17s artifact rebuild |
| Prepared Failed-import repair, lost withdrawal/reset responses, conservation and both replays | PASS, 264.28s case / 280s runner |
| Prepared working Fleet → generated reviewed reinstall → lost-install recovery → working Fleet and both replays | PASS, 444.77s case / 455s runner; includes 36.71s replacement artifact build |
| Generator funding/supply filter | PASS, 3 tests; test execution under 0.01s |
| Renamed generic Fleet unit tests | PASS, 9 tests in 0.07s |
| Four-canister host lost-Create/reconstruction/replay proof | PASS, 1.02s case / 3s runner |
| Governed catalogue required cases, ordering and uniqueness | PASS, 1 test in 0.01s |
| Protocol inventory regression target | PASS, 20 tests in 0.33s |
| Changelog governance | PASS, 1 test |
| Affected host/internal library and test Clippy, all features; protocol-inventory target Clippy | PASS, warnings denied |
| Invalid exact Rust-path selector | PASS, rejected with zero matches before test execution |
| Formatting of this batch's eleven Rust files | PASS |
| Workspace integration inventory | PASS, all 39 classified targets retained |
| PocketIC plan-only resolution | PASS, five Cargo invocations; instruction audit grouped with runtime compilation |
| Changed shell scripts and current blob-storage inventory guard | PASS |

All focused qualification for this batch is complete. The initial Failed-import attempt reached
convergence and began replay but hit its seven-minute wall-clock cap, including
over three minutes of cold compilation. It is not passing evidence.
The first synthetic generated attempt hit its nine-minute cap after applying
48 of 50 effects. Its cold artifacts took 3m24s; this interrupted attempt is
also excluded from passing evidence.
The next Failed-import run completed convergence and both replays but failed
its final mutation-log check: prepared setup bypasses the ICP controller
wrapper, so it had neither created the log nor issued those controller calls.
The fixture now initializes an empty log before effects and requires zero
wrapper controller mutations for the prepared path. The fresh journey retains
its exact per-pool controller count. The corrected case passes in full. This
logging-only correction does not change the generated journey's effect path
or its expected controller count; that nine-minute journey is not repeated.

The successful generated case applies 50 effects, versus 104 in the previous
nineteen-Workload/five-Ready case. The maintainer reported 17m06s for that older
journey. The new 9m00s result is directional evidence, not an isolated benchmark
of topology reduction: the retained older run and this candidate also differ
in source, cache state and machine load.

## Readiness

The test-throughput batch is complete and ready to push within its scope. Its
root and detailed 0.110.9 changelog draft is prepared; package versions remain
0.110.8. No remaining test-refinement blocker is known.

Concurrent CLI/restore work also extends the open 0.110.9 draft. Its
[separate handoff](cli-command-audit.md) owns its qualification. That work was
preserved and is not certified by these test-throughput checks; combined patch
readiness requires both batches' evidence. No broad gate, version transaction,
commit or publication was performed for this audit.

## Limits and follow-up

No complete workspace or release gate is run for this implementation audit.
Total release duration therefore remains unmeasured on this candidate; smaller
topology and fewer repeated effects are not a claim of a measured total speedup.
Cold builds and serial real-canister protocol calls still contribute materially.
The sealed fixture cache includes `canic-host` as a Cargo input because that
crate owns build/finalization behavior. This conservatively also invalidates
artifacts after host unit-test edits. The final test-name cleanup caused such
a miss; Cargo reuse reduced that rebuild to 55.17s. The cache input boundary is
retained: narrowing it requires evidence that every artifact-affecting host
input remains covered.
Use the runner's per-case timings on the next maintainer-directed release to
identify the next demonstrated cost. Broad concurrency or another deployment
mode is outside this batch.

Downstream applications must qualify their actual Component topology, initial
child counts, Ready reserve, network policy and cycle budget after adopting
Canic. Canic's maintained release suite proves the reusable protocol contracts.
