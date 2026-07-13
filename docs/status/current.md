# Current Status

Last updated: 2026-07-13

## Purpose

This is the compact handoff for new agent sessions. Read this first, then
inspect only the files needed for the current task. Detailed historical status
before this compaction is archived at
`docs/status/archive/2026-06-30-precompact.md`.

## Current Line

- The `0.83.x` technical-debt audit closed at `0.83.28`. Design:
  `docs/design/0.83-technical-debt/0.83-design.md`. Canonical result:
  `docs/audits/0.83-technical-debt/ledger.md`, which is `pass` with all 37
  findings fixed and no deferred findings. `0.83.29` is a post-audit state,
  build-cache, and module-hygiene hardening release rather than a reopened
  ledger slice.

- The current package/release-surface version is `0.88.0`, published and tagged
  as `v0.88.0`. The bounded 0.86 structural-maintainability line is complete;
  its design and tracker live under
  `docs/design/0.86-structural-maintainability/`. The published first Medic
  slice owns auth-renewal and blob-storage diagnostics in focused modules. The
  second Medic slice owns role-package, required runtime-feature, resolved
  role-contract, descriptor-admission, project-configuration, and state-audit
  diagnostics in focused modules. The next unreleased slice completes the
  Medic pass by moving deployment context, installed state, receipts, registry
  observation, and root readiness into one focused owner. Test and renderer
  imports now reference their actual owner modules rather than routing through
  the parent. The workspace also adopts `ic-query 0.10.2` after its focused
  cached-catalog integration passes. The complete Medic pass is published as
  `v0.86.2`.

  The next unreleased slice begins deploy-plan decomposition. `plan.rs` is
  hard-cut to the required `plan/mod.rs` directory layout, and rendering, JSON
  output persistence, and exit classification have one focused child owner.
  Command inputs, root discovery, parsing, and usage have a second focused
  owner. Report construction, diagnostic order, CLI behavior, JSON/text output,
  and exit behavior are unchanged. The workspace also adopts `ic-query 0.10.4`
  after its focused cached-catalog integration passes. Package versions remain
  `0.86.2`. This boundary slice is published as `v0.86.3`.

  The next unreleased slice moves verified context, identity, artifact,
  inventory, authority, trust-domain, and verifier-readiness evidence into one
  focused deploy-plan owner. Blocker, warning, assumption, comparison, status,
  and proposed-operation policy remain in the parent. This evidence slice is
  published as `v0.86.4`.

  The next unreleased slice moves target-resolution blockers, unsupported and
  blocking assumptions, local-state warnings, unresolved assumptions, stable
  diagnostic codes, and next-action classification into one focused owner.
  Comparison, aggregate status, proposed operations, and report assembly remain
  in the parent. This diagnostics slice is published as `v0.86.5`.

  The next unreleased slice completes deploy-plan decomposition. Proposed
  operation labels, global next actions, aggregate status, comparison status,
  and deterministic ordering move to one focused final-outcome owner. The
  serialized report types and stable label mappings move to a report-model
  owner. The parent retains orchestration, report assembly, path/profile
  helpers, and focused tests. This deploy-plan closeout is published as
  `v0.86.6`.

  The `0.86.7` slice bounds release-validation disk use. CI does not retain
  incremental compiler state, and tag runs build the release workspace without
  repeating the full checks and PocketIC jobs already running for the identical
  `main` commit. Broad local Clippy and workspace-test gates also disable
  incremental state, and a successful `make release-push` clears Cargo build
  artifacts before the next release cycle. Slice C has also started:
  state-manifest package/config and built-in `wasm_store` resolution now has one
  focused host owner while the existing facade, manifest output, blocking
  findings, and report contracts remain unchanged. This batch is published as
  `v0.86.7`.

  The current unreleased slice completes Slice C and the bounded 0.86
  structural pass. All state-manifest audit-check construction has one focused
  owner; status aggregation, next-action projection, and deterministic sorting
  have another. The parent retains the public report model, orchestration,
  facade, and focused tests. It falls from the 1,738-line Slice C baseline to
  846 lines. Finding codes, order, severity, reports, serialization, state
  contracts, and persisted bytes are unchanged. This closeout is published as
  `v0.86.8`; no further 0.86 implementation slice is required by the bounded
  design.

  The fresh 2026-07-12 health audit defines a bounded 0.87 operator-boundary
  hygiene line under `docs/design/0.87-operator-boundary-hygiene/`. Slice A is
  implemented: workspace manifest replacement is durable, project and canister
  scaffold failures use one rollback function, captured workspace/fleet bytes
  are restored exactly, and only preflight-proven new directories may be
  removed. Rollback failure is typed and retains the original operation
  failure. Slice B will hard-cut repeated ICP and installed-deployment error
  reconstruction into the existing host ICP adapter, which will become the one
  owner of external ICP diagnostic classification. Slice C audits every
  product-level `CANIC_*` environment input and removes public shortcuts that
  duplicate explicit inputs or canonical discovery. The design explicitly
  excludes a transaction framework, project-context service, global error
  framework, generic fan-out helper, and compatibility paths. Slice A is
  published as `v0.87.0`. Slice B is published in `v0.87.1`: external
  ICP CLI wording has one typed host classifier, commands retain typed ICP
  failures and their original sources, and the copied command/string transport
  errors are hard-cut. Slice C now begins with a decision ledger for every
  maintained product-level `CANIC_*` environment input. Shortcuts duplicated
  by explicit options, configuration, metadata, or project discovery are
  candidates for removal; only required process-boundary handoff values may be
  internalized rather than deleted. The first implementation hard-cuts
  `CANIC_WASM_PROFILE` in favor of typed profile inputs and deletes three
  unread child-build environment writes/cleanup paths without replacements.
  The next hard cut removes all six public workspace, ICP-root, config,
  canister-root, and manifest overrides; explicit command inputs and canonical
  discovery own selection, while only private config and ICP-root values cross
  the Cargo build boundary. Host tests no longer mutate process environment or
  carry environment locks. The final Slice C hard cut removes the public cache
  retention and Canic-specific target-directory switches, replaces ambient
  Wasm-store DID refresh with one explicit maintainer builder argument, and
  gives the embedded-release-artifact Cargo handoff one private core-owned
  name. Slice C is complete; do not extend it with a configuration service or
  compatibility paths. Slices B and C are both published in `v0.87.1`; all
  three planned 0.87 slices are complete. A post-release closeout scan found
  one install-root missing-canister-ID classifier that still erased a typed ICP
  command error. The `0.87.2` correction moves that wording into the existing
  host classifier and keeps resolution typed; package versions remain
  `0.87.1` until the human-owned bump. This is a Slice B conformance correction,
  not another slice or 0.88 carry-over.

  The fresh post-0.87 audit is recorded at
  `docs/audits/reports/2026-07/2026-07-13/codebase-health.md`. It defines the
  bounded 0.88 design under
  `docs/design/0.88-artifact-durability-and-config-errors/`: make backup
  artifact-directory finalization genuinely durable, make CLI file output
  failure-atomic, and give fleet configuration one typed error boundary. Close
  0.88 after those three slices; broad visibility churn, dependency forks,
  filesystem frameworks, and global error architectures are excluded.

  0.88 Slice A is published as `v0.88.0`. Direct snapshot
  capture and planned backup finalization now share one backup-owned directory
  commit that opens without following artifact symlinks, syncs files and
  directories bottom-up, publishes through atomic no-replace, and syncs the
  parent.
  Interruption after publication but before the journal transition recovers
  only a journal-bound `ChecksumVerified` directory after checksum
  reverification and resynchronization. Unrelated destinations fail closed,
  and failed journal persistence exposes neither `Durable` nor its completion
  metric.

  0.88 Slice B is implemented and release-noted as `0.88.1`. The host durable
  byte writer now exposes explicit replacing and create-new operations backed
  by one private staging, file-sync, atomic-publication, cleanup, and
  parent-sync engine. Shared CLI JSON and text file outputs use durable
  replacement only after serialization completes. Deployment-plan `--out`
  alone uses atomic no-clobber and still rejects missing parents. Newly created
  shared-output parent hierarchies are synchronized one level at a time.
  Scaffold output, the cycles pending log, and host/backup subsystem
  persistence remain with their existing owners. Package versions remain
  `0.88.0` until the human-owned release bump.
  The `0.84` role-aware state-contract line shipped all three accepted slices
  in `0.84.0`. Its review-revised and scope-trimmed design remains at
  `docs/design/0.84-role-aware-state-contracts/0.84-design.md`. Slice A is
  released: `canic-core::role_contract` now owns
  typed feature, capability, allocation, provenance, result, and
  finding values; one config-to-capability derivation; the feature and
  allocation catalog; canonical active ID definitions; and the pure fail-closed
  resolver. Cargo feature/default/implication parity is tested against the
  real `canic` and `canic-core` manifests. Canonical memory IDs 11-85 moved to
  the allocation authority without value or encoding changes, and storage
  owners now import them. The old `role_required_canic_features` helper is
  hard-cut; medic consumes the new typed requirements, and build-time
  capability cfgs use the same derivation, including the explicit built-in
  wasm_store path. The design supports one direct, unconditional,
  non-optional normal Canic dependency shape and rejects package-feature
  forwarding, optional/target-specific/transitive paths, multiple paths, and
  multiple Canic packages. It intentionally omits a general graph resolver,
  fingerprints, catalog digests, and schema negotiation. Replaced helpers and
  bypass paths are hard-cut without aliases or fallbacks. Feature effects are
  only `NoState` or `StateBearing`; allocation definitions contain active state
  only; surplus state-bearing features allocate normally without a warning;
  and the host validator returns only supported evidence or one unsupported
  finding rather than exposing a dependency graph to core policy.

- The `0.84.1` typed-failure-classification interruption shipped under
  `docs/design/0.84-typed-failure-classification/0.84.1-design.md`. Canic-owned
  auth expiry, registry policy, wasm-store publication, deployment-state
  assumption, install-block, blob-storage input, medic deployment-state,
  direct replica destination, and canister-signature certificate decisions now
  consume typed variants or stable codes instead of English fragments.
  `ErrorCode` adds delegated-token expiry, chain-key proof-pending, and four
  wasm-store failure codes; deployment-state assumption keys are split into
  exact missing, network-mismatch, and read-failed keys with no compatibility
  alias. A repository-wide follow-up scan also hardens exact deployment
  finding codes, observation-source labels, assumption namespaces, cycle
  statuses, replica/HTTP status parsing, Cargo workspace TOML detection,
  policy exit-class labels, backup verification kinds, and bootstrap artifact
  kinds. Raw ICP CLI stderr matching remains an isolated external diagnostic
  boundary. A final test audit deletes obsolete coverage for removed command,
  endpoint-option, config-field, and install-state shapes; current
  behavioral tests now assert typed errors, public codes, or observable state.
  Remaining text assertions cover maintained rendering/compiler diagnostics or
  isolated external-tool behavior. The developer installer also drops its
  retired npm ICP-wrapper cleanup path rather than carrying migration behavior.
  Active contracts, getting-started guides, release validation, and recurring
  audit definitions now describe only maintained auth and caller-authorization
  surfaces.
  Internal PocketIC artifact builds now validate every Canic-declared role
  package through the existing host validator before setting the private
  canonical build marker; generic non-Canic fixture stubs remain unchanged.
  Stable-memory IDs, records, encodings, migrations, and the released 0.84.0
  role contract were unchanged by 0.84.1.

- The `0.84.2` memory-map correction is published and tagged as `v0.84.2`.
  Root-named core groups are hard-cut into exact shared-runtime, auth,
  ICP-refill, pool, scaling, directory, and sharding allocations. Every
  declared role and the built-in wasm store resolves shared IDs 11-13, 15-18,
  20, 29-32, 34, and 39-42; app-registry ID 14 remains root-only; auth ID 19
  and ICP-refill ID 33 are conditional; placement IDs resolve exactly. Replay
  receipts move from ID 21 to ID 20 and ID 21 becomes unassigned. The
  lifecycle/tombstone and removed-state report surfaces are hard-cut, and state
  manifest/audit advance to schema version 2. Existing canisters with the old
  ID-20/21 ledger require destructive reinstall; there is no migration or
  fallback. The audit also restores IDs 86-99 as control-plane reserve, starts
  downstream application allocations at ID 100, and rejects definitions
  outside their owner's range. All other active IDs, records, encodings, and
  restore behavior are unchanged.

- The `0.84.3` host durability and observation-loss patch is published and
  tagged as `v0.84.3`. One private `canic-host` writer
  now owns sibling staging,
  file sync, atomic rename, and directory sync for generated Candid artifacts,
  deployment install state, deployment receipts, release-set manifests, and
  single-file fleet role mutations. Role rename also restores the original
  fleet config if the package-manifest write fails. This is the accepted
  boundary: do not add a multi-file journal, recovery subsystem, or transaction
  abstraction without concrete failure evidence.

- Metrics, cycles, and live-list fan-out convert worker panics
  into explicit per-canister errors instead of dropping those canisters.
  Successful cycle-tracker samples remain visible when the live-balance or
  top-up query fails, while the report status and error field expose the lost
  observation. Live-list cycle and Canic-version query failures render as
  `error` rather than absent data. Metrics, cycle-history, blob-storage, and
  auth response parsers return typed JSON, payload, and field failures; command
  diagnostics retain the specific malformed field instead of collapsing all
  parse failures into one message.

- The `0.84.4` environment/topology correction is published and tagged as
  `v0.84.4`. Named ICP environments resolve their declared network before Wasm
  compilation, build provenance records both selected environment and build
  network, observed missing bootstrap roles block deployment checks, and auth
  renewal reports unregistered issuers. Hard-cut recovery remains destructive
  reinstall; the patch does not seed missing state, adopt retained children,
  or restore manual proof injection.

- The `0.84.5` passive state-contract slice is published and tagged as
  `v0.84.5`. App and subnet topology indexes now have real canonical
  `AppIndexData` and `SubnetIndexData` snapshots composed of
  `IndexEntryRecord` rows. Their descriptors reference constants owned by
  those types instead of unverified string literals. The underlying stable
  B-tree keys and values, memory IDs, endpoint DTOs, and Candid are unchanged.

- The `0.84.6` passive state-contract batch is published and tagged as
  `v0.84.6`.
  Canister-pool, scaling-registry, and directory-registry exports now use real
  canonical `CanisterPoolData`, `ScalingRegistryData`, and
  `DirectoryRegistryData` snapshots composed of named entry records. Sharding
  registry, partition-assignment, and active-set state now expose real
  `ShardingRegistryData`, `ShardingAssignmentsData`, and
  `ShardingActiveSetData` snapshots; assignment tuples crossing storage, ops,
  and workflow are hard-cut to named records. Optional sharding schema types
  remain available to the unconditional descriptor registry while storage
  implementations remain feature-gated. All affected descriptors compile
  against owner-defined record and snapshot names. Stable B-tree keys and
  values, memory IDs, restore behavior, endpoint DTOs, Candid, and persisted
  bytes are unchanged.

- The `0.84.7` passive state-contract batch is published and tagged as
  `v0.84.7`. The former shared, test-only `BlobStorageData` snapshot is
  hard-cut into exact `StoredBlobsData`, `BlobDeletionPendingData`,
  `StorageGatewayPrincipalsData`, and `BlobStorageBillingStateData` allocation
  snapshots. Stable-map key/value rows are named, lifecycle ops consume the
  canonical projections, and descriptors compile against owner-defined record
  and snapshot names. Blob-storage schema types remain available to the
  unconditional descriptor registry while storage implementations remain
  feature-gated. Stable-memory IDs and encodings, record serialization,
  restore behavior, DTOs, Candid, JSON reports, and persisted bytes are
  unchanged.

- The `0.84.8` passive state-contract batch is published and tagged as
  `v0.84.8`. App registry, subnet registry, and direct-child cache storage
  now expose real canonical `AppRegistryData`, `SubnetRegistryData`, and
  `CanisterChildrenData` snapshots. Stable map rows crossing storage, ops, and
  workflow use named `AppRegistryEntryRecord` or `CanisterEntryRecord` values;
  the former `AppRegistryRecord`, `SubnetRegistryRecord`, and
  `CanisterChildrenRecord` containers are hard-cut without aliases. Their
  descriptors compile against owner-defined record and snapshot names.
  Stable-memory IDs and encodings, `CanisterRecord` serialization, restore
  behavior, DTOs, Candid, JSON reports, and persisted bytes are unchanged.

- The `0.84.9` passive state-contract batch is published and tagged as
  `v0.84.9`. Environment, application-state, and subnet-state singleton
  storage now distinguishes persisted `EnvRecord`, `AppStateRecord`, and
  `SubnetStateRecord` values from canonical `EnvData`, `AppStateData`, and
  `SubnetStateData` import/export snapshots. Storage, runtime ops, lifecycle
  consumers, access checks, and focused tests use the new snapshot boundary;
  no record aliases or compatibility paths remain. Their descriptors compile
  against owner-defined record and snapshot names. Memory IDs, singleton cell
  keys, record serialization, restore order, DTOs, Candid, JSON reports, and
  persisted bytes are unchanged.

- The `0.84.10` passive state-contract batch is published and tagged as
  `v0.84.10`. Auth singleton state now has a canonical `AuthStateData`
  snapshot around its persisted `AuthStateRecord`; replay receipts now expose
  a canonical `ReplayReceiptsData` snapshot composed of named
  `ReplayReceiptEntryRecord` rows preserving stable slot keys. The aspirational
  singular `ReplayReceiptData` name is hard-cut. Auth/replay descriptors compile
  against owner-defined names, and focused test-only round-trip helpers prove
  exact snapshot restoration without adding a production whole-state import or
  migration surface. Memory IDs, stable keys, record serialization, replay
  behavior, DTOs, Candid, JSON reports, and persisted bytes are unchanged.
  Cycle top-up history, child funding ledgers, and ICP-refill history now expose
  canonical `CycleTopupEventsData`, `CyclesFundingLedgerData`, and
  `IcpRefillRecordsData` snapshots with named rows preserving stable keys.
  Operational cycle and refill projections use those named rows instead of
  tuples. Their memory IDs, stable schemas, runtime behavior, and persisted
  bytes are unchanged.

- The `0.84.11` passive state-contract batch is published and tagged as
  `v0.84.11`. Intent metadata, records, per-resource totals, and pending indexes
  now expose exact
  `IntentMetaData`, `IntentRecordsData`, `IntentTotalsData`, and
  `IntentPendingData` snapshots. Map snapshots preserve their stable keys in
  named rows, and the test-only pending projection no longer crosses the
  storage/ops boundary as a tuple. Intent descriptors compile against names
  owned by the corresponding record and snapshot types. Memory IDs, stable
  cell/map keys, storable record encodings, runtime behavior, DTOs, Candid,
  JSON reports, and persisted bytes are unchanged.

- The `0.84.12` passive state-contract batch is published and tagged as
  `v0.84.12`. This completes the planned 0.84 implementation line. Template
  manifests and chunk-set metadata expose canonical
  `TemplateManifestsData` and `TemplateChunkSetsData` snapshots with named rows.
  Physical chunk-reference and payload memories are modeled separately as
  `TemplateChunkRefsData` and `TemplateChunkPayloadsData`, preserving stable map
  keys, vector slots, reference metadata, and payload bytes exactly.
  Control-plane subnet and local wasm-store GC cells expose
  `ControlPlaneSubnetStateData` and `WasmStoreGcStateData`. The manifest,
  chunk-set, chunk-ref, and chunk-payload state-report snapshot labels hard-cut
  their aspirational singular names to the real plural type names. No raw
  record or snapshot name literals remain in the core or control-plane
  descriptor registries. Memory IDs, stable keys, storable encodings, runtime
  behavior, DTOs, Candid, and persisted bytes are unchanged.

- The `0.84.13` feature-gating and release-safety patch is published and tagged
  as `v0.84.13`.
  `canic-control-plane::state_contract` is intentionally unconditional for
  passive host inspection, so the control-plane subnet record and snapshot
  schema are now compiled for minimal and wasm-store feature sets. The subnet
  stable cell, memory registration, transitions, ops, and workflows remain
  gated by `root-control-plane`. This fixes packaged host/CLI compilation
  without allocating root subnet state in wasm-store or minimal builds. State
  descriptors, memory IDs, persisted encodings, runtime behavior, DTOs,
  Candid, and JSON shapes are unchanged. CI and local version-bump gates now
  run the minimal, wasm-store-only, and host-consumer compile matrix, and the
  bump script refuses direct execution without a completed release gate.

- The `0.84.14` layering-correction patch is published and tagged as
  `v0.84.14`. Topology index
  storage ops now project persisted `IndexEntryRecord` values into internal
  `IndexEntryView` values before workflow use. Core index queries and
  control-plane root validation paginate or validate those projections, and
  the public control-plane support facade no longer re-exports the persisted
  record. Control-plane publication workflows likewise consume ops-owned
  publication-state and wasm-store projections rather than stable subnet
  records. The stale pool `data_to_view` response-mapper name is hard-cut to
  `data_to_response`. The layering guard scans both workflow trees and reports
  all detected violations in one run instead of stopping after the first. It
  passes with no suppression. The same patch adopts `ic-memory 0.9.0` through
  a hard-cut named macro form. All 36 key declarations and both owned ranges
  name centralized Canic core or control-plane authority constants; the
  adapter passes those constants through explicit upstream registration
  without repeating strings at allocation sites. Canic owns a direct
  `ic-stable-structures 0.7.2` dependency behind its existing CDK facade, and
  the bootstrap adapter discards the committed-allocation capability after
  successful persistence. The eight workspace Canic dependency constraints
  are corrected to `0.84.14`. Stable keys, memory IDs, persisted records and
  encodings, collection types, DTOs, Candid, JSON, and runtime behavior are
  unchanged. The patch also completes the `k256 0.14` hard cut: chain-key
  ECDSA consumes the always-returning low-S normalization API and uses the
  renamed compressed SEC1 point serializer. Signature and public-key bytes are
  unchanged. Default-target, wasm-target, and all-feature core checks, the
  complete control-plane/host feature matrix, publication bootstrap tests,
  focused chain-key normalization and verification tests, targeted Clippy, and
  repository guards pass.

- The post-0.84 codebase health audit is recorded at
  `docs/audits/reports/2026-07/2026-07-11/codebase-health.md`. It identifies
  three ordered follow-ups for the next designed line: durable replacement of
  mutating restore journals, command-local build environment propagation that
  removes both unsafe global guards, and one explicit hard cut away from the
  unmaintained CBOR implementation with stable/wire fixtures. The bounded 0.85
  design now lives at
  `docs/design/0.85-operational-safety/0.85-design.md`, with permanent progress
  tracking at `docs/design/0.85-operational-safety/status.md`. Slice A is
  complete: backup layouts, typed restore plan/journal persistence, and every
  mutating runner journal transition share one unique sibling durable replace;
  the fixed `.tmp` and truncating recovery writers are removed. Slice B is also
  complete: one explicit host-owned build context now supplies role, paths,
  profile, selected environment, resolved build network, and optional
  direct-local replica targeting to Cargo and ICP child commands. Both
  process-global environment guards, their unsafe mutation/restoration, the
  internal build override, and the local-target environment fallback are
  hard-cut. Display and provenance consume the same resolved context, and
  sequential builds cannot inherit prior config or network authority. Slice
  C is now complete: exact local replica query/status wire fixtures pass
  unchanged under a private `ciborium` adapter, and a temporary dual-codec
  differential gate proved exact stable bytes across default core,
  all-feature core, and control-plane owner suites. The core stable adapter and
  replay receipts now use `ciborium`; rich serde-shape and replay byte goldens
  remain permanent. All Canic manifests and codec call sites hard-cut
  `serde_cbor` without a fallback or migration. Current published IC signature,
  agent, transport, and PocketIC crates still select it transitively, so the
  workspace RustSec warning remains upstream. Runtime layering, memory
  allocation,
  publication boundaries,
  manifest guards, and the local vulnerability scan otherwise pass.
  These three slices shipped in `v0.85.0`. Release-set Candid argument files
  were hard-cut to piped child stdin in `v0.85.1`. Typed binary staging and
  complete artifact hash/chunk validation shipped in `v0.85.2`. Release
  artifact path containment shipped in `v0.85.3`. Canonical manifest identity
  admission shipped in `v0.85.4`. Static artifact-shape admission and
  `ic-query 0.10.0` shipped in `v0.85.5`; Canic's maintained top-level
  subnet-catalog integration required no source migration. The 0.85 line is
  complete and did not reopen the stable codec, restore, or build-authority
  contracts.

- The bounded 0.86 structural-maintainability line follows the health audit's
  deferred hub splits. It is mechanical: preserve public APIs, CLI behavior,
  diagnostics, JSON, Candid, stable state, and persisted bytes while moving
  existing private responsibilities into ordinary child modules. No framework,
  rule engine, cross-crate ownership change, wrapper, alias, or compatibility
  path is in scope. The first Medic slice extracts auth-renewal and blob-storage
  check construction while the parent retains check selection and ordering.
  It shipped as `v0.86.0`. The next slice extracts the existing role-package
  and resolved role-contract check block into one direct owner; the old parent
  definitions are removed without wrappers. Project configuration and
  state-audit check construction also move to one focused owner, leaving
  project check ordering in the parent. This slice is changelog-finalized for
  `0.86.1` and is now published. The next slice completes the Medic pass:
  deployment diagnostics move to one 676-line owner while the 268-line parent
  retains authoritative project/deployment ordering and shared ICP CLI checks.
  Test and renderer imports reference the focused owners directly rather than
  turning parent imports into a private facade. This Medic closeout is
  published as `v0.86.2`. The next slice starts the deploy-plan pass by moving
  rendering, output persistence, and exit classification into one focused
  owner and command inputs into another without changing the `deploy::plan`
  facade, command surface, or output contracts. This boundary slice is
  published as `v0.86.3`. Verified evidence construction now moves to one
  focused owner without changing report or diagnostic contracts. This evidence
  slice is published as `v0.86.4`. Blocker, warning, and assumption diagnostics
  now move to one focused owner without changing their stable codes or
  classification. This diagnostics slice is published as `v0.86.5`. Final
  comparison, status, next-action, proposed-operation, and ordering policy now
  move to one focused owner. Serialized report fields and stable labels move to
  another, completing the deploy-plan structural pass.
  This closeout is changelog-finalized for `0.86.6`.

- The current workspace dependency is now `ic-memory 0.10.0`. Canic hard-cuts
  the former commit diagnostic struct shape to 0.10's `Empty`, `Valid`, and
  `Invalid` slots plus its combined recovery result. Retirement generation is
  derived from `AllocationState::Retired`, and memory diagnostic Candid now
  reports `InvalidCommitSlots` explicitly instead of `Unknown`. Default,
  all-feature, and Wasm core checks plus focused projection tests pass.

- Slice B shipped in `0.84.0`. `canic-host::role_contract`
  resolves the exact config-declared package and validates one direct,
  unconditional, non-optional normal Canic dependency against a
  `wasm32-unknown-unknown`-filtered runtime graph. It rejects package-feature
  forwarding, optional/target-specific/transitive Canic paths, multiple paths
  or package IDs, version skew, metadata mismatch, and Cargo/catalog drift.
  Passive medic inspection uses locked offline metadata; build validation runs
  before Cargo. Canonical builds set one private marker, while direct
  authoritative wasm Cargo builds fail with `canic build` guidance. Canonical
  and generated wasm_store packages use the same validator. The former
  build-support and medic feature parsers plus blob-probe feature forwarding are
  deleted. Exported opt-in endpoint macros now gate on the Canic facade feature
  at definition time instead of requiring a mirrored caller feature.
  Medic may now emit the documented `role_contract_*` failure codes for
  unsupported package shapes; command forms and existing successful output,
  persisted state, and memory IDs are unchanged.

- Slice C shipped in `0.84.0`. `canic-core` and
  `canic-control-plane` expose allocation-keyed owner descriptors for every
  active allocation, including optional sharding and blob-storage state. The
  host validates the complete registry for missing/duplicate descriptors,
  canonical IDs and owner agreement before strictly
  joining resolved allocations into role manifests. State manifest/audit,
  medic, and release capability views now consume the same resolved contracts;
  the old state role selectors and raw release capability mapper are hard-cut.
  Runtime state summaries join observed registered memory IDs to the same owner
  descriptors instead of selecting state from a role name. Manifest and audit
  schemas remain version 1 with unchanged successful field shapes. Rejected
  contracts return no partial manifest and may emit blocking
  `role_contract_*` audit checks; medic intentionally replaces
  `role_required_canic_feature_missing` with
  `role_contract_required_feature_missing`. Memory IDs, records, encodings,
  migration behavior, and command forms are unchanged.

- The released `0.83.29` batch makes wasm-store state audits
  declare template memories 80-83 plus GC memory 85 while preserving blocking
  failures for unknown roles; removes nested Cargo execution from the
  delegation root build script; bounds and cleans artifact-only Cargo targets;
  and splits medic command, package inspection, report model, and rendering
  responsibilities without output changes.

- The `0.83.26` slice fixes `CANIC-083-DEBT-032` by
  tightening deployment-truth control-class label ownership and
  `CANIC-083-DEBT-033` by tightening external lifecycle label ownership.
  `CanisterControlClassV1` now owns the exact control-class labels used by
  canister/pool report diffs, multi-deployment inventory summaries, external
  lifecycle text, and external-upgrade verification summaries. External
  lifecycle mode, consent state, verification result, observation source,
  consent subject/channel, and verification requirement enums now own the
  exact labels used by lifecycle report text. Operator text output labels,
  diff values, command behavior, endpoint surfaces, Candid, JSON schemas,
  deployment truth schema, evidence/report schemas, and stable-state layout
  are unchanged.

- The `0.83.25` slice fixes `CANIC-083-DEBT-031` by
  tightening deployment-root verification text label ownership. Root
  verification source, evidence status, state transition, root verification
  state, and root observation source enums now own the exact labels used by
  deployment-root verification report text, receipt text, and evidence-check
  construction. Operator text output, command behavior, endpoint surfaces,
  Candid, JSON schemas, deployment truth schema, evidence/report schemas, and
  stable-state layout are unchanged.

- The `0.83.24` slice fixes `CANIC-083-DEBT-030` by
  tightening deployment-truth status label ownership. Deployment-truth safety,
  execution-preflight, execution, promotion-readiness, external-lifecycle
  plan, external-upgrade completion, and external-upgrade
  verification-requirement enums now own their stable text labels through
  `label()` methods, and deployment-truth text renderers plus medic receipt
  summaries consume those owner-defined labels.
  Operator text output labels, medic text, command behavior, endpoint surfaces,
  Candid, JSON schemas, deployment truth schema, evidence/report schemas, and
  stable-state layout are unchanged.

- The `0.83.23` slice fixes `CANIC-083-DEBT-029` by
  tightening runtime introspection enum label ownership. Runtime domain enums
  now own their canonical labels through `label()` methods, the runtime DTO
  serde-label tests compare against those owner-defined labels, and
  `canic inspect` text rendering consumes the domain-owned labels for runtime
  status, timer status, state-domain status, and recent-failure severity. CLI
  output labels, runtime JSON/Candid labels, command behavior, endpoint
  surfaces, deployment truth, evidence/report schemas, and stable-state layout
  are unchanged.

- The `0.83.22` slice fixes `CANIC-083-DEBT-028` by
  tightening state manifest and state-audit label ownership. `StateStorage` and
  `MigrationPolicy` now own their stable schema labels via `as_str()` methods,
  `StateAuditStatus` owns its stable report labels, and the state CLI text
  renderer, medic state-audit summary, and runtime state summary builder
  consume those owner-defined labels instead of duplicating local match blocks.
  State manifest JSON labels, text output, runtime state summary strings,
  command behavior, endpoint surfaces, Candid, deployment truth,
  evidence/report schemas, and stable-state layout are unchanged.

- The `0.83.21` slice fixes `CANIC-083-DEBT-027` by
  hard-cutting local runtime/config/policy and bootstrap validation metadata
  out of the Candid trait surface. `ids::BuildNetwork` no longer derives
  `CandidType` because it has no active Candid DTO or `.did` consumer after the
  delegated-auth policy metadata hard cut. `ValidationReport` and
  `ValidationIssue` no longer derive `CandidType` because they are root
  bootstrap validation metadata, not endpoint DTOs. Endpoint surfaces, Candid
  payloads, JSON, deployment truth, evidence/report schemas, and stable-state
  layout are unchanged.

- The `0.83.20` slice fixes `CANIC-083-DEBT-026` by
  hard-cutting delegated-auth verifier policy and registry snapshot metadata
  out of the Candid trait surface. `RootProofMode`, `RootKeyPolicyV1`,
  `DelegatedAuthRegistrySnapshotV1`, and
  `DelegatedAuthIssuerPolicySnapshotV1` no longer derive `CandidType`, and the
  protocol-surface test no longer pins those internal canonical-hash metadata
  shapes as Candid payloads. Active delegated token, root proof, issuer proof,
  proof install, and proof status Candid payloads are unchanged.

- The `0.83.19` slice fixes `CANIC-083-DEBT-025` by tightening
  deployment-truth authority report text-output labels. Authority report title,
  field, section, count, fallback, hard-failure, observation-gap, blocker,
  next-action, automatic-action, and external-action labels now use typed
  internal labels. Operator text output labels, command behavior, endpoint
  surfaces, Candid, JSON schemas, deployment truth schema, evidence/report
  schemas, and stable-state layout are unchanged.

- The `0.83.18` slice fixes `CANIC-083-DEBT-024` by tightening
  deployment-truth comparison report validation and text-output labels.
  Comparison report validation field names and text renderer field, section,
  count, target, and fallback labels now use typed internal labels. Validation
  error field strings, operator text output labels, command behavior, endpoint
  surfaces, Candid, JSON schemas, deployment truth schema, evidence/report
  schemas, and stable-state layout are unchanged.

- The `0.83.17` slice fixes `CANIC-083-DEBT-023` by tightening
  deployment-truth execution-preflight validation and text-output labels.
  Validation field names and text renderer field/section/status labels now use
  typed internal labels. Execution-preflight blocker codes and the static
  authority fallback subject now use typed internal labels as well. Error field
  strings, operator text output, blocker code strings, fallback subject string,
  command behavior, endpoint surfaces, Candid, JSON schemas, deployment truth
  schema, evidence/report schemas, and stable-state layout are unchanged.

- The `0.83.16` slice fixes `CANIC-083-DEBT-022` by tightening
  host install-root execution-preflight receipt labels. The execution-preflight
  receipt phase, operation ID, failure command-result code, and evidence keys
  now use typed labels internally. Deployment-truth execution-preflight
  planned-phase rows also use typed labels internally. Receipt JSON phase
  strings, operation IDs, command-result codes, evidence strings,
  planned-phase strings, command behavior, endpoint surfaces, Candid, JSON
  schemas, deployment truth schema, evidence/report schemas, and stable-state
  layout are unchanged.

- The `0.83.15` slice fixes `CANIC-083-DEBT-020` by tightening
  host install-root deployment-truth phase labels. Install-root operation
  phases, completed-phase receipts, artifact-promotion install receipts, and
  deployment-truth gate operation IDs now use `InstallPhaseLabel` internally.
  It also fixes `CANIC-083-DEBT-021` by tightening install-root timing summary
  row labels with `InstallTimingLabel`. Receipt JSON phase strings, operation
  IDs, timing table labels, command behavior, endpoint surfaces, Candid, JSON
  schemas, deployment truth schema, evidence/report schemas, and stable-state
  layout are unchanged.

- The `0.83.14` slice fixes `CANIC-083-DEBT-019` by tightening
  runtime bootstrap diagnostic phase labels. Process-local bootstrap status now
  stores `BootstrapPhaseLabel` values, and lifecycle bootstrap scheduling passes
  typed labels into `BootstrapStatusOps::set_phase` across root and nonroot
  bootstrap workflows. `snapshot()` still emits the same
  `BootstrapStatusResponse.phase` strings, and
  `canic_bootstrap_status`, runtime introspection recent-failure metadata,
  lifecycle scheduling behavior, command behavior, endpoint surfaces, Candid,
  JSON, deployment truth, evidence/report schemas, and stable-state layout are
  unchanged.

- The `0.83.13` slice fixes `CANIC-083-DEBT-018` by tightening
  replay-policy manifest constructor command labels. Endpoint, pool-admin, and
  root-capability manifest call sites now construct typed
  `ReplayCommandKindLabel` values explicitly, command-dispatch rows construct
  typed `ReplayCommandManifestLabel` values explicitly, and the private
  manifest helpers accept typed labels instead of loose command strings.
  Replay quota/reserve policy constants now use typed manifest labels as well.
  Runtime replay storage, replay guards, operation IDs, cost guards, workflow
  replay descriptors, persisted receipts, endpoint names, command-manifest
  string values, and quota/reserve policy string values are unchanged. Command
  behavior, endpoint surfaces, Candid, JSON, deployment truth, evidence/report
  schemas, and stable-state layout are unchanged.

- The `0.83.12` slice fixes `CANIC-083-DEBT-017` by tightening
  replay-policy manifest command-kind labels. `ReplayPolicy` variants now carry
  a typed static `ReplayCommandKindLabel` instead of raw string command-kind
  fields. Runtime replay storage, replay guards, operation IDs, cost guards,
  workflow replay descriptors, and persisted receipts continue to use
  `model::replay::CommandKind`. This is a pre-1.0 Rust manifest-model hard
  cut. Command behavior, endpoint surfaces, Candid, JSON, deployment truth,
  evidence/report schemas, and stable-state layout are unchanged.

- The `0.83.11` slice fixes `CANIC-083-DEBT-015` and
  `CANIC-083-DEBT-016` by tightening host-owned report labels.
  `canic state audit` report scope and check category/source labels are now
  represented by typed internal report values instead of raw strings.
  Deployment-root verification identity/evidence check-row names are also
  represented by typed internal values in the report builder and validator.
  Audit codes, subjects, details, next actions, command strings, embedded
  manifest data, and serialized `DeploymentRootVerificationCheckV1.name`
  labels remain in their existing report shapes. Command behavior, JSON
  fields, JSON labels, text output meaning, endpoint surfaces, Candid,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged.

- The `0.83.10` slice fixes `CANIC-083-DEBT-014` by tightening
  `canic deploy plan` diagnostic labels. Diagnostic category, severity, and
  source labels are now represented by typed internal report values instead of
  raw strings. Diagnostic codes, subjects, details, next actions, and embedded
  `DeploymentPlanV1` data remain in their existing report shapes. Command
  behavior, JSON fields, JSON labels, text output meaning, endpoint surfaces,
  Candid, deployment truth, evidence/report schemas, and stable-state layout
  are unchanged.

- The `0.83.9` slice fixes `CANIC-083-DEBT-012` and
  `CANIC-083-DEBT-013` by tightening `canic replica status --json` reports and
  `canic deploy plan` future-apply preview rows. Replica `status_source`
  labels and deploy-plan preview phase, operation, and status labels are now
  represented by typed internal report values instead of string literals.
  Delegated ICP CLI command/error strings, embedded ICP status payloads,
  deploy-plan diagnostic codes, subjects, details, next actions, and embedded
  `DeploymentPlanV1` data remain in their existing report shapes. Command
  behavior, JSON fields, JSON labels, text output meaning, endpoint surfaces,
  Candid, deployment truth, evidence/report schemas, and stable-state layout
  are unchanged.

- The `0.83.8` slice fixes `CANIC-083-DEBT-011` by tightening
  `canic blob-storage` action reports so the closed `query`/`update`
  method-mode labels are represented by typed internal report-model values
  instead of raw strings. Response-derived gateway `sync_action`, next-action
  guidance labels, delegated command strings, error text, and canister-derived
  data remain strings. Command behavior, JSON fields, JSON labels, text output
  meaning, endpoint surfaces, Candid, deployment truth, evidence/report
  schemas, and stable-state layout are unchanged.

- The `0.83.7` slice fixes `CANIC-083-DEBT-010` by tightening
  `canic token` and `canic cycles` wallet parsers so maintained subcommand
  sets are represented by typed internal command kinds instead of raw strings.
  Caller-provided token symbols, receivers, cycles pending-operation command
  strings, and delegated ICP CLI command/error strings remain strings. Command
  behavior, help text, endpoint surfaces, Candid, JSON, deployment truth,
  evidence/report schemas, and stable-state layout are unchanged.

- The `0.83.6` slice fixes `CANIC-083-DEBT-009` by tightening the
  `canic backup` report wrapper so create mode/layout/status, list status,
  prune status/action, and status/inspect layout-status labels are typed
  internally instead of owned as raw strings. Dynamic backup scope labels,
  paths, operation kind/state labels, errors, and canister-derived data remain
  strings. The emitted JSON labels and text output remain unchanged, including
  `dry-run`, `execute`, `existing`, `new`, `planned`, `running`, `complete`,
  `paused`, `failed`, `invalid-manifest`, `invalid-plan`,
  `invalid-plan-journal`, `ok`, `would-remove`, and `removed`.
  Command behavior, endpoint surfaces, Candid, deployment truth,
  evidence/report schemas, and stable-state layout are unchanged.

- The `0.83.5` slice fixes `CANIC-083-DEBT-008` by tightening the
  `canic blob-storage` report wrapper so report kind, Candid source, action,
  funding status, and readiness state labels are typed internally instead of
  owned as raw strings. Free-form command strings, error messages,
  blocker/warning code arrays, and canister-derived text values remain strings.
  The emitted JSON labels and text output remain unchanged, including
  `blob_storage_status`, `blob_storage_error`,
  `blob_storage_sync_gateways_result`, `blob_storage_fund_result`,
  `installed_deployment`, `sync_gateways`, `fund`, `ready`, `warning`,
  `blocked`, `funding_needed`, `not_configured`, `not_needed`, and `unknown`.
  Command behavior, endpoint surfaces, Candid, deployment truth,
  evidence/report schemas, and stable-state layout are unchanged.

- The `0.83.4` slice fixes `CANIC-083-DEBT-006` and
  `CANIC-083-DEBT-007` by tightening the `canic info metrics` and
  `canic info cycles` report wrappers so canister-row status and coverage
  labels are typed internally instead of owned as raw strings. The emitted JSON
  labels and text output remain unchanged, including `ok`, `empty`,
  `unavailable`, `error`, `covered`, `partial`, and `none`. Command behavior,
  endpoint surfaces, Candid, deployment truth, evidence/report schemas, and
  stable-state layout are unchanged.

- The `0.83.3` slice fixes `CANIC-083-DEBT-005` by tightening the
  `canic auth renewal status` report wrapper so CLI-owned report kind, local
  Candid-source, and aggregate renewal status labels are typed internally
  instead of owned as raw strings. Decoded canister response statuses remain
  response data. The emitted JSON labels and text output remain unchanged,
  including `auth_renewal_status`, `installed_deployment`, `active_attempt`,
  `configured`, `disabled`, `missing`, `unavailable`, and `drift_detected`.
  Command behavior, endpoint surfaces, Candid, deployment truth,
  evidence/report schemas, and stable-state layout are unchanged.

- The `0.83.2` slice fixes `CANIC-083-DEBT-004` by tightening
  the `canic inspect` report wrapper so command and endpoint labels,
  health/readiness slots, source attribution, response format, and aggregate
  runtime status are typed internally instead of owned as loose JSON/raw
  strings. The emitted JSON labels and text output remain unchanged, including
  `canic inspect canister`, `canic inspect deployment`, `canic_runtime_status`,
  `cli_arg`, `deployment_record`, `runtime_observed`, `candid`, and the runtime
  status labels. Command behavior, endpoint surfaces, Candid, deployment truth,
  evidence/report schemas, and stable-state layout are unchanged.

- The `0.83.1` slice fixes `CANIC-083-DEBT-002` by
  hard-cutting default-JSON advanced deploy report families from
  `--format json|text` to JSON by default plus `--text` for human-readable
  output. The affected families are deploy compare, root verification,
  authority dry-run reports, external lifecycle reports, and promotion reports.
  No aliases, shims, compatibility routes, or anti-resurrection tests are kept.
  JSON payload schemas, deployment truth, evidence/report schemas, endpoint
  surfaces, Candid, and stable-state layout are unchanged. The same slice
  removes a `canic state manifest` help breadcrumb for the removed
  `--format json` spelling.

- The `0.83.0` slice creates the docs-only technical debt audit artifact set
  under `docs/audits/0.83-technical-debt/`, runs the baseline repo-health
  commands from the design, records and fixes the first accepted finding,
  `CANIC-083-DEBT-001`, and records `CANIC-083-DEBT-002` as a follow-up. The
  fixed finding hard-cuts deployment catalog, deployment check, fleet adoption
  reports, evidence gate, and evidence compare to `--json` for raw JSON output
  and `--evidence-envelope` for stable evidence-envelope output. No aliases or
  compatibility routes are kept for the removed `--format json` /
  `--format envelope-json` report-selection forms. JSON payload schemas,
  deployment truth, evidence envelope schema, endpoint surfaces, Candid, and
  stable-state layout are unchanged. The follow-up finding covered the
  advanced deploy report families fixed in 0.83.1: deploy compare, root
  verification, authority reports, external lifecycle reports, and promotion
  reports.

- The `0.82.41` docs-only organization slice keeps the canonical 0.82 design
  at `docs/design/0.82-boundary-hardening/0.82-design.md` and moves
  supplemental docs-only hardening reports under
  `docs/design/0.82-boundary-hardening/reports/`. No source, command,
  endpoint, serialized-surface, deployment-truth, evidence/report schema, or
  stable-state behavior is changed by this organization slice.

- The `0.82.33` slice hard-cuts backup/restore parser and layout fallback
  paths by rejecting plan-only backup create layouts without
  `backup-execution-journal.json`, requiring restore upload helper output to be
  JSON with `snapshot_id`, and requiring restore stopped precondition output to
  be JSON. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-backup-restore-legacy-repair-hard-cut-report.md`.

- The `0.82.34` slice hard-cuts backup/restore JSON contract
  tolerance by rejecting unknown fields across current backup/restore manifests,
  plans, journals, receipts, command previews, and reports, and removing unused
  plan-facing backup receipt types. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-backup-restore-json-contract-hard-cut-report.md`.

- The `0.82.35` slice hard-cuts CLI help/version word aliases
  by keeping canonical `--help`/`-h` and `--version`/`-V` flag forms only,
  removing bare `help`/`version` preflight handling, and dropping stale
  removed-command fixtures from global forwarding tests. The docs-only report
  is
  `docs/design/0.82-boundary-hardening/reports/0.82-cli-help-word-alias-hard-cut-report.md`.

- The `0.82.36` slice removes remaining active
  anti-resurrection-style tests for retired bridge-backed delegation-proof
  endpoints and guards, and removes the host-local `BootstrapStatusSnapshot`
  alias so install-root readiness code consumes the canonical
  `BootstrapStatusResponse` DTO directly. The same slice renames stale
  fallback wording in the registry-role diagnostics JSON test. Replay behavior,
  install-root readiness behavior, endpoint surfaces, CLI behavior, Candid,
  JSON, deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-replay-policy-test-and-readiness-alias-cleanup-report.md`.

- The `0.82.37` slice removes a remaining protocol-surface
  anti-resurrection assertion for retired single-proof root delegation endpoint
  names while preserving positive coverage for maintained root issuer policy,
  renewal-template, renewal-status, chain-key lazy-repair endpoint constants,
  guards, bindings, and DTO round-trips. Endpoint surfaces, CLI behavior,
  Candid, JSON, deployment truth, evidence/report schemas, and stable-state
  layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-root-delegation-protocol-test-cleanup-report.md`.

- The `0.82.38` slice hard-cuts unused public Rust aliases
  from core helper and infra boundaries. `ConsentResult`, `HashBytes`, and
  `GetSubnetForCanisterResponse`, `Icrc1TransferResult`, and
  `NotifyTopUpResult` are removed in favor of concrete
  `Result<ConsentInfo, Icrc21Error>`, `Vec<u8>`,
  `Result<GetSubnetForCanisterPayload, String>`,
  `Result<Nat, TransferError>`, and `Result<Nat, NotifyTopUpError>` shapes.
  Runtime behavior, endpoint surfaces, CLI behavior, Candid, JSON, deployment
  truth, evidence/report schemas, and stable-state layout are unchanged. The
  docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-core-rust-alias-hard-cut-report.md`.

- The `0.82.39` slice hard-cuts the public ICRC-21 dispatch
  handler alias. `ConsentHandlerFn` is removed in favor of the concrete
  `Arc<dyn Fn(ConsentMessageRequest) -> ConsentMessageResponse + 'static>`
  handler shape wrapped by a private dispatcher registry value, and
  `Icrc21Dispatcher::get_handler` is narrowed to a private lookup helper. The
  same slice replaces the backup `SnapshotDriverError` alias with a concrete
  `SnapshotDriverError` struct that wraps the boxed driver source error.
  Runtime behavior, endpoint surfaces, CLI behavior, Candid, JSON, deployment
  truth, evidence/report schemas, and stable-state layout are unchanged. The
  docs-only reports are
  `docs/design/0.82-boundary-hardening/reports/0.82-icrc21-handler-alias-hard-cut-report.md`
  and
  `docs/design/0.82-boundary-hardening/reports/0.82-snapshot-driver-error-alias-hard-cut-report.md`.

- The current `0.82.40` working slice renames active Toko blob-storage
  inventory/gate wording from compatibility notes to interoperability notes.
  The gate still requires the same source, commit, blob-root mapping, and
  migration/read-through evidence. The same slice tightens active source/test
  wording around aliases, init-argument Candid boundaries, non-chain-key
  root proof rejection, gateway protocol fit, ICP token-prefix parsing, and
  diagnostic public-output stability. The final sweep classifies the remaining
  compatibility/fallback/alias wording as real current behavior, stable
  upgrade/output policy, historical operational inventory, or semantic helper
  aliases rather than retained backwards-compatibility machinery. Runtime
  behavior, endpoint surfaces, CLI behavior, Candid, JSON, deployment truth,
  evidence/report schemas, and stable-state layout are unchanged. The
  docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-blob-storage-inventory-interoperability-wording-report.md`.

- The `0.82.1` slice makes the pure-policy boundary explicit:
  core policy modules live under `domain::policy::pure`, policy input/decision
  shapes moved out of `view/`, and internal call sites use the explicit pure
  namespace. This is a no-behavior-change slice with no CLI, endpoint, JSON,
  Candid, stable-state, deployment-truth, or evidence/report surface changes.
  The root and detailed `0.82.1` changelog entries are prepared.

- The `0.82.2` slice starts with release-safety tooling:
  `make minor` and `make major` require interactive confirmation before they
  run release gates or bump version files; `release-minor` and `release-major`
  inherit the guard.

- The same `0.82.2` slice addresses the ICP refill DTO/view boundary:
  `IcpRefillStatus` and `IcpRefillErrorCode` are now owned by
  `domain::icp_refill`, `dto::icp_refill` re-exports them to preserve public
  Rust paths and Candid shape, and internal view/storage/workflow/metrics code
  imports the values from the domain owner. This has a docs-only hardening
  report at
  `docs/design/0.82-boundary-hardening/reports/0.82-icp-refill-dto-boundary-report.md`.

- The same `0.82.2` slice also moves root runtime subnet identity values to
  `domain::subnet` while preserving `dto::subnet` re-exports for the macro/init
  Candid boundary. Runtime root workflow imports the domain owner directly, and
  the docs-only hardening report is
  `docs/design/0.82-boundary-hardening/reports/0.82-runtime-identity-dto-boundary-report.md`.

- A 0.82 follow-up slice continues DTO boundary cleanup by moving
  cycle top-up event status ownership to `domain::cycles` while preserving the
  public `dto::cycles::CycleTopupEventStatus` re-export and Candid shape.
  Storage cycle ops now import the domain owner directly, with the docs-only
  report at
  `docs/design/0.82-boundary-hardening/reports/0.82-cycle-topup-dto-boundary-report.md`.

- The same 0.82 follow-up slice moves canister pool status ownership to
  `domain::pool` while preserving the public
  `dto::pool::CanisterPoolStatus` re-export and Candid shape. Pool storage
  mapping and import/recycle workflow decisions now import the domain owner
  directly, with the docs-only report at
  `docs/design/0.82-boundary-hardening/reports/0.82-pool-status-dto-boundary-report.md`.

- The same 0.82 follow-up slice extends the ICP refill DTO boundary cleanup by
  moving `IcpRefillMode` to `domain::icp_refill` while preserving the public
  DTO re-export and request/dry-run Candid shape. Manual, hub, replay, storage,
  and workflow tests now import the mode from the domain owner.

- The same 0.82 follow-up slice moves metrics selector ownership to
  `domain::metrics` while preserving the public `dto::metrics::MetricsKind`
  re-export and Candid shape. Runtime metrics projection, metrics workflow
  query, and lifecycle facade tests now import the domain owner directly, with
  the docs-only report at
  `docs/design/0.82-boundary-hardening/reports/0.82-metrics-kind-dto-boundary-report.md`.

- The `0.82.4` slice moved runtime failure severity, runtime field visibility,
  and runtime diagnostic status ownership to
  `domain::runtime` while preserving the public
  `dto::runtime::FailureSeverity`,
  `dto::runtime::RuntimeFieldVisibility`,
  `dto::runtime::RuntimeCheckStatus`,
  `dto::runtime::RuntimeDiagnosticSeverity`, and
  `dto::runtime::RuntimeStateDomainStatus` re-exports and Candid/Serde shapes.
  Runtime recent-failure, bootstrap ops, and runtime status builders now import
  the domain owner directly. Docs-only reports:
  `docs/design/0.82-boundary-hardening/reports/0.82-runtime-failure-severity-dto-boundary-report.md`,
  `docs/design/0.82-boundary-hardening/reports/0.82-runtime-field-visibility-dto-boundary-report.md`,
  `docs/design/0.82-boundary-hardening/reports/0.82-runtime-diagnostic-status-dto-boundary-report.md`.

- The `0.82.5` slice moves memory diagnostic value ownership to
  `domain::memory` while preserving the public
  `dto::memory::MemoryCommitRecoveryErrorResponse`,
  `dto::memory::MemoryRangeAuthorityMode`, and
  `dto::memory::MemoryAllocationState` re-exports and Candid shapes. Runtime
  memory ops now import the domain owner directly, with the docs-only report at
  `docs/design/0.82-boundary-hardening/reports/0.82-memory-diagnostic-dto-boundary-report.md`.

- The `0.82.6` slice moves app mode ownership to `domain::state` while
  preserving the public
  `storage::stable::state::app::AppMode` and `dto::state::AppMode` re-exports,
  Candid shape, and stable app-state serialization. App-state mapping now uses
  the shared domain value directly, with the docs-only report at
  `docs/design/0.82-boundary-hardening/reports/0.82-app-mode-domain-boundary-report.md`.

- The `0.82.7` slice moves canister status and log-visibility ownership to
  `domain::canister` while preserving the public
  `dto::canister::{CanisterStatusType, LogVisibility}` and
  `ops::ic::mgmt::{CanisterStatusType, LogVisibility}` re-exports and Candid
  shapes. Management status DTO projection now uses the shared domain values
  directly, while raw management-canister infra payload types remain separate.
  The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-canister-status-domain-boundary-report.md`.

- The `0.82.8` slice moves HTTP method ownership to
  `domain::http` while preserving the public `dto::http::HttpMethod`,
  `ops::ic::http::HttpMethod`, and `ops::runtime::metrics::http::HttpMethod`
  re-exports and Candid method labels. IC HTTP ops and runtime HTTP metrics now
  use the shared domain value directly, while raw management-canister HTTP
  infra payload types remain separate. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-http-method-domain-boundary-report.md`.
  The same slice moves runtime endpoint status ownership to `domain::runtime`
  while preserving the public
  `dto::runtime::{HealthStatus, ReadinessStatus, RuntimeStatus, TimerStatus}`
  re-exports and serialized Candid/Serde shapes. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-runtime-status-domain-boundary-report.md`.

- The `0.82.9` slice moves app command status ownership to
  `domain::state` while preserving the public `dto::state::AppStatus`
  re-export and Candid command shape. App-state storage ops now import the
  status value from the domain owner, while app command/response DTOs and
  stable app-state serialization remain unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-app-status-domain-boundary-report.md`.
  The same slice moves feature-gated blob-storage billing status ownership to
  `domain::blob_storage` while preserving the public `dto::blob_storage`
  re-exports and serialized Candid/Serde shapes. Blob-storage billing status
  builders now import the status values from the domain owner, while Cashier
  request/result DTOs and billing behavior remain unchanged. The docs-only
  report is
  `docs/design/0.82-boundary-hardening/reports/0.82-blob-storage-status-domain-boundary-report.md`.
  The same slice moves timer scheduling mode ownership to `domain::runtime`
  while preserving the public `ops::runtime::metrics::timer::TimerMode`
  re-export and projected metric labels. Timer scheduling ops and metrics
  projection now import the mode value from the domain owner, while timer
  recording behavior remains unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-timer-mode-domain-boundary-report.md`.

- The `0.82.10` slice moves platform-call metric dimension
  ownership to `domain::metrics` while preserving the public
  `ops::runtime::metrics::platform_call` re-exports and projected metric
  labels. IC call, HTTP, ledger, and management ops now import the metric
  dimension values from the domain owner, while platform-call metric recording
  and operation behavior remain unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-platform-call-metric-domain-boundary-report.md`.

- The `0.82.11` slice moves canister-op and management-call
  metric dimension ownership to `domain::metrics` while preserving the public
  `ops::runtime::metrics::canister_ops` and
  `ops::runtime::metrics::management_call` re-exports, canister-op public
  metric labels, and management-call counter behavior. Lifecycle,
  provisioning, and management ops now import the metric dimension values from
  the domain owner, while metric recording and snapshot storage remain
  unchanged. Docs-only reports:
  `docs/design/0.82-boundary-hardening/reports/0.82-canister-ops-metric-domain-boundary-report.md`,
  `docs/design/0.82-boundary-hardening/reports/0.82-management-call-metric-domain-boundary-report.md`.

- The `0.82.12` slice moves lifecycle and wasm-store metric
  dimension ownership to `domain::metrics` while preserving the public
  `ops::runtime::metrics::lifecycle`,
  `ops::runtime::metrics::wasm_store`, and `api::lifecycle::metrics`
  re-exports and public metric labels. Install-source resolution now imports
  wasm-store metric dimension values from the domain owner, while lifecycle and
  wasm-store metric recording and snapshot storage remain unchanged. Docs-only
  reports:
  `docs/design/0.82-boundary-hardening/reports/0.82-lifecycle-metric-domain-boundary-report.md`,
  `docs/design/0.82-boundary-hardening/reports/0.82-wasm-store-metric-domain-boundary-report.md`.

- A 0.82 follow-up slice removes the internal
  `ops::replay::model` compatibility shim after moving replay ops and
  replay-protected workflows to the canonical `model::replay` owner. Hidden
  control-plane support now exposes `CommandKind` through a model-shaped support
  namespace. Replay behavior, stable replay receipt layout, endpoint surfaces,
  CLI behavior, Candid, JSON, deployment truth, and evidence/report schemas are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-replay-model-shim-removal-report.md`.

- A 0.82 follow-up slice removes the internal
  `ops::replay::slot` legacy root replay adapter after routing root replay
  quota checks, reservation, commit, and purge mechanics through shared replay
  receipt helpers/storage directly. Replay behavior, stable replay receipt
  layout, endpoint surfaces, CLI behavior, Candid, JSON, deployment truth, and
  evidence/report schemas are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-root-replay-slot-adapter-removal-report.md`.

- The same 0.82 follow-up slice removes `dto::rpc` re-exports from
  `ops::rpc::request` so RPC request/response DTOs are imported from their DTO
  owner while request ops keep only dispatch helpers/errors. RPC behavior,
  capability metadata, Candid shapes, endpoint surfaces, CLI behavior, JSON,
  deployment truth, and evidence/report schemas are unchanged. The docs-only
  report is
  `docs/design/0.82-boundary-hardening/reports/0.82-rpc-request-dto-boundary-report.md`.

- The same 0.82 follow-up slice removes the workflow-layer `TimerId` re-export
  so timer handles are imported from `ops::runtime::timer`, while
  `TimerWorkflow` keeps scheduling orchestration. Timer behavior, lifecycle
  facade behavior, runtime timer metric labels, endpoint surfaces, CLI
  behavior, Candid, JSON, deployment truth, and evidence/report schemas are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-timer-id-workflow-boundary-report.md`.

- A 0.82 follow-up slice tightens the hidden control-plane support
  facade for pool status by exposing `CanisterPoolStatus` through
  `control_plane_support::domain::pool` instead of a DTO-shaped support
  namespace. Public `dto::pool` compatibility, pool behavior, endpoint
  surfaces, CLI behavior, Candid, JSON, deployment truth, and evidence/report
  schemas are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-control-plane-pool-status-support-boundary-report.md`.

- The same 0.82 follow-up slice removes the crate-private
  `support::WasmStoreGcExecutionStats` re-export in `canic-control-plane` so
  the template API imports GC stats from template storage ops directly.
  Wasm-store GC behavior, endpoint surfaces, CLI behavior, Candid, JSON,
  deployment truth, and evidence/report schemas are unchanged. The docs-only
  report is
  `docs/design/0.82-boundary-hardening/reports/0.82-control-plane-template-gc-support-boundary-report.md`.

- The same 0.82 follow-up slice removes the hidden
  `control_plane_support::workflow::prelude` wildcard support path after
  root bootstrap was narrowed to import `Principal` from
  `control_plane_support::cdk::types` directly. Root bootstrap behavior,
  endpoint surfaces, CLI behavior, Candid, JSON, deployment truth, and
  evidence/report schemas are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-control-plane-prelude-support-boundary-report.md`.

- A 0.82 follow-up slice adds maintained boundary guard tests for
  pure policy and passive DTO ownership. Pure policy modules are now checked
  against forbidden side-effect imports, async/timer/IC call fragments, and
  wire serialization fragments. Non-error DTO trees in `canic-core` and
  `canic-control-plane` are checked against internal behavior-layer imports
  and side-effect fragments, with `dto::error` documented as the public error
  boundary-adapter exception. Runtime behavior, endpoint surfaces, CLI
  behavior, Candid, JSON, deployment truth, and evidence/report schemas are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-policy-dto-boundary-guard-report.md`.

- The same 0.82 follow-up slice adds a maintained lifecycle boundary guard.
  Before-bootstrap lifecycle adapters in `canic-core` and
  `canic-control-plane` are checked to remain synchronous and timer-free, while
  root and non-root async bootstrap schedule helpers are checked to keep their
  explicit zero-delay lifecycle timer boundary. Runtime behavior, lifecycle
  macro behavior, endpoint surfaces, CLI behavior, Candid, JSON, deployment
  truth, and evidence/report schemas are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-lifecycle-boundary-guard-report.md`.

- The same 0.82 follow-up slice hard-cuts runtime introspection enum labels to
  canonical snake_case Candid/Serde labels. Candid supports explicit
  per-variant `serde(rename)` labels but not `rename_all`, so the previous
  `rename_all` plus PascalCase `serde(alias)` workaround has been removed.
  Public Rust re-export paths, endpoint routes, endpoint guards, runtime status
  builder behavior, deployment truth, evidence/report schemas, and stable-state
  layout are unchanged; the serialized runtime enum label surface is
  intentionally changed to snake_case only. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-runtime-enum-label-hard-cut-report.md`.

- A 0.82 follow-up slice adds a maintained Candid serde boundary
  guard. Canic-owned Candid source roots are checked so `CandidType` items do
  not use unsupported `serde(rename_all)` / `rename_all_fields` attributes or
  `serde(alias)`. The guard was tightened to catch combined serde attributes
  such as `#[serde(rename = "...", alias = "...")]`.

- The same 0.82 follow-up slice hard-cuts the shared HTTP method value to
  canonical lowercase labels only. `HttpMethod` keeps the canonical Candid/Serde
  labels `get`, `head`, and `post`, but no longer accepts uppercase `GET`,
  `HEAD`, or `POST` compatibility aliases. Public Rust re-export paths, HTTP
  execution, metrics labels, endpoint routes, CLI behavior, deployment truth,
  evidence/report schemas, and stable-state layout are unchanged. Docs-only
  reports:
  `docs/design/0.82-boundary-hardening/reports/0.82-candid-serde-boundary-guard-report.md`
  and
  `docs/design/0.82-boundary-hardening/reports/0.82-http-method-alias-hard-cut-report.md`.

- The same 0.82 follow-up slice performs a targeted hard-cut compatibility
  sweep. `canic inspect` now rejects `canic_runtime_status` query output that
  only contains `response_candid`; the canonical path requires typed
  `response_bytes` so `CanicRuntimeStatus` is decoded from Candid bytes. The
  test-only legacy `RootReplayRecord` manual encoder/decoder was removed, and
  the removed root replay state declaration now points at the active shared
  replay receipt round-trip test. Public error-code compatibility names, auth
  metric mirroring, and the non-IC root bootstrap subnet-identity fallback were
  classified as separate explicit hard-cut candidates in this sweep and are now
  closed in follow-up hard-cut reports.
  The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-hard-cut-compatibility-sweep-report.md`.
  The root and detailed `0.82.17` changelog entries are prepared.

- A 0.82 follow-up slice hard-cuts the public registry policy error
  codes that still used pre-service-topology singleton names. The public
  `ErrorCode` variants, host direct-query wire decoder, and checked-in
  wasm-store DID now use service-owned names for replica scaling, shard
  sharding, and instance directory policy failures. Registry policy behavior,
  messages, endpoint routes, CLI command surfaces, deployment truth,
  evidence/report schemas, and stable-state layout are unchanged. The docs-only
  report is
  `docs/design/0.82-boundary-hardening/reports/0.82-policy-error-code-hard-cut-report.md`.

- The same 0.82 follow-up slice hard-cuts auth metric compatibility mirroring.
  Auth session, bootstrap, identity-fallback, and role-attestation events now
  record only the canonical Auth metric family instead of also writing older
  Access-family rows. Auth behavior, auth identity resolution, access-expression
  guard metrics, metrics query sorting/pagination, endpoint routes, CLI command
  surfaces, Candid, JSON, deployment truth, evidence/report schemas, and
  stable-state layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-auth-metric-mirror-hard-cut-report.md`.

- The same 0.82 follow-up slice hard-cuts the root bootstrap subnet identity
  fallback. Root bootstrap no longer invents a subnet principal from
  `canister_self()` when registry discovery is unavailable; local/test builds
  use the explicit subnet identity seeded by lifecycle init, while IC builds
  fail the bootstrap phase if NNS registry subnet discovery returns no subnet
  or errors. Root init argument shape, endpoint routes, CLI command surfaces,
  Candid, JSON, deployment truth, evidence/report schemas, and stable-state
  layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-root-bootstrap-subnet-identity-hard-cut-report.md`.
  The root and detailed `0.82.18` changelog entries are prepared.

- A 0.82 follow-up slice hard-cuts CLI metrics/cycles
  `response_candid` fallback parsing. `canic info metrics` and
  `canic info cycles` now require structured JSON values for metrics, cycle
  tracker, and top-up report pages; text-only `response_candid` payloads and
  malformed structured entries with `response_candid` present are rejected.
  CLI command names/options, successful report output, endpoint Candid
  signatures, deployment truth, evidence/report schemas, and stable-state
  layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-cli-metrics-cycles-response-candid-hard-cut-report.md`.

- The same 0.82 follow-up slice hard-cuts host `canic_metadata`
  `response_candid` fallback parsing. Metadata version discovery now requires
  a structured JSON `canic_version` field; raw Candid text and text-only
  `response_candid` wrapper output are rejected. The `canic_metadata` endpoint
  Candid signature, CLI list command surfaces, successful live-list rendering,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-canic-metadata-response-candid-hard-cut-report.md`.
  The root and detailed `0.82.19` changelog entries are prepared.

- A 0.82 follow-up slice hard-cuts host cycle-balance
  `response_candid` fallback parsing. ICP CLI `canic_cycle_balance` output now
  requires a structured JSON `Ok` value, while the local replica fast path
  still decodes typed Candid bytes directly. Raw Candid text and text-only
  `response_candid` wrapper output are rejected. The endpoint Candid
  signature, CLI list/cycles command surfaces, successful live-list rendering,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-cycle-balance-response-candid-hard-cut-report.md`.

- The same 0.82 follow-up slice hard-cuts root bootstrap-readiness
  `response_candid` fallback parsing. ICP CLI `canic_bootstrap_status` output
  now requires a structured JSON status record or wrapped `Ok` record, while
  the local replica fast path still decodes typed Candid bytes directly. The
  bootstrap-status endpoint Candid signature, root bootstrap lifecycle
  behavior, install command surfaces, deployment truth, evidence/report
  schemas, and stable-state layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-bootstrap-readiness-response-candid-hard-cut-report.md`.
  The root and detailed `0.82.20` changelog entries are prepared.

- The `0.82.21` slice hard-cuts the remaining
  `response_candid` metadata from `canic inspect` runtime reports. Inspect
  still requires typed `response_bytes` for `canic_runtime_status` decoding and
  still reports `response_format: candid`, but text/JSON output no longer
  exposes response-wrapper presence fields such as
  `runtime_status.response_candid_present` or
  `runtime_status.response_bytes_present`. Inspect
  command surfaces, target resolution, endpoint guards, runtime endpoint DTOs,
  Candid, deployment truth, evidence/report schemas, and stable-state layout
  are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-inspect-response-candid-metadata-hard-cut-report.md`.

- The same `0.82.21` slice hard-cuts deployment-truth artifact
  observation across network roots. Non-local deployment-truth/deploy-plan
  artifact observation now requires `.icp/<network>/canisters` and no longer
  falls back to `.icp/local/canisters`; missing selected-network artifacts are
  reported through the existing `local_artifacts.root` gap. Deployment truth,
  deploy plan, evidence, Candid, and stable-state schemas are unchanged. The
  docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-artifact-root-network-fallback-hard-cut-report.md`.

- The same `0.82.21` slice hard-cuts deployment-truth local config
  fleet-name fallback. When local config cannot resolve a fleet name,
  deployment-truth root observations now report the existing
  `local_config.fleet_name` gap and use `fleet_template = "unknown"` instead
  of copying the deployment target name into fleet-template identity. Schemas,
  command surfaces, evidence, Candid, and stable-state layout are unchanged.
  The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-local-config-fleet-name-fallback-hard-cut-report.md`.

- The same `0.82.21` slice hard-cuts the deployment catalog's active
  legacy fleet-state warning. Catalog reports now read only current
  `.canic/<network>/deployments` state and no longer probe removed
  `.canic/<network>/fleets` paths to emit `catalog.legacy_fleet_state_ignored`.
  Current catalog schema, command surfaces, deployment truth, evidence, Candid,
  and stable-state layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-deployment-catalog-legacy-fleet-warning-hard-cut-report.md`.

- The same `0.82.21` slice hard-cuts install-root legacy fleet-state
  lookup. `read_deployment_install_state` now reads only current
  `.canic/<network>/deployments/<deployment>.json` state and returns no state
  when that file is absent; it no longer probes removed
  `.canic/<network>/fleets/<name>.json` paths. Deployment registration help now
  describes the current deployment-target boundary without 0.46 legacy recovery
  language. Schemas and command surfaces are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-install-root-legacy-fleet-state-hard-cut-report.md`.
  The root and detailed `0.82.21` changelog entries include these hard cuts.

- The `0.82.22` slice removes CLI anti-resurrection tests for
  removed command aliases and obsolete hard-cut forms while preserving current
  positive parser, help, JSON, report, and exit-code coverage. Command
  behavior, command surfaces, endpoint surfaces, Candid, JSON, deployment
  truth, evidence/report schemas, and stable-state layout are unchanged. The
  slice also removes negative help assertions that mentioned the retired
  `canic info medic` route and renames endpoint macro guard-grammar coverage
  away from compatibility-alias wording. The auth verifier legacy
  root-proof-mode rejection test remains because it protects an active
  security/config invariant. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-cli-anti-resurrection-test-cleanup-report.md`.
  The root and detailed `0.82.22` changelog entries are prepared.

- A 0.82 follow-up slice removes hidden
  `control_plane_support` facades that mirrored public validation DTO, ids, and
  replay-policy owners. Control-plane root bootstrap now imports validation
  report DTOs from `canic_core::dto::validation`, deployment workflow now
  imports `CostClass` from `canic_core::replay_policy`, and the unused ids
  support namespace is removed. Support facades for crate-private
  `SubnetConfig`, `CanisterPoolStatus`, and `CommandKind` remain because they
  are real control-plane mediation boundaries. Root bootstrap validation
  behavior, deployment cost-guard behavior, endpoint surfaces, CLI behavior,
  Candid, JSON, deployment truth, evidence/report schemas, and stable-state
  layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-control-plane-core-owner-support-boundary-report.md`.
  The root and detailed `0.82.23` changelog entries are prepared.

- A 0.82 follow-up slice removes the broad hidden
  `control_plane_support::cdk` mirror. Control-plane code now imports public
  CDK types directly from `canic_core::cdk::types`, while support facades remain
  reserved for crate-private core mediation. Runtime template publication,
  root bootstrap behavior, endpoint surfaces, CLI behavior, Candid, JSON,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-control-plane-cdk-support-boundary-report.md`.

- The same 0.82 follow-up slice removes the hidden
  `control_plane_support::protocol` mirror. The control-plane wasm-store
  template client and protocol manifest tests now import public endpoint-name
  constants directly from `canic_core::protocol`, while endpoint names,
  endpoint classifications, endpoint surfaces, CLI behavior, Candid, JSON,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-control-plane-protocol-support-boundary-report.md`.

- The same 0.82 follow-up slice cleans stale release-line wording out of
  active CLI help and error text for state manifest, deploy plan, and inspect
  output. The commands now describe the current command contracts without
  implying those surfaces are tied to their original 0.79-0.81 release lines.
  Command parsing, accepted/rejected forms, exit codes, JSON/report fields,
  Candid, deployment truth, evidence/report schemas, and stable-state layout
  are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-active-cli-release-wording-cleanup-report.md`.
  The root and detailed `0.82.24` changelog entries are prepared.

- A 0.82 follow-up slice narrows
  `control_plane_support::format` to the single formatting helper used by
  `canic-control-plane`. The hidden support namespace now exports only
  `byte_size`; host-side `cycles_tc` and `truncate` usage remains on its
  existing support path. Control-plane byte-size labels, endpoint surfaces,
  CLI behavior, Candid, JSON, deployment truth, evidence/report schemas, and
  stable-state layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-control-plane-format-support-boundary-report.md`.

- The same 0.82 follow-up slice cleans stale release-line labels out of active
  medic source comments and lint-expectation reasons. Medic report categories,
  exit-code behavior, endpoint surfaces, CLI behavior, Candid, JSON,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-active-source-release-comment-cleanup-report.md`.
  The root and detailed `0.82.25` changelog entries are prepared.

- A 0.82 follow-up slice hard-cuts unused wasm-store Rust API facade
  names and direct publication helpers.
  `canic::api::canister::template::EmbeddedTemplateApi`,
  `canic::api::canister::template::WasmStoreApi`, and the direct
  `canic-control-plane::api::template::WasmStoreApi` helper surface are
  removed; the endpoint-facing `WasmStoreCanisterApi` remains the canonical
  public wasm-store canister facade, and the local helper is private. Direct
  `WasmStorePublicationApi` action helpers are removed in favor of the typed
  `WasmStorePublicationApi::admin` / `WasmStoreAdminCommand` path. Operator
  commands, endpoint method names, Candid request/response shapes, JSON,
  deployment truth, evidence/report schemas, stable-state layout, wasm-store
  storage behavior, and GC behavior are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-wasm-store-api-facade-hard-cut-report.md`.
  The root and detailed `0.82.26` changelog entries are prepared.

- A 0.82 follow-up slice hard-cuts unused wasm-store bootstrap Rust
  helpers. Root-specific direct staging helpers, their manifest normalization
  code, the unused bootstrap binding constant, and the direct staged-release
  publication support wrapper are removed. Lifecycle-used embedded release-set
  helpers, endpoint-used bootstrap helpers, endpoint method names, Candid,
  JSON, deployment truth, evidence/report schemas, stable-state layout,
  wasm-store storage behavior, publication workflow behavior, and lifecycle
  behavior are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-wasm-store-bootstrap-helper-hard-cut-report.md`.
  The root and detailed `0.82.27` changelog entries are prepared.

- A 0.82 follow-up slice removes the private wasm-store
  `LocalWasmStoreApi` pass-through helper and collapses the remaining
  crate-private template support module into private template API helpers.
  `WasmStoreCanisterApi` now calls private template helpers directly, while
  root bootstrap and publication APIs keep the same public method surfaces.
  Endpoint surfaces, CLI behavior, Candid, JSON, deployment truth,
  evidence/report schemas, stable-state layout, wasm-store storage behavior,
  publication workflow behavior, bootstrap behavior, and GC behavior are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-wasm-store-template-support-cleanup-report.md`.
  The root and detailed `0.82.28` changelog entries are prepared.

- A 0.82 follow-up slice narrows workflow prelude usage in the pool
  and IC workflow clusters. Pool import/recycle/reset/scheduler/query/admin,
  IC call/ledger/management, provisioning, and ICP refill workflow modules now
  import boundary values from concrete `cdk`, `ids`, and `log` owners instead
  of `workflow::prelude::*`. The stale `workflow::prelude::Account` Rust
  re-export is removed in favor of the canonical `cdk::types::Account` owner.
  Operator command surfaces, endpoint names, Candid, JSON, deployment truth,
  evidence/report schemas, stable-state layout, pool behavior, IC call
  behavior, ledger behavior, ICP refill behavior, and provisioning behavior are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-workflow-prelude-boundary-report.md`.
  The root and detailed `0.82.29` changelog entries are prepared.

- A 0.82 follow-up slice finishes the workflow prelude hard cut.
  Env, runtime, auth, cascade, lifecycle, RPC request, bootstrap,
  topology-index, placement-scaling, and cycle-tracking workflow modules now
  import passive values from concrete `cdk`, `ids`, and `log` owners instead
  of `workflow::prelude::*`. The unused `workflow::prelude` module is removed.
  Operator command surfaces, endpoint names, Candid, JSON, deployment truth,
  evidence/report schemas, stable-state layout, runtime startup behavior,
  auth renewal behavior, timer behavior, cascade behavior, canister lifecycle
  behavior, RPC behavior, scaling behavior, and cycle tracking behavior are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-workflow-prelude-hard-cut-report.md`.
  The root and detailed `0.82.30` changelog entries are prepared.

- Pre-1.0 hard-cut policy is now explicit in `AGENTS.md`: do not add aliases,
  shims, compatibility wrappers, legacy fallback paths, backwards-compatibility
  layers, or anti-resurrection tests unless the maintainer explicitly asks.

- The `0.82.31` slice hard-cuts two unused Rust fallback surfaces.
  `access::expr::requires` is removed in favor of the canonical
  `access::expr::all` Rust helper, while endpoint macro `requires(...)`
  grammar remains unchanged. `CanicMetadataApi::metadata` and its core-package
  constants are removed so metadata construction stays on
  `CanicMetadataApi::metadata_for(...)`, which is the path used by the
  endpoint metadata macro with exporting-canister package metadata. Operator
  commands, endpoint method names, Candid, JSON, metadata response fields,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-access-metadata-fallback-hard-cut-report.md`.
  The root and detailed `0.82.31` changelog entries are prepared.

- The `0.82.32` slice removes remaining host/CLI `response_candid` and
  raw Candid parser fixtures from active tests and hard-cuts
  `replica_query::parse_ready_json_value` so Candid text strings such as
  `"(true)"` and unrelated truthy object fields no longer count as readiness
  success. Maintained structured JSON boolean and explicit `Ok` shapes plus
  typed response-byte decoding remain covered. Operator
  commands, endpoint method names, Candid, JSON report schemas, deployment
  truth, evidence/report schemas, and stable-state layout are unchanged. The
  docs-only report is
  `docs/design/0.82-boundary-hardening/reports/0.82-response-candid-test-fixture-hard-cut-report.md`.
  The root and detailed `0.82.32` changelog entries are prepared.

- The previous line was `0.81.x` runtime introspection. Source of truth:
  `docs/design/0.81-runtime-introspection/0.81-design.md`.

- The first post-0.80.0 working slice adds diagnostic state metadata surfaces:
  `canic state audit` and `canic state manifest`. Static Rust declarations
  cover the root canister family for canic-core stable-memory domains plus the
  retired root replay memory ID. The reports are diagnostic-only, render
  text/JSON with `schema_version = 1`, check duplicate memory IDs within a role,
  schema/storage declarations, record/snapshot naming, migration declarations,
  removed-state disposition, restore order, and post-upgrade invariant metadata.
  They do not read stable memory, run migrations, write generated manifests,
  write deployment truth, or mutate canisters. The 0.80.1 changelog entries are
  staged in the root ledger and detailed 0.80 notes.

- The `0.80.2` working slice expands the root-family manifest coverage to
  additional non-gated canic-core domains: subnet topology/state, cycle
  top-up/funding/refill records, intent store slots, canister pool, scaling
  registry, and directory registry. Ambiguous multi-memory log/cycle-tracker
  declarations and feature-gated sharding/blob-storage declarations remain
  deferred. The same slice now fails `canic state audit` if active state
  reclaims a memory ID declared by removed state, keeping retired IDs reserved
  unless a future explicit migration design handles them. The 0.80.2 changelog
  entries are staged in the root ledger and detailed 0.80 notes.

- The `0.80.3` and `0.80.4` slices add config-driven medic/build checks for
  runtime Canic feature gates implied by fleet auth settings, plus concise CI
  medic output and developer-owned Cargo.toml guidance. These are pushed.

- The `0.80.5` working slice returns to the stable-state design by summarizing
  `canic state audit` inside project-level medic as a diagnostic-only runtime
  readiness check. Medic maps the aggregate state-audit status into a single
  `state_audit_*` row and points operators to `canic state audit` for details;
  it does not inspect stable memory, run migrations, or take ownership of
  state-audit logic. The 0.80.5 changelog entries are staged in the root ledger
  and detailed 0.80 notes.

- The current cleanup slice makes mutation boundaries easier to inspect before
  running commands by putting mutation notes directly in fleet/scaffold command
  help and adding `--dry-run` previews for local source/config mutators: fleet
  create, scaffold canister, fleet role declare/attach/rename, and fleet
  delete. It keeps Cargo.toml feature fixes developer-owned and does not add
  automatic dependency feature editing. The 0.80.6 changelog entries are
  staged in the root ledger and detailed 0.80 notes.

- The `0.80.7` slice returns to the stable-state design by
  adding explicit `reserved_memory` manifest rows for allocated stable-memory
  IDs that are not yet precise active or removed state domains. Root now
  reserves the raw cycle tracker and the two stable-log memories so those
  upgrade-safety gaps are visible in `canic state manifest` and warn in
  `canic state audit`; reservation collisions with active or removed state are
  blocking findings. The same slice adds an explicit `not_applicable`
  state-storage classification for declaration-only metadata so it is not
  conflated with heap-only runtime state. The pre-existing scaffold cleanup
  diff in `crates/canic-cli/src/scaffold/mod.rs` is unrelated to this slice and
  should be preserved. The 0.80.7 changelog entries are staged in the root
  ledger and detailed 0.80 notes.

- The `0.80.8` slice tightens state-audit upgrade-window
  validation so a domain whose `min_supported_version` is zero or greater than
  its current `version` fails with `state_domain_invalid_support_window`
  instead of being treated as a no-migration case. The same slice rejects
  invalid or duplicate migration declarations before checking required
  migration edges, and fails duplicate state-domain names within one canister
  role. The 0.80.8 changelog entries are staged in the root ledger and
  detailed 0.80 notes.

- The `0.80.9` slice starts by making the top-level state
  manifest schema version part of `canic state audit`: supported manifest
  schemas emit `state_manifest_schema_version_supported`, while unsupported
  schemas fail with `state_manifest_schema_version_unsupported` before domain
  metadata is trusted. The same slice rejects duplicate canister-role entries
  with `state_role_duplicate`. The 0.80.9 changelog entries are staged in the
  root ledger and detailed 0.80 notes.

- The previous line was `0.79.12` declarative deployment plan. Source of truth:
  `docs/design/0.79-declarative-deployment-plan/0.79-design.md`.

- The first 0.79 slice is implemented: `canic deploy plan <deployment>` builds
  a deterministic, no-mutation `DeploymentPlanReport` from local project config
  by embedding the existing `DeploymentPlanV1`. It supports text output,
  `--json`, safe JSON `--out` writes, and hard-cut rejection of aliases,
  shorthand forms, `--apply`, `--write-truth`, `--evidence`, and `--force`.
  Missing installed deployment state is a warning/comparison gap, not a
  blocker; verified installed root state is surfaced as a report fact;
  unverified installed root state blocks the plan; malformed desired config
  blocks the plan. Already-available installed-state evidence now drives
  `comparison_status` to `compared`, `compared_with_warnings`, or
  `compared_with_drift`; missing installed state remains `not_available`.
  Invalid deployment target names are explicit blockers. Future-apply preview
  labels distinguish first-install `install_wasm` from known-canister
  `upgrade_wasm`. Medic next actions may point to
  `canic deploy plan`, but medic does not execute the planner.

- The 0.79.1 working slice tightens deploy-plan report facts: reports now
  surface deterministic config, topology, authority, artifact-set, and observed
  role-artifact facts that are already present in the embedded
  `DeploymentPlanV1`, without adding live observation, apply semantics, or
  mutation.

- The 0.79.2 working slice extends deploy-plan future-apply preview labels for
  configured pool expectations. Expected pool identities with no known
  canister id now emit `create_canister` preview labels such as
  `user_shards:user_shard`; these remain non-executed labels, not apply
  operation objects. Desired authority profiles with configured deployment
  controllers now also emit one deployment-scoped `set_controllers` preview
  label, with the same non-executed planning semantics.

- The 0.79.3 working slice extends deploy-plan future-apply preview labels for
  root and child registration. Expected canisters and configured pool
  identities without known ids now emit `register_root` or `register_child`
  labels alongside create/install labels; these remain report-only planning
  labels, not apply instructions. The same slice reserves the
  `unsupported.*` assumption namespace for desired shapes outside the 0.79
  planner contract so those become explicit `unsupported` diagnostics instead
  of generic blockers or warnings. The 0.79.3 changelog entries are staged in
  the root ledger and detailed 0.79 notes.

- The 0.79.4 working slice extends deploy-plan future-apply preview labels to
  include `verify_readiness` when the embedded `DeploymentPlanV1` already
  carries verifier-readiness requirements or expected role epochs, and surfaces
  the same expectation as a `verifier_readiness_expectation_resolved` report
  fact. Reports also name resolved expected canister inventory when role config
  is available. This remains non-executed and does not add live observation or
  mutation. The 0.79.4 changelog entries are staged in the root ledger and
  detailed 0.79 notes.

- The 0.79.5 working slice continues deploy-plan report visibility by
  surfacing fleet-template, expected controller-set, role-artifact inventory,
  expected pool-inventory, and root trust-anchor facts already present in
  `DeploymentPlanV1`. These are passive `verified_facts` only and do not add
  live observation, deployment truth writes, or apply semantics. The 0.79.5
  changelog entries are staged in the root ledger and detailed 0.79 notes.

- The 0.79.6 working slice aligns deploy-plan text output with the stable
  report model by rendering schema version, command identity, and each
  diagnostic source. Future-apply preview lines now also render explicit
  label, subject, and status fields. This is output-only provenance; it does
  not alter JSON shape, plan construction, comparison, observation, deployment
  truth, or mutation behavior.

- The 0.79.7 working slice continues deploy-plan report-contract hardening by
  keeping the text-output parity changes from the post-0.79.6 branch and
  adding focused tests for the documented exit-code contract: planned and
  warning reports exit successfully, while blocked and unsupported reports
  return `PlanBlocked` with exit code 1. The same slice now smoke-tests
  `canic deploy plan --help` so the planning
  command's safety-contract help remains reachable through the shared CLI
  help path. Deploy-plan coverage also pins the command help's no-mutation /
  JSON `--out` wording and the deterministic diagnostic sort order used by
  report arrays. Stable report command, preview phase, and preview status
  strings are centralized to reduce contract drift. This guards the report
  contract without changing plan construction, output schema, observation,
  deployment truth, apply behavior, or mutation semantics. The 0.79.6 and
  0.79.7 changelog entries are staged in the root ledger and detailed 0.79
  notes.

- The 0.79.8 working slice has started with a report-layer cleanup:
  `proposed_operations()` now returns sorted and deduplicated operation labels
  so repeated desired-plan inputs cannot duplicate future-apply preview lines.
  Stable severity, category, source, operation-label, and known assumption-key
  strings are also centralized so report construction, status derivation,
  assumption classification, and tests share the same serialized values.
  Diagnostic sorting now uses an explicit severity rank instead of relying on
  lexical string order, and the public JSON report test pins the complete
  sorted future-apply preview array. This does not change the embedded
  `DeploymentPlanV1`, plan construction, observation, deployment truth, apply
  behavior, or mutation semantics. The 0.79.8 changelog entries are staged in
  the root ledger and detailed 0.79 notes.

- The 0.79.9 working slice has started by adding report-only
  `upload_artifact` future-apply preview labels for each resolved
  `DeploymentPlanV1.role_artifacts` entry. The labels remain non-executed
  planning output and do not add apply operation objects, artifact registration,
  deployment truth writes, live observation, or mutation semantics. Public JSON
  and text-renderer coverage now pins the label while continuing to reject
  apply-safety wording such as `will upload`. Plans with artifact diagnostics
  now also include the top-level next action
  `run canic build or provide a build profile with resolved artifacts`. The
  same slice surfaces passive `build_profile_resolved`, `plan_id_resolved`,
  `runtime_variant_resolved`, and `planner_version_resolved` verified facts
  already present in the command options or embedded `DeploymentPlanV1`.
  The 0.79.9 changelog entries are staged in the root ledger and detailed
  0.79 notes.

- The 0.79.10 working slice has started by surfacing passive
  `config_path_resolved` and `network_resolved` verified facts from the
  deploy-plan invocation and embedded plan identity. These mirror existing
  top-level report fields and do not change plan construction, comparison,
  observation, deployment truth, apply behavior, or mutation semantics. The
  0.79.10 changelog entries are staged in the root ledger and detailed 0.79
  notes.

- The 0.79.11 working slice has started by adding a report-only
  `apply_policy` future-apply preview label when the desired authority profile
  already includes controller policy expectations. The label remains
  non-executed planning output and does not add apply operation objects,
  controller mutation, deployment truth writes, live observation, or mutation
  semantics. Text output also now prints each preview label's `phase` field so
  the human renderer mirrors the JSON `ProposedOperationLabel` shape more
  closely, and the future-apply section header names rows as non-executed
  proposed-operation labels. Command help now documents the same
  preview-label boundary. The 0.79.11 changelog entries are staged in the root
  ledger and detailed 0.79 notes.

- The 0.79.12 working slice has started by tightening the deploy-plan
  evidence/truth boundary in command help: JSON output is explicitly described
  as `DeploymentPlanReport`, not an evidence envelope, deployment truth, or
  authorization to mutate. Report-renderer coverage also pins that actual
  text/JSON reports do not include those truth/evidence/authorization claims
  or apply-safety wording. The 0.79.12 changelog entries are staged in the
  root ledger and detailed 0.79 notes.

- The previous line was `0.78.0` top-level medic preflight. Source of truth:
  `docs/design/0.78-top-level-medic-preflight/0.78-design.md`.

- The first 0.78 slice is implemented: `canic medic` is the top-level
  diagnostic surface with project and explicit deployment scopes, a
  `schema_version = 1` report model, text/JSON renderers, deterministic
  status/category ordering, and hard-cut rejection of old/shorthand forms.
  The old `canic info medic` route is removed from active CLI dispatch.

- Existing deployment-scoped diagnostics are being preserved under
  `canic medic deployment <deployment>`, including targeted
  `--blob-storage <canister-or-role>` and
  `--auth-renewal <issuer-principal>` checks.

- The post-0.78.2 working tree adds passive project-config quality checks to
  `canic medic project`: discovered roles now report
  `role_package_metadata_present` / `role_package_metadata_missing`, and
  declared-only roles report `declared_role_not_deployable` without running
  Cargo or mutating project state.

- The same working tree adds deployment-truth receipt completeness checks to
  `canic medic deployment <deployment>`: complete succeeded receipts report
  `deployment_truth_complete`, missing/unfinished receipts warn as
  `deployment_truth_incomplete`, and partial post-mutation receipts fail.

- Missing deployment-target medic runs now emit exact-match project-config hints
  when the requested deployment name matches a known fleet template
  (`fleet_name_deployment_name_conflated`) or role
  (`role_name_deployment_name_conflated`).

- Deployment-scoped medic also smoke-checks installed deployment registry
  observation through the existing resolver, emitting
  `deployment_registry_observed`, `deployment_registry_empty`,
  `deployment_registry_unavailable`, or `deployment_registry_not_evaluated`
  before targeted blob-storage/auth diagnostics.

- Targeted blob-storage medic failures now keep the stable target-resolution
  codes promised by the 0.78 design: `blob_storage_target_missing`,
  `blob_storage_target_ambiguous`, and `blob_storage_target_not_blob_storage`.

- The 0.78.4 medic readiness slice classifies invalid targeted
  auth-renewal issuers as
  `auth_renewal_issuer_invalid` before treating other auth-renewal failures as
  `auth_renewal_drift_fail`, and by distinguishing missing ICP CLI binaries as
  `icp_cli_missing` instead of the generic `icp_cli_incompatible`. It also
  keeps `local_network_implicit` / `local_network_explicit` project-only so
  deployment medic relies on its deployment-scoped network check instead of
  emitting duplicate network diagnostics. Blob-storage target resolution now
  follows the 0.78 design order by treating principal text as a canister ID
  before falling back to role names. The same released slice updates active
  `canic install` collision guidance to point at
  `canic medic deployment <deployment>` instead of the removed
  `canic info medic <deployment>` route, removes the same retired `info medic`
  leaf from top-level global ICP/network option forwarding, and keeps medic
  subcommand help usage-only:
  `canic medic project --help` and `canic medic deployment --help` render medic
  usage instead of entering project/deployment report construction, including
  when medic-local flags such as `--json` appear around the subcommand. The
  same slice wraps unbroken long diagnostic values within `MEDIC_REPORT_WIDTH`.

- The 0.78.5 slice retargets the auth-renewal installed/packaged CLI proof
  helper from the removed `canic info medic` route to
  `canic medic deployment <deployment> --auth-renewal <issuer>`, makes the
  fixture satisfy deployment medic's project-level preconditions, and asserts
  the current medic `auth_renewal_drift_warn` output shape.

- 0.77 completed the wasm-footprint feature-boundary line, including
  chain-key/root-publication feature splitting and local DTO replacements for
  helper crate fan-in. Current dependency work may include local
  `ic-memory` surface adjustments; preserve those edits if present.

- 0.76 bridge-free delegated auth is closed. Delegated-token `RootProof` is
  chain-key-only: `RootProof::IcChainKeyBatchSignatureV1`. The old
  bridge-backed canister-signature delegated root-proof renewal path is
  historical documentation only, not public runtime/API/CLI code or active auth
  stable state.

## Open Work

- No open or deferred 0.83 audit findings remain. `0.83.29` is published and
  tagged. The accepted 0.84 design, all three implementation slices, and the
  0.84.2 memory-map correction are published. Do not invent a fourth 0.84
  architecture slice.

- Continue the passive 0.80 state-contract direction through the 0.84 design:
  expand owner-provided Rust state descriptors and add more precise `*Data`
  snapshot declarations and migration coverage metadata. Do not add migration
  execution, stable-memory inspection, state dump/explore commands, generated
  manifest writes, runtime introspection endpoints, or mutation semantics.

- For subsequent automated development, run only the targeted gates for the
  touched behavior as required by `AGENTS.md`. The maintainer owns full
  workspace, deployment, and publish validation. Do not assign a new patch
  version or change Cargo package versions unless the maintainer explicitly
  asks for release preparation.

## Queued After 0.84

No queued 0.84 release fix remains. Continue the passive state-contract work
one owner/domain group at a time; do not turn it into migration execution or a
new role-contract architecture.

## Useful Validation

Focused post-0.84.9 auth/replay snapshot validation (passing):

```text
cargo check --locked -p canic-core
cargo test --locked -p canic-core auth_and_replay_descriptors_reference_canonical_data_types --lib
cargo test --locked -p canic-core auth_state_round_trips_through_canonical_data_snapshot --lib
cargo test --locked -p canic-core storage::stable::replay::tests --lib
cargo test --locked -p canic-core financial_history_descriptors_reference_canonical_data_types --lib
cargo test --locked -p canic-core cycle_history_round_trips_through_canonical_data_snapshots --lib
cargo test --locked -p canic-core icp_refill_records_round_trip_through_canonical_data_snapshot --lib
cargo test --locked -p canic-core hub_self_refill_resumes_in_flight_and_retryable_records --lib
cargo clippy --locked -p canic-core --lib --tests -- -D warnings
```

Focused post-0.84.8 runtime environment snapshot validation (passing):

```text
cargo check --locked -p canic-core
cargo test --locked -p canic-core runtime_env_descriptors_reference_canonical_data_types --lib
cargo test --locked -p canic-core ops::runtime::env::tests --lib
cargo test --locked -p canic-core storage::stable::state:: --lib
cargo test --locked -p canic-core ops::storage::state::app::tests --lib
cargo test --locked -p canic-core access::expr::tests --lib
cargo test --locked -p canic-core authorize_request_cycles_ --lib
cargo clippy --locked -p canic-core --lib --tests -- -D warnings
```

Focused post-0.84.7 topology registry snapshot validation (passing):

```text
cargo check --locked -p canic-core
cargo test --locked -p canic-core topology_registry_descriptors_reference_canonical_data_types --lib
cargo test --locked -p canic-core ops::topology::index::builder::tests --lib
cargo test --locked -p canic-core storage::stable::registry::subnet::tests --lib
cargo test --locked -p canic-core ops::storage::registry::app::tests --lib
cargo test --locked -p canic-core registry_policy_seam --lib
cargo test --locked -p canic-core index_addressing --lib
cargo clippy --locked -p canic-core -p canic-control-plane --lib --tests -- -D warnings
```

Focused post-0.84.6 blob-storage snapshot validation (passing):

```text
cargo check --locked -p canic-core
cargo check --locked -p canic-core --features blob-storage
cargo check --locked -p canic-core --features blob-storage-billing
cargo test --locked -p canic-core blob_storage_descriptors_reference_canonical_data_types --lib
cargo test --locked -p canic-core --features blob-storage storage::stable::blob_storage::tests --lib
cargo test --locked -p canic-core --features blob-storage ops::blob_storage::lifecycle::tests --lib
cargo test --locked -p canic-core --features blob-storage-billing billing_state_exports_through_canonical_data_snapshot --lib
```

Focused 0.84.6 placement snapshot validation (passing):

```text
cargo check --locked -p canic-core
cargo test --locked -p canic-core placement_descriptors_reference_canonical_data_types --lib
cargo test --locked -p canic-core pool_selection_uses_workflow_ordering --lib
cargo test --locked -p canic-core ops::storage::placement::directory::tests --lib
```

Focused 0.84.6 sharding snapshot validation (passing):

```text
cargo check --locked -p canic-core --features sharding
cargo test --locked -p canic-core sharding_descriptors_reference_canonical_data_types --lib
cargo test --locked -p canic-core --features sharding ops::storage::placement::sharding::tests --lib
cargo test --locked -p canic-core --features sharding workflow::placement::sharding::release::tests --lib
```

Focused 0.84.5 topology snapshot validation (passing):

```text
cargo test --locked -p canic-core ops::topology::index::builder::tests --lib
cargo test --locked -p canic-core index_addressing --lib
cargo test --locked -p canic-core topology_invariants_live_in_policy --lib
cargo test --locked -p canic-core descriptors_cover_declared_core_memory_ids --lib
cargo test --locked -p canic-core topology_index_descriptors_reference_canonical_data_types --lib
cargo test --locked -p canic-host complete_descriptor_registry_satisfies_state_audit_metadata_contract --lib
cargo clippy --locked -p canic-core -p canic-control-plane --lib --tests -- -D warnings
cargo fmt --all -- --check
git diff --check
```

Focused 0.84.2 memory-map validation (passing):

```text
cargo test --locked -p canic-core --lib
cargo test --locked -p canic-core placement_capabilities_select_only_their_placement_state --lib
cargo test --locked -p canic-core icp_refill_config_requires_its_feature_and_selects_its_state --lib
cargo test --locked -p canic-core --test stable_memory_abi_guard -- --nocapture
cargo test --locked -p canic-control-plane --lib
cargo test --locked -p canic-host --lib
cargo test --locked -p canic-host placement_roles_materialize_exact_placement_state --lib
cargo test --locked -p canic-cli --lib
cargo clippy --locked -p canic-core -p canic-control-plane -p canic-host -p canic-cli --all-targets --all-features -- -D warnings
cargo test --locked -p canic --test changelog_governance -- --nocapture
make fmt-check
git diff --check
```

Focused post-0.84 durable-write validation (passing):

```text
cargo test --locked -p canic-host durable_io --lib
cargo test --locked -p canic-host release_set --lib
cargo test --locked -p canic-host install_root --lib
cargo test --locked -p canic-host artifact_io --lib
cargo clippy --locked -p canic-host --lib --tests -- -D warnings
cargo fmt --all -- --check
git diff --check
```

Focused post-0.84 observation-loss validation (passing):

```text
cargo test --locked -p canic-cli metrics::transport --lib
cargo test --locked -p canic-cli cycles::transport --lib
cargo test --locked -p canic-cli cycles::tests --lib
cargo test --locked -p canic-cli list::live --lib
cargo test --locked -p canic-cli blob_storage::tests --lib
cargo test --locked -p canic-cli auth::tests --lib
cargo clippy --locked -p canic-cli --lib --tests -- -D warnings
```

Focused 0.84.1 typed-failure-classification validation (passing for the patch
isolated against released 0.84.0):

```text
cargo test --locked -p canic-core -p canic-control-plane -p canic-host -p canic-cli -p canic-backup --lib
cargo test --locked -p canic --lib
cargo test --locked -p canic --test protocol_surface
cargo check --locked -p canic-wasm-store
cargo check --locked -p canic-tests --test root_suite
cargo check --locked -p canic-control-plane --no-default-features
cargo check --locked -p canic-control-plane --no-default-features --features wasm-store-canister
cargo test --locked -p canic-host internal_pocketic_packages_are_validated_before_the_marker_is_granted --lib
cargo test --locked -p canic-tests --test pic_sharding_bootstrap -- --test-threads=1 --nocapture
cargo clippy --locked -p canic-core -p canic-control-plane -p canic-host -p canic-cli -p canic-backup -p canic -p canic-wasm-store --all-targets --all-features -- -D warnings
cargo clippy --locked -p canic-host -p canic-testing-internal -p canic-tests --all-targets --all-features -- -D warnings
cargo test --locked -p canic --test changelog_governance -- --nocapture
bash scripts/ci/check-diagnostic-consistency-audit.sh
bash scripts/ci/check-recovery-runbooks.sh
cargo fmt --all -- --check
git diff --check
```

Focused 0.84 Slice A validation:

```text
cargo check --locked -p canic-wasm-store
cargo check --locked -p root_probe
cargo test --locked -p canic-core role_contract --lib
cargo test --locked -p canic-core -p canic-control-plane -p canic -p canic-cli --lib
cargo clippy --locked -p canic-core -p canic-control-plane -p canic -p canic-cli -p canic-wasm-store --all-targets --all-features -- -D warnings
```

Focused 0.84 Slice B validation:

```text
cargo test --locked -p canic-host role_contract --lib
cargo test --locked -p canic-host generated_wasm_store_wrapper --lib
cargo test --locked -p canic-cli medic --lib
cargo test --locked -p canic --lib
cargo clippy --locked -p canic-core -p canic-host -p canic -p canic-cli --all-targets --all-features -- -D warnings
cargo run --locked -p canic-cli -- build --profile fast --workspace <workspace> --icp-root <icp-root> --config <canic.toml> <fleet> <role>
```

The corresponding direct `cargo build --target wasm32-unknown-unknown` is an
expected rejection, not a passing validation command.

Focused 0.84 Slice C validation:

```text
cargo test --locked -p canic-core state_contract --lib
cargo test --locked -p canic-host role_contract::descriptor --lib
cargo test --locked -p canic-host state_manifest --lib
cargo test --locked -p canic-host configured_role_capabilities --lib
cargo test --locked -p canic-cli state --lib
cargo test --locked -p canic-cli medic --lib
cargo clippy --locked -p canic-core -p canic-control-plane -p canic-host -p canic-cli --all-targets --all-features -- -D warnings
```

Focused 0.83 closeout validation:

```text
cargo test --locked -p canic-host deployment_truth::tests::execution_receipts::resume --lib
cargo test --locked -p canic-host promotion --lib
cargo test --locked -p canic-host deployment_truth --lib
cargo clippy --locked -p canic-host --all-targets -- -D warnings
cargo test --locked -p canic --test changelog_governance
```

Focused 0.78 medic validation:

```text
cargo test --locked -p canic-cli medic
cargo test --locked -p canic-cli status
cargo test --locked -p canic-host deployment_truth --lib
```

Broader CLI validation after command-surface edits:

```text
cargo test --locked -p canic-cli
```

Retained auth validation when a change touches live delegated-auth behavior:

```text
cargo check --locked -p canic-core -p canic
cargo test --locked -p canic-core chain_key --lib
cargo test --locked -p canic-core chain_key_batch --lib
cargo test --locked -p canic-core workflow::runtime::auth --lib
cargo test --locked -p canic --test protocol_surface
cargo check --locked -p canic-tests --test root_suite
```

Focused aliases added for ordinary local iteration:

```text
make test-auth
make test-auth-chain-key
make test-cli
make test-runtime-fast
```

When PocketIC is available and the change touches live 0.76 auth behavior,
run:

```text
POCKET_IC_BIN=/home/adam/projects/canic/.tmp/test-runtime/pocket-ic-server-14.0.0/pocket-ic \
  cargo test --locked -p canic-tests --test root_suite auth_076 -- --nocapture --test-threads=1
```

## Standing Constraints

- Preserve dirty worktree state and keep edits scoped.
- Do not change Cargo versions, workspace dependency versions, release script
  defaults, install URLs, or matching lockfile package versions unless the
  maintainer explicitly requests a version bump or release-preparation change.
- Follow `docs/governance/ci-deployment.md` for command, git, versioning, and
  release boundaries.
- Follow `docs/governance/changelog.md`; ordinary development goes under root
  `CHANGELOG.md` `Unreleased` when release notes are warranted.
- Follow `docs/governance/code-hygiene/README.md`; use directory modules with
  `mod.rs`, keep DTOs passive, and keep dependency direction
  `endpoints -> workflow -> policy -> ops -> model/storage`.
