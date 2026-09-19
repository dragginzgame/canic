# Release-test throughput qualification

## Fresh recovery-fixture installation — 2026-09-19

The next accepted speed slice reduces repeated setup while keeping each case's
replica, identity, host directory and journal independent. The shared preparation
helper still creates canisters through the production Ledger adapter. It now
installs the three infrastructure canisters directly through PocketIC, using the
exact selected Wasm bytes, checked file digest, operator and production initializer
compiler. The compiler's existing request/error/function are exposed under the
host's `local-fleet` feature; there is no alternate initializer or runtime mode.
Fresh-only typed initialization and complete newly allocated identities are
required. Controllers, generated planning, paid recovery, lost replies, terminal
conservation and effect-free replay retain their existing paths and assertions.
The complete fresh journey and interrupted reset installs retain ICP transport.

### Focused evidence

The unchanged control ran the estate-funding and issued-creation-pause cases in
one process. The first candidate also passes both but rebuilt artifacts for its
changed host source; its whole duration is not comparable. A subsequent five-case
qualification passes activation reset, four-Workload Failed-reserve repair, those
same two funding cases and native-child funding recovery. The latter proof retains
the exact original child claim across native withdrawal/recovery. No case or
assertion was removed, and no broad suite ran.

| Measured phase, seconds | Control | Candidate, warm artifacts |
| --- | ---: | ---: |
| Estate funding: infrastructure installs | 12.078 | 6.044 |
| Issued-creation pause: infrastructure installs | 12.048 | 5.889 |
| Estate funding: complete case | 55.026 | 49.842 |
| Issued-creation pause: complete case | 110.207 | 105.135 |
| Both installation phases | 24.126 | 11.933 |
| Both complete cases | 165.233 | 154.977 |

Installation setup falls by about 50.5%; these two complete cases improve by
10.3 seconds (6.2%) in the observed comparison. The warm candidate cases run
within the five-case process, after the other two scenarios; the control process
runs only the pair. Shared-server history and machine load are not identical, so
this is a local observation, not a controlled whole-suite speed guarantee. Their
artifact preparation is comparable: 6.344/5.694 seconds in the control versus
5.766/5.717 in the warm candidate. The other candidate install phases take
5.992, 6.285 and 5.923 seconds. The five-case run takes 951.65 seconds including
cold builds for additional configurations; do not compare that with the pair.

Both subjects use the active Canic checkout on published
`1a32d9594871fe8de838843b857fd9385c90a503`, package .25, unchanged dependency
lock/toolchain, Fast Wasm profile, ICP 1.5.0 and PocketIC 16.0.0. The candidate
only changes fixture installation and exports the already-used compiler; its
production compiler body, runtime sources and recovery assertions are unchanged
from the control. There is no measured Toko deployment or complete release-gate
improvement from this test-only setup change.

Logs: `/tmp/canic-funding-install-control.log`,
`/tmp/canic-funding-install-candidate.log`,
`/tmp/canic-funding-install-qualification.log`.
Final native checks pass: exact generated Coordinator/Root/Store initializer
authority, generated multi-component estate planning/apply/replay, and governed
registry ordering. Warning-denied host/internal library/test all-feature Clippy
passes. Logs: `/tmp/canic-fixture-install-native.log` and
`/tmp/canic-fixture-install-clippy.log`. The complete bounded .26 batch and both
changelog surfaces are ready for the maintainer-selected release gate; package
versions and dependency pins remain unchanged.

### Discarded whole-instance reuse trial

A preceding private prototype saved the complete stopped replica and exact host
files for the two identical initial funding recipes. Its file restoration check
passed, but reopening the instance advanced time by four nanoseconds and changed
one prepared canister's balance by 261,450 cycles. The pinned server performs a
checkpoint round on save and initialization rounds on load; see the
[PocketIC 16 source](https://github.com/dfinity/ic/blob/fc21803c3c3a8dd452b3b58b959751c41fecb89c/rs/pocket_ic_server/src/pocket_ic.rs).
This was not a drop-in replacement for the exact prepared accounting. The trial
was discarded rather than adding a balance tolerance or altering recovery inputs.
Its module, host snapshot helper, registry entry and builder changes are removed.
This does not rule out a separately designed checkpoint-based fixture with its
own accounted preparation boundary. Evidence:
`/tmp/canic-funding-setup-candidate.log`,
`/tmp/canic-funding-setup-candidate-retry.log`.

## Expanded CANIC-160/172/176 acceptance — 2026-09-19

The four-item Toko handoff expands the existing .26 batch. Its bounded outcomes
are implemented and qualified, ready for the maintainer-selected release gate.
Changes remain in Canic; application pins, package versions and sibling
repositories are unchanged. Earlier sections record preceding slices; this
acceptance supersedes their narrower scope and outstanding qualification notes.

### Observation and continuation boundaries

Planning owns an explicit read-only snapshot shared by Root management,
configured-canister observation and protocol preparation. Nested consumers reuse
it. Success, error, pacing, a changed reviewed desired input and an effect all
end the relevant evidence lifetime. No persisted cache or inferred cross-command
freshness was added. The transport regression reduces the first Root/configured
read group from five attempts to four, then proves fresh reads after pacing,
input rebinding, failed scope exit and the next invocation.

Completed replay proves terminal inventory first, then uses one fresh observation
of the merged estate for convergence and conservation. The native recovery
fixtures require exactly one replay observation and no additional mutation.
A just-admitted successor also carries its predecessor observation once to the
first exact action's funding check. The digest-bound handoff exists only in the
current invocation; restart, mismatch or an intervening effect cannot reuse it.
All existing debit, controller/module, canonical action, interruption and terminal
conservation checks remain.

`Planning` and `FleetSnapshot` timings are inclusive, with `parent_stage` naming
the enclosing stage. Remote attempts, local identity resolutions and actually
served cache hits explain collection versus cached projection. Parent and child
counts overlap; adding them would double-count work. Logical host attempts are
not all IC messages, HTTP packets or replica execution costs.

The report's `continuation_forecast` is a projection outside the immutable plan.
It exposes known import identities, separately reviewed funding estimates, the
actual successor authority ceiling and categories still requiring live evidence.
A completed preparation, Root-reset or Root-start prerequisite does not imply
a completed Fleet and keeps its pending discovery projection. A recovery estimate cannot manufacture
successor authority. Decimal cycle values retain `u128` precision. This is not
full RF3 live descendant collection or a complete funding quote.

Focused qualification: 132 host cases passed across platform, continuation,
workflow, JSON and generated-estate selections; three opt-in cases were not run.
The corrected generated forecast passes, as do 21 Fleet CLI cases and scoped
warning-denied host/CLI all-feature Clippy. The funded-estate transfer and
creation lost-response PocketIC case passes in 62.97 seconds (137-second runner,
including its compilation), with conservation and effect-free replay. Logs:
`/tmp/canic160-expanded-host-tests.log`,
`/tmp/canic160-expanded-recovery.log`,
`/tmp/canic160-forecast-final.log`,
`/tmp/canic160-expanded-clippy-final.log`.

Final active-checkout validation also passes: 21 Fleet CLI cases, generated
estate/replay and warning-denied all-feature library/test Clippy for host, CLI
and testing-internal. These checks include the final report-only refinement
that preserves live discovery for every non-Full prerequisite scope. Logs:
`/tmp/canic160-final-main-tests.log`, `/tmp/canic160-final-main-clippy.log`.

### CANIC-176 publication and installed-binary acceptance

Toko's manifest, `tool-versions.env`, CLI version and Cargo installation record
identify tag `v0.110.24`, commit
`933a35b66403f6a94f99803252cc52c0aec32958`. The installed executable is
`/home/adam/.cargo/bin/canic`, SHA-256
`edfa5a47163a7133af2cea27cca5ee42429b04d8027469ac7cb52b994924f943`.
Cargo metadata is provenance evidence, not a reproducible-binary attestation;
the actual executable was therefore exercised directly.

Both .24 and .25 published host/CLI archives were downloaded read-only, checked
against registry checksums and their `.cargo_vcs_info.json`, and the relevant
source files compared with the exact local tags. Lock-owner diagnostics, CLI
rendering and the session filter are byte-identical across those tags and the
current tree. This corrects the assumption that a .24 pin by itself means those
features have not been adopted.

| Published archive | SHA-256 |
| --- | --- |
| canic-host 0.110.24 | `7794689a9f6afb7efaec83c252d6c39135f06c7c8f10b879537d19e10fe3b145` |
| canic-cli 0.110.24 | `78b567c40349a043c4837bca57e2d8b2ab724c85b97bd2fb1a328acc38cf906e` |
| canic-host 0.110.25 | `d7b2463f0f74fdcc2fe3266b63021189af328987fddd0513cbacba33b3657c7b` |
| canic-cli 0.110.25 | `f1983c52c621113080ca1d68f7b4e588f411ffaa3cc8abc981e07183cf6a07c4` |

Acceptance used an isolated archive of that exact .24 tag, its locked dependency
graph, `canisters/audit/root_probe/activation.toml`, Local network and Fast
profile. The selected installed CLI built canonical Coordinator, Store and Root
plus the audit application. No replica or deployment was used for this build
check. The active .25 checkout correctly fails the .24 CLI's dependency-version
admission and is not a valid matched subject; an active-workspace dependency
source scan also encountered an unrelated node_modules symlink. Neither was
bypassed or edited. The isolated tag removes those uncontrolled inputs.

| Installed CLI case | Result |
| --- | --- |
| Initial four-artifact build | Complete, 314.36 seconds |
| Identical warm repeat | Four verified complete-release hits, 4.61 seconds |
| Only session/thread IDs changed | Four hits, 4.68 seconds |
| Held kernel lock, matching launcher environment | All owner fields printed; waits; four hits, 5.83 seconds including 1.10 seconds acquiring the lock |

The first two warm checks retain release
`d0bb4b07ce998dfb8b8f96fc2798dc5cf0aafcbd26666247b9f4f2e53eabffcf`.
All 18 sealed release files retain their hashes. An initial lock-test launcher
changed PATH through a nested login shell, correctly invalidating complete reuse
and producing release
`b930586c91489cd77d733383eed2a94390cbda64da4df17f0419c68251e1a5a9`.
That confounded run is not warm-reuse evidence. Repeating with the identical
launcher held the actual existing lock inode, verified that the CLI stayed
blocked, checked PID/workspace/profile/start-time/path diagnostics, then released
the kernel lock and observed complete reuse. Both releases' files remain unchanged.
The separate native lock regression kills an owner and proves safe interrupted
ownership recovery; oversized/invalid metadata remains advisory. The real-Cargo
warm/session regression also rejects changed source/configuration/build inputs.

A final acceptance starts two actual installed CLI processes with the same
launcher environment. The contender reports the owner's real executable PID,
workspace, profile, start time and lock path, then acquires the kernel lock after
the owner exits. Both report four complete-release hits without compilation;
all 36 files across both retained releases remain unchanged. Owner and contender
elapsed times are 12.76 and 17.27 seconds, respectively, with the latter including
lock waiting. These are contention diagnostics, not a faster warm-build claim.

Evidence: `/tmp/canic176-publication-v24.json`,
`/tmp/canic176-warm-regressions.log`,
`/tmp/canic176-installed-first.log`, `/tmp/canic176-installed-warm.log`,
`/tmp/canic176-installed-session.log`,
`/tmp/canic176-installed-lock-matched.log`,
`/tmp/canic176-initial-artifacts.json`, `/tmp/canic176-real-contention.json`,
`/tmp/canic176-real-owner.log`, `/tmp/canic176-real-contender.log`.
This qualifies the inspected installed CLI and synthetic Canic application;
it does not establish an exact Toko eight-role build or Toko deployment duration.

### Matched complete reinstall comparison

Both sequential exact
`pic::fleet_registry::baseline::tests::generated_reinstall_recovers_lost_install_and_reaches_working_fleet`
runs pass, comparing isolated .25 host source with a frozen candidate. Both use the
same current disposable journey and artifact builder, exact dependency lock and
existing tooling. The baseline's unselected timing unit test retains its old DTO
literal; the executed journey and timing implementation are identical. No recovery
assertion or estate cardinality is reduced: nineteen Workloads and five Ready
assets, lost install reply, authority rejection, exact receipts/controllers,
bounded native debit, retained Root Ledger balance, terminal conservation and
immediate effect-free replay. Source manifests are retained in
`/tmp/canic160-comparison-identities.json`; logs are
`/tmp/canic160-reinstall-control.log` and
`/tmp/canic160-reinstall-candidate-isolated.log`. The first candidate listing
attempt reused an older host library from the shared Cargo target and failed on
its missing timing fields before running the test. It is excluded from evidence.
The candidate therefore uses a separate target directory. Only native fingerprint
markers for the four experiment-built packages were invalidated in the shared
target; artifacts and dependency caches were retained so subsequent main-tree
checks rebuild their own source. Compilation and cache warmth are not comparable
elapsed-time evidence. Final main-tree checks above pass against the actual
active source, not either experiment directory.

| Input | Identity |
| --- | --- |
| Control | `v0.110.25`, commit `1a32d9594871fe8de838843b857fd9385c90a503` |
| Frozen candidate source-manifest SHA-256 | `e45dcd7421df51aacc22f8fe616bd3903b3f377776ef7e0e66370363773c0895` |
| Shared Cargo.lock SHA-256 | `1df28d519b52f3eb5fabc4980eefe495d7de36c0ee40fbf8b6337793d052468e` |
| Shared executed baseline.rs SHA-256 | `9d93b0e64f5a6d282121aaccc78146115b8f08b2dfb2d260f1a6f6753747d5cd` |
| Rust / Cargo | 1.98.1 (`48a229cea 2026-09-01`) / 1.98.1 (`797e8a9bc 2026-08-05`) |
| Wasm / IC tooling | Fast profile; ic-wasm 0.11.1; ICP 1.5.0; PocketIC 16.0.0 |

The frozen manifest remains unchanged throughout qualification. The subsequent
active-source prerequisite forecast refinement affects reporting only and is
covered by the final native checks; measured observation/execution code is
unchanged. Runtime source and configuration match across subjects. Initial and
replacement release identities also match:
`a4c128728412f11837b79ce8562e3115451db387e17361b79b4f15d02cbb36ae` and
`33888cd229a38503c7d4d2ce3fd53c48e003dd5856473bef2165c5a67889010a`.

Count only the common named stages below, excluding the new inclusive Planning
and FleetSnapshot parents. Attempts are logical host observations, not total
network messages, effects or IC instructions.

| Observation measure | Control | Candidate |
| --- | ---: | ---: |
| Common-stage attempts | 1,044 | 1,017 |
| RootManagement attempts / events | 31 / 19 | 31 / 19 |
| ConfiguredCanisters attempts / events | 380 / 24 | 371 / 23 |
| ProtocolActions attempts | 184 | 169 |
| TerminalInventory attempts | 378 | 378 |
| PoolBalances remote attempts | 0 | 0 |
| PoolBalances elapsed seconds | 3.289 | 0.830 |
| PoolBalances local identity attempts | Not exposed | 36 |
| PoolBalances served cache hits | Not exposed | 180 |
| Common-stage elapsed seconds | 85.398 | 83.284 |

One full snapshot and 27 logical attempts (2.59%) are removed. RootManagement
and terminal inventory retain their distinct fresh-check purposes in this
journey. Zero PoolBalances remote attempts now identifies cached projection
with local identity work; it does not mean the stage is free.

| Phase, seconds | Control | Candidate |
| --- | ---: | ---: |
| Initial artifact preparation | 289.910 | 311.257 |
| Initial working Fleet | 91.666 | 95.769 |
| Replacement artifacts/generation | 44.497 | 43.674 |
| Root review/authority rejection | 5.247 | 5.264 |
| Lost-install response/recovery | 12.095 | 11.618 |
| Successor reviews/convergence | 188.578 | 189.117 |
| Retained-estate state/replay | 27.318 | 26.262 |
| Complete retained-estate reinstall, including replacement preparation | 277.737 | 275.936 |
| Whole test / runner | 677.296 / 754 | 701.158 / 896 |

The observed reinstall difference is only 1.801 seconds (0.65%) in one pair.
It does not establish a substantial developer wait reduction or a faster full
release gate. The colder isolated candidate build makes whole-runner timings
unsuitable for attributing this optimization.

The estate is functionally equivalent, not byte-identical: five of seventeen
artifacts per release match exactly (four Candid files and the fixture manifest).
Wasm hashes differ and embedded absolute checkout paths are present. App,
Coordinator and Root raw sizes match at 3,905,687, 5,332,062 and 9,704,247 bytes;
Store raw size changes 3,771,978 to 3,772,120, with code size 3,463,388 to
3,463,400. Generated Cargo paths differ between isolated subjects. No precise
latency or IC-instruction attribution is claimed from these binaries. Artifact
hashes and stage measurements are retained in
`/tmp/canic160-artifact-comparison.json`,
`/tmp/canic160-control-measurements.json` and
`/tmp/canic160-candidate-measurements.json`.

The four handoff requests are accounted for within this batch. Further safe
terminal-inspection reuse needs separate evidence and design; full RF3 paid
descendant collection/live quotes remain outside this scope. Neither the
synthetic reinstall nor the installed-CLI build proves exact Toko workload
latency. No Toko edit, dependency bump, broad validation or publication ran.

## CANIC-160 observation work and transient reads — 2026-09-19

The maintainer accepted the follow-up after a read-only Toko review. Its latest
handoff reports 18.390 seconds across three local observation stages, plus two
roughly 1.4-second PoolBalances stages with zero logical remote attempts. These
are phase observations, not total reinstall time or proof that intervening
effects permit a stale full-estate snapshot.

### Implemented boundary

Pool-balance preparation shares the Root/operator admission check within each
existing batch of at most four PendingReset/Failed assets. Each asset still
checks controller/module authority; all issued reads drain, failures retain
input order, and only a fully validated batch publishes balances. The next
batch checks the selected operator again. Existing snapshots still expire after
the observation, on success or failure, before effects.

The nine-asset regression reduces identity Principal lookups from nine to three.
Its first pass reports 19 logical remote attempts; a repeated pass in the same
snapshot reports zero attempts, three identity lookups and twelve served cache
hits. This identifies local work behind a zero-call phase without claiming it
was free. A changed operator between batches rejects before later reads, and a
new snapshot rechecks authority. No whole-phase cache survives mutations.

Coordinator provisioning-status queries now use the existing authenticated
agent so retry decisions retain typed transport errors. An immutable encoded
request and one verified signer/network bind at most three logical attempts,
with 250/500 ms backoff, ten seconds per attempt and thirty seconds overall after
authority resolution. The 8 MiB HTTP response bound and Candid decoding quotas
bound response handling. Signature verification stays enabled. Agent-internal
HTTP retries and certificate reads share the time budget but are not separately
counted logical attempts.

Only timeout, selected transient HTTP and connection/body transport failures
retry. Authentication, signature, response-size, application and decoding failures
stop. Typed `STATE_UNAVAILABLE` retains the existing absent-operation decision;
operation/plan checks and terminal validation stay with their current owners.
No mutation dispatch is part of this retry helper. Exhaustion returns the typed
failure and leaves ordinary same-operation recovery responsible for the journal.

### Qualification and limits

- 41 pool-related host tests, 20 current-protocol tests and 17 inventory tests
  pass. One manual matched-latency test stays ignored.
- Three new transport tests pass: typed retry/exhaustion, fail-closed permanent
  failures, and actual HTTP 502 responses with identical query requests. The
  isolated HTTP error server disables query signature verification only in its
  test agent because it does not implement certificate `read_state`; production
  verification remains enabled and is exercised by PocketIC.
- The CLI observation rendering test and three internal timing tests pass.
  Total focused native evidence is 85 passing tests.
- The exact funded-estate transfer/autonomous-creation recovery journey passes:
  54.78 seconds test execution, 71 seconds runner. It covers successful native
  status decoding, lost-response recovery, terminal conservation and effect-free
  replay. The growth fixture wrapper exposes its PocketIC network/root key to
  native reads, matching its existing CLI routing. The first run exposed that
  missing fixture wiring before any query could be sent; it did not justify
  weakening production authority resolution.
- Scoped warning-denied host/CLI/internal Clippy passes, including a final
  internal recheck after the fixture correction. Changed-file formatting and
  whitespace checks pass. The complete bounded .26 batch and both changelog
  surfaces are ready for the maintainer-selected release gate.

This preceding slice does not simulate a 502 during a live Toko deployment, measure a complete
matched local reinstall or qualify larger across-phase reuse. Adjacent
RootManagement checks still have distinct planning, before-effect and terminal
purposes. The expanded acceptance above subsequently qualifies bounded reuse,
continuation forecasting and a complete local reinstall; full RF3 remains
separate work. Toko's 4/4-to-4/18 continuation is not itself duplicate execution.

Source is the uncommitted .26 candidate on
`1a32d9594871fe8de838843b857fd9385c90a503`; packages remain .25. Cargo lock SHA-256
is `1df28d519b52f3eb5fabc4980eefe495d7de36c0ee40fbf8b6337793d052468e`.
Qualification uses Rust 1.98.1, ICP CLI 1.5.0, ic-agent 0.49.2 and PocketIC 16.0.0.
Logs: `/tmp/canic160-observation-regressions.log`,
`/tmp/canic160-query-retries.log`, `/tmp/canic160-cli-timing.log`,
`/tmp/canic160-internal-timing.log`, `/tmp/canic160-scoped-clippy.log`,
`/tmp/canic160-fixture-clippy.log` and `/tmp/canic160-query-recovery-pocketic.log`. No broad validation, package bump,
publication or sibling mutation ran.

## Fixture builder isolation and measured setup — 2026-09-19

The complete fixture artifact recipe previously hashed the entire
`fleet_registry/baseline.rs` journey file. An assertion or timing-only edit
therefore invalidated the sealed artifact set. Artifact preparation, compilation,
sealing and staging now live under `baseline/tests/release_artifacts/`; the audit
Root builder moves there too, with explicit imports and its unchanged gzip
implementation. The cache hashes that builder, the audit builder and shared
`pic/artifacts.rs` helpers. It retains the existing exact Cargo graphs, lockfiles,
tools, profile, network, config, fixture content and release identity inputs.
No source exclusion is added to the Cargo input collector.

The governed regression uses the real artifact transaction cache: changing the
journey source retains the entry, changing each build-helper input invalidates
it, and restoring each input recovers the original entry. Existing tests still
prove distinct release identities and fixture-authority restoration. The
regression is registered in the normal governed catalogue.

### Exact funding-case qualification

Every PocketIC run selected only
`pic::fleet_registry::baseline::tests::funded_estate_recovers_transfer_and_autonomous_creation_responses`
through `make test-pocketic-case`. Production Ledger creation, CLI installation,
lost-response recovery, funding checks and effect-free replay remain intact.

| Run | Artifact build/seal | Complete artifact resolution | Test execution | Targeted runner |
| --- | ---: | ---: | ---: | ---: |
| Instrumented control | 250.69 s | 261.43 s | 316.27 s | 478 s |
| Extracted builder, warm lower caches | 20.11 s | 25.52 s | 76.78 s | 93 s |
| Journey-only comment edit | skipped (complete cache hit) | 4.86 s | 56.10 s | 73 s |

All three runs pass. All 17 sealed output files from the control and extracted
builder have identical SHA-256 digests, including Wasms, Candid and manifests.
The last run changes only a setup-timing explanation in the journey file; it
reuses the candidate's complete artifact entry and contains no build/seal span.
The subsequent source changes only expose/register the regression for the
governed runner; its inventory and final Clippy checks cover that registration.

New nested setup spans measure creation, each installation, aggregate installation
and controller setup. In the control, these take 0.476, 3.550/6.081/2.746, 12.377
and 0.008 seconds respectively. Generation/initial review still totals 50.66
seconds and includes later provisioning work; these are nested spans and must
not be summed. This measurement does not justify replacing production setup or
introducing shared mutable Fleet baselines. Installation transport is unchanged.

Cache warmth and machine load differ. The 250.69-to-20.11-second difference is
not an optimization result. The supported result is complete artifact reuse
after a journey-only edit, while preserving artifact bytes and recovery behavior.
There is no full release-gate or downstream deployment saving measured here.
Release-version changes and actual build-input changes still require new entries.
Audit-Root construction is relocated without changing its function bodies; its
separate child-funding PocketIC case was not rerun for this extraction.

### Retained evidence

- Published base and dependency lock remain those recorded below. Rust 1.98.1,
  ICP CLI 1.5.0 and PocketIC 16.0.0 were used; packages remain .25.
- Control: `/tmp/canic-infrastructure-setup-timing.log`; extracted candidate:
  `/tmp/canic-helper-cache-funding-candidate.log`; journey-only replay:
  `/tmp/canic-helper-cache-journey-edit-replay.log`.
- Artifact inventories: `/tmp/canic-artifact-isolation-before.json` and
  `/tmp/canic-artifact-isolation-candidate.json`. Control cache entry:
  `ff6db7fd456af0a81122d82d5b41c1a6327bc67d0772da0e2942cbb90cd0d2c3`;
  candidate/replay entry:
  `16c5161f0658831c31a1fec80de3e7b71f3904164e364230bc256da1ba60fc2e`.
- Focused cache-boundary test: `/tmp/canic-helper-cache-regression.log`;
  both existing release-cache tests: `/tmp/canic-release-cache-regressions.log`;
  catalogue check: `/tmp/canic-helper-cache-inventory.log`;
  scoped warning-denied Clippy: `/tmp/canic-helper-cache-clippy.log`.

The .26 bounded speed batch combines this fixture-cache improvement with the
per-call release-input sharing below. Its implementation, focused evidence and
changelog are ready for the maintainer-selected release gate; no broad gate,
version mutation or publication was performed.

## Published .25 baseline and authority-input reuse — 2026-09-19

The maintainer confirms .25 is live. Its successful test log is
`target/validation-runs/20260918T210334Z-2561.DRB1FA/0.log`: runner wall time
2,674 seconds, with the enclosing record rounding to 2,675. Internal PocketIC
takes 2,421 seconds; the host proofs take 15, runtime integrations 133, blob
storage 15 and payload checks four. The earlier .24 runner took 4,187 seconds.
Different cache warmth and source state prevent controlled attribution.

Ten production-adapter journeys account for 1,725.25 seconds. Their generation/
initial-review spans total 582.25 seconds and include infrastructure preparation,
not only generation. Eight artifact-build/seal spans total 260.40 seconds.
Mixed topology takes 528.86 seconds and generated reinstall 390.35. These nested
spans must not be added together. No live compiler-target clearance appears in
this successful run; the logged 9 GiB clearance is the sparse retention test.

### Shared evidence within one compilation

`compile_root_authorities` previously loaded the complete release and verified
fixture payloads before the loop, then repeated both for every Root. Each Root
also loaded the same application union. Root and Store initializer compilation
repeated the first two checks in its inner authority compiler.

The authority compiler now receives private, call-local release inputs. The
outer boundary still validates finalization, infrastructure binding and fixture
payloads; the application union must match the same complete manifest. Each Root
retains its own topology validation, projection, digest, placement and Principal
bindings. Nothing survives the call or is reused across effects or release IDs.
Coordinator initialization keeps its existing admission boundary.

For R Roots, complete-manifest reads and fixture verification calls fall from
R+1 to one; application-union loads fall from R to one. For a Root/Store init,
complete-manifest reads and fixture verification fall from two to one. These
are source-derived work counts, not measured duration or complete-suite savings.

### Focused qualification

- `cargo test --locked -p canic-host --lib generated_` under the governed scratch
  wrapper: ten pass, one unrelated opt-in Node.js test ignored. The existing
  generated-estate native fixture exercises four-Root results against separate
  compilation, actual Coordinator/Root/Store init decoding, alteration of each
  complete/infrastructure/application/fixture manifest followed by rejection,
  restored-evidence retry and its existing planning/application/replay journey.
  Log: `/tmp/canic-authority-input-reuse-tests.log`.
- `cargo clippy --locked -p canic-host --lib --tests --all-features -- -D warnings`
  passes after narrowing a test helper's visibility. Log:
  `/tmp/canic-authority-input-reuse-clippy.log`. Changed-file rustfmt and whitespace
  checks pass. No Canic lifecycle or remote effect changed, so this slice did not
  start PocketIC or a broad gate.
- Base `1a32d9594871fe8de838843b857fd9385c90a503` (`v0.110.25`); Cargo lock SHA-256
  `1df28d519b52f3eb5fabc4980eefe495d7de36c0ee40fbf8b6337793d052468e`.
  Package versions and lockfile are unchanged by this slice.

The native fixture has empty provisioning payloads; it proves authority and
fresh validation, not large-payload throughput. A deployment or full-release
timing reduction remains unmeasured. Continue the .26 speed batch on the measured
infrastructure/setup costs without replacing complete recovery proofs with this
local result. Toko's latest source records successful staging recovery/reset and
a new CANIC-160 typed transient-observation retry request; no sibling was changed.

## Avoided Root builds and stable preparation — 2026-09-18

The child-funding and retained-estate fixtures built a canonical Root before
replacing it with an audit Root. They now build the audit Root directly, retaining
package admission and finalization. All 21 artifacts in the child-funding release
are byte-identical. Consecutive warm artifact-build phases take 38.56 and 13.93
seconds; cache history and machine load prevent a whole-release speed claim.

The initial qualification also reproduced a late cache-input rejection after
410 seconds of artifact work. Coordinator/Store generated packages were created
inside the snapshotted audit package. Their preparation now precedes the snapshot;
the cold-directory regression retains rejection of later source changes.

Both affected PocketIC journeys and scoped Clippy pass. This completes the
selected .25 planning/fixture throughput batch; shared host/runtime baseline
isolation remains separate work. [Evidence and limitations](../../reports/2026-09/2026-09-18/fixture-build-throughput.md).

## PocketIC duplication audit — 2026-09-18

[The source/log review](../../reports/2026-09/2026-09-18/pocketic-redundancy.md)
finds one repeated invalid-state lifecycle upgrade and one smaller uncertain
refill case subsumed by the four-asset case. The internal runner executed every
registered case once, with no duplicate registrations. Most expensive overlap
is repeated artifact/Fleet setup or different recovery triggers, not equivalent
tests that can simply be deleted. Keep capacity and same-operation recovery
proofs, and measure any reduced fixture before replacing its current owner.
The subsequent authorized cleanup removes the duplicate lifecycle upgrade and
smaller uncertain refill case. All six lifecycle tests and the surviving exact
four-asset PocketIC proof pass. The accepted B1 protocol requires its warm-ups,
and the larger journeys retain distinct reset/funding contracts. Shared host/runtime
fixtures require their own isolation design; a canister-only snapshot is insufficient.
Cold compilation in these focused runs does not establish a release speedup.
The dynamic governed inventory check, scoped warning-denied Clippy for both
changed targets, formatting and whitespace checks also pass.

## Published .24 baseline and next planning slice — 2026-09-18

The successful retained test log is
`target/validation-runs/20260918T140257Z-60259.NaOVNO/0.log`: 4,187 seconds runner
wall time (the enclosing validation record rounds to 4,188). Ordinary tests take
194 seconds, the internal PocketIC suite 3,621, host governed proof 24, runtime
PocketIC integrations 288, blob storage 54 and payload checks 5. These are phase
diagnostics from one release, not a controlled comparison against .23.

Within the internal suite, the mixed-topology journey takes 665.225 seconds and
generated reinstall 457.827. Ten artifact build/seal spans total 829.537 seconds;
ten generation/initial-review spans total 605.850. Nested spans overlap their
parents and must not be added to the total. Finalization already overlaps;
dependency admission is small in the retained samples. Do not promise an
hour-scale improvement from another small observation optimization.

The next completed planning slice overlaps Root/Store authority pairs within the
existing four-read bound. The matched synthetic phase comparison preserves exact
results and call counts, improving multi-Root latency while one Root stays flat.
[Evidence and limitations](../../reports/2026-09/2026-09-18/root-authority-throughput.md)
record source identity, samples and focused checks. This does not measure total
release or deployment improvement. The larger speed target remains repeated
artifact/setup work in the expensive journeys with all current safety assertions
and explicit workload/reserve capacities retained.

## Fixture compiler cache retention — 2026-09-18

Read-only apparent-size measurements (`du -sb`) of the retained fixture compiler
targets found Local at 4,682,199,036 bytes (4.36 GiB) and Ic at 3,084,675,040 bytes
(2.87 GiB). These are retained footprints, not a claim that every file is needed
by the next build. The prior policy's 4 GiB threshold is below the Local target.
Published ic-testkit 0.10.0 clears the entire mutable target when a due retention
check finds it oversized; it preserves the target directory and coordination
metadata. Canic previously rendered these outcomes only in verbose output.
The normal .23 log therefore does not establish whether a historical clearance
caused any particular cold build.

The threshold is now 8 GiB per Local/Ic fixture compiler target, about 1.83 times
the observed Local footprint. This is a maintenance allowance with headroom,
not a reserved allocation, hard in-build disk cap or canister memory limit.
Seven-day idle expiry, hourly maintenance, locking and best-effort failure
handling remain. The existing 2 GiB immutable artifact-cache pruning policy is
unchanged. The production App compiler target is a separate owner. No source,
release identity, network, profile, feature or output checks are relaxed.

Normal output now exposes completed clearances with target path and before/after
logical bytes, and reports maintenance failures as warnings. Retained, missing
and skipped results remain verbose-only. Future pressure above the new threshold
will therefore be visible instead of silently causing another cold compiler target.

Focused qualification:

- Six internal artifact tests pass, including an actual ic-testkit maintenance
  call retaining a 5 GiB sparse compiler file and clearing a 9 GiB file. Both
  preserve the coordination lock and source fixture. No live cache was cleared.
- The clearance is visible in ordinary, non-verbose output. Sparse files exercise
  logical-size accounting without allocating gigabytes of physical test data.
- Strict Clippy passes for the internal library and tests; formatting and
  whitespace checks pass. No PocketIC or full workspace suite ran.

Logs: `/tmp/canic-compiler-retention-tests.log` (0.16 seconds test execution;
90 seconds native compilation) and `/tmp/canic-compiler-retention-clippy.log`.
Base remains `732b4629e690cc5f014078b73201129cd9630271` plus the open .24 work;
lockfile and source hashes are retained in `/tmp/canic-compiler-retention-identities.txt`.
Keep this bounded cache-policy correction in .24. It prevents the measured
retained footprint from triggering size-based clearance under the new policy;
no complete-release timing reduction has been measured. Long recovery journeys
and distinct artifact recipes remain separate throughput work.

## Ordinary test graph consolidation — 2026-09-18

The same successful .23 log records 109 seconds for workspace library/binary
checks, followed by 102 seconds for ordinary integrations. The latter compiles
Canic control-plane, host, CLI and internal testing again and spends 91 seconds
in Cargo. Those times include different test selections; they are not a measured
91-second saving or additional time outside the 4,791-second runner total.

The full and ordinary lanes now select workspace library/binary tests and every
registered ordinary integration in one Cargo invocation. Integration names come
from the inventory; the former hard-coded package list is removed. Cargo retains
`--no-fail-fast`, libtest's default parallelism and exclusion of ignored tests.
The existing failure barrier still precedes PocketIC. Fast, exact-case and
PocketIC-only selections remain separate. A target name cannot cross inventory
selection classes, preventing workspace-wide name selection from admitting a
serial or external-composition target. Integration tests now intentionally use
the same dependency feature union as ordinary workspace unit tests.

Focused evidence uses the actual runner and inventory guard in a disposable
three-package Cargo workspace. Only dependency-tool version preflight is stubbed;
Cargo compilation and test execution are real. It proves:

- unit, binary and registered integration execution with the workspace feature union;
- exclusion of ignored proofs and serial/external integration targets whose source
  deliberately fails compilation if selected;
- a retained failure when a unit test fails, continued integration execution and
  rejection before PocketIC startup;
- rejection of a cross-class target-name collision;
- later host-proof executable reuse (`fresh=true`);
- a package-scoped integration control builds a different dependency artifact,
  while repeating the combined graph reuses its original artifact (`fresh=true`).

Fixture: `/tmp/canic-ordinary-graph.eHHC7f`; script/log:
`/tmp/canic-ordinary-graph-probe.sh`, `/tmp/canic-ordinary-graph-probe.log`.
The guard checks exactly one ordinary invocation and equality with the complete
selected integration inventory. Ordinary/full/PocketIC/fast/exact-case plans,
release-integrity guard, focused ShellCheck, Bash syntax and whitespace checks
pass. Logs: `/tmp/canic-ordinary-graph-plan.log`,
`/tmp/canic-ordinary-graph-contract.log`. Source identities are recorded in
`/tmp/canic-ordinary-graph-identities.txt`. The release guard's deliberate
negative cleanup fixtures are expected.

Keep in the existing .24 batch. No Canic workspace tests or long PocketIC suite
were rerun; complete-release savings await the maintainer's selected release run.
This removes another known graph switch but does not claim to resolve the
roughly hour-long internal PocketIC workload. Inspection found distinct network,
configuration and release identities in costly Root/App artifact builds; none
were collapsed. The nineteen-Workload/five-Ready proof remains intact.

Toko's latest read-only feedback confirms .23 paired adoption with IcyDB 0.258.0,
255 native tests, strict Clippy, Wasm/Candid checks and both managed tests passing.
Live CANIC-166 retained-source review/application remains separate evidence.

## Release-test compile-graph reuse — 2026-09-18

The successful .23 validation log is
`target/validation-runs/20260918T075716Z-43326.AIQJTN/0.log`. Its test runner
took 4,791 seconds; preceding successful gates took about 66 seconds. Internal
PocketIC accounts for 4,146 seconds. The largest cases were mixed topology
(727.54 seconds) and retained reinstall (459.46 seconds). Ten recorded
`artifact_build_and_seal` spans total 1,049.17 seconds; these are nested inside
the tests and must not be added to suite totals. Recorded protocol-action
observations total 92.43 seconds, so the preceding status-read optimization alone
cannot remove the dominant release-test cost.

### Confirmed native graph switch

Ordinary tests executed `canic_host-19b4daf9e74d17e5`; the later package-only
host proof used `canic_host-711eb51430b17a73`, after 155 seconds of compilation
for 1.08 seconds of test execution. Their retained Cargo fingerprint records
agree on host features (`[]`), rustc, profile and rustflags. Ten dependency
fingerprints differ, including Canic core/control-plane, Candid, ic-agent,
ic-testkit and syn. Re-selecting only the host changed the resolved native graph.

The full runner now repeats `--workspace --lib --bins` for the governed host
filter, matching its ordinary compile graph. The same ignored-only filter and
single test thread remain. Other harnesses select no tests. The ordinary failure
barrier still precedes any PocketIC startup. PocketIC-only and targeted commands
remain scoped because those invocations have no ordinary workspace build to
reuse. No compiled binary is guessed from a directory or executed outside Cargo;
Cargo retains freshness checks and test runtime environment ownership.

### Focused qualification

A temporary three-package Cargo workspace models a shared dependency whose
feature is enabled by a peer. With unchanged sources, lock and compiler:

| Invocation | Host artifact | Fresh |
| --- | --- | --- |
| Ordinary workspace library/binary tests | `fixture_host-2ca0cc7ddaa7cb00` | No, initial build |
| Scoped host ignored proof | `fixture_host-c5ac1d9d368afc94` | No, different graph |
| Repeated workspace graph, host ignored filter | `fixture_host-2ca0cc7ddaa7cb00` | Yes |

The selected proof passes with the appropriate dependency features. Unrelated
ignored core/peer tests deliberately panic if run and remain unselected. This
proves graph/executable reuse, not a Canic end-to-end latency result.
Fixture/script/results: `/tmp/canic-feature-graph.GKUQh5`,
`/tmp/canic-feature-graph-probe.sh`, `/tmp/canic-feature-graph-probe.log`.

The exact current-source Canic proof passes through the governed runner:
`make test-pocketic-case CASE=fleet_ensure::tests::governed_pocketic_fresh_estate_recovers_creation_and_replays_without_effects`.
It compiles in 2m30s and executes in 1.29 seconds; this scoped run qualifies
recovery coverage, not the full-run graph optimization. Log:
`/tmp/canic-release-graph-host-proof.log`.

Full/PocketIC-only plans, the release-integrity contract guard, Bash syntax,
scoped ShellCheck with repository exclusions and whitespace checks pass.
The contract retains the scoped standalone path and rejects ignored host-filter
matches outside the host package. Logs: `/tmp/canic-release-graph-full-plan.log`,
`/tmp/canic-release-graph-pocketic-plan.log`, `/tmp/canic-release-graph-contract.log`.
The contract's deliberate failure fixtures do not indicate a failed guard.

Base is `732b4629e690cc5f014078b73201129cd9630271`, plus the retained .24
worktree. Rust/Cargo are 1.98.1; Cargo.lock is unchanged. Fingerprint comparisons
and candidate script SHA-256 identities are retained in
`/tmp/canic-release-graph-evidence.json`. No broad Canic suite was rerun.

### Artifact disposition and next boundary

The same log already shows exact release-artifact cache hits around 1.8–2.0
seconds. Source inspection finds distinct configurations, Local/Ic compilation,
audit Root variants and deliberately distinct initial/reinstall release nonces.
They must not share final Wasms by dropping release or configuration binding.
No blanket artifact cache change is justified by this inspection. The accepted
nineteen-Workload/five-Ready capacity case remains required; reducing its count
would remove an explicitly retained contract rather than optimize execution.

Keep the compile-graph change in .24. It targets the observed second host build;
its complete-release saving still requires the maintainer's next full run.
Further work should attribute expensive artifact compiler/link steps and repeated
setup within exact long-running journeys while preserving capacity, interruption,
conservation and effect-free replay coverage. No claim is made that this change
resolves the remaining roughly hour-long internal PocketIC workload.


## Protocol-owner status overlap — 2026-09-18

CANIC-160's next bounded latency change removes sequential waiting in
`current_protocol_owners_are_ready`. Each protocol-planning pass previously read
all present Coordinator/Root/Store statuses serially before checking their exact
modules. It now uses the existing four-read collector. Configured-canister and
protocol-owner paths share the same status preparation/consumption helpers.

| Healthy observation | Previous scheduling | Candidate scheduling |
| --- | --- | --- |
| Three distinct owners | Three sequential reads | One group of three reads |
| Nine distinct owners | Nine sequential reads | Three groups: four, four, one |

These are request schedules, not measured latency ratios. The native synchronized
transport fixture proves four simultaneous requests, nine completions, no
outstanding request on return and no extra healthy-path calls. Missing owners and
module decisions stay after the status scan. Results are consumed in configured
order: an earlier stopped owner still returns not-ready before a later transport
failure, while an earlier transport error retains its exact typed failure even
if another response arrives first. No later group starts after either outcome.
Issued requests in a failed group are drained; an early failure can therefore
incur up to the remaining reads in that group. Remote attempts retain existing
accounting. No deployment mutation is parallelized or skipped.

Fresh repeated observations see changed running/module state. Injected failures
are not retained across retries, and the next successful call reads current
statuses. Existing per-observation cache scope and authorization remain intact.
The change reuses the existing platform collector rather than adding a cache,
transport, polling loop or test framework.

Qualification at base `732b4629e690cc5f014078b73201129cd9630271` plus the
open .24 batch, Rust/Cargo 1.98.1 and unchanged Cargo.lock:

- `ICP_ENVIRONMENT=local bash scripts/ci/run-with-test-scratch.sh cargo test --locked -p canic-host --lib fleet_ensure::ops::platform::tests::`: 46 pass.
- `ICP_ENVIRONMENT=local bash scripts/ci/run-with-test-scratch.sh cargo clippy --locked -p canic-host --lib --tests -- -D warnings`: pass.
- Changed-file rustfmt and whitespace checks: pass.

Logs: `/tmp/canic-owner-status-tests.log`, `/tmp/canic-owner-status-clippy.log`.
Candidate platform.rs SHA-256:
`3e5fbdb71d7331355ada42e0baa24002b6f9d786f95d0530c0aae72338ef70df`.
Toko feedback remains the snapshot recorded below; no new confirmed Canic blocker
was found. Keep this bounded scheduling improvement. Native transport evidence
does not establish PocketIC/mainnet elapsed savings or a whole-release speedup;
no broad suite or deployment was run. Both speed changes remain in the same .24
changelog draft, with package versions unchanged.


## Agent-session build reuse — 2026-09-18

CANIC-176 requests a controlled explanation for launcher-environment cache
misses. Toko's eight-role 111.26-second run also changed source/configuration and
cannot isolate the cost of its reported CODEX differences. This Canic experiment
isolates only `CODEX_SESSION_ID` and `CODEX_THREAD_ID`, using the existing host
complete-build fixture and a real Cargo/build-script/compiler Wasm probe.

### Source and experiment identities

Base: published Canic 0.110.23, commit
`732b4629e690cc5f014078b73201129cd9630271`. Rust is
`1.98.1 (48a229cea 2026-09-01)`; Cargo is
`1.98.1 (797e8a9bc 2026-08-05)`. Both runs use the fixture's unchanged Fast
Wasm profile and `wasm32-unknown-unknown` target. Workspace manifest and lock
remain unchanged. SHA-256 identities:

| Input | SHA-256 |
| --- | --- |
| Workspace Cargo.toml | `c6fcf70a441a60cfc499e7931ab19faf0868fc04221ea666f5a0abd0146a86c5` |
| Workspace Cargo.lock | `6b05c3e38e5ad453f71f2039a036acfb5933fd1f5ae29ef00bae0bd443ab49e6` |
| Control build_environment/mod.rs | `cfcefaff7ce5cf82b44578bba57e7a8c0b542514507636c52e4e2df275937182` |
| Candidate build_environment/mod.rs | `381822287c6690ead4286c538f69019ea8d02ac619a240a1ccc0408ea03c57d3` |
| Control complete/environment/mod.rs test | `ccb4285520cb0f378f0e0ef0828880a7550a1fdf0554f37529952c003c503b44` |
| Candidate complete/environment/mod.rs test | `c508221aa67d98168f89795ab7ba4fbe318beeb4a29dcbe652a0de53595ea0fe` |
| Candidate candid_cache/tests.rs | `8ea6c5ee74d12af679d6bd9766b6f5cc3a643087ecdfbf265fadea1713689d8f` |

The control first runs with both IDs absent, then with synthetic A and B pairs.
Each within-run pair keeps source, features, profile, tools, generated dependency
lock and every other inherited environment input fixed. The candidate repeats
those cases with absence assertions in the build script and Rust compiler;
its probe also asserts that an unknown `CODEX_BUILD_FIXTURE_INPUT` remains
visible. Those additional probe assertions distinguish the candidate fixture
source from the control: this is a cache-behavior experiment, not a matched
cross-version raw-Wasm-size or compile-time benchmark.

| Case | Control | Candidate |
| --- | --- | --- |
| First build with absent IDs | Miss | Miss |
| Session A, all other inputs unchanged | Miss, identical compiler Wasm | Hit, identical inputs/release/compiler Wasm |
| Session B, all other inputs unchanged | Miss, identical compiler Wasm | Hit, identical inputs/release/compiler Wasm |
| Actual build-script input changes | Miss, compiler Wasm changes | Miss, compiler Wasm changes |
| Dependency source or governed config changes | Miss | Miss |

The candidate also preserves Candid-extraction hits for absent/A/B/absent IDs,
with output equal to fresh extraction. The native extractor verifies that the
IDs are absent at execution. Independently changing an unknown CODEX input
invalidates extraction reuse; repeating its unchanged value hits. Complete-build
diagnostics retain that unknown input's changed-key attribution without exposing
values. Existing credential and shell-depth checks remain covered.

### Adopted boundary and qualification

The existing `build_environment` owner removes exactly the credential-path key
and these two correlation IDs; it continues setting `SHLVL=0`. The same exact
list governs child execution and cache inputs. Make, CI, sandbox/permission,
compiler and unknown launcher keys remain bound. Both cache owners include the
environment policy source in identity, so adoption incurs one initial miss.
No new cache, release-identity relaxation or downstream filtering is introduced.
Build scripts can no longer consume the two withheld session IDs.

Focused commands, each through `scripts/ci/run-with-test-scratch.sh` with
`ICP_ENVIRONMENT=local`:

- Control: `cargo test --locked -p canic-host --lib canister_build::reuse::tests::complete::environment:: -- --nocapture`.
  One test passes, recording two `reuse=false unchanged_wasm=true` observations.
- Candidate: `cargo test --locked -p canic-host --lib canister_build::`.
  70 pass; one existing real-extractor benchmark remains ignored.
- `cargo clippy --locked -p canic-host --lib --tests -- -D warnings` passes.
  Changed-file rustfmt and whitespace checks pass.

Logs: `/tmp/canic-session-reuse-control.log`,
`/tmp/canic-session-reuse-candidate.log`, `/tmp/canic-session-reuse-clippy.log`.
Control input hashes are also in `/tmp/canic-session-control-inputs.json`.

Limits: complete-release manifests are synthetic; compiler Wasm is real. The
fixture deliberately invokes Cargo even on a complete-cache hit to compare its
output. It does not measure elapsed time saved by skipping an actual production
pipeline. Extraction uses a native fixture, not a newly qualified real extractor.
No matched direct/Make/CI launcher matrix, Toko workload build or live deployment
was run. Other environment changes can still cause misses. Keep this bounded
change; leave whole-deployment speedup and wider launcher qualification unclaimed.

### Feedback snapshot

During the work, Toko recorded .23 adoption paired with IcyDB 0.258.0. Its new
CANIC-166 attempt stopped at its own qualified release's tool-version binding,
before Canic live review. No new confirmed Canic defect was found. Downstream
managed qualification and actual source review/apply/replay remain outstanding;
no sibling files were changed. Feedback snapshots read after the update:

| Read-only feedback | SHA-256 |
| --- | --- |
| Toko docs/upstream/canic.md | `9e0285f0625843018bc7cb1b922759bf84285dc5c7de85fda7c5dfc004c954e3` |
| Toko docs/upstream/icydb.md | `314c2d1623486012fb46c01b01cdbd8b591641161d4c1aeaa22e2569a53a0629` |


## Native debug-information reduction — 2026-09-17

The maintainer requested another speed attempt before publishing .22. Retained
native test executables exceeded 800 MB, with some internal harnesses above
900 MB. The development profile requested full debug information; the test
profile inherits it. The accepted change uses `debug = "line-tables-only"` in
`profile.dev`. Every other manifest value is unchanged, including native
optimization, debug assertions, overflow checks and Fast/Release Wasm settings.

This preserves filename/line-number backtraces but removes variable/type debugger
detail. Full debugging remains opt-in with `CARGO_PROFILE_DEV_DEBUG=2` or
`CARGO_PROFILE_TEST_DEBUG=2`, respectively. Cargo documents the debug modes,
overrides and test inheritance in its [profile reference](https://doc.rust-lang.org/cargo/reference/profiles.html#debug).
The setting applies to this workspace; consuming Canic as a dependency does not
change a downstream workspace's profile. Switching profiles incurs a one-time
native rebuild. No test, recovery assertion or Wasm optimization is removed.

### Controlled harness comparison

At source `9393893e31cc9ff75b65c54e00c961e119b62228` plus the existing .22
changes, the experiment used Rust/Cargo 1.98.1, unchanged source/lock/features,
nonincremental compilation and the same already-compiled dependencies. Each
command selected only the `canic-host` library test harness:

```text
CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=0 cargo rustc --locked --offline \
  -p canic-host --profile test --lib --message-format=json -- \
  -C debuginfo=<2|line-tables-only> --cfg canic_native_debug_measurement
```

The identical unused cfg ensures both measurements recompile their harness
instead of reusing previously retained outputs. Cargo artifact records confirm
that `canic_host` is the only compiled unit in both measured runs. An earlier
143.48-second warm-up compiled dependencies and is excluded; subsequent cached
repeats are also excluded.

| Harness debug mode, dependencies unchanged | Compile/link command | Executable bytes |
| --- | ---: | ---: |
| Full | 68.415s | 834,493,736 |
| Line tables | 54.320s | 694,189,024 |

The candidate reduces this measured compile/link step by 14.095s (20.6%) and
its executable by 140,304,712 bytes (16.8%). This is one matched pair, not a
statistical benchmark or a complete-gate measurement; sibling builds were active
on the same machine. Both inventories hash to
`66b264d7239db929939ad1c06164d1bf70e5e56efa459792ce0a2fd3f69f0f86`, and all six
selected reinstall guards pass in each executable. Reports/logs:
`/tmp/canic-debug-forced-pair-report.json`,
`/tmp/canic-debug-full-3.log`, `/tmp/canic-debug-lines-3.log` and corresponding
`-test.log` files. The excluded attempts remain in
`/tmp/canic-debug-pair-report.json`.

### Adopted profile qualification

With line tables applied through the native dependency graph, the host harness
is 296,419,976 bytes (64.5% below the full-debug baseline) and has the same test
inventory hash. This larger footprint reduction includes dependencies; no
matched whole-graph compile-time reduction is claimed. The actual native build
takes 2m19s, then 68 build/cache tests pass in 19.38s (one pre-existing extractor
benchmark ignored). All six reinstall guard tests also pass. A temporary probe
using the exact development profile confirms file/line backtraces, active debug
assertions, overflow rejection and the full-debug override. It adds no retained
test framework or runtime behavior.

Logs: `/tmp/canic-22-native-profile-tests-retry.log`,
`/tmp/canic-22-native-profile-guards.log`,
`/tmp/canic-22-debug-proof-lines.log`,
`/tmp/canic-22-debug-proof-full-override.log`. Result metadata is retained at
`/tmp/canic-22-native-profile-result.json`. The initial invocation was stopped
while waiting for an editor-started workspace check; no competing test was left
running. Its log remains `/tmp/canic-22-native-profile-tests.log`.

Final Cargo.toml SHA-256 is
`cc764930daf50776d67ae35715f7030095c52d09b4fd6ee9057e0df99f67fd15`; Cargo.lock
remains `b8eb9b563a218b95ce9a9637fcb8ec33c9fa432fe3f8c312108a93ddf3cb55d9`.
Keep: reduced native compile/debug-data cost with explicit debugger tradeoffs.
No broad validation, new IC execution claim, whole-release duration, version bump
or publication is claimed. The preceding live recovery qualification remains
that earlier source checkpoint; no runtime/host Rust was changed in this slice.

## Shell-depth normalization and reinstall verification — 2026-09-17

The maintainer extended the existing .22 batch to investigate Toko's
`candid-generate`/`qualification` cache miss and repeated artifact/reinstall cost.
Source context is `9393893e31cc9ff75b65c54e00c961e119b62228` plus the scoped
Canic changes recorded below; packages remain .21. Rust is 1.98.1, PocketIC
16.0.0 and ICP CLI 1.5.0. The lockfile SHA-256 is
`b8eb9b563a218b95ce9a9637fcb8ec33c9fa432fe3f8c312108a93ddf3cb55d9`.
No sibling source, version, release binding or publication was changed.

### Controlled launcher reproduction

Copies of Toko's actual generation, qualification, toolchain and managed-output
scripts ran through the same recursive-Make launcher shape in a temporary
workspace. A controlled environment and stub Canic command stopped before any
real build or deployment. Environments were compared in memory; only key names
and source hashes were retained. The two launchers added/removed no keys and
differed only in `SHLVL`. The generation script uses command substitution and
the qualification script pipes Canic output through `tee`.

Canic now sets `SHLVL=0` for build/tool child processes and includes that same
value in complete-build and extraction fingerprints. This changes execution and
identity together. Other variables, including Make's variables, remain bound;
there is no blanket launcher exclusion. Child shells may increment their own
nesting normally. The existing deployment-credential exclusion is unchanged.

The real Cargo/build-script fixture proves normalized compiler/build-script
inputs, identical release/Wasm reuse across changed, absent and malformed shell
depths, and rejection after a genuine environment, source or configuration
change. Extraction reuse has the corresponding regression. All 68 selected
build tests pass (one existing real-extractor benchmark remains ignored).

This reproduces one concrete launcher difference, not the exact historical .21
failure: that log retained no changed-key evidence, and the synthetic launch does
not compile Toko. A new representative downstream pair is still needed to
establish whether other inputs differ and to measure deployment savings.
Read-only Toko feedback remains SHA-256
`e9c5086465168de13405a8ef95661d0bb1ed81ec816b03270b2553f1dc9e7874`.

Reproduction report: `/tmp/canic-launcher-pair-report.json`. Exact copied inputs:

| Script | SHA-256 |
| --- | --- |
| generate-candid-bindings.sh | `973ae50cda7dff68d809837cd5085e782484799e40e38980862eaf1b18b6c3bd` |
| qualify-canic-adoption.sh | `04a2a44e3124d8c2591e6c265f6d2809d2932c7891e32e2fd645c8a27874b19b` |
| canic-toolchain.sh | `83bc7a1b660431fd7046a9f00e4d5a50e4666fee69ad5d03398c3121a659c505` |
| didc-toolchain.sh | `2a64eaa7e9d44fa72e11b64e99d3790ed1a8be3dcb3e626648999b92ae535b9b` |
| managed-output.sh | `e1197e13a039d3a1cc52961016c1d060b5df88afb36a0134d7130f3a82d0c8ed` |

### Artifact cost and rejected intermediate-cache experiment

The earlier mixed-topology log's 411.44-second selected-build reinstall span
includes two deliberate wipes and their recovery. It is not 411 seconds of
compilation. Within it, replacement artifact resolution took 50.73 seconds
(application build 41.96 seconds), first wipe 193.92 seconds and second wipe
166.75 seconds. Initial artifact preparation was a separate 326.14 seconds.
The earlier terminal-descendant section retains its original run totals.

A small controlled Cargo pair tested a shared `CARGO_BUILD_BUILD_DIR` with
separate declaration/runtime final output directories. Both Release and Fast
preserved distinct outputs but compiled the same number of units: three for
the first declaration, three for the first runtime, then zero for each unchanged
repeat. The host build helper was not reusable between the different profile
inputs in either layout. Discard: no production directory/cache change is
justified by this experiment. These are native synthetic unit counts, not
Canic/Toko Wasm or end-to-end speed evidence. Reports:
`/tmp/canic-intermediate-pair-report.json` and
`/tmp/canic-intermediate-fast-report.json`.

### Bounded reinstall authority verification

Review-time asset inspection was already bounded. The subsequent before-reset
and terminal verification loops still read each retained asset serially. They
now share the existing collector with at most four assets in flight. Each asset
still gets fresh reserve evidence and a protected management inspection through
the exact Root/Candid binding. Before reset checks exact controllers and module;
terminal verification retains its existing controller contract. IC-side paid
call admission remains authoritative.

Each issued group drains before the first input-order mismatch or transport
error is returned; no later group starts after rejection. This may complete up
to the rest of the current group's reads after a mismatch. There is no added
successful-path request, observation reuse, concurrent destructive effect or
change to reset/recovery authority. Source-bound contract validation remains
outside and before the group.

Six focused host tests pass. They prove nine-asset bounded overlap, drainage,
first-input mismatch before a later transport error, no later group after
rejection, and fresh controller/module/reserve observations on retry. Scoped
host library/test all-feature Clippy passes with warnings denied. Scoped
formatting, whitespace and changed-file built-in secret scanning also pass. Logs:
`/tmp/canic-22-shell-depth-tests.log`,
`/tmp/canic-22-reinstall-guards-tests.log` and
`/tmp/canic-22-speed-clippy.log`.

The exact mixed-topology PocketIC case passes, including initial convergence,
both distinct deliberate reinstalls, interruption recovery, state/conservation
checks and effect-free terminal replay. The case takes 1072.47 seconds and the
focused runner 1265 seconds, including native harness compilation. Candidate
initial artifacts take 484.47 seconds; replacement artifact resolution takes
50.00 seconds and selected-build reinstall 407.24 seconds (first wipe 193.61,
second wipe 163.59). Log:
`/tmp/canic-22-reinstall-verification-pocketic.log`.

This is correctness qualification, not a controlled latency improvement. The
older run used different source/dependencies; this run rebuilt artifacts and
shared machine resources with sibling builds. The recorded selected-reinstall
cost is close to the earlier 411.44 seconds, so this fixture does not demonstrate
a substantial end-to-end gain. The native regression proves bounded overlap;
a representative matched Toko run remains necessary to measure its benefit.

Changed production source SHA-256:

| Source | SHA-256 |
| --- | --- |
| build_environment/mod.rs | `cfcefaff7ce5cf82b44578bba57e7a8c0b542514507636c52e4e2df275937182` |
| fleet_ensure/ops/platform.rs | `138ffce5b70d28c230981d398c192da11a98f7c52790d248e3b5119e627aeb57` |

Keep the execution/fingerprint normalization and bounded verification. Discard
the unhelpful shared-intermediate-directory experiment. Both changes extend the
same .22 draft; no whole-suite or live Toko deployment result is claimed.

## Bounded Candid extraction — 2026-09-17

After declaration Cargo completes, the configured-role build now extracts Candid
in groups of at most four. Runtime profile derivation waits for the complete
successful result vector. Each extraction retains its existing exact extractor
and Wasm checks, environment binding, optional cache validation and atomic cache
writes. Every started worker is joined before returning the first input-order
failure or scheduling another group. No Cargo invocation, feature grouping,
release binding or runtime compilation runs concurrently as part of this change.

The existing real-extractor qualification compares sequential and bounded
extraction through the same production entrypoints, using separate empty caches
and alternating order across three rounds. The six retained Fast declaration
Wasms are `canister_app`, `canister_scale_hub`, `canister_scale`,
`canister_user_hub`, `canister_user_shard` and `canic_fleet_root` under
`target/canic-wasm/declarations/wasm32-unknown-unknown/fast/`. The final log records
every Wasm hash, extracted Candid hash and the native extractor hash. All outputs
equal independently extracted Candid bytes.

| Extraction with empty application cache | Run 1 | Run 2 | Run 3 | Median |
| --- | ---: | ---: | ---: | ---: |
| Sequential | 5245ms | 5240ms | 5441ms | 5245ms |
| Up to four workers | 2732ms | 2711ms | 2781ms | 2732ms |

Keep: the median reduction is 2513ms, approximately 48% of this phase. Operating
system caches are warm; this does not measure cold-machine builds, Cargo time,
finalization, peak memory or whole-deployment speed. Up to four extractor
processes may now consume resources together. Existing verified cache hits still
skip extraction. The source is the current .20 checkout plus open .21 changes;
these measurements are not an immutable release-validation receipt.

All ten focused extraction/normalization tests pass, including the explicitly
selected real-extractor case, duplicate cache entries, ordered results,
failed-batch draining, no later batch, and existing cache/tool/input rejection.
Host all-target/all-feature Clippy with warnings denied, layering and whitespace
pass. Logs: `/tmp/canic160-candid-batch-final.log`,
`/tmp/canic160-candid-batch-clippy.log` and
`/tmp/canic160-candid-batch-layering.log`. The earlier exploratory comparison also
improved from median 5186ms to 2682ms; final-source results above own the claim.
This completes the next bounded speed slice in the same .21 draft.

## Terminal descendant receipt reads — 2026-09-17

The first requested speed continuation after CANIC-178 overlaps the independent
Root allocation-receipt queries for each parent's children. Previously each
query completed before the next began; the existing collector now issues at
most four at once. Every receipt must validate before that child set's existing
management inspections begin. Pagination and parent traversal remain ordered;
all exact allocation, release, role, parent and pool bindings remain checked.
No request is removed, no evidence survives a terminal pass and no mutation
concurrency is introduced.

The new host transport regression requires groups of four requests to overlap.
It injects distinct invalid receipts in the second group and proves input-order
error selection, completion of all eight issued reads, no ninth request and no
management inspection. This exercises actual host request encoding/decoding
through a synthetic ICP executable; it is not IC-execution or latency evidence.
All 20 inventory/collector tests pass, as do host all-target/all-feature Clippy
with warnings denied, layering and whitespace checks.

The existing exact PocketIC case
`pic::fleet_registry::baseline::tests::generated_mixed_topology_and_ready_reserve_recover_one_reviewed_operation`
passes in 910.92 seconds, with 978 seconds for the focused runner. It retains
initial convergence, interruption recovery, two deliberately distinct reinstalls,
conservation and effect-free replay. Initial artifact preparation takes 326.14
seconds; selected-build reinstall takes 411.44 seconds. These are candidate-run
costs, not measured savings. The fixture has one child per parent, so it qualifies
the integration path while the transport regression proves sibling concurrency.
No matched baseline or representative Toko deployment comparison was run.

Source context is published Canic .20 (`cf51d9dbd`) plus the open .21 worktree,
IcyDB 0.257.21, the unchanged mixed-topology fixture and existing Fast/local
artifact tools. Logs: `/tmp/canic160-descendant-reads-tests.log`,
`/tmp/canic160-descendant-reads-clippy.log`,
`/tmp/canic160-descendant-reads-pocketic.log` and
`/tmp/canic160-descendant-reads-layering.log`. Toko's refreshed read-only review
finds no new Canic funding defect and keeps publication/live adoption separate.
This completes the selected bounded continuation in .21; broader serial-effect,
Registry acquisition and end-to-end throughput work remain follow-ups.

## Terminal partition reads and build-phase visibility — 2026-09-17

The post-.20 batch follows CANIC-178's funding correction with the current
CANIC-160 speed/progress requests. Terminal inventory previously queried each
Component's active partition sequentially before its existing bounded management
inspection phase. These independent queries now use that same four-read collector.
All partition responses must validate before any management inspection begins;
input order still selects failures, issued reads drain and a failed batch cannot
publish partial inventory. There is no additional request, retained observation
cache, new executor or mutation concurrency.

For N independent partitions this changes the scheduling shape from N serial
query waits to at most four concurrent reads per batch. It does not establish a
wall-time improvement: no matched end-to-end deployment comparison was performed.
The focused host selection passes 21 tests, including exact partition authority,
receipt/current-head rejection, concurrency bounds, ordered failure and draining.

Toko's new .20 adoption feedback separately reports a silent runtime Cargo/link
phase despite active compiler work. The existing child-output boundary now emits
a declaration/runtime heartbeat every 30 seconds. It preserves child arguments,
build identity, captured output, exit status and launch failures, and explicitly
includes possible Cargo-internal lock waits rather than claiming CPU progress.
The heartbeat's output-preservation and immediate launch-error tests pass. Both
phase labels were also observed while building the focused CANIC-178 IC fixture.
This is progress visibility, not a compilation-speed claim.

Evidence: `/tmp/canic-178-speed-tests.log`, `/tmp/canic-178-cli-heartbeat.log`,
`/tmp/canic-178-pocketic.log` and `/tmp/canic-178-clippy-final.log`. Packages stay
at .20 with an open .21 draft. The complete release gate, representative remote
latency comparison and downstream acceptance were not run for this slice.


## Stopped-Root recovery funding and reset authority reads — 2026-09-16

Canic base: `45db483d08213ad0286dc17726b2ee788cd3ec1e`, packages .19,
open .20 draft. Qualification includes existing dirty work and IcyDB 0.257.19;
it is not an immutable release receipt. The read-only Toko feedback digest is
`5fb157bf1dd1c6fa34bbd42341af541c44894b0174fe9c828e0829561d4c17ce`.

### Behavior and accounting

The existing Root prerequisite compiler inserts one native Fund between Stop
and Reinstall only when required. With `p = observation_bound + update_bound`,
the Root must already cover `p` to stop. A three-effect reset needs `3p`;
when native balance `b < 3p`, its reviewed credit is `(3p - b) + p`, its
expected balance is `4p`, and its total burn allowance is `4p`. Each payment
adds the selected Ledger fee to the operator debit. Another Root's balance
does not pay this Root's initial stop requirement. These are conservative
allowances, not predicted consumption or a complete successor quote.

Before initial Stop, apply validates all payment arithmetic, the exact Ledger
fee, the unchanged observed operator account and coverage of the full debit.
Before Fund, it checks the installed Principal/module/controllers/subnet,
Stopped status, retained pre-payment balance and remaining reviewed margin.
A restarted Root, missing/inconsistent pre-payment evidence, changed fee or
excessive new burn cannot receive the credit. Retry keeps the exact withdrawal
identity and does not demand the already-spent operator balance again. After
install/start, terminal verification checks the historical funding receipt
and retained balances, followed by whole-operation conservation. It does not
mistake new-runtime consumption for an uncompleted old payment.

The source-bound activation compiler now retains the reset's funding and fee
budgets and adds its extra paid-effect burn allowance. Its preceding preparation
still transfers no funds and retains its former unfunded headroom requirement.
Native policy regressions qualify this branch; the PocketIC case below qualifies
the management-only reset branch. No new executor, journal schema, protected
predecessor protocol, production canister behavior or reduced burn bound was added.

### Focused evidence

- Final host selection: 23 passing tests in
  `/tmp/canic156-160-final-host-refactor.log`. Includes per-effect authority,
  initial account/fee coverage, inconsistent plan and retained prebalance
  rejection, lost-response retry admission, activation accounting, settlement,
  current-source seal guards and install evidence.
- Host/internal-testing all-target/all-feature Clippy with warnings denied:
  `/tmp/canic156-160-final-clippy-pass.log` passes.
- The existing `generated_reinstall_recovers_lost_install_and_reaches_working_fleet`
  case uses its local audit Root to reduce native balance to 8T while retaining
  ordinary 2T observation/update bounds. It proves zero withdrawals after
  controller or restarted-Root drift, durable Stop/Fund intent, one successful
  withdrawal despite a lost reply, interrupted reinstall recovery, prerequisite
  replay, full nineteen-Workload/five-Ready convergence, six depleted assets,
  exact operator debit accounting and terminal effect-free replay.
  Final result: PASS, 423.08 seconds for the case and 477 seconds for its runner.
  Final log: `/tmp/canic156-stopped-root-pocketic-final-retry.log`.
  This is local fixture evidence with a mocked Ledger and an instrumented audit
  Root; it is not a mainnet deployment or exact production Root artifact.

An earlier version forced funding by inflating burn bounds to a fraction of
Root balance. Its new stop/payment assertions passed, but a later successor
review rejected `InsufficientCycleConservation` (available 13344913560359310,
required 14617230696448900, shortfall 1272317136089590). Retain
`/tmp/canic156-stopped-root-pocketic.log` as that limit's evidence; the passing
ordinary-bound fixture does not establish acceptance of this high-bound case.
The first final launch failed before any test because the sandbox could not bind
PocketIC's localhost socket; the final retry used the required local-server access.

### Call-count qualification and remaining work

`reinstall_authorities` now completes version evidence from the Root management
status already read in that inspection. Missing versions retain the management
fallback. A matched fake-ICP transport fixture observes:

| Inspection | Remote attempts | Status target sequence |
| --- | ---: | --- |
| Before | 5 | Root, Coordinator, Root, Store |
| After | 4 | Root, Coordinator, Store |

Both include the same operator-balance observation. The next pass changes Root
module/version/status/controllers, forces Store-read failure, then retries and
sees fresh values. Baseline log: `/tmp/canic160-authority-pass-baseline.log`;
final candidate log: `/tmp/canic156-160-final-host-refactor.log`.
No cache crosses a pass or mutation. This removes one call per inspection,
not a measured percentage of whole-deployment latency.

RF3 remains incomplete: the current startup scenario and known top-ups do not
collect budgeted descendant live usage/reservations or establish complete recovery
demand. Preserve RF2's accepted sequence before RF3. Toko's representative
24-pool/nine-workload comparison and further paid-effect timing remain open.
Its persisted Registry-prefix candidate also needs an upstream-owned API:
Canic's `MainnetCatalogClient` wraps ic-query 0.43.1's `LiveSubnetCatalogSource`,
whose acquisition/prefix state is private and memory-only. Fresh-head/agreement,
restart, corrupt-prefix and endpoint-disagreement qualification must accompany
that extension; a second Registry collector in Canic is not proposed.

The .20 draft and operator docs are updated. The complete recovery/throughput
batch is not yet push-ready. No broad suite, version transaction, commit,
publication, live deployment or sibling mutation ran.

## CANIC-139 deployment credential exclusion — 2026-09-16

Toko feedback is unchanged at SHA-256
`62e20bfa84b2586971051f279b51221edef50414c42c06487190568722065f92`.
Build subprocesses now remove inherited `CANIC_ICP_IDENTITY_PASSWORD_FILE`;
complete-build and Candid-extraction identities and diagnostics exclude the same
key. Cargo, compiler/cache probes, Wasm tools, provenance and tool acquisition
share the boundary. Every other inherited environment key remains bound.
Deployment identity unlocking, exact release/output checks and source-drift
rejection are unchanged. This is an environment contract, not a build sandbox.

Isolated child invocations qualify changing, removing and restoring the credential:
real build.rs/rustc and a native extractor see no inherited credential; compiler
Wasm stays identical; the same synthetic sealed release is found before Cargo.
A genuine build-script input, dependency source and configuration each invalidate
reuse. This uses a minimal synthetic dependency workspace and release manifests;
it does not measure production Fleet or Toko build time. The affected build/tool/
provenance/identity selection passes 115 tests, with one existing real-extractor
qualification ignored. One cache-probe assertion failed in the initial selection;
its diagnostic now preserves the actual typed error, and the same selection
passes on rerun. The initial cause remains unconfirmed. Host all-target/all-feature
Clippy with warnings denied passes after private-module visibility cleanup.
Logs: `/tmp/canic139-build-environment-regression-retry.log` and
`/tmp/canic139-build-environment-clippy-retry.log`; the initial failure is retained
in `/tmp/canic139-build-environment-regression.log`.

The .20 changelog and build documentation are updated; packages remain .19.
The accepted batch remains open for management-only funding protection, complete
recovery forecasts and measured remote-call reductions. RF2 remains preserved;
downstream adoption and complete deployment-speed qualification remain separate.
No broad validation, version bump, commit, push, deployment or sibling mutation ran.

## Native funding preparation contraction — 2026-09-16

Base remains `45db483d08213ad0286dc17726b2ee788cd3ec1e`, packages .19,
with the dirty .20 batch preserved. CANIC-160 requested sharing duplicate
pre-withdrawal observations within one fresh effect boundary. The executor now
gets a new native-funding intent's starting balance from its first protected
effect observation. Ops constructs the record; workflow persists intent before
withdrawal and consumes the observation once. A retained intent has no in-memory
observation to reuse. Post-payment checks remain fresh and receipt-bound. No paid
effect is parallelized and no funding or burn bound changes.

The first baseline failed during artifact construction because an implicitly
selected sccache server retained `.tmp/test-runtime.tyouds`, already removed by
the preceding test. Log: `/tmp/canic-160-funding-before.log`. The direct scratch
runner now selects the existing repository wrapper when none is explicit,
preserving explicit wrappers and explicit disabling. Both successful comparison
runs use that corrected launcher and its stable socket/temp directory. The
existing executable release-integrity contract covers selection, empty/custom
overrides and persistent cache lifetime; `/tmp/canic-160-scratch-guard.log` passes.
This is a targeted launcher correction, not a fix for arbitrary external cache
server environments. Native test incremental settings remain unchanged.

Matched exact PocketIC case:
`pic::fleet_registry::baseline::tests::four_workloads_and_four_failed_assets_repair_without_new_creation`.
Logs are `/tmp/canic-160-funding-before-stable.log` and
`/tmp/canic-160-funding-after.log`; both pass. The case retains four withdrawals,
zero transfers, no new creation during repair, lost funding/reset responses,
reconstructed adapters, eight retained assets, cycle conservation and effect-free
replay. The four raw Wasms are byte-identical, under fixture release identity
`a4c128728412f11837b79ce8562e3115451db387e17361b79b4f15d02cbb36ae`.
Fast profile, Rust 1.98.1, ICP CLI 1.5.0 and PocketIC 16.0.0 are unchanged.

| Scope | Baseline | Candidate |
| --- | --- | --- |
| Funding inspections across four top-ups and recovery | 13 | 9 |
| Remote attempts inside those inspections | 52 | 36 |
| Sum of funding inspection time | 10.689458s | 7.135785s |
| Median individual inspection | 849.319ms | 752.499ms |
| Individual inspection range | 755.022–917.948ms | 734.339–859.465ms |
| Live journey excluding initial artifacts | 78.187370s | 75.863417s |
| Initial artifacts | 54.936221s | 31.093670s |

Four fresh intents remove one four-call inspection each. These inspection counts
are separate from the unchanged 80 observation-stage records and their 224
remote attempts; they are not whole-journey call totals. Initial build/cache and
compilation costs differ, so runner totals are not a controlled speed measure.
The live journey is about 3.0% lower in this one pair; do not extrapolate to
mainnet, 24-asset estates or full release duration. Temporary timing probes were
removed. Summary: `/tmp/canic-160-funding-summary.json`; inventories:
`/tmp/canic-160-funding-{before,after}-wasms.json`, equal SHA-256
`1c49b2bacd5dcbd34c080682c36ab93e6d754f4110f1ec82378df90f2ee782bd`.

All 58 selected native Fleet workflow tests pass (two governed PocketIC cases
remain ignored in that native selection), including durable intent before payment,
fresh retry after a lost receipt and observation failure before any funding intent
or payment. Log: `/tmp/canic-160-funding-native.log`. Final host all-target/all-feature
Clippy passes in `/tmp/canic-160-funding-clippy.log`, as do scoped formatting,
layering, shell lint, whitespace and .20 release-notes preflight. Final source
SHA-256: effect preparation
`02abdb0c86d986a5503ce23ef89baae2de465c7325e80f8142dd4fb80e335d67`,
workflow `7017e7eb333ee56ca8b807af2f3b66134a0612f11d46f9147e10b405e7988ad1`,
scratch runner `f38d095517b67e927659d1d8bcbf9b5c0a3710a6386fadca233f39431901072d`.

During closeout Toko added staging latency feedback at commit
`3e2d16c8cf77b30407ce7893a401a24f2eeef753`, feedback SHA-256
`af360d5009c2be2693edac2fd0e35c594f4a3c09dfc58ef655e1dcfa6adf5406`.
Receipt: `../toko-miner/docs/upstream/artifacts/staging-0.2.10-latency-review-2026-09-16.json`.
Its credential-environment separation, incremental Registry refresh, effect
attribution and narrower observation requests remain candidates. Their reported
category sums are not removable wall time. Operator docs now explain non-zero
phase time with zero newly issued remote calls. The design tracker records the
remaining boundaries; no sibling or mainnet changes were made.

## Rejection evidence and sealed funding — 2026-09-16

Base remains `45db483d08213ad0286dc17726b2ee788cd3ec1e` (0.110.19), with
the existing dirty .20 work preserved. Read-only Toko feedback remains at
`e314310837501adce7d84925504e217c0c80540f73891325b1082e323a3401ef`.

CANIC-139 post-build comparison failures now attempt to retain a maximum 256 KiB
record under `.canic/build-reuse/rejected-<release-build-id>.json`. Evidence
contains snapshot fingerprints/counts, the failing path's available snapshot
values and before/after invocation/output-root locations. It contains no source
contents or environment values. Null input values mean not in that inventory;
fresh rechecks of dropped paths are not represented as final-inventory entries.
Diagnostics are optional and cannot supply cache authority. A successful retry
preserves the failed comparison. Three complete-build tests pass, covering
changed-source evidence, no successful record on rejection, missing/unwritable
diagnostics, final symlink refusal and verified-hit independence from corruption:
`/tmp/canic-139-rejection-evidence.log` (5.55s execution).

CANIC-156 seal preparation previously pooled Root and Coordinator balances while
authorizing no transfer. It now rejects each authority below its individual
`update + 8 × observation` allowance. The conservative bound is unchanged. Two
native preparation regressions pass in `/tmp/canic-156-seal-headroom.log`,
including surplus on either other authority and exact per-authority headroom.
The first new fixture omitted the Root's topology parent; correcting that fixture
made the test reach the intended funding boundary, with no topology relaxation.

The exact existing PocketIC case
`pic::fleet_registry::baseline::tests::restored_root_preserves_its_inventory_but_cannot_allocate`
passes in `/tmp/canic-156-sealed-funding.log` (133.83s test execution; 199s runner
wall time including compilation/fixture preparation). It rejects registered-child
funding with typed `AUTHORITY_INACTIVE` before and after a 4T Root top-up, observes
no child credit and less than 1B Root debit per rejection, resumes the live fence
explicitly, permits the same 5T request and observes no second credit on replay.
The restored snapshot retains the original physical inventory, suspended timers
and sealed authority, rejects resume and rejects allocation. Top-up uses PocketIC
native cycle injection: this is fence qualification, not Ledger receipt proof,
automatic host protection, mainnet cost qualification or a speed comparison.

Host and internal-testing all-target/all-feature Clippy with warnings denied
passes in `/tmp/canic-139-156-clippy.log`. No production runtime behavior changes
were needed for the live fence proof. Automatic pre-top-up protection for the
management-only prerequisite, complete live forecasts and protected-call cost
remain open; no predecessor protocol call or compatibility lane was introduced.

## Build discovery and startup review — 2026-09-16

Base is `45db483d08213ad0286dc17726b2ee788cd3ec1e` (0.110.19), with the
existing dirty .20 work preserved. Toko feedback SHA-256
`e314310837501adce7d84925504e217c0c80540f73891325b1082e323a3401ef`
adds a 126-second release-build rejection for declaration `OUT_DIR/translations.json`.
The read-only receipt is
`../toko-miner/docs/upstream/artifacts/release-0.2.10-2026-09-16.json`, with log
`/tmp/toko-0210-release-patch-retry.log`. Its rejected build is
`46fec436bed3e8079a32cfa08d93e4615c46db51338339d5a072a53795f3a555`.

The current Canic snapshot of that checkout names only the authored catalogue,
not either generated copy (`/tmp/canic-139-translation-diagnosis.log`). The
temporary read-only probe was removed. The retained Cargo regression now edits
an authored catalogue before a warm build, verifies both generated copies really
change, admits that build, and rejects later catalogue or generator edits.
All 20 reuse cases pass (`/tmp/canic-139-final-reuse.log`). This does not reproduce
or close Toko's latest historical failure: the pre-failure dependency inventory
is not retained. Do not weaken source-drift admission or claim a retry proves cause.

Separately, Canic now observes its three known path exports in retained Cargo
build-script output before compilation. A real-Cargo fixture removes role `.d`
records while retaining exports into another private checkout copy, verifies
pre-build observation and successful unchanged regeneration, and still rejects
newly discovered or subsequently edited foreign inputs. This handles that
controlled discovery boundary, not arbitrary copied Cargo state. Source and
generator bytes remain bound, and generated outputs are excluded only beneath
the invocation's selected output roots.

Inspection also found a concrete path-classification defect: the output-prefix
check preceded canonicalization. It could misclassify an alias into an output
tree as source, or skip an authored input reached through `target/../...`.
Classification now resolves existing inputs and output roots first. A focused
regression covers both directions, fresh authored-byte observations, unresolved
missing parent traversal, and foreign generated files. This is an independently
demonstrated boundary defect, not evidence that Toko's lost inventory used aliases.
All 21 reuse tests pass after this correction
(`/tmp/canic-139-resolved-reuse.log`).

CANIC-156's review uses the same configuration-bound calculation as startup
prepayment: `max(configured minimum, startup minimum + steps × (update + 3 ×
observation))`. It exposes those parts before Root reinstall, with explicit
fresh-child/full-publication assumptions and no reuse credit. Root allowances
overlap the Fleet continuation ceiling. The projection grants no new funding,
does not pause child grants or reserve balances, and excludes complete live
successor funding/fees. Native forecast/prepayment, overflow, required-JSON and
continuation-authority regressions pass, as do 18 Fleet CLI tests. Logs are
`/tmp/canic-156-forecast-regressions.log`,
`/tmp/canic-feedback-139-156-tests.log` and `/tmp/canic-156-forecast-cli.log`.

The existing nineteen-Workload/five-Ready production-adapter PocketIC case
`generated_reinstall_recovers_lost_install_and_reaches_working_fleet` passes
(`/tmp/canic-156-160-live.log`, 496.79s test execution). Added assertions prove a
nonempty Root startup forecast before any reset mutation, zero prerequisite
funding/continuation authority, and the configured per-step burn. Existing
controller-drift rejection, lost-install-response recovery, successor convergence,
cycle conservation and effect-free replay all remain in that same journey.

Temporary timing probes, removed after the run, attribute 163 successful
inspection pairs: median reserve-preflight path 78.253ms, median protected-update
path 401.031ms, and median complete pair 473.169ms. These include host transport,
process and replica costs; they are not IC instruction measurements or isolated
CPU timings. The 53 nontrivial preparation batches (over 1ms) have median 82.738ms.
Six full configured-estate observations each make 52 remote attempts and take
4.059–4.611s. Per-call durations overlap inside four-wide batches and must not be
summed as wall time. Summary: `/tmp/canic-160-inspection-summary.json`.

This is one local attribution run, without a matched baseline or mainnet candidate
measurement. It supports investigating the protected-call path before more local
preparation tuning, not removing reserve/authority checks or increasing concurrency
without qualification. It does not explain Toko's 43–52s mainnet observations or
establish a whole-release speedup. The probe-bearing platform/test diff SHA-256 was
`f500d329517b9b4350b393cdd3008d54c5d6c87b7e5c692c008f210bdaabbfb3`.
No permanent measurement framework or canister runtime change was introduced.

After probe removal and path-boundary cleanup, scoped host/CLI/internal-testing
all-target/all-feature Clippy passes with warnings denied
(`/tmp/canic-feedback-final-clippy.log`). Formatting, layering, diff whitespace
and the .20 release-notes preflight pass. Package versions remain .19; no broad
gate, publication, deployment or sibling mutation ran. The remaining accepted
work is recovery-funding protection, downstream first-build acceptance and
further protected-inspection cost reduction, not an unrun broad gate alone.

## Disposable Candid argument I/O — 2026-09-16

Configured pool inspection already issues at most four target-specific
preflight/inspection pairs concurrently. Source inspection found no basis for
removing their live authority/reserve checks. This continuation instead removes
an unnecessary host disk flush from temporary arguments: the three writers in
typed Canic calls, raw ICP calls and Fleet initialization now share the ICP
writer. Observatory uses that same owner. Each file is exclusively created,
written completely and closed before the child opens it. Unix permissions stay
`0600`, Fleet's 16KiB bound remains, and callers retain cleanup. Failed writes
remove incomplete scratch. Durable intent, journal and artifact writers are
unchanged. No protocol, IC call, timer or retry semantics change.

The isolated probe extracts the two existing transport writer bodies and
compares them with only `sync_all` removed. Each round creates, reads back,
compares and removes 100 files; three rounds run per size on repository ext4
scratch, with Rust 1.98.1 and `rustc -O`. Median per-file costs:

| Writer / bytes | With disk flush | Without disk flush |
| --- | ---: | ---: |
| Typed Canic / 64 | 1.742ms | 0.051ms |
| Typed Canic / 4KiB | 1.734ms | 0.051ms |
| Typed Canic / 1MiB | 3.864ms | 1.088ms |
| Raw ICP / 64 | 1.942ms | 0.055ms |
| Raw ICP / 4KiB | 1.676ms | 0.055ms |
| Raw ICP / 1MiB | 3.917ms | 1.061ms |

This measures file I/O, excluding child startup and the network. The disposable
probe is `.tmp/canic-argument-io-probe/probe.pl`, SHA-256
`f11ef3101c29bdfd9de3b00a2c9bb616d514adcbde9de7fddc0d8ae947b621a1`.
Raw logs are `/tmp/canic-argument-io-{protocol,raw}-{0,1}.log`; no benchmark
framework or timing threshold was added to maintained tests.

The exact existing PocketIC case
`pic::fleet_registry::baseline::tests::four_workloads_and_four_failed_assets_repair_without_new_creation`
passes before and after, including recovery, no-new-creation repair, conservation
and effect-free replay. Both retain the same four raw Wasms and 80 observations /
224 attempts, with every stage's call count unchanged. Observation totals are
21.522s / 21.202s; the live journey excluding initial artifacts is
79.537408s / 78.319863s. These small single-pair differences do not establish a
reliable whole-journey gain. Initial artifacts differ (36.326525s / 14.411679s),
and native compilation differs; runner totals 128s / 143s are not an attributable
speed comparison. Keep the simpler shared writer and directly measured I/O
reduction; mainnet and full-release improvements remain unmeasured.

Both use base `45db483d08213ad0286dc17726b2ee788cd3ec1e` plus the preceding
dirty .20 fixes, package .19, the same lock/config/release identity, Rust/Cargo
1.98.1, ICP 1.5.0, PocketIC 16.0.0 and local/Fast Wasms. `Cargo.lock` remains
`03de13af98c25e09813e1d5147ce8cd2983e1fd7e8328eeb679a08c913feda51`;
`baseline.rs` remains
`6212632868b6c6089b5aa0417d62d29ac9daa98abfab332977879787ca209b5e`.

| Changed writer source | Before SHA-256 | Measured candidate SHA-256 |
| --- | --- | --- |
| Typed Canic | `4e4392d4abd7f7b54d2fb2d6f85d76d2271d1d6ab36dcfbc3579baab7dfce635` | `9d4b73f09455f7cec88c89800f6f70db8a7353b74fc66942d5938c121bbb42c7` |
| ICP | `9a7cd8af7b38698e2703c05d621399d6d9d0926ce3b05c51b28532b120ae69c6` | `f2ede05f72b6317746e0a933c9a0aa60b11a39851c15354ccf48932d6bf128a7` |
| Fleet | `87e88265a9a1617823477a92a4b6e8a49f658b0ae09ac2b3603d0455012837b2` | `d51073213acbc8944254eb920126ea8b535c5f6afbd9a5e4e226013c49e5f0cc` |

All changed source identities are in `/tmp/canic-argument-{before,after}-identities.txt`.
Live logs are `/tmp/canic-argument-live-{before,after}.log`; extracted phase and
observation records are `/tmp/canic-argument-{before,after}-{phases,observations}.jsonl`.
Wasm inventories are scoped to `test-runtime.sVqTyW` / `test-runtime.ORPjJu`,
with equal normalized inventory SHA-256
`bb7d54c5ee5251fa65c3ca44b3c91c953416aabab77b2d09ceab4c83c4f8020d`.
The final lint-only visibility correction inside the private ICP module keeps
the helper's crate-only re-export and behavior; final ICP source SHA-256 is
`a3617d0d36c8192c3a23191f620b5b319377f15247ab0d532c27a87d32639535`.

All 56 selected native tests and host all-target/all-feature Clippy pass. New
coverage proves a child reads complete 1MiB Candid arguments, fresh paths for
both call modes, cleanup after success/command failure/invalid response, and
the exact Fleet size boundary. Existing private-file checks remain. Logs:
`/tmp/canic-argument-{native,observatory,clippy}.log`.

The final read-only Toko check advances to SHA-256
`79d5292f7ca93d0f79de8d77ce399dc2feb04e2ab4a27e067ae34e8b397eb233`.
Its new CANIC-175 entry confirms mainnet funding restoration across ten targets,
none omitted/unconfirmed and no reconciliation failures. It adds released-.19
acceptance, not a new blocker or injected mainnet partial-failure proof. Existing
CANIC-139/156/160 boundaries remain. No sibling changes or mainnet actions ran.

## CANIC-160 retained pool balance concurrency — 2026-09-16

New Toko feedback records 24 retained canisters on released .19: configured
observations take 43–52s with 52 remote attempts, and pool balances another
11–13s. Source SHA-256 is
`ddeb320f5df4b07b0ac2c0c2fcee9ff2973b80b46bab74409bb6142586ba7789`.
This is mainnet baseline evidence, not a candidate deployment result. The
pool-balance stage still inspected PendingReset/Failed assets sequentially.

The candidate uses the existing four-wide inspection collector. It retains
per-target reserve preflight, operator/Root/controller authority, the existing
observation scope and exact cycle projection. It drains issued reads, validates
in inventory order and publishes each batch's balances only after validation;
a failure does not start the next group. Other lifecycle states are skipped.
No calls are removed or shared across reviews, effects or retries.

The existing exact PocketIC case
`pic::fleet_registry::baseline::tests::four_workloads_and_four_failed_assets_repair_without_new_creation`
passes before and after, including funding/recovery, repair without creation,
conservation and effect-free replay. All four raw Wasms match. Both runs retain
80 timed observations and 224 remote attempts, with identical counts per stage.

| Scope | Baseline | Candidate |
| --- | --- | --- |
| Pool balances: 11 observations, 30 attempts | 8.112s | 2.579s |
| All timed observations | 26.607s | 21.310s |
| Live journey excluding initial artifacts | 84.337432s | 79.303343s |

The pool stage is 68.2% lower; the live journey is 6.0% lower in this one pair.
Initial artifacts take 41.611382s / 14.132044s, so total runner times (181s/147s)
are not a controlled speedup measure. Keep this scheduling change. Mainnet
candidate acceptance, the larger configured-canister cost and a full release
duration remain unmeasured. Host timing does not measure IC instructions/cycles.

| Input | SHA-256 |
| --- | --- |
| Baseline `ops/platform.rs` | `fe37d5c2d19a2446d69743912701b15a2f853706fc55b94fbf492296909c34be` |
| Candidate `ops/platform.rs` | `61e42d48055693d18a1a1023252b62f6409ee38c7b932c2b817f624b0575d547` |
| Shared `Cargo.lock` | `03de13af98c25e09813e1d5147ce8cd2983e1fd7e8328eeb679a08c913feda51` |
| Shared `baseline.rs` | `6212632868b6c6089b5aa0417d62d29ac9daa98abfab332977879787ca209b5e` |
| Shared normalized four-Wasm inventory | `938ecd4bbb3e99d745e26dc20ff499df60d52c3fca315ed6951a7861adef862e` |

Both use the same local/Fast four-Workload configuration, .19 package versions,
Rust/Cargo 1.98.1, ICP 1.5.0 and PocketIC 16.0.0. Configuration, release identity
and individual Wasm hashes match the request-runtime experiment below. The
baseline already includes the preceding .20 fixes and two-worker request default;
only the host pool-balance scheduling and its native tests differ. Logs and
extracted measurements are `/tmp/canic-pool-balances-{before,after}.log`,
`/tmp/canic-pool-balances-{before,after}-{observations,phases}.json`,
`/tmp/canic-pool-balances-{before,after}-identities.txt` and
`/tmp/canic-pool-balances-{before,after}-wasms.normalized`. Artifact inventories
are scoped to `test-runtime.s6y85o` / `test-runtime.KL4bG5`, respectively.

Both new native tests pass. They prove nine selected assets across full/partial
batches, skipped Workload state, drained failures, no failed-batch publication,
inventory-order error selection and fresh retry balances. Log:
`/tmp/canic-pool-balances-tests.log`. No broad gate or mainnet action ran.

After qualification, all 244 selected Fleet native tests pass (two governed
cases remain ignored), and Canic/host all-target/all-feature Clippy passes with
warnings denied. The only lint correction was a paragraph break in the generated
writer's API documentation; measured runtime/host behavior is unchanged. Logs:
`/tmp/canic-speed-followup-fleet-tests.log` and
`/tmp/canic-speed-followup-clippy.log`. Formatting, layering and release-note
preflight pass; package versions remain .19 and the .20 draft is current.

## CANIC-139 linked outputs and repeated dependency reads — 2026-09-16

A controlled Cargo probe uses the exact generated-source writer body extracted
from Canic, with the same absolute config environment watch, `OUT_DIR` output,
canonicalization and exported include path. A regular copied target reruns the
producer with a local path. Replacing only the generated output with a symlink
emits the original checkout's path; changing the new checkout's config then
overwrites that original output. The corrected writer rejects the link before
reading or writing through it, including equal bytes and a dangling target.
The original output remains unchanged. This is a reproduced hazard, not proof
that Toko's retained state was created through this exact sequence. Existing
Cargo metadata is not repaired or declared portable.

The disposable probe stays inside Canic `.tmp/`, with no sibling writes.
Its script is `.tmp/canic-139-relocation-probe/run.sh`, SHA-256
`5fdead96e09b8f1a9135896ecfedb3cedd05737bec77b11e05906b1405071d87`.
Baseline/candidate extracted writer units are
`0b3caa45cbd54bd925d78beed75aca8a32a2bed5a6567a0e824887da8a75bf5a` /
`286097ae62a089ae1263afc73614a15d3d38333f5e60a0de80e753debde41036`.
Matched source directories end in `canic-139-relocation.jwmnby` and
`canic-139-relocation.LbNgJo`. Logs:
`/tmp/canic-139-relocation-{before,after}.log`; the candidate's expected Cargo
exit 101 is the early linked-output rejection. This is a native Cargo producer
probe, not composed-Wasm or downstream deployment qualification.

Fixture persistence also distinguishes `FixtureArtifactError::Symlink` and
names the offending component, including a `.canic` parent. Nine fixture tests,
13 build-support tests and both real build-macro integration tests pass. The
existing missing/tampered output repair and unchanged-timestamp checks remain.
Logs: `/tmp/canic-139-{symlink-tests,source-tests,build-macro}.log`.

The next change removes repeated hashing of a path while consuming Cargo
dependency records. It uses only evidence already acquired in that same input
snapshot, including complete earlier package scans. Each new snapshot starts
empty and rechecks bytes. It neither admits unobserved external inputs nor
retains source evidence between build invocations.

Using the 16 retained top-level Fast dependency records in Canic's current
target tree, three collector-only passes take 362/348/343ms before and
162/163/169ms after. Median time falls 53.2%, saving 185ms for this step.
All 780 paths and content hashes remain byte-identical. This is a small local
verification improvement, not an end-to-end build or release comparison. The
probe starts with an empty input map; additional reuse of package-scan evidence
is not measured. A whole-workspace probe refused a `node_modules/.bin` symlink
inside the frontend consumer example; that boundary was not bypassed. No broad
validation or compiler benchmark ran. Temporary timing instrumentation was
removed after the comparison; the regression retains only behavior checks.

| Identity | SHA-256 |
| --- | --- |
| Collector before | `2ddfffe3ede630e709f7c5767530b1c2a728d21b7d172c43563e963867896e7f` |
| Collector after | `21b2ef16d70afb65150574fbc9afac2e407357b92105ad40d3de6789ad5c4308` |
| Shared dependency-record inventory | `190639372e8c61f638b8d1105535d2bb4e83b3538007d25f5e9bfe20b3538252` |
| Both collected file maps | `aca3d1cf40def8fde3f3ac9019b1e40b8569b791878055886c3ce057c4f56c9a` |

Logs/maps: `/tmp/canic-input-collection-{before,after}.log`,
`/tmp/canic-input-collection-{before,after}-files.json`,
`/tmp/canic-input-collection-{before,after}-records.sha256`.
All 19 build-reuse regressions pass, including preservation of the first
observation and fresh detection of later edits/deletion:
`/tmp/canic-139-reuse-final.log`. Keep the optimization with its limited timing
claim. Package versions and release identity policy are unchanged.

## Request runtime and terminal inventory — 2026-09-16

The existing exact live case
`pic::fleet_registry::baseline::tests::four_workloads_refill_four_ready_with_lost_funding_and_creation_responses`
qualifies both experiments below. It retains four top-level Components, four
Ready assets, lost funding/creation responses, conservation and effect-free
replay. Its nested funding adapter now uses the existing observation timing
callback, also present in the measured baseline. No new measurement framework
or recovery behavior was introduced.

First, partition-status reads were moved into the existing four-wide runner,
before the existing inventory inspections. Both live runs passed with identical
Wasms and call counts, but the two terminal observations increased from 4.674s
to 5.192s. The experiment was discarded. Final `current_inventory/mod.rs` is
byte-identical to baseline, SHA-256
`26eabd86477abcf2850e47bf2574f59010381c8cbd2450c53c62cdb58fffb0a8`.
Logs: `/tmp/canic-inventory-{before,after}.log`; normalized Wasm inventories:
`/tmp/canic-inventory-{before,after}-wasms.normalized`.

Second, short-lived ICP request processes default to two Tokio workers when the
operator has not set `TOKIO_WORKER_THREADS`. The measurement host exposes 64
CPUs; the inherited setting was absent. Tokio's default otherwise follows
available CPUs. A second worker leaves room for background I/O without creating
a worker per CPU for each request. This is an overrideable request default,
not a claim that two is globally optimal. Generic commands and replica startup
retain their previous environment. The original serial partition path was
restored before qualifying this candidate.

| Observation scope | Baseline | Request candidate | Remote calls in each |
| --- | --- | --- | --- |
| All 65 timed observations | 17.654s | 15.432s | 169 |
| Terminal inventory, two observations | 4.674s | 4.148s | 36 |
| Protocol planning, six observations | 7.657s | 6.559s | 73 |

Observation time is 12.6% lower in this one pair. Every stage retains its
observation/call count. The live journey excluding its initial artifact phase
is 125.393248s / 125.767792s: essentially unchanged. Cold/warm compilation
differences invalidate total runner comparisons. There is no measured reduction
of the complete 90-minute release gate. Keep the bounded request startup change
for the observed request-overhead reduction; do not extrapolate it to the whole
release or interpret host time as IC instruction/cycle measurements.

Both use source base `45db483d08213ad0286dc17726b2ee788cd3ec1e`, current dirty
Fleet fixes, Rust/Cargo 1.98.1, ICP 1.5.0, PocketIC 16.0.0 and local Fast Wasms.
The measured source difference is confined to the four ICP adapter files.

| Shared input | SHA-256 |
| --- | --- |
| `Cargo.lock` | `03de13af98c25e09813e1d5147ce8cd2983e1fd7e8328eeb679a08c913feda51` |
| Timing-enabled `baseline.rs` | `6212632868b6c6089b5aa0417d62d29ac9daa98abfab332977879787ca209b5e` |
| `four-workloads.toml` | `dc4740791789028dda0d219adb05390006baef34cd8e7b99e0cb1066c6a885cc` |
| Normalized four-Wasm inventory | `938ecd4bbb3e99d745e26dc20ff499df60d52c3fca315ed6951a7861adef862e` |

Release identity is
`a4c128728412f11837b79ce8562e3115451db387e17361b79b4f15d02cbb36ae`.
All four raw Wasms match: application
`6947231adb54d0f42475d2f0b79c84824ad59fb6eb5326fa54b4f8a941de8b22`, Coordinator
`01f2b0161bb03e28d02ee81da04fe1f034e356c82a12bcff5a2ad007235a05fd`, Root
`9521914e601807658ec792a2a1e97e3aea7edf06bb235b8626aa566764b962ff`, Store
`e16d0d581bc394de5654b7ad1e1e4239e01805383a3a33f56cdd41f963b38175`.

| ICP source | Before SHA-256 | After SHA-256 |
| --- | --- | --- |
| `command.rs` | `ef41d649c3ac922640b1f5b359575b8c66d6f38b2216657ca10cd097e4084813` | `e46060df2fca984d0fa71ba0cbe21128f73b11135465d1408aa0e5bf87236aa7` |
| `identity.rs` | `0e99e0283ceda43c7773c3f000e238a79219800ff26d1a9f6d372978be8c259e` | `b4ca800b965d17ee8f0039e4961514f172ad4f9d6c9887cdd1f3056f40ae86c8` |
| `balance.rs` | `7a688acbe0e5e413d7a1a63034e29df05a6f7a70d6e6a13bdfef8fcb22c92505` | `9512b7d3a84d55653581e44fa7b3fe98316a2cdd2f578b61013927872a1f6b74` |
| `management.rs` | `e2fd1096296bb801aaca09fd0b5f0e622ef687986ff0e2fb31c5be128ebe179d` | `25fed8a9b86e40144b047941f92a4d948d7be820028e5d814a7e2630c86ca2f1` |

Evidence: `/tmp/canic-inventory-before.log`,
`/tmp/canic-icp-runtime-after.log`,
`/tmp/canic-icp-runtime-{before,after}-identities.txt`,
`/tmp/canic-icp-runtime-after-{observations,phases}.json` and
`/tmp/canic-icp-runtime-after-wasms.normalized`. The matched baseline inventory
uses scratch `test-runtime.PuL0b5`, candidate `test-runtime.XStEQj`; unfiltered
raw scratch inventories also contain older unrelated runs and are not parity
evidence. Forty ICP tests, 242 Fleet native tests (two governed cases ignored),
and host/internal all-target/all-feature warning-denied Clippy pass. Logs:
`/tmp/canic-icp-runtime-tests.log`, `/tmp/canic-speed-fleet-final.log`,
`/tmp/canic-speed-clippy.log`. No broad gate ran.

## CANIC-139 isolated-checkout diagnosis — 2026-09-16

Toko's reported first-build rejection names an absolute path under
`/home/adam/projects/toko-miner/target/canic-wasm/declarations/`, while the build
ran under `/tmp/toko-miner-staging-0.2.9-20260916/toko-miner/`. Its retained Root
build-script `output` also exports `CANIC_CONFIG_SOURCE_PATH`,
`CANIC_CONFIG_MODEL_PATH` and `CANIC_ROLE_RUNTIME_AUTHORITY_PATH` under the
original checkout. Its declaration `.d` names both original and current compact
config files. The rejection is therefore of an external input, not a failure
to exclude this invocation's own generated output. Evidence is read-only:
`/tmp/toko-029-staging-isolated-regular-ci.log` and the generated Root's
`target/canic-wasm/declarations/wasm32-unknown-unknown/fast/build/canic-fleet-root-1e0c3f1b6f73ea4c/output`.
The retained evidence does not identify exactly how those foreign paths entered
the build output; a controlled relocation/cache reproduction remains open.

A real Cargo regression builds the same generated-config fixture into runtime
and declaration targets, verifies unchanged input authority on the first build,
then rejects a modified producer and a first-discovered external generated
file. No production input exclusion was widened. Build documentation now
recommends independent `.canic` and Cargo directories with compiler-cache
sharing, rather than transplanted Cargo records or symlinked mutable state.
This does not claim the downstream 106.20-second retry is fixed.

All 18 focused build-reuse tests pass, including the real-Cargo regression and
the existing external-input, symlink and corrupt-output checks. The fixture
clears the inherited intermediate-directory override so its generated files
actually reside under each selected target tree. Log:
`/tmp/canic-139-generated-inputs.log`. Final changed-host Clippy is retained at
`/tmp/canic-speed-host-final-clippy.log`.

## CANIC-160 activation visibility — 2026-09-16

Toko's .19 acceptance reports 23 identical waiting messages across 63.185s at
`50/52`, despite existing Coordinator evidence identifying `ActivatingRuntimes`,
directory Roots `1/1`, runtime Roots `0/1` and three Components. The feedback
source remains SHA-256
`1bf36355cbed0cdd9343f894e8e61646730a362db2d40c1a16965a26d2bbdebc`.

The existing current-protocol observer now projects a fixed-size informational
summary directly from the typed status. The existing workflow wait event carries
that summary and monotonic elapsed seconds, and the existing CLI renderer shows
both in text/JSON. Elapsed time starts when processing the current effect (or
terminal check) in this invocation, includes issuing/observing the effect, and
resets on resume. It does not become stall or retry authority. No internal
progress identity is parsed or exposed, no additional event or remote poll is
introduced, and funding/completion observations remain unchanged.

Qualification: 242 focused Fleet host tests and 18 Fleet CLI tests pass.
Host/CLI all-target/all-feature Clippy passes with warnings denied; eight
selected provisioning regressions pass after lint cleanup. Projection tests
retain separate directory and runtime denominators and absent-status handling.
The bounded ten-wait fixture checks operation/plan bindings, stage/counts,
monotonic elapsed values, existing pacing, one issuance and effect-free replay.
CLI tests reproduce the reported `50/52` presentation in text and typed JSON.
Logs: `/tmp/canic-160-{host-tests,cli-tests,clippy,regressions}.log`.

This is a diagnostic improvement, not a timing measurement or an activation
speedup. No new PocketIC, downstream deployment or broad gate ran. Toko live
acceptance and its matched latency comparison remain open. The Store-planning
measurements below predate this projection and retain their own exact source
identities. The .20 draft includes both changes; package versions remain .19.

## Store template observations within planning — 2026-09-16

The .19 mixed-topology case took 861.774s in the retained complete release run,
including 136.508s initial artifacts and 530.210s selected-build reinstall. Its
27 protocol-planning observations made 690 calls. Source inspection confirms
that manifest, chunk-set and individual chunk predicates repeatedly query the
same Store template status without an intervening host effect.

`current_protocol::bind_unapplied_actions` now owns a temporary map of successful
template observations, keyed by exact Store Principal, Candid path/digest,
template ID and version. Only that read-only compilation shares responses.
Every action revalidates its Candid bytes and applies its original predicate;
ordering and provisioning/readiness dependency handling are unchanged. Errors
abort the pass without retaining failed evidence. The ordinary execution-time
observer creates a fresh scope for every action. No state, response or failure
survives into another plan, effect, retry or terminal-inventory observation.

All 19 focused `fleet_ensure::ops::current_protocol::` native tests pass,
including three new transport regressions for reuse/freshness, failed reads and
Candid drift, and isolation across each query-identity field. Host
all-target/all-feature Clippy passes with warnings denied. Logs:
`/tmp/canic-store-staging-tests.log`, `/tmp/canic-store-staging-clippy.log`.

The existing exact PocketIC case
`pic::fleet_registry::baseline::tests::generated_mixed_topology_and_ready_reserve_recover_one_reviewed_operation`
passes before and after. It retains initial convergence, source-selected
replacement, interruptions/lost replies, funding recovery, two deliberate
wipes with distinct operations, cycle conservation and effect-free replay.
The fixture and its assertions are unchanged. Both runs use the final workspace
IcyDB 0.257.18 lock, .19 package versions, Fast/local artifacts, Rust/Cargo
1.98.1, native ICP 1.5.0, PocketIC 16.0.0 and repository `target/`.

| Input | SHA-256 |
| --- | --- |
| Baseline host `current_protocol/mod.rs` | `9655552e38a9efe1c91800af51bffecd6b7b6b7b88360875d7748de12d0d722c` |
| Candidate host `current_protocol/mod.rs` | `a8ea46c2edb87703d2af2ea892cea8c24dc37536812684ef6fd33cf66aadb2ea` |
| Candidate `current_protocol/tests.rs` | `199b66b2b22ae7016b876af82c7646f93e15201e7ff1d7524262670e584085c9` |
| Shared `Cargo.lock` | `03de13af98c25e09813e1d5147ce8cd2983e1fd7e8328eeb679a08c913feda51` |
| Shared fixture `baseline.rs` | `92f1abf598fb85a7db9555e1e9a29d1702e5dc91df28e3c8260877828463b790` |
| `apps/test/test-configs/generated-mixed-topology.toml` | `420311d1a4872bb4380976c66174840117e18e6bec725fc04604f08d1baa0f4f` |

The source base is release commit `45db483d08213ad0286dc17726b2ee788cd3ec1e`.
The prior standalone IcyDB audit-fixture pin alignment is present in both runs;
that fixture is not this test's subject. Initial release ID is
`a4c128728412f11837b79ce8562e3115451db387e17361b79b4f15d02cbb36ae`, replacement
ID `33888cd229a38503c7d4d2ce3fd53c48e003dd5856473bef2165c5a67889010a`.
All sixteen raw Wasms match byte-for-byte between runs, comparing each release
separately. Normalized hash inventories are retained in
`/tmp/canic-mixed-topology-{before,after}-wasms.normalized` and share SHA-256
`89ab8e191145d70a734d8ae697cfa272ab83bca1c7ff7b7eb74efa5462f72953`.

| Scope | Baseline | Candidate |
| --- | ---: | ---: |
| Protocol-planning observations | 27 | 27 |
| Protocol-planning calls | 690 | 430 |
| Protocol-planning time, seconds | 61.232 | 42.305 |
| Initial convergence, seconds | 154.279 | 143.194 |
| Initial terminal replay, seconds | 36.804 | 32.155 |
| First deliberate wipe, seconds | 229.969 | 221.403 |
| Second deliberate wipe, seconds | 193.760 | 189.638 |
| Derived live journey excluding both artifact phases, seconds | 624.822 | 595.615 |
| Initial artifacts, seconds | 475.405 | 53.150 |
| Replacement artifact resolution, seconds | 56.569 | 46.886 |
| Complete journey, seconds | 1156.795 | 695.652 |
| Runner including native compilation, seconds | 1289 | 759 |

Planning removes 260 calls (37.7%); its observed time drops 18.927s (30.9%).
Every other observation stage retains its observation count and call count,
including all twenty terminal inventories and their 460 calls. The derived live
phase subtracts initial artifacts and nested replacement artifact resolution
from the complete journey; its 29.207s reduction (4.7%) is one pair, subject to
machine and simulator variation, not all attributable to this change. Nested
planning timings must not be added to parent phases. The cold baseline compiles
declarations for 153.69s versus 1.06s on the warmed candidate; overall runner
ratios are not evidence of this optimization or a whole-release speedup.

Logs are `/tmp/canic-mixed-topology-{before,after}.log`; derived observation and
phase records use the same prefix with `-observations.json` and `-phases.json`.
Both exact-case commands run through the existing governed targeted PocketIC
runner and `/usr/bin/time -v`, without a new measurement framework. Native and
Wasm compilation remain serial with respect to other Canic build commands.

CPU attribution is limited but useful: the baseline command accounts for
1,609.70 CPU-seconds over 1,288.64 elapsed seconds, about 1.25 cores on average,
including compilation. Within a separate 77-second initial-convergence window,
`/proc` reports 5.32 native-host CPU-seconds and 33.50 CPU-seconds in its completed
children; the top-level PocketIC server adds 8.80 CPU-seconds. This excludes its
unmeasured sandbox descendants and is not a complete machine utilization trace.
Raw ticks (100/second) are in `/tmp/canic-mixed-topology-cpu-sample.tsv`.
Twenty read-only `icp --version` launches take 0.36s elapsed / 1.08s system CPU
with the inherited environment, versus 0.14s / 0.13s with
`TOKIO_WORKER_THREADS=2`. That small probe is a lead for representative transport
qualification, not a recommendation to change production thread settings.
No thread settings, simulator pacing or test coverage changed in this slice.

The .20 changelog includes this qualified change. The broader throughput batch
remains open. The final Toko recheck records successful .19 local reinstall and
CANIC-150 verification. CANIC-160 now reproduces 63.185s at `50/52` with 23
identical awaiting-progress messages despite available `ActivatingRuntimes`,
directory `1/1` and runtime `0/1` evidence. A bounded typed stage/count/elapsed-wait
projection through existing progress was the next priority and is now recorded
above, without extra polls or exposing the internal progress-identity string. Terminal-inventory,
process-startup and remaining journey costs follow;
no full validation, version bump, commit, push or deployment was performed.

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
