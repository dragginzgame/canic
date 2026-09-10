# VS1 throughput qualification

2026-09-10. Working-tree qualification for the open 0.110.14 batch.

VS1 is qualified on the settled ic-memory 0.13.1 candidate. Both exact
production-adapter journeys pass with an unchanged 1,686-file source inventory.
The candidate shares validated Root inspections within one observation and
selects pinned native ICP for governed PocketIC runs. All 144 duplicate balance
queries disappear. These measurements retain their original dependency
checkpoint. The temporary CANIC-162 deferral was qualified separately; the
maintainer then requested newly published IcyDB 0.257.4 with ic-memory 0.13.2.
No timing below is relabelled as a measurement of either later candidate.

## Final source qualification

| Phase | Mixed topology and two resets | Retained-estate reinstall |
| --- | ---: | ---: |
| Whole journey | 1,153.72s | 575.12s |
| Initial artifact resolution/build | 424.62s | 85.50s |
| Replica/transport setup | 0.66s | 0.73s |
| Initial review | 6.48s | 19.52s |
| Initial working Fleet | 172.04s | 99.72s |
| Initial terminal replay | 52.57s | Included below |
| Two resets / retained-estate reinstall | 497.25s | 369.54s |
| Governed runner, including native compilation | 1,156s | 578s |

The first and second deliberate resets take 255.30s and 241.95s. Both preserve
the complete interruption, real row wipe, newly written row preservation,
distinct operation identity, canister identity and effect-free replay checks.
Retained recovery includes replacement artifacts/generation (35.76s), lost Root
install/recovery (15.27s), successor reviews/convergence (280.80s) and final
state/replays (36.44s). All 24 balance observations issue zero additional remote
calls and take 7.187s total. Configured-canister inspection, authority checks,
funding reviews, cycle conservation and fresh terminal inventory remain intact.

All 23 focused host cases, three timing cases and the exact catalogue regression
pass on this source, as does warning-denied all-target/all-feature Clippy for
host and internal fixtures. Each journey has 33 completed, paired phase spans.
Both owned servers and invocation scratch directories are cleaned by the runner;
shared Cargo and verified artifact caches remain available. Server high-water
RSS is 609,472 KiB for mixed and 552,692 KiB for retained recovery.

The earlier attribution runs took 1,906.37s and 1,056.83s. Dependency and cache
states differ, so those elapsed differences are observations, not an isolated
causal percentage or a whole-gate prediction. The source inventories, tool
context, exact commands, output hashes and raw logs are retained in the archive.
No broad suite, package version, publication or sibling edit ran.

## Native CLI and recovery follow-up

The installed `icp` on PATH was an npm launcher. Its native payload and the
governed installation at `/home/adam/.cargo/bin/icp` are byte-identical ICP 1.4.0
binaries (SHA-256 recorded in the qualification receipt). Seven version probes
give median startup times of 105.5ms through Node and 15.9ms directly. Production
adapters repeatedly launch the CLI, including real version checks. The runner
now selects the pinned native executable inside private scratch before starting
PocketIC; ordinary and plan-only lanes retain their existing tool resolution.
Explicit `CANIC_TEST_ICP_BIN` remains authoritative. Missing tools, launchers and
version mismatches fail before Cargo. No version checks or production transport
calls are bypassed. Focused selection/refusal tests, scoped ShellCheck and the
release-integrity contract guard pass.

| Retained-estate phase | npm control | Native control | Native plus inspection reuse |
| --- | ---: | ---: | ---: |
| Whole journey | 1,177.14s | 529.72s | 757.98s |
| Initial artifacts | 214.93s | 4.40s | 236.06s |
| Initial working Fleet | 157.89s | 97.56s | 106.91s |
| Replacement artifacts/generation | 38.80s | 7.45s | 46.19s |
| Lost Root install/recovery | 32.78s | 15.41s | 16.34s |
| Successor reviews/convergence | 613.99s | 346.43s | 289.07s |
| Retained state/replays | 84.05s | 37.19s | 39.97s |
| PoolBalances remote attempts | 144 | 144 | 0 |
| PoolBalances stage total | 125.15s | 77.77s | 7.82s |

All three exact runs pass. The two controls use the previous host observation
implementation; their 24 Wasm/gzip/Candid hashes match exactly across fresh
invocation roots. The native control hits both existing sealed release caches.
The candidate includes ic-memory 0.13.0 and rebuilds those artifacts, so its
whole-journey duration is not a controlled speedup ratio. Post-run inventories
also differ after memory-adoption work; that alone does not establish edits
during a running test. The memory owner's handoff confirms the 0.13.1 update
waited for the 0.13.0 candidate command to finish. Earlier wording that treated
that later source difference as an overlap was too strong and is corrected here.
The final candidate records 24 balance observations with no additional remote
calls, while configured-canister inspections and fresh authority checks remain.
Its runner takes 867s including native compilation; the native control runner
takes 532s. These are focused checkpoint observations, not a new complete-gate
duration or a sub-hour result.

The existing verified artifact cache already avoids repeated builds across
private invocation roots. Replica setup remains about one second, so adding
mutable fixture reuse is not justified by these measurements. Keep fresh
mutable estates and the complete recovery cases. Dominant removable work is
CLI launch overhead and repeated inspections within one decision.

## Measured baseline

| Phase | Mixed topology and two resets | Retained-estate reinstall |
| --- | ---: | ---: |
| Whole journey | 1,906.37s | 1,056.83s |
| Initial artifact resolution/build | 388.04s | 96.66s |
| Replica/transport setup | 1.06s | 1.02s |
| Initial review | 7.27s | 31.20s |
| Initial working Fleet | 363.33s | 161.60s |
| Initial terminal replay | 105.46s | Included below |
| Two same-release resets / retained-estate reinstall | 1,041.06s | 766.34s |

The first reset takes 530.97s; the second takes 510.09s. Retained-estate reinstall
includes replacement artifacts/generation (39.11s), Root review (3.01s), lost
install response/recovery (32.79s), successor funding reviews/convergence
(607.01s), and terminal state/replays (84.41s). Nested timings are inclusive;
do not add parents to their children. Setup alone is not the dominant cost.

Both runs used an existing shared Cargo target and missed their sealed release
artifact caches. The second run benefited from the first run's Cargo work.
These are attribution baselines, not a controlled comparison with each other,
with the historical 0.110.12 gate, or with the later reuse change. Concurrent
sibling work adds timing variability. The runner totals are 1,937s and 1,089s;
they include native compilation outside the journey spans. Environment changes
between native commands and Make also rebuilt dependencies; subsequent commands
explicitly use `ICP_ENVIRONMENT=local` to match the governed runner.

The retained run records 144 PoolBalances remote attempts, taking 124.450s,
across six observations with 24 pending assets each. Source inspection shows
that configured-canister projection already inspected these pending assets in
each same observation. Other recorded stage totals include ConfiguredCanisters
150.091s, ProtocolActions 87.686s and TerminalInventory 66.578s. Stage durations
include local work; remote-attempt counters exclude CLI version/identity
subprocesses and replica handshakes. Unattributed time is not established as
pacing, simulator or subprocess cost.

## Implemented boundary and coverage

The existing observation snapshot now retains successful inspection responses
by exact Root and asset principals. Each consumer still validates its module
requirement and exact Root controllers; active operator checks remain fresh.
The snapshot expires on success or error before an effect or another decision.
Inspections outside that scope remain fresh. Transport and authority failures
are not retained. Terminal inventory and replay workflows are unchanged.

Three new native regressions exercise shared-call bounds, changed balances,
expiry after success/failure, distinct Roots and targets, stricter module
requirements, operator changes, and retry after transport/controller failure.
All 23 focused platform cases pass. Three diagnostic tests verify parentage,
monotonic duration, failure/incomplete state and failed observation records.
The exact governed catalogue ordering/membership regression passes, as does
warning-denied all-target/all-feature Clippy for host and internal fixtures.

The two baseline journeys preserve every existing assertion: mixed startup,
real user-row wipes, a distinct second reset operation, interrupted installs,
reconstructed adapters, retained identities/controllers, nineteen Workloads and
five Ready assets, reviewed funding bounds, cycle conservation and effect-free
replays. No catalogue case or assertion was removed. The separate Store-origin
outage proof retains its existing owner and earlier .14 evidence.

Each baseline has 33 paired completed spans with valid identity/parent bindings
and no unmatched start. Records use schema 1, process/span IDs, monotonic
microseconds and explicit started/completed/failed/incomplete states. Killed
processes may leave unmatched starts; these are incomplete evidence, never a
zero-duration success. Output is emitted as one JSON-line buffer.

## Evidence and next boundary

[The structured qualification](qualification.json) retains source/log hashes,
commands, phase records, observation counts and resource context.
[The evidence archive](evidence.tar.gz) contains the raw logs and checkpoint
records. The [historical baseline](baseline.json) remains separate.

The first mixed run preceded the observation callback and atomic-write
correction. The retained run includes both diagnostics. Both precede inspection
reuse. Later concurrent Canic memory-observability edits changed runtime source
and the shared fixture; a subsequent timing cannot be attributed solely to VS1
without controlling that additional source difference. A transient missing
`Sha256` import in that work failed isolated catalogue compilation; its owner
corrected it and the exact catalogue regression then passed. The same owner's
explicit fixture drop resolved the subsequent lifetime lint; combined scoped
Clippy passes on the corrected source.

The inspection-reuse candidate is restored after the controls. The final runs
use the normal native-selecting runner and include the 0.13.1 reset fixtures.
Their before/after inventories match in content and membership. The original
control checkpoints remain separate from this final qualification.

Attribution identifies CLI launch overhead and redundant same-observation reads
as the removable costs. Existing recipe-keyed artifact reuse already preserves
exact outputs across invocation roots, and sub-second replica setup does not
justify additional mutable fixture pooling or catalogue splitting. Retain that
existing ownership and every recovery case. This is the measured disposition
of VS1's artifact/setup slice; no speculative fixture or release-identity
redesign remains pending in VS1.

The throughput outcome and its changelog are qualified. CANIC-162 was briefly
deferred, then restored when IcyDB 0.257.4 aligned the memory dependency. See the
[current disposition](../../reports/2026-09/2026-09-10/canic162-memory.md) for
that separate composition proof. No total gate speedup or sub-hour result is
claimed. Package versions remain 0.110.13; the maintainer chooses the release gate.
