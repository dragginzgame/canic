# Current handoff — 2026-10-12

**Canic 0.111.0 is live; the integration batch remains uncommitted.** The maintainer
explicitly waived the 0.111 closeout audit for completing Auth, Backup and Blob
integration, then a cleanup slice to reduce Canic's surface. That decision is
recorded on [#517](https://github.com/dragginzgame/canic/issues/517#issuecomment-6119790876).
It authorizes this in-repository batch, not release/version/Git effects or a
blanket exception for future minors. The pending incompatible public family is
classified under the existing **0.112.0** changelog entry. Own package versions
remain **0.111.0**. The objective is to delete duplicate mechanisms in favor of
upstream owners; moving declarations alone does not demonstrate faster builds.

## Published Testkit selection — current graph

The maintainer removed the Testkit sibling link. Root now selects published
Testkit 0.35.0, archive SHA-256
`3313fa4e49094266268ccc732512933d8f159946a7176cf7213a90f2cfa49228`.
The temporary path exception is removed, and the standard locked server selector
passes using the prepared published 0.35 runner. Root also selects Query 0.55.0,
Backup 0.21.1, Timers 0.21.1, Host 0.15.1, Metrics 0.8.1, Blob/contracts 0.22.14,
Memory 0.35.19, Auth/protocol-types 0.5.7 and IC Build 0.1.1.

Query's current catalog loader is asynchronous. Host calls its maintained name;
fixture callers now await it or drive it on their owning runtime. Ten catalog
regressions and four cache/timing tests pass, including cancellation, cache
preservation, assurance and use inside an existing runtime. Host local-Fleet
and internal-testing library checks, strict Host library/test Clippy, governed
test compilation, dependency inheritance/pinning and locked metadata pass.
Both standard published-runner managed cases pass: maximum-state authorization
uses 989,364 instructions (unchanged 1,000,000 ceiling), with bounded restoration,
cleanup and memory checks; two-target recovery/composed certification also passes.
The embedded preflight detected genuinely stale bytes. Canonical refresh and the
read-only verifier now pass; original artifact/evidence and failure remain retained.
Evidence and held inputs: `.tmp/testkit-registry-adoption-20261012/`.
Earlier source-path proofs below retain their historical graph identity; the
whole Auth/Backup release batch remains unfinished.

## Host 0.15 artifact-interface adoption

The maintainer authorized applying the prepared Host migration to Canic. Incoming
root Host 0.15 and Metrics 0.8 selections were preserved. Canic Host enables
`ic-interface`, uses the moved text API and inspects Candid methods without
building rich argument presentation trees. Borrowed canonical Wasm facts retain
name/kind and metadata checks around transforms; function renumbering remains
allowed. Artifact comparison now includes modes and maps oneway to update,
retaining Canic's internal-export policy and release/type authorities.

Linux checks pass: 24 artifact tests, 12 Candid/extraction/cache tests and strict
Host library/test Clippy. The existing real-extractor case remains intentionally
ignored. Locked metadata passes, all prior package identities/checksums and
Windows dependency selections remain, and the lock adds only the parser edge to
Host tools. Evidence is retained in `/tmp/host015-consumer-apply/canic/`.
[Canic #458](https://github.com/dragginzgame/canic/issues/458) and
[Host #71](https://github.com/dragginzgame/ic-host-tooling/issues/71) own acceptance.
Changes remain uncommitted; full gates and native macOS were not run. This scoped
result does not qualify the other ongoing integration work below.

## Blob exact installation decoding

The targeted [#524](https://github.com/dragginzgame/canic/issues/524) repair retains
zero CONFIGURATION_LIMITS skipping quota and unchanged ordinary endpoint limits.
Its two native dedicated/embedded regressions now pass after the old Auth blocker
cleared. The managed fixture retains exact original initialization bytes, enabling
installed cases to vary only nested application arguments under the real protected
envelope. This helper is host-side test support, not reset/recovery authority.

Both actual managed compositions pass in frozen temporary checkouts at
`/tmp/canic-524-isolated.v5md68w9/`, with a separate owning target and explicit
published Testkit 0.35 CLI / PocketIC 16.1 server. Source snapshots cover Canic and
its selected Testkit path dependency. The dedicated/embedded cases reject unknown
fields and extra arguments, preserve exact stable bytes/configuration and app
state, retain the required Blob fence, and recover through the existing operator
same-instance workflow. Existing permission/capacity and restoration journeys also
pass. Scoped denied-warning Canic/testing Clippy passes. Old attempts remain.

Dedicated build: `f665751a199eb49b43d11b0a38b7c59263ac44dc6d5900e27140a9f858d3f85b`.
Embedded build: `e0225bbcdcb30a50a24c3b28e246abde37fb7fbf3f25d25f2625acc763ccd752`.
The first installed assertion incorrectly expected an unfenced view after failed
reinstall; its original log/artifacts remain. The correction requires the fence,
then performs real recovery rather than directly clearing it.

Original root attempts remain in `.tmp/issue-524/recheck.*`: native successes,
stale fixture-lock refusal, interrupted check during owner build contention,
standard published-CLI admission refusal for source-path Testkit, and a later
16-error Timers 0.21 caller mismatch during that independent adoption. The timer
owner's repaired calls are frozen for the passing cases; fixture locks are aligned
only with the directly required published Blob/Auth/Timers APIs. The corrected
regression and locks are copied back only after original files match the frozen
baseline and its build directory is idle. The whole live Auth/Backup batch is not
qualified by this local proof. #524 remains open pending family delivery; no full
gate, source fallback, package version or Git release effect is introduced.

## Scoped metrics qualification repair

The Metrics maintainer explicitly authorized two tool-resolution assertion repairs,
embedded allocation-peer regeneration and the named held-HTTP PocketIC case.
Warning-denied Host Clippy and all four resolver tests pass on Linux. The canonical
refresh and verifier pass against root Metrics 0.7.3, Host 0.14.9, Memory 0.35.17
and Testkit 0.34.5. Exact Testkit/PocketIC setup and offline admission pass.
The case passes inventory/fixture preflight and starts PocketIC, then refuses the
required `canister_test` Wasm build because unfinished Auth integration leaves
`ISSUER_CANISTER_SIG_DOMAIN` and `IssuerProofPrepareFailed` unused under denied
dead-code warnings. No overlap/completion-order pass is claimed.
[#491](https://github.com/dragginzgame/canic/issues/491) owns that integration;
[#450](https://github.com/dragginzgame/canic/issues/450) retains native qualification,
and [#99](https://github.com/dragginzgame/canic/issues/99)/
[#447](https://github.com/dragginzgame/canic/issues/447) remain open.

Complete repair logs, exact selected inputs, original and regenerated fixture
identities, the missing-runner attempt and failed private scratch are retained in
`.tmp/metrics-qualification-20261012/` outside Cargo clean. Existing integration
work, own package versions and Git index/refs are preserved; no broad gate ran.

## Earlier qualified Blob graphs

Earlier Blob qualification held
root at Auth/protocol-types 0.5.3, Backup 0.20.1, Blob/contracts 0.22.11,
Memory 0.35.15, Timers 0.19.1, Metrics 0.7.2, Host 0.14.7, Query 0.54.13 and
Testkit 0.34.4 / PocketIC 16.1.0. Those selections are preserved. The root Blob
requirements now start at 0.22.11 because the adapter uses its delivered API.

- Cargo.toml SHA-256: `095be1d4f9b1f56d6cfbec6f0e60ec53971b8b8279ff733c524188819dbb9ac9`.
- Cargo.lock SHA-256: `c21bc626993d7845628c1e711d6fd05838ea0b4d22f464399217fa5cb3d39e41`.

Both private Blob locks independently select Auth 0.5.1, Blob/contracts 0.22.11,
Memory 0.35.14, Timers 0.19.0 and Metrics 0.7.1. They retain private package
versions 0.1.0; the adapter uses the main workspace version/catalog. Actual Blob
proofs below bind these fixture graphs and their retained artifacts, rather than
claiming the later root patches were installed in those canisters. Scoped final
Clippy compiles the newest root graph with warnings denied.

## Current integration

Auth's SDK session/backend adoption is implemented under
[#491](https://github.com/dragginzgame/canic/issues/491). The durable backend
stages a session/replay pair, checks all resource limits before mutation and
commits synchronously. Bounded stable vectors and derived indexes replace the
whole-store admission rewrite. The SDK owns replay/admission and live authority
checks; status/audit also recheck current Root policy. Core's generic canister
signature adapter delegates canonical DER, certificate freshness and cryptography
upstream, removing its direct signature-crate dependencies.

Current focused managed resource cases pass on Auth/protocol-types 0.5.7,
Timers 0.21, Host 0.15 and the explicit Testkit 0.35 source library with the
prepared published 0.35 server. Maximum state (2,048 sessions / 4,096 replays)
uses **989,121 authorization instructions**, below the unchanged 1,000,000
limit; one/median state uses 822,370 / 988,222. The maximum case also passes
bounded restoration/cleanup and no cleanup memory-growth checks. Previous
3.12M/2.66M/2.46M/1.17M failures remain historical measurements, not current
failures. Logs and held input hashes live in `.tmp/auth-session-20261012/`.
The current-graph two-target recovery/composed-certification case and real
Registry-bound Root-attestation verifier case pass (16.69s / 18.71s).
Eighty-eight selected native cases, warning-denied Core library/test Clippy and
default/session-enabled Wasm compilation also pass. The changed Timers probe's native API check passes. Direct fixture Wasm Cargo
compilation correctly refuses outside Canic's governed artifact builder; that
separate installed probe was not run. Actual SDK-facade single/combined denial acceptance
remains [#525](https://github.com/dragginzgame/canic/issues/525#issuecomment-6130059637), and publication /
downstream acceptance remain separate. Keep the whole integration batch open.

Auth now owns canonical request hashing, certificate issuance rules, enabled
SEC1/prehash checks and low-S signer normalization. Default request construction
requires no crypto feature. Canic retains configured scope/TTL diagnostics,
issuer/key policy, exact caller/operation custody and committed replay/session
transactions. Invalid raw Canic role projection precedes shared certificate
lifetime validation as a deliberate hard cut; the combined-invalid regression
observes zero proof callbacks. The certificate adapter contracts from 124 to 51
production lines. Token preparation and complete current-policy proof checks
already delegate upstream; the positive-proof cache is retired.
[Auth adoption](https://github.com/dragginzgame/ic-auth/issues/32#issuecomment-6120465762)
and [#491](https://github.com/dragginzgame/canic/issues/491) own remaining acceptance.

Timers 0.21 registration uses the typed initial start policy. Lifecycle one-shots
use atomic scheduled creation instead of separate scheduling/rollback. Retained
registrations remain inactive until their owner schedules. Metrics 0.8 owns the performance aggregate; totals
are `u128`; the public Rust PerfEntry field changes accordingly. Existing u64
views omit unrepresentable totals while retaining the exact boundary value.
No second memory/timer runtime or compatibility wrapper was added.

Backup owns checksums/staging, checked digest validation, journal locks and
private sidecar admission. Canic retains command/descendant custody, original
restore authority and terminal reference release. Published 0.19.2 adds
capture-only settlement qualification but does not supply guarded terminal
reference release. [Backup #29](https://github.com/dragginzgame/ic-backup/issues/29#issuecomment-6120465396)
and [#513](https://github.com/dragginzgame/canic/issues/513) retain that prerequisite;
[#394](https://github.com/dragginzgame/canic/issues/394) owns live Fleet backup.
Do not substitute unrestricted deletion or all-Applied accounting.

Blob schemas, storage, policy, accounting and installation-memory grants already
belong upstream. The dedicated and embedded hosts retain one Canic memory and
lifecycle owner. Production Root-driven target-bound installation passes on the
delivered 0.22.11 dedicated graph, including controller/target refusals, restart,
lost-response recovery, conflicting rebinding and effect-free replay. The
complete four-artifact Fast build is
`f7aca7dd2fc40c90fcdc1a61d54eaa9c5824268e74551150f273669565830db5`;
the Root journey reuses its verified complete artifacts in 2.69s.
[Blob #61](https://github.com/dragginzgame/ic-blob-storage/issues/61)'s published
endpoint owner now replaces Canic's duplicate declaration block.
Browser/provider, completion verifier, GC, billing and actual application
acceptance remain separate under [#444](https://github.com/dragginzgame/canic/issues/444),
[Blob #31](https://github.com/dragginzgame/ic-blob-storage/issues/31),
[Blob #32](https://github.com/dragginzgame/ic-blob-storage/issues/32),
[Blob #33](https://github.com/dragginzgame/ic-blob-storage/issues/33),
[Blob #34](https://github.com/dragginzgame/ic-blob-storage/issues/34) and
[#494](https://github.com/dragginzgame/canic/issues/494). Framework proof does not
establish provider acceptance or authorize paid probes.

## Focused evidence

Current logs and held inputs: `target/review-validation/upstream-integration-20261012/`.
Session request/retry/conflict and endpoint policy, signer, certificate construction,
token verification/preparation, timer custody, wide totals and u64 projection cases
pass. Scoped Core/Blob warning-denied Clippy and enabled Wasm library checks pass.
Those initial native/Clippy proofs select Host 0.14.2; its later patch to 0.14.3
changed only the four Host selections. The managed and Backup runners were rebuilt
on 0.14.3. All 12 Backup artifact and three layout-custody cases pass on that latest
graph. The first managed timer/metrics attempt caught a missing watchdog initial
schedule argument; it was corrected after the owner finished. Its corrected rerun passes in 10.25s, rebuilding only the runtime probe in
4.62s while reusing both other fixtures. All 15 timer inventory cases pass.
The latest locked independent public client crosses Auth, Backup and Host
type boundaries successfully; the default Wasm library check passes and its
normal/build tree excludes Auth cryptography.
The first direct runtime-probe Wasm check refused the authoritative build-entrypoint
guard; actual canister proofs use the managed Host owner without bypassing it.
Failed/drifted attempts remain distinct; no full workspace or broad gate ran.

Both consumer shapes now have actual current-fixture-graph managed proofs.
The preceding detailed handoff and earlier target-based logs were removed by an
external Cargo clean; retained Wasm artifacts and failed scratch remain. New
completion evidence lives in `.tmp/blob-completion-20261012/` so another Cargo
clean does not erase it.

## Blob completion slice

The two formerly ignored composition journeys now belong to the governed
framework catalogue. A single isolated build worker uses the production CLI,
admits current-source complete cache reuse and supplies finalized manifest-bound
artifacts. Remove the adapter's testing-only dependency; it no longer carries
its own ignored PocketIC test runner. The dedicated, embedded and production
Root journeys pass in 18.05s, 68s and 15.42s. Permissions, two uploader grants,
lost tenant-mutation results, distinct capacity accounting, snapshot fencing,
same-release restoration and embedded counter retention are covered. Production
binding retains controller/target refusals and interruption/exact replay. Its
four-artifact cache reuse takes 2.77s. The final worker run totals 182.57s.

The initial parent-side cache recheck refused the test runner's compiler wrapper;
current-input admission remains in the correctly isolated build owner. Failed
scratch retains that attempt. The exposed timer instruction-total field now
carries `u128` after Timers 0.19; exact Candid/JSON wide-value roundtrip passes.
Existing u64 metric rows omit unrepresentable timer totals. No compatibility
reader or clipping was introduced.

The maintainer explicitly authorized [Blob #61](https://github.com/dragginzgame/ic-blob-storage/issues/61)
in `/home/adam/projects/ic-blob-storage`. Its opt-in contracts macro and standalone
adoption shipped in 0.22.11 at `ebb560f492b6b53925faae8890316fc66bfd6def`.
Canic consumes the actual registry packages through its root catalog and both
private locks, without a Blob source override. The adapter endpoint module is
113 lines rather than 376; the standalone module is 61. Canic retains endpoint
authentication, bounded decoding, actual caller context and the single lifecycle.

Published adoption evidence lives in `.tmp/blob-published-20261012/`. Dedicated,
embedded and production Root journeys pass in 23.80s, 2m03s and 15.12s; the
owning runner totals 177.26s. Both actual 32-method Blob Candid projections pass
strict structural comparison against the exact released standalone contract.
An initial runtime pass exposed missing Candid registrations from an imported
emitter alias. Its run was interrupted and retained; the final adapter uses a
lexical emitter. Both composition cases compare generated Candid against every
callable Wasm endpoint using the existing Host validator. Scoped warning-denied
Clippy passes on the newest root graph. The final source and catalog are
qualified; the earlier compile-only candidate was not delivery evidence. The
catalog and both fixture locks remained fixed throughout these checks. No broad
suite ran.
No Canic commit or publication was performed. Concurrent upstream commits belong
to their owner; this agent did not create them.

Provider/browser, verifier, GC, funding and application acceptance remain the
separate existing obligations. These framework proofs do not close those issues
or make the entire Auth/Backup/Blob release batch push-ready.

## Retained issue fixes and next work

The bounded config-reader fix for [#509](https://github.com/dragginzgame/canic/issues/509)
is applied locally. Host rejects oversized input before all three YAML consumers;
exact limit, missing file, UTF-8, followed/dangling symlink, directory and device
cases pass among all 23 scoped config tests. Host Clippy passes for all targets
and features with warnings denied. Its reported provisioning helper was shortened
without changing the assertions; its owning regression also passes. Both native
macOS jobs now run the config tests; their execution remains pending. Evidence
is retained in `.tmp/issue-509-20261012/`.

These latest Host checks select Auth/protocol-types 0.5.4, Backup 0.20.3,
Blob 0.22.12, Host 0.14.9, Memory 0.35.17, Metrics 0.7.3, Query 0.54.15,
Testkit 0.34.5 and Timers 0.19.3. The final check inputs stayed fixed. Earlier
Blob and Auth proofs retain their recorded graphs; this Host proof does not
relabel their installed artifacts or establish integration completion.

The macOS worker-fixture blocker in [#474](https://github.com/dragginzgame/canic/issues/474)
is corrected locally: the fixture deliberately creates redundant path separators,
then selects one physical path before passing scratch custody. Its previous
trailing-slash `TMPDIR` failure and corrected worker/workspace-runner fixture
passes are retained
under `target/review-validation/macos-scratch-20261012/`, using admitted Testkit
0.34.3 on Linux. Scoped [PR #519](https://github.com/dragginzgame/canic/pull/519)
contains only this fix and compatible draft notes against released main. Its
`baaf95619` source passes both fixtures with released Testkit 0.33.1; native
macOS results are pending in [CI](https://github.com/dragginzgame/canic/actions/runs/38233266774).

[#516](https://github.com/dragginzgame/canic/issues/516) retains the first concrete
provisioning dependency, bounded rejection evidence and exact approved resume
command. Permanent refusals suspend; transport failures back off. Its earlier
managed recovery/permanent-refusal proofs and aligned three-boundary client proof
retain their Auth 0.4.7 / Host 0.14.1 input identity. The installed Toko Root was
not modified, and these fixtures do not constitute application acceptance.
[#517](https://github.com/dragginzgame/canic/issues/517) requires rebuilding direct
consumers on the current exposed family without aliases.

Remaining Auth work includes qualifying the adopted shared signature store and
finishing the session engine's protected authority/transaction contract. Preserve exact
caller/operation records, original retrieval clocks and one certification root.
Complete Auth/Backup integration and their owning recovery proofs before the
requested cleanup slice; guarded reference release remains issue-owned. The
Canic-owned Blob endpoint/composition reduction is complete; application/provider
acceptance remains separate.
The whole integration/release batch is still open, so this slice alone is not a
push-readiness recommendation.

Shared Tooling 0.9.2 is adopted at
`c118062a070a2e238f1eb3989b6fd700f369ce1e`, with 110 verified payloads and
recorded entrypoints. Canic uses the public consumer Make include; paired
formatter commands retain all three workspaces and the custom hook. The unused
generic Cargo installer fixture is retired. Cargo-sort-derives 0.14.1 is prepared
and admitted offline. Focused adoption evidence lives in `.tmp/shared-tooling-091/`;
the directory retains the 0.9.1 preparation and subsequent 0.9.2 refresh separately.
The broader Auth/Backup integration batch remains unfinished. Shipping Rust floor is 1.91.
Sibling repositories remain read-only outside the explicit Blob #61 authority.
Preserve dirty work, archives, failed
scratch and `.canic/local-work/` / incident-repair material. No staging, commit,
version transaction, push, publication, deployment or paid effect ran for the
integration batch. The maintainer-authorized scoped PR above is isolated from it.

## IC Build registry adoption and audit

The maintainer authorized moving generic Testkit build/cache mechanics and
Canic feature-equivalent Cargo grouping to IC Build
([IC Build #1](https://github.com/dragginzgame/ic-build/issues/1)).
Canic now requires crates.io `ic-build 0.1.1`, archive SHA-256
`e6c3b1a1040d38c3f10e9f12fc456e7ba718ceaaa86dc4967ac719b5487e4aeb`,
from published source `dbb17982557174eed29e1a499a2b1a7bb278d5a7`.
The incoming lock already selected this release; raising the minimum requirement
left the lockfile and all unrelated selections unchanged.
The obsolete IC Build sibling-path exception was removed. This audit used the
then-selected Testkit incubation path; current shipping selection is published
as recorded above. No sibling sources were edited by this audit.
The engine preserves v1 cache/stamp/lock bytes and namespaces, returned artifact
lifetime, typed failures, input-change rejection and explicit maintenance.
PocketIC lifecycle stays in Testkit; Canic's complete-release admission, recipes,
credentials, generated inputs and release identities remain consumer policy.
The separate complete-release reuse/finalization path is not replaced by a
generic cache-hit predicate in this batch. IcyDB adoption and native macOS
qualification remain separately scoped. No package versions, refs, tags or
release artifacts are changed by the extraction.

The Host generated-infrastructure input-stability case and all six selected
internal artifact cases pass with registry IC Build 0.1.1 and Host 0.15.1.
Nine direct published-API cases pass for transactional reuse/retention, pruning,
changed-input and malformed-output refusals, nested cache exclusion, real Cargo
feature grouping and structured complete/truncated failure evidence. Duplicate
and empty package names refuse before command-factory effects; an empty overall
list remains an effect-free plan. The real Rust 1.99.0 Wasm fixture passes cold
build, exact reuse, source invalidation and custody of retained old bytes. These
fixtures do not qualify Canic canister deployment. The published artifact engine
is unchanged from 0.1.0; this patch changes grouping admission and diagnostics.

Current evidence lives in `.tmp/ic-build-011-adoption-20261012/`. The manifest
and lock stayed fixed through passing checks at SHA-256
`f291ddc878ced81eac46ddaa406c48409f266a650b3d22fdf2724975282eb343`
and `d1dd5b874b813ef4ce1c9dd17c0a7735b6db6b14b7cca74470860c78c8e10791`.
Dependency inheritance/pinning and scoped diff checks pass. A final optional
cache timing check was terminated while queued behind another build, before
compilation or test execution; it is not counted as passing evidence.

Earlier 0.1.0 evidence and failures remain in `.tmp/ic-build-audit-20261012/`.
The previous Timers 0.21 compilation blocker no longer appears on current source;
the passing 0.1.1 artifact cases supersede that blocked consumer qualification
without relabeling its old inputs. The completed adoption does not qualify the
whole unfinished Auth/Backup/Blob integration batch as ready to push.

Published and consumer-verified grouping findings
[IC Build #3](https://github.com/dragginzgame/ic-build/issues/3) and
[#4](https://github.com/dragginzgame/ic-build/issues/4), plus publication
documentation [#2](https://github.com/dragginzgame/ic-build/issues/2), are closed.
Native macOS qualification and production complete-release convergence remain
with [#1](https://github.com/dragginzgame/ic-build/issues/1); its published-source
[CI run](https://github.com/dragginzgame/ic-build/actions/runs/38288344214) was
queued when inspected. Complete-release admission remains Canic-owned. No broad
gate, PocketIC, commit, own package-version change or release ran.
