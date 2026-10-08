# Canic sibling ownership and offload audit

Canic can delete more duplicated machinery by completing existing sibling
adoptions. The clearest immediate targets are single-executable publication,
Wasm resource comparison and Backup directory-checksum framing. PocketIC
process ownership is a larger opportunity, with a remaining diagnostic-custody
gap. Auth offers a substantial future boundary but its current libraries are
unpublished and implement contracts and encoding, not verification.

**Verdict: PASS WITH FINDINGS for the source ownership audit.** This is not a
build, runtime-safety, performance, deployment or release-readiness verdict.
The maintainer reported that 0.110.53 has pushed. Its historical handoff is not
authority for reopening that publication.

## Scope and evidence identities

Trigger: the maintainer requested a full audit of sibling repositories to
reduce Canic tooling, complications and redundant code. The audit inventoried
all 15 sibling Git checkouts under `/home/adam/projects`, then traced their
reusable contracts to Canic production, test, CI and integration consumers.
Application payloads, database algorithms and frontend internals were inspected
for ownership eligibility, rather than subjected to independent correctness
audits. No sibling files were changed.

Primary method: `audits/flow-convergence-and-duplication.md`; companion:
`audits/complexity-and-technical-debt.md`. Both are the adopted Shared Tooling
snapshot `a3430b34b32a60f3b245a2b4f7e2f5321556fe56`. Product authority is
`AGENTS.md`, including the supplied session instructions and governance links.
The complexity overlay is version 4 at
`docs/audits/recurring/system/complexity-accretion.md`; its older recorded
snapshot is descriptive history, not the method revision used here.

Canic HEAD: `4c51a87c6a32397196bb3f65d064641194df10a5`, release 0.110.53.
At input capture only `Cargo.toml` and `Cargo.lock` were dirty. Those incoming
changes were preserved. Their captured SHA-256 values are respectively
`51c6c01d797e333cd59d65fc634e6a518d5ba7477664d4fedaf3df5018608bb5`
and `81ac35235bf34a3807d2a6d74c08c3c9c7bc06746f6e948f2ce82113a0595d5f`.
The lock changed during inspection: its final captured Testkit selection is
0.22.0, superseding the initially observed 0.21.3. It selects Backup 0.7.0,
Host 0.5.1, Memory 0.31.3, Metrics 0.2.11 and Timers 0.14.15. Query 0.48.1
still introduces Host Artifacts/FS 0.4.6, so the graph has two Host generations.

[Source snapshot](source-snapshot.json) records complete repository HEADs,
dirty-path inventories, relevant source hashes and selected IC packages.
Checkouts were changing independently during the audit; these are point-in-time
observations. Uncommitted upstream changes are proposals, not adopted releases.
The audit ran on Linux through source reads, Git inspection, registry-cache
inspection and read-only GitHub queries. It ran no Cargo compilation, Make gate,
PocketIC suite, provider call, release command or performance measurement.
Prior tests remain bound to their original inputs. Structural comparisons with
older scored audits are **N/A (method change)**.

## Sibling inventory and convergence owners

| Repository | Captured HEAD prefix / version | Owner and Canic disposition |
| --- | --- | --- |
| shared-tooling | `0ba0ad00ed94` / committed 0.1.23 | Common release, CI, installation and verification mechanics; adopt reviewed committed bytes, retain product adapters. |
| ic-host-tooling | `81f980986115` / 0.5.1 | Artifact codecs, hashes, durable files, owned children, tool admission and Wasm facts; strongest immediate deletion target. |
| ic-backup | `f836d5e58170` / 0.7.0 | Checked artifact identities, staging, local persistence and bounded snapshot protocol contracts; consume framing now, full runners later. |
| ic-testkit | `2951fd19e587` / 0.22.0 | PocketIC server/child lifetime and generic test artifact caching; move process ownership, retain Canic case registry and recovery semantics. |
| ic-auth | `61b113961e9a` / dirty 0.1.1 | Passive auth contracts and canonical encoding; unpublished. Current proposed types package is `ic-auth-protocol-types`, not the unrelated registry `ic_auth_types`. |
| ic-memory | `692fbc81d698` / 0.31.3 | Shared memory manager/bootstrap/ledger; already delegated. Canic owns grants, namespaces and Fleet authority. |
| ic-metrics | `69b110b8fbef` / 0.2.11 | Pure arithmetic and metric summaries; already delegated. Canic owns sampling and attribution. |
| ic-query | `08012b4bedd2` / 0.48.1 | Domain inventory/query/cache/reporting; already consumed. Its prepared Host 0.5 adoption is not yet selected through the published Query graph. |
| ic-timers | `ae26b854a1a4` / 0.14.15 | Timer registry and cadence/watchdog mechanics; already delegated. Canic owns durable timer authority and fencing. |
| ic-blob-storage | `704b8ebf6bea` / 0.18.0 | Blob service/storage contracts; already externalized. New runtime-free contracts are a later native-consumer opportunity; Canic adapter still pins 0.17.1. |
| icydb | `cb8cefca1d68` / 0.267.1 | Consumer database composition, not a destination for Canic control-plane code. Shared Host/Memory/Testkit remain the convergence owners. |
| toko | `44d4e2c6d41` / 0.3.1 | Application consumer; can consume a canonical Canic Wasm report instead of maintaining its parser. Retain application qualification. |
| toko-miner | `aeed004b03b9` / dirty 0.3.19 | Application/frontend and Blob consumer, not a generic infrastructure owner. |
| ichelper | `0670a9887d72` / scripts | Ad hoc operator scripts do not supply Canic's paid-effect custody or bounded recovery contract. Do not move production workflows here. |
| toko-miner-assets | `79e2407ed0d7` / assets | Payload repository; no reusable Canic tooling owner found. |

## Five prioritized findings

### 1. Complete Host adoption and remove local executable publication and Wasm comparisons

**MEDIUM — duplicate flow and late convergence.** Owner:
[Canic #458](https://github.com/dragginzgame/canic/issues/458), with the consumer
Wasm reporting surface in [#481](https://github.com/dragginzgame/canic/issues/481).

Trace: `tool_install::publish_executable` creates a destination-local temporary
file, copies bytes, sets executable permissions, syncs, checks the staged hash,
runs admission, renames and syncs the parent. Host 0.5's
`ic_host_fs::durable::write_validated_with` now owns this single-file transaction
and closes its writer before admission. Replace the local transaction with that
API and project its producer/admission/cleanup/publication failures honestly.
Keep caller-selected pins, byte budgets, executable mode and version admission.
Keep `stage_executable` where the independently used `publish_bundle` still
needs it: an immutable executable/dylib bundle is a different transaction.

Trace: `artifact_io::wasm` projects shared Wasm facts but independently enforces
code-section and defined-function limits. `ic_host_tools::install_limits`, under
the explicit `ic-limits` feature, already supplies `InstallLimits::report` and
signed headroom. It distinguishes code-body bytes from the encoded function
count prefix and includes imported plus defined globals. Its reference limits
are bound to IC revision `9499f64bda8bcf087188dd8a1bb594115640ba9e`, not a promise
about every deployed subnet. Keep Canic target selection and admission policy;
delete redundant comparison functions after mapping their public error contract.
Keep complete artifact size and custom-section reporting separately.

The incoming Host 0.5 selection also requires changing both
`encode_gzip(..., Compression::best(), ...)` calls to the maintained numeric
level API. Remove the production `flate2` dependency only after checking remaining
test/fixture uses. Manifest selection alone is not completed adoption.

Expose shared facts through the existing proposed `canic wasm check/report`
surface so Toko can delete `bin/wasm-postprocess/wasm_metrics.js` rather than
copy another parser. Host derives facts, Canic carries selected policy/results,
applications project those results. Raw parser size is not an estimate of net
deletions or a measured performance benefit.

**Trigger:** a coherent Host adoption slice, with focused old-destination survival,
closed-writer admission, typed failure and Wasm boundary tests. Public error/API
changes must respect the maintainer-owned minor boundary. No new wrapper crate.

### 2. Move PocketIC lifetime ownership to Testkit, preserving diagnostic custody

**MEDIUM — duplicate process ownership and material adoption gap.** Owners:
[Canic #484](https://github.com/dragginzgame/canic/issues/484),
[#474](https://github.com/dragginzgame/canic/issues/474) and new upstream
[Testkit #29](https://github.com/dragginzgame/ic-testkit/issues/29).

Trace: `run-workspace-tests.sh` owns server spawn, port-file readiness polling,
TTL, logs and teardown; worker scripts own job groups and scratch; the stopper
discovers servers through Linux `/proc`. Testkit 0.22 provides explicit
`PocketIcStartupConfig`, `start_managed_server`, `run_command`, and the
`ic-testkit-server run -- COMMAND` entrypoint. It delegates server and command
groups to Host's `OwnedChild`, preserving caller command IO and distinguishing
borrowed external servers from owned servers. This is a replacement for process
mechanics, not for Canic's test inventory or recovery behavior.

The remaining gap is exact: Testkit's private `StartupFiles::drop` deletes raw
server logs; its public output retains at most the first 16 KiB per stream.
`run_command` drops the server before returning a failed child status. Canic
requires complete failed/interrupted attempt evidence. Upstream #29 requests
caller-controlled diagnostic-file custody independently of process teardown;
Canic still owns retention and reporting policy. Do not replace local ownership
until that contract is available and qualified.

Keep Canic registered cases, ordering, two-worker scheduling, interruption/resume
suffix evidence and embedded-fixture qualification. Do not copy these into
Testkit. The version-neutral embedded source snapshot has Canic-specific package
normalization; no second equivalent consumer or ready shared replacement was
established. Keep it local rather than extracting a speculative framework.

**Trigger:** diagnostic custody is delivered, then qualify startup failure,
server exit, cancellation, inherited descendants, complete retained logs and
Linux/native macOS teardown. The current console buffering defect remains
[Canic #492](https://github.com/dragginzgame/canic/issues/492); adopting a process
owner does not itself repair that log filter.

### 3. Finish Backup framing adoption without moving publication authority

**MEDIUM — duplicated identity derivation.** Owner:
[Canic #490](https://github.com/dragginzgame/canic/issues/490); upstream
[Backup #26](https://github.com/dragginzgame/ic-backup/issues/26) delivered the API.

Trace: `persistence/artifact_commit.rs::sync_tree` admits and synchronizes
descriptors, obtains file checksums, then calls
`ArtifactChecksum::from_relative_file_checksums`. The latter still locally
sorts paths and hashes `path + NUL + checksum + newline`, with an infallible
UTF-8 assumption. `ic_backup::ops::artifacts::checksum_relative_files` now owns
that exact framing and checked relative-path/duplicate validation. Convert
records and return its typed failure at the existing adapter boundary. Never
reopen paths after descriptor synchronization to recompute this identity.

Keep Canic manifests, exact Fleet/release/controller authority, descriptor
custody, no-replace publication, crash barriers and same-operation recovery.
The previous artifact traversal/private-staging extraction in closed
[#488](https://github.com/dragginzgame/canic/issues/488) is already completed;
do not count it again as prospective savings.

Full capture/restore orchestration is a later opportunity. Backup already has
protocol, accounting and provider contracts, but complete runners and a qualified
production provider are not delivered. The ICP transport obstacle remains
[Backup #25](https://github.com/dragginzgame/ic-backup/issues/25). Keep live-create
refusal intact; do not enable it merely because Backup 0.7 is selected.

**Trigger:** qualify exact existing directory identities, path ordering, typed
malformed/duplicate rejection and owning publication/recovery cases against the
selected release. Backup 0.7 exact-source CI was still pending at observation.

### 4. Adopt committed Shared Tooling mechanics while keeping thin product adapters

**MEDIUM — ownership spread across copied helper bodies.** Owners:
[Canic #453](https://github.com/dragginzgame/canic/issues/453),
[#485](https://github.com/dragginzgame/canic/issues/485) and
[#469](https://github.com/dragginzgame/canic/issues/469).

Canic records Shared Tooling 0.1.18. The committed 0.1.23 source has the explicit,
read-only `check-runner-disk-space.sh` API and a generic stable-runtime sccache
launcher. Replace local capacity parsing with the shared helper; Canic selects
paths, thresholds and diagnostic presentation. Delegate stable sccache runtime
setup while preserving Canic's explicit cache-error/compiler fallback policy,
which the shared launcher does not implement.

Review the committed release runner and update its exact-commit consumer adapters
together under #453. Preserve structured version/receipt/tag/source checks and
the agent commit prohibition. Do not copy Shared Tooling's dirty release or
installer work, and do not delete product release guards merely because there
are many targets. Equivalent alternate entry boundaries are not duplicate
execution simply because they name the same guard.

Completed consolidation matters: Canic's five-line `install-sccache.sh` already
delegates to `install-ci-tool.sh`; `prepare-rustsec-db.sh` matches its declared
snapshot hash and the dependency-risk caller already uses it. Existing
[#468](https://github.com/dragginzgame/canic/issues/468) and
[#472](https://github.com/dragginzgame/canic/issues/472) need evidence/disposition
review, not another copy of those implementations. Vendored shared scripts and
the thin Canic validation adapter are intentional delivery boundaries.

**Trigger:** a reviewed exporter transaction and focused real adapter/stub checks.
Keep thresholds, package inventories, risk policy, retained-artifact policy and
release authority in Canic. Do not introduce a new release gate or language.

### 5. Finish Auth extraction in contract-sized cuts after publication

**MEDIUM — accepted ownership extraction remains incomplete.** Owner:
[Canic #491](https://github.com/dragginzgame/canic/issues/491).

The new Auth repository contains passive application-token/proof contracts,
validated protocol identifiers and canonical signed encoding adapted from
Canic. Its packages remain unpublished, and its current dirty proposal names
the types package `ic-auth-protocol-types`. It does not implement token verification,
session admission or wallet login. Do not treat encoding as an authorization result.

The first eligible cut moves the exact passive contracts and canonical encoding
from Canic DTO/delegated-auth paths to their published owner, with golden bytes,
Candid/facade/macro propagation and complete removal of the superseded owner.
Later verification can move only with explicitly supplied caller, issuer,
audience, trust, time and replay contracts. Canic retains Fleet issuer selection,
Root/control-plane workflow and durable spending/authority semantics. A library
must not accept trust policy from the token it is checking.

**Trigger:** published packages plus exact wire/canonical parity and the accepted
minor's coordinated hard cut. No compatibility aliases, remote request on every
authorization check, second canonical encoder or new Canic auth facade layer.

## Retained boundaries and additional candidates

Paid ICP execution is distinct from short-lived test processes. Host's
`OwnedChild` preserves a configured command and owns remaining process-group
members; it is not a sandbox for descendants that leave that group. Canic's
inherited lock custody, secret-output disposal, uncertain paid effects and
no-duplicate-spending obligations require direct qualification. Keep these
boundaries under [Host #5](https://github.com/dragginzgame/ic-host-tooling/issues/5)
and Canic #458 rather than replacing all `Command` calls indiscriminately.

Canic also has two read transport families: bounded authenticated `icp/query`
already uses `ic-agent`; local readiness `replica_query` constructs anonymous
CBOR and HTTP with `TcpStream` and `read_to_end`. This is a further consolidation
candidate under #458: reuse existing agent/HTTP libraries for transport mechanics
while retaining the local anonymous caller, endpoint/network selection and
readiness contract. The two caller identities are not interchangeable. The
audit did not qualify a replacement or make a new transport safety claim.

Testkit's development artifact cache does not own production Canic build reuse,
selected Fleet graphs or release manifests. Keep those boundaries. Evaluating
the two unpublished Canic testing packages under
[#489](https://github.com/dragginzgame/canic/issues/489) may remove local package
plumbing, but requires no new sibling abstraction.

Memory, Timers and Metrics already own the shared mechanics. Canic's memory
grant policy, durable timer authority and instruction sampling have distinct
semantic owners. Keep Blob adapters thin around the service owner and evaluate
its new contracts only where an actual native consumer needs them. IcyDB and
application database composition remain downstream. Do not move Fleet/Root
control-plane workflows, cycle-conservation accounting or application databases
into a general-purpose tooling repository.

## Change rehearsals and adoption order

These are source-based recommendations, not new implementation or release
authority. GitHub issues own acceptance and sequencing decisions.

| Rehearsal | Expected convergence | Present blocker and required evidence |
| --- | --- | --- |
| Change executable admission or IC structural comparison | Host derives one publication/report contract; installer and Wasm consumers translate it | Remaining local transactions/comparisons and numeric gzip API mismatch; retain typed errors, bundles and focused boundary evidence. |
| Change PocketIC readiness or teardown | Testkit owns lifetime; Canic scheduler borrows the server URL | Diagnostic-file custody in Testkit #29 and Canic failure retention; qualify cancellation, raw logs and supported hosts. |
| Change directory identity validation | Backup derives checked framing; Canic descriptor transaction consumes it | Local infallible composition and adapter error mapping; exact golden identities and recovery/no-publication refusals. |

Recommended first outcome: finish the existing Host and Backup adoptions with
their callers and cleanup, then adopt the reviewed Shared Tooling helpers as a
separate bounded outcome. Recommended next outcome: replace test-process ownership
after the diagnostic contract is delivered. Auth proceeds through its separately
accepted extraction design after publication; it is not an immediate drop-in.
Do not allocate a release per helper or cross a minor boundary without the
maintainer's required audit/acceptance.

## Verification and limits

Exact-source GitHub CI observations:

- Host 0.5.1 [run 37750135927](https://github.com/dragginzgame/ic-host-tooling/actions/runs/37750135927): completed successfully.
- Testkit 0.22.0 [run 37752946475](https://github.com/dragginzgame/ic-testkit/actions/runs/37752946475): completed successfully; a separate rerun was queued.
- Shared Tooling 0.1.23 [run 37746567888](https://github.com/dragginzgame/shared-tooling/actions/runs/37746567888): completed successfully.
- Backup 0.7.0 [run 37753291659](https://github.com/dragginzgame/ic-backup/actions/runs/37753291659): in progress; a second run was queued.

These are upstream aggregate status observations, not new Canic native or managed
qualification. Registry-cache source identities support API inspection; the
audit did not independently download or hash new registry archives. No net
deletion, build-time improvement or runtime saving has been measured.

Repository/source inventories, API/caller traces, committed helper comparisons
and issue searches completed. Recorded Canic input hashes remained unchanged;
the JSON inventory and local report/handoff links verify, and `git diff --check`
passes. Saved GitHub issue/comment URLs were read back. New upstream feedback
is Testkit #29; the other
findings use existing owners. Implementation, native consumer qualification,
artifact refresh if selected inputs require it, and appropriate changelog work
remain with those adoption batches. This audit adds frozen evidence, updates the
descriptive handoff and records GitHub feedback; it removes no source symbols
and changes no versions or release authority. The proposed adoption batches
are not yet implemented or push-ready.
