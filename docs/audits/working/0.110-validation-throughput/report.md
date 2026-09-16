# Release-test throughput qualification

## Bounded retained-asset inspections — 2026-09-16

Activation reset inventory formerly inspected each retained asset serially.
It now uses the existing bounded observation collector, at most four assets per
Root at a time. Every asset still obtains fresh reserve preflight and management
inspection evidence; the observation cache cannot satisfy these checks. Identity,
lifecycle, exact controllers, module and cycle validation remain intact. Issued
reads drain before the first error in inventory order returns; later batches do
not start on failure, and failed batches publish no partial observation updates.
Successful remote call counts remain two per asset. A failed batch can have
other already-issued reads, which are drained before retry becomes possible.

Four native `reinstall_inspections` regressions pass: nine-asset bounded overlap
and exact sorted inventory; reversed-completion failures with fresh retry
balances/modules; cross-batch duplicate and lifecycle admission; and fresh low
reserve rejection despite previously cached evidence. Host all-target/all-feature
Clippy passes with warnings denied. Logs are
`/tmp/canic-throughput-reset-inspections-tests.log` and
`/tmp/canic-throughput-reset-inspections-clippy.log`.

The existing PocketIC case
`pic::fleet_registry::baseline::tests::activation_reset::source_bound_activation_reset_recovers_and_replays`
passes in both modes. It exercises two retained assets, source-bound preparation,
lost Root install response, recovery, conservation, exactly one reinstall and
effect-free replay. The native fixture separately proves the full four-wide
bound. No recovery assertion or production canister behavior changed.

The matched control changes only the collector call to sequential iteration;
per-asset checks, batch inputs, fixture, config, lockfile and tools are identical.
Candidate source was restored byte-for-byte before its run. Source is the dirty
.19 candidate over `eb2bfbeba`, with .18 package versions. Exact SHA-256 identities:

| Input | SHA-256 |
| --- | --- |
| Candidate `fleet_ensure/ops/platform.rs` | `6f71174972aa3f57a8886a63d1d6c70a71ac132d37a8d69dc7fd87f075db1598` |
| Serial scheduling control | `9e5e11dd813eb3d68fd01c8358799f4e7015ca28a530423d8dc2039854f8fd8b` |
| Measurement `Cargo.lock` (IcyDB 0.257.15) | `2a047eaa8005b4adfa8f387300742b4ba2c1de237f029eb1f6c719f031993a9a` |
| Fixture `baseline.rs` | `92f1abf598fb85a7db9555e1e9a29d1702e5dc91df28e3c8260877828463b790` |
| `canisters/audit/root_probe/activation.toml` | `c95d9c44417675c352666ebfda6619cd2eb45344abf052f10c322fd4edda1aa6` |

Both runs use repository `target/`, Fast/local fixture builds, Rust/Cargo 1.98.1,
ICP 1.5.0 and PocketIC 16.0.0. Initial fixture release ID is
`a4c128728412f11837b79ce8562e3115451db387e17361b79b4f15d02cbb36ae`.
The case derives its replacement release through the existing fixture path.
The scheduling diff and identity receipt are retained in
`/tmp/canic-reset-inspections-scheduling.diff` and
`/tmp/canic-reset-inspections-build-identities.txt`.

| Scope (seconds) | Serial | Bounded |
| --- | ---: | ---: |
| Initial artifact acquisition | 31.841 | 29.418 |
| `generation_and_initial_review`, including nested replacement artifacts | 113.761 | 105.720 |
| Nested replacement artifact resolution | 29.482 | 26.834 |
| Derived live phase excluding nested artifacts | 84.279 | 78.886 |
| Complete test | 146.19 | 135.73 |
| Runner including native compilation | 148 | 191 |

The derived live phase saves 5.392s (6.4%) in this one pair. It includes more than
inspection work, and machine/transport variability remains; the entire delta
cannot be attributed solely to concurrency. The candidate runner also recompiles
native code after restoring scheduling, whereas the serial binary was fresh.
Neither artifact timing nor runner timing establishes a controlled build speedup.
There is no whole-release timing claim. Logs:
`/tmp/canic-throughput-reset-serial.log` and
`/tmp/canic-throughput-reset-parallel.log`.

An earlier unrelated generated-reinstall case was stopped after source inspection
showed it did not directly exercise this path. Its retained
`/tmp/canic-throughput-reset-unrelated-aborted.log` is not passing qualification
or performance evidence; the two successful runs above use the correct case.

The bounded run completed at 10:33:23 +02:00. A concurrent IcyDB update changed
the manifest at 10:33:57 and lockfile at 10:34:02 to 0.257.18, after qualification.
The new lock SHA-256 is
`da87e705b548d465b2be85c23e4095f17a2a4c70bad847357d5ca9fe20f25c7c`.
Those changes are preserved. At the maintainer's request, the optional new-lock
recheck was interrupted during compilation; the passing evidence above belongs
to the stated measurement lock. Dependency requalification does not hold this
slice open. The .19 changelog includes it; broader compiler/link and live-journey
cost reduction remains open, with no broad validation or publication performed.

## Complete Fleet journey build pipeline — 2026-09-16

The read-only Toko recheck retains upstream source SHA-256
`5e6b42624ce254bcdeffc0c8b3bd4e6f6f659930e833787c1c0c983d54807d53`:
no newer feedback beyond the locally corrected CANIC-150/014. The next speed
slice replaces the complete-journey fixture's three serial builder calls with
the existing production `build_workspace_app_artifacts` call. Cargo remains
serial under the same target lock; privately captured Coordinator and Store
outputs finalize alongside later compilation. One inclusive `app_artifacts_build`
span replaces the former sequential build spans. Audit-Root substitution and
release manifest sealing still occur after all outputs return successfully.

The full release-artifact cache now explicitly binds `baseline.rs`, which owns
its producer. This deliberately invalidates prior records and future records
when that source changes; it does not claim a cache hit across producer changes.
No new production pipeline, mutable estate reuse, feature unification or
cross-release final-Wasm reuse is introduced.

The opt-in `pipelined_release_artifacts_match_serial_builds` qualification uses
the same process, config, source/lock, target, Fast/local profile and exact release
ID for the sequential control and pipelined candidate. It compares raw Wasm,
gzip, Candid, package/version, role/capabilities and protocol identity/digests.
All five roles match byte-for-byte:

| Role | Raw Wasm bytes | SHA-256 |
| --- | ---: | --- |
| Coordinator | 5,308,239 | `725a3a0551d45bc3a236347519a4cff46583d6988ef28d165a59cdb051a4ba7b` |
| Store | 3,738,409 | `b27a55e2a5035c15a59be1a4e997999a519ae6f9acce2e01027652d3724ef469` |
| Root | 10,221,946 | `029175006313a634652ee92bbfd12a958d0581aafc5786a86fabb80a778eaa93` |
| User Hub | 4,133,171 | `a4668c42259f33173efcf2e388675270fd876bd4284a3c4a2a549534da801505` |
| User Shard | 4,490,248 | `8a3f626209080492cfd789792b17ab5614c1d53aec1fbf7acb233869c9c9ad23` |

Source remains the dirty .19 candidate over `eb2bfbeba`, with package versions
.18 and the unchanged Cargo.lock identified below. `baseline.rs` SHA-256 is
`92f1abf598fb85a7db9555e1e9a29d1702e5dc91df28e3c8260877828463b790`.
Config is `apps/test/test-configs/literal-zero-initial-shard.toml`, SHA-256
`898fd951dbf73b8a88e59f3809933a064641db3ce18ec40e643c4271b10df506`.
Release ID is `a4c128728412f11837b79ce8562e3115451db387e17361b79b4f15d02cbb36ae`.
The governed runner uses Rust/Cargo 1.98.1, native ICP 1.5.0 and PocketIC 16.0.0.
The explicit `CARGO_TARGET_DIR` is the repository's `target/`; the host resolves
runtime Wasm beneath that target and declaration builds beneath `target/declarations`.
Changed Rust-source hashes are retained in `/tmp/canic-throughput-app-source-sha256.txt`.

The focused existing four-initial-shard activation and effect-free replay test
passes in 61.17s (64s runner), including 14.47s artifact build/seal and 20.92s
total artifact resolution. Its complete-artifact cache misses and is committed
after sealing. Both existing release-cache regressions pass, retaining distinct
repeatable release identities and restoration of fixture authority. Internal
package all-target/all-feature Clippy passes with warnings denied.

Logs: `/tmp/canic-throughput-app-parity.log`,
`/tmp/canic-throughput-app-activation.log`, `/tmp/canic-throughput-app-cache.log`,
`/tmp/canic-throughput-app-clippy.log`. The parity runner takes 506s, including
both control/candidate builds and initial native compilation. Declaration/runtime
compilation takes 170.49s/186.95s initially and 6.22s/17.78s on the warmed repeat;
the later live case settles further to 0.70s/0.79s. These differing cache states
and concurrent sibling load prohibit attributing those ratios to pipelining.
Infrastructure finalization totals roughly five seconds in the control, placing
this overlap improvement in perspective. Larger compiler/link and live-journey
costs remain the next work; no complete-gate speedup or push-ready throughput
outcome is claimed. The open .19 changelog includes this qualified slice.

## Cargo build-input correction — 2026-09-16

After qualifying the new CANIC-150/014 feedback corrections recorded in the
current handoff, the next speed investigation found two executable causes of
build-script invalidation. A tiny isolated Cargo consumer invokes the real
already-linked Canic build macro. No substitute macro or measurement framework
is introduced, and nested Cargo has a private target and no extra dependencies.

The original macro rewrites `canic.role-runtime-authority.rs` and Root's generated
sources even when their contents match. Cargo's fingerprint diagnostic confirms
the generated authority file is newer than its build-script output record; a
second unchanged invocation executes the macro again. Preserving unchanged
bytes/timestamps stops this repeated invalidation. After that change alone,
the regression still fails when an unrelated `.canic/release-builds/artifact.wasm`
is written beside the config: the macro's recursive parent-directory watch
causes the settled run count to increase from two to three. Replacing that watch
with exact config and package-manifest tracking removes this second cause.

Output repair watches remain intentional. A newly written output may cause one
settling rerun, after which unchanged builds must remain fresh. The final test
proves that property, unrelated-artifact isolation, changed-config invalidation,
exact byte restoration after missing/tampered generated output, and rejection
of missing explicit config or mismatched manifest role. This is real Cargo
freshness evidence, not a complete production-Wasm or release timing comparison.
No release identity, feature selection, runtime configuration bytes or final
artifact-cache admission is weakened.

Both build-cfg tests, thirteen build-support tests, existing changelog governance
and scoped all-feature Canic library/build-cfg Clippy pass. Logs:
`/tmp/canic-build-freshness-before.log` (output self-invalidation),
`/tmp/canic-build-freshness-parent-before.log` (neighbor invalidation),
`/tmp/canic-build-freshness-after.log`, `/tmp/canic-build-support-tests.log`,
`/tmp/canic-build-freshness-clippy.log`. Changed-source/lockfile identities for
the combined feedback and speed correction are in
`/tmp/canic-019-feedback-speed-source-sha256.txt`. The earlier measurements below
retain their own source checkpoints. Whole-release savings remain unmeasured;
remaining compiler/link and complete-journey costs stay next in this batch.

## Shared infrastructure builder — 2026-09-16

The governed Fleet fixture acquired Coordinator and Store through separate
`cargo run --profile fast -p canic-host --example build_artifact` calls. This
compiled a native copy of the host in `target/pic-wasm`, despite the fixture
binary already linking that production owner. Removing only Coordinator's
launcher would leave Store paying the native compilation cost later.

Both now use the existing in-process artifact helper. The production host
still owns package admission, Wasm compilation, exact release binding and
finalization. Fixture cache recipes retain exact source/dependency/tool inputs,
explicitly include their producer source and resolve the actual target through
`canic_host::canister_build::canister_build_target_root`. The duplicated target
resolver in the complete-journey fixture is removed. Ordinary Root-baseline
script qualification is a separate surface and remains unchanged.

The initial control successfully compiled its native helper in 69 seconds,
then its Wasm build was blocked by the sandbox's compiler-cache socket policy.
That is evidence of removable native compiler work, not a successful end-to-end
baseline. Repeating with socket access proved identical raw Wasm, gzip and
Candid outputs. The first successful Coordinator comparison took 52.759s via
the example and 3.084s directly, but the former warmed the Wasm target, so that
ratio is not a speedup claim. A warm repeat took 3.196s / 3.032s, with native Cargo
reporting the example fresh. The improvement is removal of the extra native
compilation when source changes or its target is empty; warm artifact work is
essentially unchanged. No new complete-release timing is claimed.

The focused ignored regression compares both infrastructure roles against the
maintained example under identical Wasm profile, target, config and release ID.
It also proves Coordinator cache reuse and exact restored bytes. No timing
threshold affects success. Final qualification passes for both roles:

| Artifact | Raw Wasm bytes | Example, native fresh | Direct |
| --- | ---: | ---: | ---: |
| Coordinator | 5,308,239 | 3.361s | 3.108s |
| Store | 3,738,410 | 4.935s | 4.777s |

Raw Wasm, gzip and Candid are byte-identical for each pair. The Coordinator
Wasm SHA-256 is
`725a3a0551d45bc3a236347519a4cff46583d6988ef28d165a59cdb051a4ba7b`;
Store is `a2fa9cae1f083ba90ac1134cdcd9cc707fb2fb01ee61dcac75cacfdd59c56eb2`.
Both use the exact fixture release
`a4c128728412f11837b79ce8562e3115451db387e17361b79b4f15d02cbb36ae`, Fast profile,
local network and `target/pic-wasm`, with the unchanged .18 dependency lock and
pinned Rust/Cargo identified below. Configs are `apps/test/canic.toml` for
Coordinator and `canisters/test/delegation_root_stub/canic.toml` for Store.

The final single Fleet deployment restore case passes: 61.76s case, 68s runner,
including a Store cache miss/build of 46.05s. Coordinator acquisition takes
4.10s; the Root stub is cached. The intermediate Coordinator-only change passed
in a 201s runner with additional artifact misses. Those different cache states
prevent attributing the elapsed difference to the source change. This live
proof uses the governed runner, pinned PocketIC 16.0.0 and native ICP 1.5.0.
Four artifact-helper tests and both existing exact-release/cache-authority
regressions pass. Host and internal-testing all-target/all-feature Clippy pass
with warnings denied. No registered recovery case or serial ordering changes.

Logs: `/tmp/canic-throughput-coordinator-parity.log` (initial socket-blocked
control), `/tmp/canic-throughput-coordinator-parity-unrestricted.log`,
`/tmp/canic-throughput-coordinator-parity-warm.log`,
`/tmp/canic-throughput-coordinator-restore.log` (intermediate live qualification).
Final logs: `/tmp/canic-throughput-infrastructure-parity.log`,
`/tmp/canic-throughput-infrastructure-restore.log`,
`/tmp/canic-throughput-infrastructure-unit.log`,
`/tmp/canic-throughput-infrastructure-cache.log`, and
`/tmp/canic-throughput-infrastructure-clippy.log`. Final changed-source identities
are in `/tmp/canic-throughput-infrastructure-source-sha256.txt`.
The whole throughput batch remains open. Next, attribute the expensive
configured-role declaration/runtime rebuilds across fixture configurations:
the retained .18 four-Shard case spent 107.70s in declarations and 139.85s in
runtime compilation, while the following initial-artifact group spent 75.18s
and 62.71s respectively. Establish exact invalidation causes before changing
cache ownership; preserve per-role features, config and release binding.

## Post-.18 catalog reuse — 2026-09-16

The maintainer prioritizes release-test throughput before restoring RF2.
The completed .18 source is `ee5ec6abc436a4a143fd659774c4f03e23ea5e05`,
released at `eb2bfbeba`. Its retained test log is
`target/validation-runs/20260915T182140Z-31559.KC6uLs/0.log`.

| Completed .18 test stage | Seconds |
| --- | ---: |
| Parallel library/binary tests | 289 |
| Ordinary integration tests | 212 |
| Internal ordered PocketIC stage | 4,682 |
| Host governed PocketIC stage | 151 |
| Runtime PocketIC stage | 272 |
| Blob-storage PocketIC stage | 53 |
| Payload-limit PocketIC stage | 6 |
| Complete test runner | 5,667 |

The enclosing validation target reports 5,668 seconds. Ten nested
`artifact_build_and_seal` spans total 1,083.22 seconds; do not add them to the
stage total. The mixed-topology journey takes 773.15 seconds and generated
reinstall 445.99 seconds. These historical complete-run measurements identify
where time went; they are not a matched performance control for new source.

The bounded host change shares Cargo catalog/target metadata within a single
role inventory, leaving each role's package-selected feature tree independent.
It applies to capability projection, terminal inventory and internal Wasm batch
preflight. Evidence is discarded when that operation returns. Canonical Root
projection does not materialize a generated package. No remote observation,
release binding, recovery case, build profile or PocketIC scheduling changes.

Focused measurements use Rust 1.98.1 (`48a229cea`) and Cargo 1.98.1 (`797e8a9bc`),
the unchanged .18 workspace lockfile, the existing test profile and populated
Cargo caches. All commands stay inside Canic. Other host workloads are not
controlled, and these are short local measurements rather than full-gate claims.

| Measured operation | Separate metadata | Shared metadata |
| --- | ---: | ---: |
| Mixed fixture contracts, including duplicate/unknown-role checks | 5.950s | 2.541s |
| Eight-role internal test-Wasm preflight | 8.32s | 3.43s |
| Metadata commands in that eight-role preflight | 17 | 3 |
| Package-selected tree commands in that preflight | 8 | 8 |

The role comparison runs the prior per-role algorithm and new batch algorithm
in one binary and asserts equal complete resolutions. Its first sample was
5.738s / 2.389s. The Wasm-preflight comparison runs the same existing exact test
before and after only sharing its local `PackageValidationCache`; an external
scratch wrapper counts `metadata` and `tree` invocations and forwards every
command to the pinned Cargo. No timing threshold is a test assertion. These
samples establish reduced local work, not minutes saved across a release.

Qualification: 66 focused host regressions pass across role contracts,
capability projection and terminal inventory. The new mutation regression
changes package metadata between operations and requires a typed rejection;
the order/equivalence regression includes unknown roles, duplicates, empty
input and canonical Root. Existing role-graph tests retain dependency isolation,
and internal build admission checks that the lockfile is unchanged.
Final focused commands use `CARGO_NET_OFFLINE=true`. Host all-target/all-feature
Clippy passes with warnings denied, as do changed-file formatting and whitespace
checks.

Logs: `/tmp/canic-throughput-role-contracts-final.log`,
`/tmp/canic-throughput-wasm-before.log`, `/tmp/canic-throughput-wasm-after.log`,
`/tmp/canic-throughput-wasm-before.commands`,
`/tmp/canic-throughput-wasm-after.commands`,
`/tmp/canic-throughput-regressions.log`, `/tmp/canic-throughput-clippy.log`.
The exact changed Rust file and lockfile hashes are retained in
`/tmp/canic-throughput-source-sha256.txt`.
No full suite, deployment, version bump, commit, push or sibling mutation ran.
The larger throughput objective remains open: the next investigation must
attribute the remaining artifact-build and complete-journey costs, retaining
exact release binding and all recovery/conservation checks.

## Earlier VS1 qualification


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
