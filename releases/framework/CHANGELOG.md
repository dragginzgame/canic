# Canic framework changelog

Covers the framework, macros, contracts, control plane and private
framework qualification packages in [the group policy](../../ci/cargo-release-groups.json).
Host tooling has its own [ledger](../host/CHANGELOG.md). Independent releases
continue after coordinated 0.114.0; earlier releases remain
in [shared history](../history.md). Adapter changes have their own
[IcyDB](../icydb/CHANGELOG.md) and [Blob](../blob-service/CHANGELOG.md) ledgers.

## [0.115.0]

### ⚠️ Breaking

- Require `canic/build-support` on build-script dependencies. Core native source
  rendering and its proc-macro2/quote dependencies are explicitly selected;
  runtime configuration installation and native parsing remain available.
  Move Wire Principal schema support from the workspace catalog to `wire-schema`.

- Select canonical Types directly in passive contracts, with empty defaults.
  External Nat conversions and Nat-bearing status/refill DTOs now require the
  `external-candid` feature; runtime and Host consumers opt in explicitly.
  Preserve the DTO external Principal alias and existing Wire layouts. The
  published Types 0.18.1 Serde correction is required and qualified (#731).

- Adopt Timers 0.40.3, Metrics 0.29.1 and Metrics Storage 0.68.0 together,
  with Platform 0.22.0 and Logging 0.24.6. Public measurement types now use
  Metrics' instrumentation-only contract; Canic retains report DTOs and projection.
  Rebuild consumers against the matching family. Preserve Runtime 0.19 and
  its single execution guard, retained registrations and domain recovery fences.

- Extend the current v1 async recovery record with fixture attempt custody and
  expose typed interruption evidence. Rebuild consumers and clean reinstall at
  the release boundary; same-release retries retain their installed assignment.

- Return a typed Root funding acceptance outcome, including zero-transfer relieved
  demand, and retain a distinct relieved-demand reason in funding receipts.
  Temporary availability failures retain the exact reserved operation for retry.
  Rebuild both ends of the current v1 Wire contract and clean
  reinstall at the release boundary. Expected refusals retain automatic funding
  observations; authority and invariant rejection remain terminal.

- Add `max_sessions` to portable receiver configuration and the native receiver
  policy. Admission, restoration and retention use one proportional capacity
  profile; undersized restoration refuses without eviction. TOML defaults to
  2,048, the selected Auth 0.33 engine ceiling. The requested higher default
  requires a supplying Auth release and is not enabled by this change.
  A prepared 100,000-default adoption passes actual-adapter native capacity,
  restoration, retry and logout checks against the isolated supplier package.

- Retain autonomous allocation and retirement demand in the current v1 Root
  checkpoint and Coordinator removal reservation.
  Rebuild and clean reinstall at the release boundary; same-release Root upgrades
  reconstruct the native continuation without caller replay.

- Adopt Testkit 0.85.1 for public testing helpers and installed fixtures. Import
  PocketIC clients, DTOs and typed errors from `ic_testkit::pocket_ic`; remove
  Platform's `pocket-ic` feature and the separate client package. Consumers of
  these native testing types must use the matching Testkit identity.

- Extend the current v1 issuer checkpoint with exact account/login issuance
  intents, original credentials/deadlines, operation/login epochs, bounded retention,
  monotonic Root evidence and compact exact-result registration receipts. Issuer
  requests and lookups carry their original operation epoch; renewal also carries
  the login's original epoch. Clean reinstall is required; no predecessor-state
  decoding or migration is added.
- Extend the current Root enrollment checkpoint with retained registration
  intents, challenges and original approvals. Derive completed results from the
  exact current authority instead of retaining a third grant copy. This remains
  the current v1 hard cut and requires clean reinstall; no predecessor-state
  migration is supplied.
- Use Storage 0.66 as the sole allocator, backing and collection owner. Host
  composition supplies namespace grants; allocation reports expose directory
  counts, opaque u64 IDs and capacities. Remove numeric pools, bucket settings
  and the separate ledger query. Consumers must rebuild and clean reinstall;
  same-release interruption recovery retains its current authority.
- Stable codecs and collections now return typed failures, including Wire budget
  refusal. Align Platform 0.22, Timers 0.40, Metrics 0.29, Metrics Storage 0.68,
  Auth 0.33, Wire 0.16 and Testkit 0.85 identities. Rebuild consumers against
  these public identities with Rust 1.99.1 or later.

### 🔧 Changed

- Keep non-root bootstrap retries on one native registration with the existing
  bounded retry window. Report Store GC callback failures truthfully while Root
  retains exact-operation retry authority.
- Treat an unchanged Root provisioning receipt awaiting publication as retryable
  availability during Coordinator directory confirmation. Preserve its original
  intent and reject changed receipt authority instead of suspending valid progress.
- Adopt Platform 0.22.0 and Testkit 0.85.1, preserving one Runtime and Timers
  identity per deployed graph. Verify the current Shared Tooling 0.29.4 snapshot.

- Isolate IC Auth's delivered backend conformance helper from Cargo workspace
  membership while retaining Core's stable-storage integration test. Keep the
  helper's upstream version and source intact, outside all Canic release groups.

- Qualify timer recovery, snapshot suspension/resume, lifecycle reconstruction
  and both installed private consumers on Timers 0.40.3. Align both private
  lockfiles with Types 0.18.1. Clarify that Timers owns callback execution while
  Canic retains business demand, paid-operation custody and authority fences.

- Compile Auth's random-challenge operation only when receiver authorization,
  portable issuance or Root delegation is selected. Default-feature native tests
  no longer retain an unused IC management operation; enabled callers keep the
  same randomness and validation path.

- Bound unfinished fixture callbacks to three attempts under the installed
  assignment and last committed position. Commit custody before the native worker
  runs, preserve callback rollback, and expose typed interruption evidence.
  Source outages preserve the callback budget; authenticated application retry
  retains the checkpoint. Installed failure, same-release upgrade and recovery
  checks cover completion without duplicate chunk effects.

- Qualify installed Root funding after external credit relieves retained demand:
  accept no grant, replay the exact no-grant receipt, keep one observation timer,
  and fund renewed demand once. Temporary availability retains the original
  reserved operation for bounded-backoff retry instead of discarding uncertain
  acceptance custody. Grant replay does not repeat the transfer.

- Admit the shared expiry-cleanup schedule before allocating local reservation IDs
  or creating cost-permit quota/cycle reservations. Scheduling refusal leaves those
  reservations unchanged; both cost intents use one validated deadline. TTL-free
  work needs no timer admission, and a retained cleanup owner can still schedule
  at full registry capacity. Installed refusal/retry and native deadline/suspension
  checks qualify the boundary.

- Refresh Shared Tooling to 0.29.4 at `628ccc75`, including canonical validation
  helpers and updated dependency ownership rules. Preserve the current compiler,
  package versions and product-specific release selection.
- Qualify installed pending-issuance expiry: retry after the original proof
  deadline refuses without exposing a credential or replacing retained intent.
- Qualify the delivered Auth browser client's signed HTTP admission/logout on two
  receivers and two devices, with authenticated replies and device-isolated logout.
  Add an explicit npm-backed test command and an NNS-backed Fleet fixture.
  Drop accepted ingress responses and qualify fresh-process retries: original
  session records/expiry and the other device's sessions remain unchanged.
  Qualify real Chromium IndexedDB logout across page reopening: the upstream
  journal retains its deadline/signing refusal, resumes cleanup after lost replies,
  and retains completion while the other device stays admitted.
  Extend that case through account-signed issuer HTTP: after discarded completed
  replies and page reopening, reconcile the original credential once per device
  and admit both receivers without reissuing or extending the deadline.
  Qualify real HTTP caller isolation: browser, unrelated and anonymous callers
  cannot claim the account credential; its owner still retrieves the exact result.
- Qualify Auth against a real sealed Root snapshot load: changed history blocks
  resume, registration and fresh issuance without exposing a reconciled credential.
  Extend acceptance through actual issuer retirement: restoring an older sealed
  Root or an issuer heap cannot regain issuance authority while Root retirement
  or its snapshot seal remains authoritative.
  Restrict the Coordinator funding-availability policy to its owning feature so
  Root-only Wasm builds retain warning-denied compilation.

- Reuse the retained nonce when retrying interrupted portable issuance instead
  of requesting and discarding fresh randomness. Keep the original deadlines
  and recheck live local authority, fresh Root authority and application policy.

- Qualify installed issuer permission rechecks after randomness and Root calls.
  Revoked or changed application policy retains the exact intent, exposes no
  credential to reconciliation and rejects browser retargeting.

- Schedule Store activation/upgrade and standalone-local bootstrap before the
  optional asynchronous user initialization block. Application setup and phase
  hooks still follow that block. Installed local acceptance covers a held real
  HTTP await, delayed completion and an upgrade callback trap without blocking
  framework readiness. Generated Store acceptance also covers held activation,
  permanently pending and trapping upgrade hooks, and upgrade without an optional
  hook, using its actual lifecycle and a controller-only readiness probe.

- Keep portable issuer, receiver and Root registration helpers behind their owning
  Auth features. Qualify standalone issuer/receiver builds and compare retained
  challenge/credential payloads on exact preparation and issuance replay.

- Qualify Canic's stable Auth adapter with the delivered upstream backend contract:
  admission retry, logout/reopen, generation and scope refusal, and maximum record
  shapes use the actual adapter. Keep the supplier package development-only.

- Add Root-authorized portable-session inspection with bounded caller-key pages.
  Extend the protected operator relay with sessions and policy observations;
  distinguish configured policy from disabled, unavailable, unqualified-clock,
  waiting-for-enrollment and currently ready receiver state.
  Report issuer/subject identity, scopes, original timestamps and generation;
  mark expired or stale records without exposing credentials or replay evidence.

- Replace Coordinator admission, applied funding rotation and requested Root removal
  deferrals with one retained native transition owner and one shared Coordinator
  watchdog. Staged funding and unrequested removal reservations remain idle.
  Root-removal acceptance journals execution demand before remote effects, so
  unavailable Roots and lost replies retain one exact recoverable operation.
  Snapshot preparation releases idle Coordinator declarations while refusing pending
  demand and retaining the native running-callback and unmanaged-claim fences.
  Restore Coordinator timer owners from accepted journals after same-release upgrade,
  restoring authority snapshot suspension before scheduling any work. A sealed
  authority retains its exact history-bound resume refusal after an upgrade.
  Scope Root/Store test fixtures to their owning features so the isolated
  Coordinator test graph passes warning-denied lint.
  Bound Coordinator transitions and ordinary/peer allocation, Component and subtree
  removal to 16 attempts without observed progress, with retained capped backoff
  and review-required diagnostics. Charge before dispatch; replay, interruption and
  upgrade preserve spending. Controller retry status and exact-generation resume
  renew a suspended window without changing paid-effect journals or authority.
  Root admission and Registry synchronization share one native continuation and
  the existing Root watchdog, including same-release reconstruction and terminal
  declaration removal. Registration failures return typed errors.
  Return exact ordinary/peer allocation replay from retained caller and authority
  evidence without requiring Store availability. Exhausted pending allocation and
  retirement still refuse snapshot sealing after their timer stops. Treat an exact
  unchanged Root publication before runtime activation starts as waiting, preserving
  the Coordinator intent without manufacturing a review-required conflict.

- Deduplicate Coordinator provisioning through one retained native timer owner.
  Preserve durable retry deadlines and review suspension; Coordinator watchdog recovery
  rechecks pending journals before rearming abandoned work.

- Replace standalone Component and subtree removal chains with one retained
  native retirement owner. Preserve exact journals and capacity, reconstruct
  requested work after restart, and leave nested retirement with its parent owner.
  Exact terminal Component-removal retries and status reads retain their receipt
  after live membership disappears; replay no longer requires Store availability.
- Give ordinary and peer allocation one native continuation owner with durable
  demand, fair operation selection, exact replay, upgrade reconstruction and shared
  watchdog recovery. Keep aggregate Component-group provisioning under its own owner.
- Replace anonymous child-allocation timer chains with one retained native owner.
  Advance eligible children fairly, preserve persisted retry deadlines and budgets,
  and recover abandoned registrations through Root's existing watchdog. Scheduling
  refusals propagate as typed errors while durable demand remains recoverable.

- Extend actual receiver storage qualification at the 2,048-session limit:
  restored authorization and exact retry remain usable, durable logout preserves
  replay denial and other devices, and a new caller can occupy the freed slot.

- Update managed application and component-group qualification helpers for
  Testkit 0.84 typed PocketIC errors. Preserve transport
  evidence through installation, settled calls and progress polling. Refresh the
  embedded allocation peer and its source/tool provenance for the selected family.

- Prepare Timers 0.39 with Runtime/Guard 0.19, Platform 0.20, Metrics 0.28,
  Metrics Storage 0.67, Storage 0.66 and Wire 0.16 across the root and both private
  consumers. Preserve domain recovery demand, retained registration custody,
  lifecycle reconstruction and authority snapshot fences. Qualify the matching
  Auth 0.33 family with installed timer contracts, Root snapshot/rearm and both
  private consumer installation/recovery cases.
- Correct the application registration example to supply `TimerStart` with its
  initial schedule, and reject missing or duplicate Timers providers during
  dependency adoption admission.
- Admit the reviewed scalar digest/hash cohorts used by self-authenticating
  principal identities even on zero-auth roles. Keep exact package identity per
  cohort and refuse every signature stack on those roles.
- Add explicitly selected trusted-local issuer preparation, registration and account issuance.
  Generate Ed25519 keys inside the canister, retain only bounded public retry
  history, authenticate the installed Root and clear the signer before restoration
  hooks. Preserve original registration work and reject stale callbacks or reused
  keys. Require application account permission independently of Root's grant ceiling,
  capture the actual IC subject and refresh Root before signing canonical Auth claims.
  Retain original login/credential deadlines across retries, expiry and key loss;
  lookup-only reconciliation returns original material or unresolved. Reuse Auth Wire
  credentials and shared grant/Root-history checks. Remove the deterministic issuer
  fixture and host-built credential path. Receiver roles cannot select this signing
  feature. Reclaim elapsed credential history without recreating forgotten operations
  or renewing live login windows. Close epochs atomically before deletion, retain
  original browser/login records independently and keep lookup read-only. Root-only
  explicit maintenance and new admissions share the same bounded cleanup owner.
  Qualify full key/login/issuance count boundaries through genuine installed APIs
  and existing protected instruction metrics, including fresh last-slot signing,
  retained-result lookup and renewal under the original login window after cleanup.
  Reserve readable completed-result bytes for every pending issuance before
  accepting intent and again before signing. Retain exact Root registration
  commitments without duplicating grants, and drop the signer if authenticated
  Root progress cannot be stored. Add maximum-grant and journal-byte refusal,
  recovery and installed cost qualification on the selected Auth 0.33 family.
  Give the Root journal a separate bounded decoder budget and prove it readable
  before writes; keep external frame budgets unchanged.
  Production remains closed while clock/restore qualification, full-state cost,
  browser delivery and other acceptance are unfinished.
- Consolidate the private Canic harness and integration suites into `canic-tests`.
  Shared fixture support stays in its library; integration targets, maintenance
  examples and the opt-in governed catalogue retain separate selections.
- Add guarded Root-side portable issuer registration: explicit ordinary-subnet
  custody approval, domain-bound Ed25519 key possession, exact retained intent
  before randomness, callback supersession, original deadlines, lookup-only
  recovery and terminal key revocation. Keep records public-only and share the
  existing bounded randomness boundary with receiver boot. Controller-only
  preparation/retirement and approved-issuer-only completion/read-only lookup use
  bounded Wire contracts and one shared signing format. Remove the whole-registry
  bootstrap surface; the installed fixture uses issuer-to-Root possession.
  Production issuer key custody, browser delivery and clock acceptance remain unfinished.
- Render Auth's enrollment and receiver timing values through the existing
  grouped-integer formatter so generated consumer code passes strict lint.
- Integrate the persistent Auth receiver and bounded Root enrollment components
  into Core's existing owners. Retain independent proof/logout budgets, clock
  high-water marks and terminal key retirement; restoration closes enrollment
  readiness until a fresh receiver-bound Root exchange. Export the guarded Root
  snapshot update and Auth Wire 0.32 admission/read/logout endpoints. An explicit
  controlled-local receiver policy composes activation, native timer refresh,
  durable attempt recovery and bounded reclamation. The application guard uses
  the complete current session engine; queries avoid stable writes and logout
  remains independent of enrollment freshness. Preserve first-activation reset
  floors across same-release restoration. Production clock qualification,
  issuer custody and browser delivery remain unfinished. Qualify
  controlled-local direct Root pulls, two receivers/devices, scoped guard cost,
  exact retries, restoration and durable logout in PocketIC
  ([#491 — Complete portable Auth production integration](https://github.com/dragginzgame/canic/issues/491)).
- Use Auth's complete session/replay/login pair and physical count validation
  instead of duplicating those rules. Preserve Canic's replay uniqueness,
  byte/index budgets, generation rechecks and atomic stable writes; stale pairs
  retain Auth's typed `StaleAuthority` refusal
  ([#720 — Reuse Auth pair and count validation](https://github.com/dragginzgame/canic/issues/720)).
- Remove generic intent cleanup's Auth state/deadline path, which could fence the
  retained receiver. Keep bounded Auth reclamation and its metrics with that
  owner; qualify authorization and fresh admission across unrelated intent
  cleanup success/failure. Replace the synthetic denial measurement adapter with
  actual caller-aware SDK guard measurements. Remove duplicate session scope
  construction, unused error variants and the test-only token verifier facade;
  keep current Root proof installation/configuration under explicit private
  owners. Remove `CanonicalApplicationScopes::for_session`,
  `ApplicationScopeError::AggregateTooLarge` and the unused `Disabled` denial
  without aliases; session requests use Auth's checked constructors. Drop the
  issuer fixture's obsolete internal-test feature selection.
- Check the selected Wasm graphs for one Storage allocator and one Platform
  Runtime/Guard owner. Correct the graph gate's stale Memory requirement so it
  admits the maintained Storage-only family and still refuses duplicate owners.
- Adopt Auth/protocol 0.32, Metrics 0.27.2, Storage 0.65.4, Timers
  0.38.1, Wire 0.15.1 and Testkit 0.80.2. Convert grant scopes through
  Auth's checked ScopeLabel constructor, preserving scope order and signed
  bytes; malformed scopes retain typed refusal. Update portable-session fixtures
  to construct validated labels.
- Remove duplicated internal child-funding, cycle top-up and metric-row DTOs and
  their bidirectional transport converters. Storage/metric ops project directly
  into the existing canonical native Wire types; direct reads, replicated
  observation hops, Root relays and Host/CLI consumers share those types. Keep
  the existing Wire schemas, method identities and stored records
  ([#613](https://github.com/dragginzgame/canic/issues/613),
  [#707](https://github.com/dragginzgame/canic/issues/707)).
- Replace exact-total log pagination with bounded native cursors, at most 1024
  scanned candidates, 1000 rows and 1 MiB of copied text per read. Verify the actual
  Wire reply budget, preserve continuation after empty filtered pages and refuse
  oversized rows with typed errors. Query logging retains operator output while
  skipping retained-log mutation and retention scheduling. Keep native Timers as
  the sole deadline/recovery owner ([#100](https://github.com/dragginzgame/canic/issues/100),
  [#101](https://github.com/dragginzgame/canic/issues/101)).
- Hard-cut configured retirement drains to the shared owned Wire request/reply.
  Replace Candid paths/hashes in desired state and retained transfer actions with
  bounded, admitted Wire schemas; bind the exact schema to review and execution.
  Keep original operation authority, typed refusal and two-sided cycle
  conservation before stop/delete ([#613](https://github.com/dragginzgame/canic/issues/613)).
- Store the managed checkpoint fixture's business state through bounded native
  Wire in one typed stable cell. Admit the current record identity and byte/
  traversal limits while preserving atomic counter/freeze custody and same-release
  interruption recovery ([#381](https://github.com/dragginzgame/canic/issues/381)).
- Encode compiled payload-contract metadata with bounded native Wire and its
  current identity. Update declaration producers and the actual fixture reader
  together, retaining the external Candid carrier and schema/method admission
  ([#713](https://github.com/dragginzgame/canic/issues/713)).
- Remove the unused private Auth scaffold and its refusal macro, workspace/catalog
  entries and release-group edges. Portable Auth production integration remains
  with existing Canic owners ([#491](https://github.com/dragginzgame/canic/issues/491)).
- Require Rust 1.99.1 as the minimum, development and release compiler. Align
  root/private manifests, CI and generated App/Fleet manifests; admit actual
  rustc/Cargo identities and reject conflicting compiler overrides before
  qualification. Keep the existing feature and native-host obligations.
  Configure a locked Wasm check of the facade, Core and Control Plane at that
  supported minimum ([#601](https://github.com/dragginzgame/canic/issues/601),
  [#342](https://github.com/dragginzgame/canic/issues/342)).

- Establish independent framework, Host, IcyDB and Blob service release groups
  in one workspace and lockfile. Host and CLI share their own release identity;
  exact framework admission follows their linked Core version. Adapters own
  their release versions, changelogs and selected publication; retain existing
  numeric identities and explicit cross-group dependency requirements.
  Normalize allocation-peer copies from Cargo's actual member versions and
  literal manifests, preserving selected external dependencies across groups.

- Adopt reviewed Shared Tooling 0.28.0 at
  `464e32ec30b9fcad2fb89432d1bfb35df79de044`. Adopt the current compiler support
  policy, canonical owned cache maintenance and its required Perl module.
  Native tool identity tracks relevant
  build packages, admitted tools discard successful compiler scratch, and
  current consumer requirements protect selected generations. Keep Canic's
  release artifacts and governed recovery evidence under their existing owners;
  disposable adoption/cleanup/release fixtures default to the root build tree.
  Include source-bound release-group validation-plan support and its complete
  linked validation-runner guide. Route selected releases through a Canic-owned
  exhaustive roster and conservative group mapping, keeping workspace CI and
  complete lint/check/tests. Shared/uncertain inputs and unqualified mappings
  run every stage; eligible confined releases require prior full same-source
  qualification before omitting unrelated Blob fixtures or Control Plane feature
  checks. Bind qualification to the exact Cargo graph and carry it only through
  the canonical version transaction; feature/dependency/shared changes require
  fresh qualification. Bind plan/policy, source/recipe checkpoints, native host/compiler
  configuration and successful execution logs to each group receipt and exact
  recovery; failed, missing or changed evidence refuses preparation. Preserve
  the separate fast-patch lane and unrelated groups' versions and ledgers.
- Adopt Logging 0.24.3 with the coordinated current runtime family on Rust
  1.99.1. Use the published Metrics Storage adapter and preserve typed provider
  corruption refusal, last-good observation times and read-only diagnostics.
  Host dependency adoption and Blob composition remain in their own ledgers.
- Require exactly one Platform runtime and executor guard in each normal Wasm
  consumer graph, alongside the sole Storage owner. Pure protocol facades
  can coexist without introducing another executor; missing or conflicting
  runtime identities stop admission before compilation.
- Run private Blob qualification from a reusable source workspace under the
  root build directory, keeping independent consumer manifests and locks in
  source. Native fixture caches use the same cleanup boundary. Explicit root
  cleanup removes generated fixture packages, compiler caches and release
  artifacts while preserving operational journals and publication evidence.
- Add repeatable ICRC qualification on published Platform 0.16.0 for default,
  feature-only, discovery and consent managed-App profiles. Inspect actual Wasm
  exports, preserve the public query/update modes and exact upstream contracts,
  and activate through the ordinary framework owner. Anonymous consent preserves
  application argument/language/display decisions and typed errors without
  executing or authorizing the described operation. Reject malformed/oversized
  envelopes and out-of-bound header nesting; retain direct handler selection
  across same-release restoration. Keep the small optional adapter in Canic
  ([#706](https://github.com/dragginzgame/canic/issues/706)).
- Remove Canic's generic retained-claim creation/custody rollback transaction.
  Intent cleanup, log retention, sampling, renewal, funding, acknowledgement and
  application startup use native `reconcile_once`; IC Timers publishes new
  retained custody only after successful initial reconciliation. Preserve native
  earlier-deadline requests and exact business-attempt recovery. Collapse the
  identical Root/non-root initializers into the shared lifecycle initializer.
  Canic retains exact handle access for snapshot suspension, lifecycle deferral
  and domain demand; scheduling and callback authority remain upstream.
- Select Auth/protocol 0.30 in all three locks. Application session admission,
  replay, scopes and refusal classification use the upstream credential and
  session engines. Adapt SEC1/signature fixtures to Platform's current API,
  retaining the existing vectors and high-S refusal controls.

### 🩹 Fixed

- Preserve the original Component commit receipt while initial-child bootstrap
  advances the live directory. Treat validated unfinished initial descendants as
  pending work, keeping automatic allocation retries active; invalid bootstrap
  authority still requires review.
  Keep the cycles-conversion error re-export behind `external-candid`, matching
  its definition so the retirement ledger fixture builds without that feature.

- Identify the failing stage and forwarded public diagnostic when Component
  membership readiness, caller publication or synchronization fails. Clarify
  that transient timer owners remove stopped declarations while retained owners
  keep idle declarations for later demand.

- Bound each existing Store GC callback to sixteen fixture cleanup transactions,
  retaining exact per-entry accounting, operation identity and Root-owned
  continuation. Maximum-size chunk qualification covers interrupted snapshot
  recovery and terminal replay; no second cleanup timer owner is introduced.

- Refuse private Timers dispatch during shared ingress inspection before copying
  its payload. Retain Timers' independent self-caller authentication for
  inter-canister calls. Include `register_watchdog` in registration custody
  checks, including aliases and macro-generated calls.

- Delegate retained login replacement rules to IC Auth. Expired reservations can
  become logout tombstones without prior cleanup; Canic transfers physical pool
  occupancy atomically while preserving replay evidence. Keep byte accounting
  exact when the protected clock grows, including after refused operations. Route
  enrollment refresh, receiver startup and session execution through one storage
  clock owner; rollback closes access without rewriting retained state.

- Restore application-session storage after Canic installs the role and managed
  identity, before Auth authority reconciliation and receiver activation. Avoid
  reading role-specific receiver configuration during general storage startup,
  which caused Auth-enabled managed canisters to fail initialization with E19.

- Compile receiver capacity profiles for native validation, tests and enabled
  application authorization only. Keep the shared default available to ordinary
  Wasm builds, which now pass warning-denied compilation without unused profiles.

- Preserve the child-allocation retry budget across changing diagnostics and
  same-release reconstruction. Only actual work progress or completion clears
  failures; alternating transient errors cannot renew the automatic retry budget.
- Keep exact repeated Root-failure observations effect-free at the Coordinator.
  Preserve the original diagnostic and observation time without another journal
  write or repeated warning; fresh local failed attempts and changed remote
  evidence still record.

- Treat identity-exact unchanged Root provisioning/activation observations as
  pending work. Retain the issued Coordinator intent without a new failure or
  journal write, and delay unchanged polls while allowing genuine progress to
  continue immediately ([#158](https://github.com/dragginzgame/canic/issues/158)).
- Honor retained Root retry and estate funding deadlines before Coordinator
  dispatch, including reconstructed callbacks. Expired deadlines do not turn
  unchanged progress into immediate polling; failure retention must succeed
  before the callback schedules another retry.
- Classify exact Root authority/codec refusals and invalid Store authority for
  review, including forwarded diagnostics. Preserve the originating failure and
  issued work; generic activation state diagnostics, temporary unavailability
  and capacity shortages retain backoff.
- Replace Root aggregate provisioning's anonymous timer chain with one named
  native registration. Replay requests its successor, terminal replay clears
  obsolete demand, and the existing Root watchdog recovers missing or abandoned
  work without replacing live callbacks. Registration capacity is a typed refusal.
- Retain one native Root-removal timer across exact replay and reconstruct it
  from the removal journal after same-release upgrades. Keep draining pool
  maintenance stopped and recover abandoned callbacks through Root's existing
  watchdog; paid-effect journals remain authoritative.
- Classify Twiggy named data segments, names subsections and custom sections as
  non-code in Wasm capability reports. Classification revision 3 retains exact
  artifact accounting and distinguishes named functions from producer metadata
  ([#724](https://github.com/dragginzgame/canic/issues/724)).
- Admit Platform's internal certification/BLS ownership and the current ECDSA
  digest/field API cohorts in canonical Wasm graphs. Keep one full identity per
  reviewed cohort, reject unknown or duplicate cohorts and extra signature
  providers, and preserve zero-auth roles without a signature stack.
- Scope native entrypoint symbols to their consuming Cargo crate, allowing
  applications to reuse actual canister DTO libraries in one host process.
  Preserve exact Wasm names, codecs, guards and executor contexts. The paired
  Platform exporter fix remains required for Platform-owned entrypoints
  ([Canic #721](https://github.com/dragginzgame/canic/issues/721),
  [Platform #129](https://github.com/dragginzgame/ic-platform/issues/129)).
  Native metrics fixtures use the scoped symbols for enabled, disabled and
  unmounted endpoints under both authorisation policies.

- Permit explicit ICRC selection in governed managed-package admission while
  preserving the exact declared role feature closure
  ([#706](https://github.com/dragginzgame/canic/issues/706)).
- Report cascade request encoding as `CODEC_FAILED` before transport and malformed
  replies as `CODEC_INVALID` after successful transport. Preserve decoded remote
  application errors and the original unbounded wait.
- Qualify the real stateless call adapter with owned Wire bytes, explicit waits,
  local liquidity refusal, reverse-order callback refunds, malformed replies,
  callee rejection/trap, caller callback trap/cleanup, dropped futures and actual
  bounded `SYS_UNKNOWN` after a committed callee effect. Metrics count polled
  attempts and transport results; they never supply settlement or replay authority.
  Preserve raw rejection codes internally without expanding public error envelopes.
  Exercise production management, external Ledger codecs and cascade clients in
  PocketIC; keep domain journals and optional Relay adoption with their owners
  ([#700](https://github.com/dragginzgame/canic/issues/700),
  [#368](https://github.com/dragginzgame/canic/issues/368)).
- Report retained intent cleanup as enabled and idle when commit/rollback
  removes the last finite expiry. A later finite intent rearms the same owner;
  TTL-free work creates no cleanup deadline. Current deadlines remain separate
  from historical last-arm measurements, and explicit cancellation/suspension
  retains its disabled meaning
  ([#545](https://github.com/dragginzgame/canic/issues/545)).
- Rearm abandoned cycle top-up, placement acknowledgement, issuer-renewal and
  pool-maintenance registrations through their native callbacks. The callback
  resumes its durable business attempt and returns the actual retry, deadline
  or stop decision to Timers. An expired lease does not displace a scheduled or
  running callback; explicit cancellation and invariant/control failures remain
  separate from recovery demand
  ([#398](https://github.com/dragginzgame/canic/issues/398)).
- Observe IC history before suspending Root producers. Recheck quiescence and
  suspend both Core and Control Plane owners in the same callback that commits
  the snapshot seal. Reconstruct both owner sets before opening mutation on
  resume; exact completed resume replay returns its stored receipt without
  observing history or rebuilding timers
  ([#176](https://github.com/dragginzgame/canic/issues/176)).
- Extend framework-owned PocketIC fixtures with actual pre/post-await top-up
  traps, a live held IC reply beyond lease expiry, overlapping snapshot commands,
  watchdog trap/cancel/rearm and composed lifecycle admission. Inspect
  the composed Wasm's lifecycle and provider exports; reject controller and
  outsider calls to private dispatch. Keep actual database startup qualification
  in the downstream owner ([#590](https://github.com/dragginzgame/canic/issues/590)).
- Match the workflow's physical workspace when recovery fixtures call the
  durable owner directly. Symlinked temporary ancestors now exercise interrupted
  grant publication and exact replay without weakening production path custody
  ([#450](https://github.com/dragginzgame/canic/issues/450)).
- Remove the hard two-hour PocketIC server lifetime from command-owned test
  execution. Keep the two-hour idle timeout, owned-process cleanup, cancellation
  handling and complete failure evidence
  ([#474](https://github.com/dragginzgame/canic/issues/474)).
- Regenerate the embedded allocation peer and its structured provenance for the
  current locked runtime dependencies, then independently verify the result
  ([#551](https://github.com/dragginzgame/canic/issues/551)).
