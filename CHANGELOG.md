# Changelog

All notable, and occasionally less notable changes to this project will be
documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## Unreleased

## [0.81.x] - 2026-07-04 - Runtime Introspection

Detailed patch breakdown: [docs/changelog/0.81.md](docs/changelog/0.81.md)

- `0.81.5` surfaces deterministic guarded runtime feature metadata in
  `canic_runtime_status` and mirrors that feature inventory in
  `canic inspect` reports.

- `0.81.4` hardens runtime-introspection privacy and report contracts by
  completing field-visibility metadata, bounding runtime-facing diagnostic
  labels, and replacing raw inspect Candid fallback output with presence
  metadata.

- `0.81.3` hardens runtime-introspection closeout coverage by proving the live
  PocketIC endpoint DTOs and controller guards, and by pinning rejected
  `canic inspect` command expansions.

- `0.81.2` makes runtime-introspection DTOs roundtrip through Candid correctly
  and lets `canic inspect` decode typed runtime status, including failing
  status exit behavior.

- `0.81.1` connects deployment medic observations to explicit runtime
  inspection next actions and hardens `canic inspect` report metadata for
  Candid response bytes versus rendered Candid fallback.

- `0.81.0` adds guarded runtime-introspection endpoints and top-level
  `canic inspect` commands for one explicit live canister target, with typed
  runtime reports, timer/state metadata projections, a bounded heap-only
  recent-failure ring, and help wording that separates live inspection from
  deployment-truth artifact inspection.

## [0.80.x] - 2026-07-04 - Packaged Wasm Store Bootstrap

Detailed patch breakdown: [docs/changelog/0.80.md](docs/changelog/0.80.md)

- `0.80.10` closes the state-audit cleanup follow-ups by extending manifest
  declarations to control-plane state, normalizing snapshot metadata, and
  tightening removed-state and export/import audit coverage.

- `0.80.9` hardens top-level state-manifest validation for manifest schema
  compatibility and duplicate canister-role ownership.

- `0.80.8` hardens state-audit metadata validation for support windows,
  migration declarations, and duplicate domain ownership.

- `0.80.7` makes allocated-but-unmodeled stable-memory IDs explicit in state
  manifests and separates declaration-only metadata from heap-only runtime
  state.

- `0.80.6` makes local mutation boundaries discoverable in fleet/scaffold help
  and adds dry-run previews for source/config mutators.

- `0.80.5` surfaces `canic state audit` status inside project-level medic as a
  diagnostic-only readiness check while preserving state-audit ownership.

- `0.80.4` improves auth feature-gate developer experience with concise CI
  medic output, copy-pasteable manual Cargo.toml snippets, scaffold guidance,
  and explicit config-to-feature documentation.

- `0.80.3` adds config-driven medic and build-time checks for runtime Canic
  feature gates implied by fleet auth settings.

- `0.80.2` expands diagnostic state-manifest coverage and reserves removed
  stable-memory IDs so active domains cannot reclaim them without an explicit
  migration design.

- `0.80.1` adds diagnostic state manifest and audit surfaces backed by
  Rust-authored root state declarations, without stable-memory inspection,
  migration execution, generated manifest writes, or canister mutation.

- `0.80.0` fixes packaged downstream root `fast` builds for the implicit
  `wasm_store`, keeping bootstrap-store source resolution and profile handling
  owned by Canic instead of downstream projects.

## [0.79.x] - 2026-07-02 - Declarative Deployment Plan

Detailed patch breakdown: [docs/changelog/0.79.md](docs/changelog/0.79.md)

- `0.79.12` hardens deploy-plan evidence/truth boundaries by documenting the
  JSON report as diagnostic output and expanding renderer guards against
  truth, evidence, authorization, and apply-safety claims.

- `0.79.11` tightens deploy-plan future-apply preview output by surfacing
  policy-application labels, aligning text rows with JSON preview fields, and
  documenting that preview rows are non-executed labels only.

- `0.79.10` mirrors deploy-plan config path and network context into passive
  verified facts so automation can consume the same invocation context exposed
  by the top-level report fields.

- `0.79.9` expands deploy-plan report visibility with artifact upload preview
  labels, artifact-specific next actions, and passive planner context facts
  while preserving the no-mutation boundary.

- `0.79.8` hardens deploy-plan report determinism by centralizing report and
  assumption vocabulary, deduplicating future-apply preview labels, and pinning
  diagnostic severity ordering.

- `0.79.7` hardens the deploy-plan text/report contract by naming explicit
  preview fields and pinning exit-code, help, and deterministic ordering
  behavior.

- `0.79.6` aligns deploy-plan text output with the stable report model by
  adding schema, command, and diagnostic source provenance.

- `0.79.5` expands deploy-plan verified facts for fleet template,
  controller, artifact, pool, and trust-domain resolution already present in
  the embedded plan, while preserving the no-mutation report boundary.

- `0.79.4` tightens deploy-plan visibility for verifier-readiness and expected
  canister inventory already present in the embedded plan, while keeping output
  diagnostic-only and non-mutating.

- `0.79.3` completes deploy-plan preview/report classification by surfacing
  root/child registration labels and explicit unsupported-shape diagnostics
  without adding apply semantics or mutation.

- `0.79.2` expands deploy-plan future-apply preview labels for configured
  pool canister creation and desired controller reconciliation while keeping
  them non-executed planning labels only.

- `0.79.1` tightens deploy-plan report facts and JSON contract coverage by
  surfacing only resolved config, topology, authority, and artifact observations
  already present in the embedded deployment plan.

- `0.79.0` adds the no-mutation `canic deploy plan <deployment>` planning
  surface with text/JSON reports, safe JSON `--out`, installed-state comparison
  status, hard-cut rejected forms, and medic handoffs to the planner.

## [0.78.x] - 2026-07-01 - Top-Level Medic Preflight

Detailed patch breakdown: [docs/changelog/0.78.md](docs/changelog/0.78.md)

- `0.78.6` closes the medic preflight audit follow-ups by selecting unique
  recorded deployment networks before local fallback, fixing non-local readiness
  source labels, and documenting the final stable check codes.

- `0.78.5` aligns installed and packaged downstream CLI proof fixtures with
  the hard-cut medic auth-renewal surface and its project-level preconditions.

- `0.78.4` hardens medic readiness and command-surface behavior by preserving
  precise ICP/auth/target diagnostics, removing retired `info medic`
  survivorship, keeping subcommand help usage-only, and wrapping long
  diagnostic values.

- `0.78.3` expands medic preflight with project package-metadata checks,
  deployment-truth and registry-observation diagnostics, deployment-name
  conflation hints, and precise blob-storage target-resolution codes.

- `0.78.2` tightens deployment-scoped medic by reporting invalid installed
  deployment record network/root preconditions before live root readiness
  checks.

- `0.78.1` tightens the medic CLI exit-code contract and JSON-only output
  regression coverage while cleaning up drop-lifetime warnings for the
  `significant_drop_tightening` lint.

- `0.78.0` promotes medic to the top-level `canic medic` preflight surface
  with project/deployment scopes, schema-versioned text/JSON reports, and
  hard-cut removal of the active `canic info medic` route.

## [0.77.x] - 2026-07-01 - Wasm Footprint Feature Boundaries

Detailed patch breakdown: [docs/changelog/0.77.md](docs/changelog/0.77.md)

- `0.77.1` removes the remaining ICRC ledger and management-canister helper
  crate fan-in from ordinary canister builds while preserving ICP refill,
  HTTP outcall, and test/fleet management-call behavior through local Candid
  DTOs.

- `0.77.0` splits chain-key ECDSA and wasm-store control-plane features so
  store/control-plane-only wasm builds avoid unnecessary auth/signing and
  root-publication fan-in while preserving delegated-token verification,
  root renewal, and root publication behavior.

## [0.76.x] - 2026-06-30 - Bridge-Free Delegated Auth

Detailed patch breakdown: [docs/changelog/0.76.md](docs/changelog/0.76.md)

- `0.76.13` closes the 0.76 audit follow-up by tightening chain-key verifier
  config validation, delegated-token verifier feature boundaries, auth-renewal
  CLI surface proofing, and deployment-truth executor cleanup.

- `0.76.12` tightens deployment-truth report producer module boundaries by
  making leaf-local diagnostic constants private while preserving intentional
  test and sibling report consumer imports.

- `0.76.11` continues deployment-truth diagnostic-code cleanup across
  authority, executor, comparison, root-verification, and receipt artifact
  gates without changing serialized output or delegated-auth runtime behavior.

- `0.76.10` centralizes deployment-truth diagnostic codes and diff categories
  in their owning host report modules without changing serialized output or
  delegated-auth runtime behavior.

- `0.76.9` cleans up host/operator diagnostics and active auth docs after the
  hard cut, replacing stale canister-signature and deployment-state wording,
  clarifying removed auth command-tail tests, and centralizing touched
  warning/finding identifiers without changing runtime auth behavior.

- `0.76.8` finishes the post-release auth structure pass by keeping delegated
  auth on the chain-key path while splitting protocol/API ownership into
  smaller concern modules, aligning active configuration docs, compacting the
  handoff, and adding focused local validation aliases.

- `0.76.7` completes the pre-1.0 auth cleanup pass by tightening
  chain-key auth operator wording, clarifying role-attestation data-certificate
  errors, refreshing recurring audit templates, and splitting the chain-key
  batch implementation into smaller private modules without behavior changes.

- `0.76.6` hard-cuts delegated-auth root proof survivorship by making
  delegated `RootProof` chain-key-only, splitting role-attestation root proof
  material into a separate DTO, removing historical bridge/provisioner auth
  stable fields, and marking superseded bridge-era auth audits as historical.

- `0.76.5` stops serializing empty historical bridge delegated-auth stable
  fields while keeping populated legacy values decode-compatible for upgrades.

- `0.76.4` removes unused delegated-auth stable-record scaffolding for
  non-persisted root-key policy and registry snapshots, and documents the
  retained bridge renewal/provisioner stable fields as decode-only.

- `0.76.3` removes the remaining top-level CLI global-option forwarding
  compatibility for the deleted `auth renewal run-once` and
  `auth renewal provisioner` command tails.

- `0.76.2` removes the old bridge-backed delegated-auth root-proof
  provisioning endpoints, DTO/API/CLI surfaces, replay rows, and provisioner
  access predicate, leaving delegated-token liveness on chain-key renewal and
  issuer lazy repair only.

- `0.76.1` hardens chain-key batch retry/install state and makes local
  workspace test runs avoid redundant clippy and PocketIC wasm cache churn
  while preserving CI cleanup behavior.

- `0.76.0` replaces bridge-backed delegated-auth renewal with chain-key batch
  root proofs signed by root through management-canister ECDSA, including
  timer renewal, issuer lazy repair, explicit `chain_key_batch` trust anchors,
  multi-issuer batching, and hard rejection of legacy bridge root-proof
  provisioning in chain-key mode. Potentially breaking for deployments or
  tooling that still depend on bridge-backed root proof renewal.

## [0.75.x] - 2026-06-28 - Release Publishing Recovery

Detailed patch breakdown: [docs/changelog/0.75.md](docs/changelog/0.75.md)

Note: `0.75.0` was accidentally published and yanked. `0.75.1` is the first
supported `0.75` release.

- `0.75.1` fixes release publishing resumptions by checking exact crates.io
  versions instead of search results, so patch-line recovery can continue after
  a newer or yanked version exists.

## [0.74.x] - 2026-06-27 - Root-Managed Delegation Renewal

Detailed patch breakdown: [docs/changelog/0.74.md](docs/changelog/0.74.md)

- `0.74.14` closes the follow-up structure/audit pass by completing the
  root-renewal and blob-storage module splits, keeping public behavior
  unchanged while recording the refreshed module-structure and change-friction
  audit results.

- `0.74.13` reduces root-managed renewal change friction by splitting the
  largest core renewal ops and CLI auth modules into smaller responsibility
  modules without changing operator behavior.

- `0.74.12` narrows internal RPC capability workflow handler visibility and
  refreshes the capability-scope audit so its counted capability-facing surface
  stays below the growing-surface threshold.

- `0.74.11` stabilizes the retained scheduled-renewal PocketIC sharding
  scenario by isolating its timer-dependent root setup from cached snapshot
  restores while simplifying the shared sharding test setup helpers.

- `0.74.10` closes out the 0.74 renewal design note so it reflects the shipped
  root-managed renewal surface and leaves daemon/host bridge operation as the
  remaining follow-up.

- `0.74.9` adds bounded issuer-level renewal attempt metrics so root-managed
  renewal exposes scheduled, completed, retryable, expired, disabled, and
  drift-capable outcomes alongside the existing sweep/retrieval/install
  counters.

- `0.74.8` finishes the operator documentation pass for root-managed renewal
  and tightens renewal startup/error-code hygiene, including prompt sweeps after
  enabling templates and auth-layer public-error constructors.

- `0.74.7` hardens root-managed renewal recovery by recording prepare-stage
  failures per issuer, pruning expired scheduled batch transport records, and
  rejecting expired provisioner installs before issuer calls.

- `0.74.6` refreshes root-managed renewal state after successful
  controller/manual proof installs when the proof exactly matches the issuer's
  enabled renewal template.

- `0.74.5` makes renewal template disable deterministic by cancelling any
  active scheduled issuer attempt and exposing it as `Disabled` without
  increasing the failure count.

- `0.74.4` adds `canic auth renewal provisioner` list/enable/disable commands
  for the constrained renewal provisioner ACL and extends retained CLI proofs
  across those operator flows.

- `0.74.3` extends retained packaged/installed CLI proofs across auth renewal
  help, no-work bridge runs, drift status, and medic drift output, while
  fixing top-level `--network`/`--icp` forwarding for `auth renewal status`.

- `0.74.2` adds passive root-vs-issuer drift reporting to
  `canic auth renewal status` and surfaces the same delegated-auth renewal
  diagnostic through targeted medic output.

- `0.74.1` adds an operator-facing auth renewal status command and fixes the
  retained scheduled-renewal PocketIC scenario so it validates repeated renewal
  cycles with fresh delegated-token replay request ids.

- `0.74.0` adds root-managed delegated-auth proof renewal: root-owned issuer
  renewal templates/status, scheduled renewal attempts and constrained bridge
  retrieval/install flows, renewal provisioner ACLs, bounded renewal
  metrics/logging, the `canic auth renewal run-once` bridge command, and the
  design/operator notes for the direct-query renewal model.

## [0.73.x] - 2026-06-26 - Post-Hardening Recovery Polish

Detailed patch breakdown: [docs/changelog/0.73.md](docs/changelog/0.73.md)

- `0.73.1` replaces the root-subnet evidence check's `icq` process call with
  the `ic-query` 0.5.11 shared library, removes repo-managed `icq` tooling, and
  leaves standalone NNS inspection to the upstream `ic-query-cli` package.

- `0.73.0` tightens post-hardening recovery behavior: delegated-token prepare
  returns stable auth-proof error codes, wasm-store publication and clear calls
  use bounded waits, inventory drift reports recoverable workflow errors, and
  the maintainer ICP CLI pin moves to `icp-cli 1.0.1`.

## [0.72.x] - 2026-06-26 - Security Defaults Hardening

Detailed patch breakdown: [docs/changelog/0.72.md](docs/changelog/0.72.md)

- `0.72.0` hardens access and funding defaults: endpoint openness must be
  explicit, whitelists fail closed when absent, child cycles funding has finite
  stable accounting, and wasm-store GC completion blocks concurrent clear runs.
  Potentially breaking for code that relied on implicit public endpoints,
  missing whitelist config, or unlimited child funding.

## [0.71.x] - 2026-06-23 - Blob Storage Operator Readiness

Detailed patch breakdown: [docs/changelog/0.71.md](docs/changelog/0.71.md)

- `0.71.8` splits CI validation across fresh runner jobs for static checks,
  full unit/PocketIC tests, and build validation, while disabling restored
  `target/` caches so generated Rust/wasm artifacts no longer exhaust disk
  space before tests start.

- `0.71.7` makes `canic info metrics` text output compact by default, splits
  performance metric payloads into count and average-per-call columns, and
  keeps full raw diagnostics available through `--verbose` and unchanged JSON.

- `0.71.6` adds a read-only `canic blob-storage status --check-ready`
  automation mode that preserves normal status output while exiting `4` when
  uploads are not ready.

- `0.71.5` cleans up the blob-storage CLI output contract by centralizing
  stable readiness, funding, warning, Candid-source, and command-error codes in
  the render-ready model layer while preserving the existing operator command
  surface.

- `0.71.4` closes the blob-storage operator-loop validation gap with a private
  CLI runtime seam, scripted status/sync/fund/recheck coverage, and shared
  packaged/installed CLI fixture proofs for dry-run, live sync, live fund, and
  final readiness JSON output.

- `0.71.3` hardens blob-storage operator-readiness release validation by
  extending installed and packaged CLI proofs to cover blob-storage help and
  structured JSON error output, while clarifying ICP CLI upgrade guidance.

- `0.71.2` completes live blob-storage provisioning CLI execution and targeted
  medic diagnostics, including structured `--json` error reports and the
  versionless operator runbook.
  ```bash
  canic blob-storage sync-gateways <deployment> <canister-or-role>
  canic blob-storage fund <deployment> <canister-or-role> --cycles <amount>
  canic info medic <deployment> --blob-storage <canister-or-role>
  ```

- `0.71.1` adds live `canic blob-storage status` support, calling the guarded
  0.70 status endpoint with read-only status semantics and rendering stable
  JSON/plain readiness output while keeping sync/fund mutation transport
  deferred.

- `0.71.0` starts the blob-storage operator-readiness line with the
  first-class `canic blob-storage` command group, strict funding input parsing,
  installed-target and local-Candid validation for dry-run gateway sync/funding
  previews, and the design contract for completing live status and provisioning
  support. Live status, sync, and fund transport remain intentionally gated for
  follow-up slices.

## [0.70.x] - 2026-06-20 - Blob Storage Billing MVP

Detailed patch breakdown: [docs/changelog/0.70.md](docs/changelog/0.70.md)

- `0.70.17` continues the cleanup/audit closeout by refreshing the ops,
  workflow, auth-abstraction, and lifecycle recurring audits, recording their
  retained June 22 reports, and narrowing small host helper surfaces without
  changing runtime behavior.

- `0.70.16` closes the post-`0.70.15` cleanup/audit pass by fixing a
  blob-storage API layer leak around billing config storage records,
  refreshing the recurring layer/access/audience audit definitions, and
  recording the retained June 22 audit reports.

- `0.70.15` cleans up the blob-storage PocketIC test readability by
  centralizing probe/mock method names as constants and adding clear helper
  section banners for the gateway, billing, Cashier failure, funding,
  install/config, lifecycle, and upgrade paths. It also adds typed helper
  wrappers for repeated funding, Cashier balance, mock last-top-up, and probe
  counter calls, plus named fixture values and helper wrappers for gateway
  sync, direct Cashier sync, billing status, and one-shot mock failure
  controls. The long billing wrapper scenario now reads as named phases for
  setup, failure recovery, rejection cases, and final funding metadata.

- `0.70.14` hardens blob-storage funding failure recovery coverage by proving
  a transient Cashier top-up failure through the generated funding endpoint
  releases the in-flight funding guard and allows an immediate subsequent
  valid top-up to succeed. It also proves malformed Cashier top-up success
  payloads release the same guard, tightens reserve-skipped funding coverage to
  preserve prior top-up metadata, and adds core guard coverage proving the
  transient funding lock is released during unwinding as well as normal drop.
  The in-flight funding error mapping is pinned to the public `Conflict` code.

- `0.70.13` pins the blob-storage billing configuration DTO protocol surface
  by adding Candid roundtrip and field-shape guards for
  `BlobStorageBillingConfig`, covering the operator-facing Cashier/reserve/
  balance/gateway-limit configuration contract. It also guards that the
  generated billing endpoint macro does not expose billing configuration as a
  public admin surface, and adds config validation coverage for oversized
  Candid `nat` values.

- `0.70.12` hardens direct Cashier gateway-sync coverage by proving the
  lower-level sync helper normalizes duplicate Cashier gateway principals,
  rejects too many distinct gateway principals with `InternalRpcMalformed`
  before mutating sync metadata, and recovers with a fresh timestamp when the
  configured maximum allows the distinct gateway set.

- `0.70.11` hardens Cashier gateway-sync failure coverage by proving empty and
  invalid Cashier gateway lists, and trapped Cashier gateway-list calls,
  preserve the last successful gateway-sync timestamp as well as the previously
  synced gateway registry. The trapped gateway-list path also proves a
  subsequent valid sync recovers and records a fresh success timestamp after
  the mock trap hook is explicitly cleared.

- `0.70.10` hardens Cashier gateway-principal sync by rejecting empty
  Cashier gateway lists as malformed before mutating local gateway state. The
  PocketIC billing flow now proves empty and invalid Cashier lists both fail
  with `InternalRpcMalformed` while preserving the previously synced gateway
  registry. It also aligns the Cashier protocol inventory with that behavior
  and makes the inventory gate require explicit gateway-list behavior fields,
  including the empty-list malformed/preserve-state invariant.

- `0.70.9` hardens blob-storage funding-report coverage with PocketIC tests
  proving successful project-cycle funding reports requested, attached, reserve,
  Cashier total, and cycle-balance metadata, and reserve-skipped funding reports
  zero attached cycles with unchanged project-cycle balance metadata. It also
  pins the funding report Candid shape, roundtrips both success and skipped
  reports through the protocol-surface tests, and guards that the generated
  funding endpoint keeps returning the structured top-up report.

- `0.70.8` expands blob-storage billing status coverage with PocketIC tests
  proving `get_blob_storage_status` reports endpoint-visible readiness blockers
  for missing gateway principals, insufficient Cashier balance, and
  reserve-blocked funding.

- `0.70.7` hardens blob-storage billing endpoint authorization with PocketIC
  coverage proving the generated gateway-sync, project-cycle funding, and
  billing-status endpoints reject non-controller callers with `Unauthorized`
  before reaching billing logic. It also serializes the blob-storage PocketIC
  test file around shared standalone wasm artifacts so full-file runs do not
  race upgrade wasm reads against concurrent probe builds.

- `0.70.6` hardens blob-storage billing upgrade behavior with PocketIC
  coverage proving billing config, Cashier-synced gateway principals, pending
  gateway deletion visibility, and last successful gateway-sync metadata
  survive a probe canister upgrade. It also proves explicit funding remains
  usable after upgrade, so the transient funding guard is not restored as a
  stale lock. The same upgrade path now pins that status-requested gateway sync
  remains read-only after upgrade and does not replace the synced gateway set.
  It also proves the explicit gateway-sync endpoint still uses the persisted
  billing config after upgrade and can replace the local gateway set.
  It also covers the no-billing-config upgrade path so status stays
  `NotConfigured`, local gateway state is preserved, and funding remains
  blocked until config exists.

- `0.70.5` makes blob-storage project-cycle funding all-or-nothing against the
  configured reserve: reserve-blocked requests now return a skipped report and
  attach zero cycles instead of partially topping up Cashier. It also rejects
  unsafe billing configs with zero upload-balance thresholds or gateway limits
  that cannot fit the target runtime. It consumes `ic-memory 0.7.1` and exposes
  a controller-only `canic_memory_ledger.memories` inventory of Canic stable
  memories with live backing sizes, while retaining raw allocation-record
  `memory_size` diagnostics.

- `0.70.4` maps malformed Cashier response decoding failures to the stable
  `InternalRpcMalformed` public error code and updates gateway-sync PocketIC
  coverage to pin the more precise error while preserving no-mutation behavior.
  It also distinguishes malformed Cashier balance payloads in backend billing
  status instead of reporting them as transient balance unavailability, and
  covers malformed top-up success payloads through the generated funding
  endpoint.

- `0.70.3` adds mock-Cashier one-shot failure controls and PocketIC coverage
  proving generated blob-storage status/funding endpoints surface Cashier
  balance and top-up failures with stable public state and error codes. It
  also pins that invalid Cashier gateway lists fail gateway sync without
  replacing the existing local gateway set.

- `0.70.2` rejects zero-cycle blob-storage project funding requests with
  `InvalidInput` instead of reporting a misleading reserve-violation skip, and
  maps known Cashier top-up failures to stable public error codes. It pins the
  funding attachment decision with unit and PocketIC coverage.

- `0.70.1` adds a transient in-flight guard around project-cycle funding so
  overlapping blob-storage funding calls fail with a typed conflict instead of
  double-submitting against stale observed state, and tightens billing-only
  feature gates plus read-only status decision coverage.

- `0.70.0` starts the blob-storage billing line with source-backed Cashier
  DTOs, typed Cashier wrappers, stable billing config, gateway-principal sync,
  project-cycle funding, read-only backend billing status, opt-in billing
  endpoint emission, and mock-Cashier PocketIC coverage.

## [0.69.x] - 2026-06-20 - Blob Storage Protocol Preflight

Detailed patch breakdown: [docs/changelog/0.69.md](docs/changelog/0.69.md)

- `0.69.5` strengthens blob-storage regression coverage for malformed hashes,
  canonical stable keys, idempotent lifecycle edges, and gateway confirmation
  behavior without changing the 0.69 runtime surface.

- `0.69.4` makes local blob-storage status easier to consume with a named
  counters DTO and pins create-certificate hash echo compatibility while
  keeping Cashier/billing surfaces deferred.

- `0.69.3` prepares the non-billing blob-storage MVP for downstream validation
  with an integration runbook, updated design/handoff status, local status
  counters, and gateway-principal revocation coverage while keeping
  Cashier/billing surfaces deferred.

- `0.69.2` continues the non-billing blob-storage backend by adding the
  gateway endpoint macro, standalone probe canister, and PocketIC lifecycle
  coverage for live roots, pending deletion, gateway filtering, and deletion
  confirmation across a post-upgrade round trip.

- `0.69.1` updates Canic's ICP CLI compatibility gate for the 1.x stable line
  and makes `canic info medic` guidance readable for terminal operators.

- `0.69.0` starts the blob-storage line as a protocol-preflight release by
  recording the source-backed gateway inventory requirement, documenting the
  current no-source finding, and hardening CI gates so implementation remains
  blocked until gateway and Toko compatibility evidence is complete.

## [0.68.x] - 2026-06-17 - Canister Signatures & Provisioning Gates

Detailed patch breakdown: [docs/changelog/0.68.md](docs/changelog/0.68.md)

- `0.68.26` closes the root-proof provisioning audit/hygiene pass by
  refreshing the oldest recurring audit definitions, recording clean
  invariant/change-friction reports, and marking the 0.68 line ready to hand
  focus back to blob-storage work.

- `0.68.25` restores full workspace clippy compliance after enabling
  missing-panic documentation, documenting intentional panic contracts and
  removing avoidable backup preflight panics.

- `0.68.24` hardens the layer-boundary guard for root provisioning drift and
  normalizes ops-layer module hygiene around runtime and storage boundaries.

- `0.68.23` removes obsolete root issuer test-material provisioning hooks and
  fixes root provisioning layer-boundary drift found by the audit.

- `0.68.22` adds the controller-only root issuer policy upsert endpoint needed
  to register issuer canisters before root proof batch provisioning.

- `0.68.21` documents the implemented root proof provisioning MVP with a
  versionless operator runbook, source-map handoff notes, and issuer-canister
  terminology alignment for developer validation.

- `0.68.20` bounds root proof batch pending metadata with MVP quotas, prunes
  expired provisioning state opportunistically, and documents the retained
  signature-map leaf behavior.

- `0.68.19` aligns private root batch install workflow, test, and active-doc
  wording around issuer-local active-proof installation while preserving the
  public batch install protocol and outcome enum.

- `0.68.18` cleans up root/issuer canister-signature proof internals by relying
  on caller-bound pending keys and aligning the root batch provisioning helper
  wording with root-proof terminology, without changing the public protocol.

- `0.68.17` removes the legacy single-proof root delegation prepare/get route,
  leaving batch prepare, direct root query retrieval, and batch install as the
  only active root proof provisioning contract.

- `0.68.16` closes the root proof provisioning MVP regression loop by proving
  issuer nested-query retrieval fails for the root certificate-context reason
  and signer-local issuance survives root unavailability after proof install.

- `0.68.15` maps root proof retrieval failures without a root data certificate
  to the stable `RootDataCertificateUnavailable` error, making direct-query
  context failures distinguishable from ACL failures.

- `0.68.14` strengthens root proof provisioning MVP regression coverage for
  refresh/expiry status, expired signer-local issuance blocking, and partial
  batch install retry behavior.

- `0.68.13` finalizes the controller-only root provisioning MVP by documenting
  provisioner ACL as the later automation target, proving signer-local
  delegated-token issuance after root proof install, and making batch prepare
  request-id retries idempotent.

- `0.68.12` enables root delegation proof batch install by validating
  submitted proofs against pending batch metadata, broadcasting valid proofs to
  signer install endpoints, and returning per-signer outcomes with
  `AlreadyInstalled` idempotency.

- `0.68.11` enables direct root query retrieval for root delegation proof
  batches, returning prepared proofs from pending metadata while keeping
  provisioning out of issuer nested-query paths.

- `0.68.10` turns root delegation proof batch prepare into a real MVP prepare
  step that validates request metadata, certifies root signature leaves, caches
  pending batch metadata, and returns batch metadata.

- `0.68.9` wires root delegation proof batch prepare preflight to persisted
  root issuer registry state so issuers must be registered and policy-valid
  before provisioning continues.

- `0.68.8` adds pure root delegation proof issuer policy validation for issuer
  enablement, allowed audiences/grants, certificate TTL, and refresh timing.

- `0.68.7` starts the root provisioning hard cut by disabling issuer-driven
  root proof self-provisioning, pinning the batch provisioning protocol
  surface, and adding signer-local active-proof status for provisioners.

- `0.68.6` reserves 0.68 for root delegation proof provisioning repair and
  expands blob-storage gateway and Cashier inventory-gate regression coverage
  while later blob-storage implementation remains gated.

- `0.68.5` adds a Cashier protocol inventory scaffold and executable gate so
  0.70 blob-storage billing remains blocked until Cashier protocol provenance
  is recorded.

- `0.68.4` records local Toko blob/asset compatibility evidence and tightens
  blob-storage inventory gates so billing/Cashier work also remains blocked
  until gateway protocol provenance and `BlobRootHash` mapping are proven.

- `0.68.3` adds an executable blob-storage inventory gate so CI and local
  release/test gates block premature gateway endpoint, DTO, feature, and API
  implementation until the external protocol inventory is complete.

- `0.68.2` records an accidental version-only release artifact with no code,
  API, or behavior changes beyond release metadata and install-version
  references.

- `0.68.1` hardens delegated-token public prepare by binding issuance to the
  caller subject and limiting open self-issued grants to login/session scopes,
  preserving subnet-wide login while blocking self-granted privileged scopes.

- `0.68.0` completes the backup cleanup closeout by splitting
  `canic-backup` journal, manifest validation, persistence, runner operation,
  and plan internals into focused modules while preserving backup behavior.

## [0.67.x] - 2026-06-13 - IC query extraction

Detailed patch breakdown: [docs/changelog/0.67.md](docs/changelog/0.67.md)

- `0.67.50` continues `canic-backup` cleanup by splitting execution journal
  and backup plan tests into focused modules while preserving backup behavior.

- `0.67.49` continues `canic-cli` backup cleanup by splitting backup
  create/verify tests, option parsing, create execution, list/reference
  resolution, and dispatch/error ownership into focused modules while
  preserving backup CLI behavior.

- `0.67.48` continues `canic-cli` backup test cleanup by splitting backup
  inspect, list, options, reference, prune, status, and fixture coverage into
  focused modules while preserving backup CLI behavior.

- `0.67.47` continues `canic-cli` backup cleanup by splitting create
  planning, persistence, runner, rendering, and focused backup tests into child
  modules while preserving backup CLI behavior.

- `0.67.46` continues `canic-cli` cleanup by splitting evidence policy-gate
  envelope/render internals and backup test fixtures into focused child
  modules while preserving CLI behavior.

- `0.67.45` continues `canic-cli` evidence cleanup by splitting command
  construction, option parsing, envelope comparison, policy-gate handling, and
  tests into focused modules, with related API/design documentation cleanup.

- `0.67.44` continues `canic-cli` fleet cleanup by splitting command
  construction, option parsing, rendering, and adoption report internals into
  focused child modules while preserving CLI behavior.

- `0.67.43` continues `canic` endpoint macro cleanup by aligning endpoint
  emitter module documentation while preserving emitted endpoint surfaces.

- `0.67.42` continues `canic-core` replay-policy cleanup by splitting the
  manifest inventory into focused modules while preserving the public
  replay-policy API.

- `0.67.41` continues `canic-core` workflow cleanup by splitting pool and ICP
  refill internals into focused modules while preserving behavior and public
  call sites.

- `0.67.40` adds the Canic `release_partition_key` Rust API for reclaiming
  sharding assignments from the current pool owner with bounded release
  metrics.

- `0.67.39` continues canic-host cleanup by splitting release-set
  path/package-version helpers and install-root phase operations into focused
  modules while preserving host behavior, operator output, and public APIs.

- `0.67.38` continues canic-host release-set cleanup by splitting raw config
  projection, config mutation, and staging helpers into focused modules, and
  adds forward-looking blob-storage design notes while preserving host
  behavior and public APIs.

- `0.67.37` adds GitHub runner disk diagnostics around Rust cache and
  validation phases, and installs pinned ShellCheck through local developer
  setup while keeping pinned `icq` installs sourced from crates.io by default.

- `0.67.36` continues canic-host deployment-truth promotion cleanup by
  decomposing shared promotion support plus transform/readiness and execution
  receipt/wasm-store catalog internals into focused directory modules while
  preserving behavior and public APIs.

- `0.67.35` continues canic-host deployment-truth promotion cleanup by
  decomposing artifact identity, source-build materialization, wasm-store,
  policy, and provenance-report implementation internals into focused
  directory modules while preserving behavior and public APIs.

- `0.67.34` continues canic-host deployment-truth cleanup by decomposing the
  remaining large passive model files into focused directory modules while
  preserving JSON shapes, host behavior, and public APIs.

- `0.67.33` continues canic-host structural cleanup by decomposing build,
  promotion/comparison, policy-gate, and install-root test modules into
  focused directory modules while preserving host behavior and public APIs.

- `0.67.32` continues canic-host decomposition by splitting adoption,
  deployment-truth authority, and ICP command/query internals into focused
  modules while preserving host behavior and public APIs.

- `0.67.31` continues canic-host deployment-truth decomposition by splitting
  observation, receipt, and root-verification internals into focused modules
  while preserving host behavior and public APIs.

- `0.67.30` continues canic-host deployment-truth cleanup by moving shared test
  fixtures plus authority, lifecycle, promotion, and root-verification text
  rendering into focused directory modules without changing host behavior, text
  output, or public APIs.

- `0.67.29` finishes the large deployment-truth unit-test file split by
  moving authority, diff, local observation, execution receipt, and root
  verification tests into focused directory modules without changing host
  behavior or public APIs.

- `0.67.28` reorganizes the largest deployment-truth lifecycle and promotion
  unit-test files into focused directory modules without changing host
  behavior or public APIs.

- `0.67.27` continues canic-host deployment-truth lifecycle decomposition by
  splitting authority-plan, lifecycle digest, and lifecycle error internals
  into focused directory modules while preserving public host APIs.

- `0.67.26` continues canic-host deployment-truth lifecycle decomposition by
  splitting passive external lifecycle and external upgrade implementation
  logic into focused directory modules while preserving public host APIs.

- `0.67.25` decomposes the passive deployment-truth model surface into focused
  authority, comparison, lifecycle, promotion, and root-verification model
  modules while preserving public host APIs.

- `0.67.24` finishes the release-set config split by isolating raw-source
  projection helpers from file-backed wrappers and keeping release-set behavior
  unchanged.

- `0.67.23` continues canic-host decomposition by splitting policy-gate
  evaluation and release-set config mutation internals into focused directory
  modules while preserving public host APIs and operator behavior.

- `0.67.22` continues host/CLI boundary cleanup by centralizing readiness,
  subnet-registry, metadata, response parsing, and endpoint Candid parsing in
  canic-host while preserving operator command behavior.

- `0.67.21` centralizes cycle-balance querying in canic-host so list, cycles,
  and install-root paths share the same local-replica preference and ICP CLI
  fallback behavior.

- `0.67.20` decomposes canic-host direct local replica querying into focused
  transport, status, and wire-decoding modules with colocated tests while
  preserving the public replica-query API and CLI behavior.

- `0.67.19` continues local-query cleanup by centralizing direct
  `canic_subnet_registry` decoding in canic-host and using decoded registry
  roles for install-root readiness diagnostics before falling back to ICP CLI
  JSON.

- `0.67.18` continues canic-host install-root decomposition by isolating
  preparation, activation, install-state receipt, command-output,
  build-environment, build-target, local-cycle funding, readiness, and timing
  helpers while preferring decoded direct local replica queries for install
  readiness and cycle-balance checks.

- `0.67.17` continues canic-host install-root decomposition by isolating
  deployment registration, root-canister resolution, and read-only install
  truth/preflight helpers while preserving public host APIs.

- `0.67.16` continues canic-host install-root decomposition by isolating
  artifact-promotion receipts, install operations, plan artifacts, and current
  execution gate helpers while preserving install behavior.

- `0.67.15` continues canic-host install-root decomposition by isolating
  deployment-truth gate enforcement/rendering, execution-preflight receipts,
  release-set staging evidence, and phase-receipt scope handling while
  preserving install-root behavior and public host APIs.

- `0.67.14` continues canic-host deployment-truth report decomposition by
  isolating verifier-readiness and safety-report helpers while preserving
  deployment diff behavior and public report APIs.

- `0.67.13` continues canic-host deployment-truth report decomposition by
  isolating controller, module-hash, and runtime-config digest helpers while
  preserving deployment diff behavior.

- `0.67.12` continues canic-host deployment-truth report decomposition by
  isolating artifact, canister, and pool diff comparison helpers while
  preserving deployment diff behavior and public host APIs.

- `0.67.11` decomposes deployment-truth text rendering and starts
  deployment-truth report decomposition by isolating receipt-resume and
  root-subnet evidence helpers while preserving public host APIs.

- `0.67.10` completes the current deployment-truth lifecycle decomposition by
  isolating external-upgrade report construction and validation, and starts
  install-root decomposition by isolating command/build, root-verification, and
  receipt IO helpers while preserving the public host API.

- `0.67.9` continues deployment-truth lifecycle decomposition by isolating
  external lifecycle pending, check, handoff, and critical-fix report
  construction and validation while preserving the public lifecycle API.

- `0.67.8` continues deployment-truth lifecycle decomposition by isolating
  lifecycle authority-report and external lifecycle-plan construction and
  validation while preserving the public lifecycle API.

- `0.67.7` finishes the current deployment-truth promotion decomposition by
  isolating artifact identity, wasm-store identity/catalog, transform/readiness,
  and artifact-plan helpers while preserving the public promotion API.

- `0.67.6` continues deployment-truth promotion decomposition by isolating
  source-build materialization evidence, identity-report, output-group, and
  transform-link validation helpers while preserving the public promotion API.

- `0.67.5` continues deployment-truth promotion decomposition by isolating
  artifact-promotion provenance and execution-receipt construction/validation
  in a focused child module while preserving the public deployment-truth API.

- `0.67.4` decomposes the largest host deployment-truth lifecycle and
  promotion internals into directory modules while preserving the public
  deployment-truth API.

- `0.67.3` tightens registry-record ownership so workflow and
  endpoint-boundary code use ops-owned subnet registry projections instead of
  consuming raw registry records.

- `0.67.2` finishes the post-helper-split CLI and audit cleanup by trimming
  stale NNS command docs, recording the June 13 audit pass, refreshing helper
  tooling pins, and closing the pool recycle workflow/storage boundary leak.

- `0.67.1` audits `canic-host` after the `icq` split by removing stale host
  module wiring and reorganizing large host unit-test files into focused
  modules without changing CLI or runtime behavior.

- `0.67.0` splits the NNS query surface into the external `ic-query`/`icq`
  tool, removes the linked NNS registry query stack from Canic host and CLI,
  and pins `icq` as required external tooling for Canic validation.

## [0.66.x] - 2026-06-12 - 0.65 audit, testing, and fixing

Detailed patch breakdown: [docs/changelog/0.66.md](docs/changelog/0.66.md)

- `0.66.11` adds cached NNS topology capacity, regional distribution, and node
  provider distribution commands for topology allocation audits.

- `0.66.10` adds cached NNS topology health and gap inspection commands and
  cleans topology text reports so operator-facing tables avoid filler headings.

- `0.66.9` adds cached NNS topology coverage/version inspection and hardens
  backup plus wasm-store recovery paths so unsafe persisted state fails
  explicitly.

- `0.66.8` separates shared auth-proof trust-anchor support from delegated-token
  endpoint verification, renames the private root delegation proof client, and
  cleans active delegated-auth ops and docs away from stale shard/mint/signing
  wording.

- `0.66.7` renames root and issuer canister-signature helper APIs away from
  abbreviated `*_sig_*` wording so active auth code no longer resembles the
  removed `root_sig` token field.

- `0.66.6` removes the unused delegated-token verifier build cfg, renames
  internal delegated-token preparation code away from stale mint terminology,
  and normalizes active auth docs around issuer token issuance.

- `0.66.5` separates the no-default-features minimal audit baseline from the
  metrics-enabled baseline, makes build-script endpoint cfgs honor the `canic`
  metrics feature, and renames the manual sandbox away from the minimal audit
  role.

- `0.66.4` restores full workspace all-features validation by exposing the
  public runtime memory bootstrap helper and bootstrapping direct metrics facade
  tests before stable-memory-backed metric families are queried.

- `0.66.3` closes the strict delegated-auth verifier gap by requiring effective
  raw IC root-key material before protected endpoints run, gating delegated-token
  verification on explicit verifier config, keeping wasm-store free of issuer
  provisioning endpoints, and pinning `time` at `0.3.47`.

- `0.66.2` publishes the 0.65 closeout audit line against the 0.66 package
  surface so follow-up cleanup can proceed from a versioned baseline.

- `0.66.1` enforces delegated-auth verifier trust anchors by pairing network
  labels with raw IC root keys, fixes issuer-proof Candid terminology, and
  cleans closeout docs around certified-data ownership and local verifier
  purity.

- `0.66.0` opens the post-0.65 stabilization line: existing endpoint
  perf-observability planning moves to 0.67, while 0.66 is reserved for audit,
  testing, and fixes that prove the zero-management-ECDSA 0.65 auth epoch
  without adding new auth protocol features.

## [0.65.x] - 2026-06-10 - Canister-signature delegated auth

Detailed patch breakdown: [docs/changelog/0.65.md](docs/changelog/0.65.md)

- `0.65.31` refreshes the compact handoff and 0.65 design status so new
  sessions start from the current zero-ECDSA auth state, and restores the
  compatible `time 0.3.41` lockfile line after broader test and host
  compilation exposed the drift back to `0.3.48`.

- `0.65.30` catches up release notes for `.28` and `.29`, normalizes current
  release wording away from deleted source-shape guard language, and records a
  focused auth/protocol validation pass for the zero-ECDSA closeout line.

- `0.65.29` refreshes the checked-in wasm-store Candid sidecar for the current
  zero-ECDSA auth surface, removing stale delegated root-key snapshot fields,
  exposing delegated-token prepare/get methods, and restoring the compatible
  `time 0.3.41` lockfile line for `ic-agent 0.47.3`.

- `0.65.28` reconciles the 0.65 implementation status with the landed hard-cut
  design, closing stale pending checklist language and recording the remaining
  work as release validation rather than further protocol design.

- `0.65.27` continues the proof terminology cleanup by renaming internal
  cache-hit delegated-token verification helpers, role-attestation hash domain
  constants, and root startup messages away from stale signing/signature names.

- `0.65.26` cleans active auth proof terminology by removing unused
  token-signature error variants, reporting root/issuer canister-signature
  failures as proof failures, and renaming remaining issuer-proof cache/comment
  wording away from stale signer/signature language, including the stale design
  reservation for issuer signer generation rotation.

- `0.65.25` removes the unsupported issuer signer generation hook from active
  delegation certificates, stable auth records, canonical cert bytes, issuer
  proof binding hashes, Candid, fixtures, and active auth documentation.

- `0.65.24` renames the active delegation test issuer surface from
  signer terminology to issuer terminology, including
  `delegation_issuer_stub`, the managed test role `issuer`, issuer/verifier
  endpoint names, PIC role-attestation fixture metadata, and release-set
  build inputs.

- `0.65.23` removes the remaining internal-invocation proof error variants from
  active auth scope errors and renames non-root delegated-token bootstrap
  checks/logging around issuer canister-signature support instead of stale
  signer-key material wording, while reporting delegated-token root verifier
  failures as root-proof failures instead of root-signature failures. It also
  renames the active canister auth config key from `delegated_token_signer` to
  `delegated_token_issuer` and removes ECDSA key settings from the current auth
  config documentation, while locking `time` to `0.3.41` so the IC/PocketIC
  build path does not pick up the incompatible `0.3.48` transitive update.

- `0.65.22` uses ICP CLI 0.3.2 local Candid support for Canic CLI and host
  canister calls when a generated `.icp/<network>/canisters/<role>/<role>.did`
  sidecar is available, covering registry, list, medic, metrics, cycles,
  backup, snapshot, and ICP-refill call paths, centralizing CLI sidecar
  resolution and preserving the existing no-sidecar fallback.

- `0.65.21` deletes the isolated threshold-ECDSA signing adapter, feature flag,
  replay external-effect variant, and ECDSA platform metric surface from active
  code, removes dead ECDSA config knobs and absence-only legacy tests, and
  updates active auth docs away from shard/ECDSA token language while removing
  deleted-design source-shape guard scripts/tests, removed-design
  macro/parser/CLI tests, and stale replay actor metadata in favor of current
  behavior tests and the documented audit result.

- `0.65.20` pins the auth certified-data ownership boundary in CI so only the
  root and issuer canister-signature helpers may call `certified_data_set`, and
  both must commit the exact labeled `"sig"` tree.

- `0.65.19` removes caller-provided delegated-token nonce input and derives
  signed token nonces issuer-side from caller, prepare operation id, subject,
  issuer, and cert hash, with a CI guard preventing async or
  management-canister calls in token preparation.

- `0.65.18` adds the explicit 60-second future-skew allowance for delegated
  token cert/token and role-attestation verifier not-from-the-future checks
  while preserving strict expiry boundaries.

- `0.65.17` deletes remaining legacy protected-internal call, attestation
  key-set, verifier root-key cache, shard secp256k1 verification, and
  threshold-ECDSA public-key auth surfaces from active code and test canisters,
  leaving normal auth on delegated-token and role-attestation canister
  signatures.

- `0.65.16` moves `SignedRoleAttestation` onto root canister signatures with
  explicit prepare/get endpoints, local embedded-proof verification, no
  verifier-side root key refresh, and no ECDSA key rotation semantics for role
  attestations.

- `0.65.15` removes active shard ECDSA key material from delegated-token
  delegation certs, renames the token authority fields to issuer terminology,
  binds certs to issuer canister-signature authority, removes threshold
  public-key fetching from root proof preparation, and updates issuer startup
  checks, test canister features, and the wasm-store Candid surface.

- `0.65.14` flips delegated tokens from shard ECDSA signatures to issuer
  canister-signature proofs, adds issuer-local token prepare/get endpoints,
  replay-protects token prepare, removes test fleet one-shot issue wrappers,
  updates the PIC helper to install active proof material before minting, and
  removes stale shard/threshold ECDSA signing cost classes from normal auth.

- `0.65.13` adds issuer canister-signature create/verify feature gates,
  issuer SignatureMap prepare/get/verify primitives, issuer-proof prepare
  metrics, and an issuer canister-signature replay cost class for the upcoming
  token-issuer hard cut.

- `0.65.12` exposes the controller-gated
  `canic_install_active_delegation_proof` non-root issuer endpoint, pins its
  replay-policy classification, and refreshes the canonical wasm-store Candid
  surface for the active proof install DTOs.

- `0.65.11` adds active-delegation-proof install validation and request/response
  DTOs, verifying issuer binding, cert validity time bounds, canonical cert
  hash, and root canister-signature proof before persisting issuer authority.

- `0.65.10` adds the persisted `ActiveDelegationProof` foundation for issuer
  token issuance, with explicit stable auth records and auth-state accessors
  that fail closed outside the proof validity window.

- `0.65.9` replaces the legacy global delegated-token audience with explicit
  canister, Canic-subnet, and project audiences, wires local verifier audience
  context through token checks, and binds signed token `ext` into token-issue
  replay identity.

- `0.65.8` adds the issuer-proof DTO, canonical hashing, binding-hash, verifier
  message, and future cache-key foundations for the zero-ECDSA token leg.

- `0.65.7` adds opaque issuer-signed delegated-token `ext` bytes to the
  current token claims and request shape, with canonical hash coverage and a
  bounded payload size.

- `0.65.6` adds a bounded positive delegated-token verifier cache that binds
  the exact proof, claims, current token signature, and caller while still
  rerunning endpoint-local authorization checks after cache hits.

- `0.65.5` removes the standalone delegated-grant capability proof success path:
  legacy `CapabilityProof::DelegatedGrant` envelopes now fail before payload
  decode, hash checks, secp256k1 verification, replay, or execution.

- `0.65.4` tightens the closeout target to zero-management-ECDSA normal auth,
  records the remaining issuer/role-attestation/delegated-grant blockers, and
  rejects inbound root-capability role-attestation proof envelopes as retired
  ECDSA input.

- `0.65.3` removes remaining normal-auth one-shot role-attestation wrappers,
  stale test-stub attestation endpoints, and the outbound root-capability
  role-attestation fetch/cache fallback.

- `0.65.2` removes obsolete one-shot root ECDSA client routing and the dead
  outbound protected-internal fresh-proof client/cache surface, while retaining
  root rejection endpoints and protected descriptors as hard-cut metadata.

- `0.65.1` split threshold-ECDSA public-key fetch support from
  threshold-ECDSA signing during the earlier root-proof-only transition. The
  0.65 closeout target supersedes that state: normal auth removes
  threshold-ECDSA signing/public-key features and issuer token proofs use IC
  canister signatures.

- `0.65.0` hard-cuts delegated-token root proofs from root threshold ECDSA to
  IC canister signatures with an update/query prepare-get flow, reusable
  self-contained endpoint tokens, configured verifier trust anchors,
  nanosecond-native auth DTOs, and no legacy root ECDSA verifier branch.

## [0.64.x] - 2026-06-09 - Service/singleton topology split

Detailed patch breakdown: [docs/changelog/0.64.md](docs/changelog/0.64.md)

- `0.64.3` closes the 0.64 topology design with no required deferred work and
  adds root-index regression coverage for stale direct-root singleton residue.

- `0.64.2` tightens AppIndex and SubnetIndex imports so propagated and full
  snapshots cannot accept roles outside the configured service-filtered index
  sets.

- `0.64.1` hardens service/singleton runtime policy so directory, scaling, and
  sharding manager pools require service parents while parent-scoped singleton
  child creation remains valid.

- `0.64.0` separates root-scoped services from parent-scoped singletons with
  `kind = "service"`, making root bootstrap, SubnetIndex, and current AppIndex
  validation service-driven while preserving singleton for downstream
  parent-owned child canisters.

## [0.63.x] - 2026-06-08 - NNS topology inspection and surface reduction

Detailed patch breakdown: [docs/changelog/0.63.md](docs/changelog/0.63.md)

- `0.63.5` hard-cuts delegated-token auth to stable Canic/project audiences
  plus signed role grants, allowing one session token to authorize multiple
  role-scoped operations without additional per-role ECDSA token minting, and
  moves raw deployment-truth artifacts under `canic deploy inspect`.

- `0.63.4` hard-cuts live deployment inspection under `canic info`, replacing
  the former top-level metrics, endpoints, and medic commands while adding a
  compact text summary for `canic deploy check`.

- `0.63.3` hardens CI and maintainer workflow by sharing operations-doc guard
  helpers, merging tag validation into the main checks job, moving first-party
  GitHub Actions pins to Node 24-compatible majors, adding an optional `gh` CI
  helper, aligning root topology tests with the memory-ledger opt-in contract,
  and moving historical post-46 leftovers into optional design ideas.

- `0.63.2` adds joined-topology coverage metrics to `canic nns topology
  summary`, showing whether cached nodes and node operators resolve to known
  provider, operator, and data-center rows, and centralizes external dev-tool
  pins in `tool-versions.env` with deterministic ICP CLI install verification
  and newer-release warnings.

- `0.63.1` adds one-shot cached mainnet NNS topology refresh and makes the
  controller-only `canic_memory_ledger` diagnostic opt-in with per-role
  `diagnostics.memory_ledger`, shrinking the default wasm-store Candid/runtime
  surface without changing package versions, dependencies, or lockfiles.

  ```text
  canic nns topology refresh
  canic nns topology refresh --dry-run
  canic nns topology refresh --format json
  canic nns topology refresh --source-endpoint https://icp-api.io
  ```

- `0.63.0` expands cached mainnet NNS topology inspection with an aggregate
  topology summary and filtered node reads, without changing package versions
  or cache schemas.

  ```text
  canic nns topology summary
  canic nns node list --subnet <subnet|subnet-prefix>
  canic nns node list --kind <application|cloud_engine|system|unknown>
  canic nns node list --data-center <data-center|data-center-prefix>
  canic nns node list --node-provider <node-provider|node-provider-prefix>
  canic nns node list --node-operator <node-operator|node-operator-prefix>
  ```

## [0.62.x] - 2026-06-08 - Release durability

Detailed patch breakdown: [docs/changelog/0.62.md](docs/changelog/0.62.md)

- `0.62.6` adds the non-versioned RC readiness audit and CI guard, records
  that 0.62 implementation work is ready to close, and assigns remaining
  package/install, broad workspace, local ICP/canister, tag, and final release
  gates to RC/full validation rather than another implementation slice.

- `0.62.5` adds the non-versioned release package/install validation checklist
  and CI guard, classifying package, installed CLI, packaged downstream, local
  canister, artifact, and release-flow ownership gates without changing
  runtime, Candid, CLI, JSON/output, package, dependency, or lockfile surfaces.

- `0.62.4` adds the non-versioned diagnostic consistency audit and CI guard,
  classifying existing public errors, logs, metrics, tests, docs, and
  public-output impact rules without changing runtime, Candid, CLI,
  JSON/output, package, dependency, or lockfile surfaces.

- `0.62.3` adds non-versioned recovery/retry runbooks and a CI guard,
  documenting safe operator actions for replay-sensitive failures and
  uncertain operations without changing runtime, Candid, CLI, JSON/output,
  package, dependency, or lockfile surfaces.

- `0.62.2` adds the non-versioned upgrade/state compatibility audit and CI
  guard, classifying replay-sensitive state areas, required RC gates, and
  current evidence without changing runtime, Candid, CLI, JSON/output, package,
  dependency, or lockfile surfaces.

- `0.62.1` adds the non-versioned release-validation matrix and CI guard,
  separating slice close-out, implementation close-out, RC promotion, final
  release/tag checks, focused replay/auth/cost gates, package/install probes,
  and environment-specific local ICP/canister validation.

- `0.62.0` starts the bounded post-0.61 release-durability line. This
  docs-only charter/reconciliation slice adds the 0.62 design, replaces stale
  tracked Broad NNS 0.62 changelog content, and keeps 0.62 scoped to release
  validation, upgrade confidence, operator recovery, governance, targeted tests,
  and minimal diagnostics.

## [0.61.x] - 2026-06-04 - System replay protection

Detailed patch breakdown: [docs/changelog/0.61.md](docs/changelog/0.61.md)

- `0.61.40` fixes control-plane lifecycle create call sites that missed the
  new deployment permit and cleans up 0.61 readiness wording. Bootstrap and
  wasm-store publication creation now reserve a management deployment guard;
  the design now marks the slice plan as historical implementation record.

- `0.61.39` adds an aggregate release-candidate manifest gate. Replay-policy
  tests now fail if endpoint, root-capability, or pool-admin manifests contain
  any remaining `ReleaseBlocker` entries.

- `0.61.38` pins durable-publication replay policy coverage. A replay-policy
  regression now proves the durable-publish cost class is scoped to
  wasm-store/template publication endpoints with quota and reserve metadata.

- `0.61.37` puts actual canister upgrade installs behind a management
  deployment `CostGuardPermit`. Already-current upgrades still skip before
  quota or cycle reservation.

- `0.61.36` threads root provision's deployment permit through lifecycle
  creation. Provisioning allocation, pool top-up, canister create, and initial
  install now use permit-required management wrappers.

- `0.61.35` tightens the threshold-ECDSA signing boundary. `EcdsaOps::sign_bytes`
  now requires a `CostGuardPermit`, and a source guard pins permit construction
  and expensive-adapter call sites.

- `0.61.34` tightens the ICP refill value-transfer boundary. Ledger transfer
  and CMC notify ops now require a `CostGuardPermit`, so refill execution
  cannot cross those adapter calls without the reserved value-transfer guard.

- `0.61.33` adds shared pending replay receipt quotas. Fresh shared receipts
  now reject with `ResourceExhausted` once an actor has 64 pending receipts or a
  command kind has 512 pending receipts, while committed replays still return
  before quota checks.

- `0.61.32` adds write-before-send pending operation logging for generated
  ICP-refill IDs. `canic cycles convert` now records generated live canister
  refill IDs in `.canic/operations/pending.json`, reuses matching pending IDs
  after a crash, and marks entries completed after successful CLI return.

- `0.61.31` makes generated ICP-refill operation IDs visible in the CLI.
  Non-JSON `canic cycles convert` canister-mode output now prints generated
  IDs before live calls and in dry-runs so retries have the client ID. The
  slice also starts warning on `used_underscore_binding` and cleans affected
  test canister endpoints.

- `0.61.30` adds the public `OperationIdRequired` hard-cut error. Missing
  operation IDs on replay-sensitive auth, pool, and root capability paths now
  return that stable code while malformed TTLs stay `InvalidInput`.

- `0.61.29` adds replay receipt stable-shape guards. Stable record tests now
  pin committed, pending, and recovery-required receipt round-trips plus
  controlled rejection of unsupported receipt schemas.

- `0.61.28` adds a delegated-auth hard-cut source guard. Live `canic-core`
  source now fails tests if the removed verifier-local token-use replay store or
  APIs are reintroduced.

- `0.61.27` adds delegated-token mint replay decision coverage. Mint replay
  tests now pin committed, conflicting, and in-progress receipt behavior plus
  token-signing quota rejection before ECDSA.

- `0.61.26` closes delegated-token mint wrapper manifest coverage. Fleet/test
  `*_issue_token` wrappers are now explicit replay-protected signing endpoints
  in the replay policy inventory, with a scanner test for future wrappers.

- `0.61.25` starts delegated-token mint replay hardening. Public token issue
  and mint helpers now require replay metadata, reserve shared receipts, and
  sign shard tokens only through logged cost-guarded ECDSA boundaries.

- `0.61.24` graduates root `ProvisionCanister` and the root capability RPC
  endpoint. Provisioning now reserves deployment quota/cycles and marks the
  create/install replay effect before lifecycle work can cross the boundary.

- `0.61.23` graduates root `RequestCycles` to implemented value-transfer
  replay policy. Cycles transfers now reserve value-transfer quota/cycles and
  mark the `deposit_cycles` effect before the management await.

- `0.61.22` splits root capability RPC replay policy into a command manifest.
  The endpoint is now pinned as command-dispatch, with `ProvisionCanister` and
  `RequestCycles` left as explicit command-level blockers.

- `0.61.21` graduates `canic_icp_refill` to implemented
  replay-protected value-transfer policy. Fresh refill effects now reserve
  value-transfer quota and cycle budget before transfer or notify execution.

- `0.61.20` marks ICP refill transfer and notify external-effect boundaries in
  shared replay receipts. Post-boundary transport failures now preserve
  recovery-required receipts, while known retryable responses keep refill
  business records resumable.

- `0.61.19` wires ICP refill into shared replay receipt reservation and
  committed response replay. Fresh refills reserve shared receipts, terminal
  responses are cached, and replay conflicts map to public conflict errors.

- `0.61.18` starts the ICP refill shared replay-core migration. Refill requests
  now build shared replay identity and reserve-input data while the existing
  refill business store continues to own transfer/notify progress.

- `0.61.17` graduates `canic_canister_upgrade` to implemented
  response-idempotent replay policy. Upgrade planning now treats matching
  installed target module hashes as no-ops and carries replay metadata through
  upgrade RPC dispatch.

- `0.61.16` graduates the `canic_pool_admin` endpoint dispatcher by tying it to
  the pool admin command manifest. Manifest tests now pin the endpoint as
  command-dispatch and fail if any pool admin variant regresses.

- `0.61.15` graduates pool `Recycle` to implemented response-idempotent
  behavior. Recycle records a metadata-preserving pending-reset pool entry
  before management reset so duplicate retries stop at existing pool state.

- `0.61.14` graduates pool `ImportImmediate` to implemented
  response-idempotent behavior. Immediate import now stops at existing ready or
  pending-reset pool entries and adds stricter ICP CLI compatibility failures.

- `0.61.13` reclassifies `canic_attestation_key_set` as implemented
  snapshot-convergent behavior. The slice also standardizes ICP CLI 0.3.0
  installation for local setup and CI, including explicit project-root use.

- `0.61.12` reclassifies `canic_canister_status` as an implemented
  update-shaped read-only endpoint. It reads management status without Canic
  mutation or deployment/signing/value-transfer effects.

- `0.61.11` graduates pool `ImportQueued` to implemented snapshot-convergent
  behavior. Repeated queued imports converge on one pending-reset entry per
  canister and avoid duplicate queued pool records.

- `0.61.10` finishes root auth-material replay recovery for role-attestation
  and internal-invocation proof issuance. Both signing paths now mark the ECDSA
  effect boundary and preserve recovery-required receipts for uncertain exits.

- `0.61.9` routes root role-attestation and internal-invocation proof signing
  through signing cost guards. Fresh signing reserves quota and in-flight
  cycles before threshold ECDSA while replay recovery remains a later slice.

- `0.61.8` rejects same-caller request-id reuse across different root
  capability command kinds. This prevents a receipt committed for one root
  capability variant from being treated as fresh for another.

- `0.61.7` adds command-level replay policy coverage for every
  `canic_pool_admin` variant. Immediate pool import now succeeds without reset
  when the canister is already present in pool state.

- `0.61.6` makes pool `CreateEmpty` replay-protected and deployment
  cost-guarded. Fresh requests reserve replay, quota, and cycle budget before
  management create and commit the created pool principal for replay.

- `0.61.5` adds the shared cost-guard foundation and applies it to root
  delegation-proof signing. Fresh signing reserves quota and in-flight cycles,
  while committed replay returns bypass current quota and reserve checks.

- `0.61.4` makes root delegation-proof issuance replay-protected and adds
  cached NNS data-center inspection. Delegation proof requests reserve shared
  receipts before signing and replay committed proof bytes for duplicates.

  ```text
  canic nns data-center refresh
  canic nns data-center list
  canic nns data-center list --verbose
  canic nns data-center info <data-center-prefix>
  canic nns data-center list --format json
  ```

- `0.61.3` migrates root RPC replay onto the shared replay receipt store and
  expands NNS inspection. Root capability replay now uses receipt-backed
  reserve/commit/abort, TTL expiry, actor capacity checks, and cached replay.

  ```text
  canic nns registry version
  canic nns node refresh
  canic nns node list
  canic nns node list --verbose
  canic nns node info <node-prefix>
  canic nns node list --format json
  canic nns node-operator refresh
  canic nns node-operator list
  canic nns node-operator list --verbose
  canic nns node-operator info <node-operator-prefix>
  canic nns node-operator list --format json
  ```

- `0.61.2` adds the shared stable replay receipt store and reserve/replay/commit
  API. Receipts now model command-scoped keys, actors, payload hashes,
  committed responses, external effects, recovery, and bounded failures.

- `0.61.1` starts the shared replay-core extraction and enriches NNS node
  provider metadata. The slice adds `OperationId`, command kinds, replay
  actors, payload hashing, bounded errors, and registry-version provenance.

- `0.61.0` starts the replay-safety hardening branch. It adds the endpoint
  replay manifest, cuts verifier-local delegated-token update consumption, and
  makes `canic_app` set-style commands response-idempotent.

## [0.60.x] - 2026-06-04 - NNS subnet inspection

Detailed patch breakdown: [docs/changelog/0.60.md](docs/changelog/0.60.md)

- `0.60.10` adds a read-only NNS governance view for node providers:
  `canic nns node-provider list` and `canic nns node-provider info` query the
  mainnet NNS governance canister, render compact five-character principals by
  default, include registry-derived assigned-node counts, and keep
  full-principal/reward-account detail in verbose text and JSON output.

  ```text
  canic nns node-provider list
  canic nns node-provider list --verbose
  canic nns node-provider info <node-provider-prefix>
  canic nns node-provider list --format json
  ```

- `0.60.9` finishes another `canic-cli` hygiene pass by centralizing Clap help
  rendering, required argument extraction, numeric value parsers, and defaulted
  string lookup helpers without changing commands, flags, help text, JSON
  output, or operational behavior.

- `0.60.8` finishes the low-risk `canic-cli` Clap cleanup by moving evidence
  output formats and Canic-owned cycles amount/hex/e8s values to Clap parsers,
  removing stale manual parser error variants, and standardizing string
  extraction through shared CLI helpers without changing commands, flags, JSON
  output, or operational behavior.

- `0.60.7` moves remaining simple `canic-cli` option validation onto Clap
  value parsers and typed match extraction, so invalid values fail at the CLI
  parse boundary without changing command names, flags, JSON shapes, NNS
  catalog behavior, or output columns.

- `0.60.6` moves the public subnet inspection surface under `canic nns`, records
  packaged downstream CLI proof for the 0.60 subnet catalog line, and
  simplifies catalog stale-cache help. The publishable crate chain packages
  cleanly, an isolated downstream CLI build still works with
  `canic-subnet-catalog`, `canic-ic-registry`, `canic-host`, and `canic-cli` in
  the graph, and `canic nns subnet list/info` now use the 7-day freshness
  default with `canic nns subnet refresh` as the force-refresh path instead of
  exposing stale-policy knobs on read-only inspection commands. `canic nns
  subnet info <x>` also accepts unique cached subnet-principal prefixes for
  subnet lookups. README and current operator docs now point at the final
  `canic nns subnet ...` namespace.

  ```text
  canic nns subnet list
  canic nns subnet info <subnet|canister|subnet-prefix|deployment-target>
  canic nns subnet info <subnet-prefix>
  canic nns subnet refresh
  bash scripts/ci/verify-packaged-downstream-cli.sh
  ```

- `0.60.5` teaches the shared NNS registry adapter to reconstruct
  high-capacity registry values through `get_chunk` with SHA-256 validation,
  makes `canic nns subnet list` compact by default with `--verbose` for the
  full text view, and refreshes help text for the current catalog and
  deployment surfaces.

  ```text
  canic help
  canic nns help
  canic nns subnet list
  canic nns subnet list --verbose
  canic nns subnet refresh help
  ```

- `0.60.4` records the operator proof for the catalog-derived estimate source:
  a live refresh, catalog list, known-canister routing lookup, and canonical
  instruction-footprint report all use the refreshed mainnet catalog without
  changing measured instruction rows.

  ```text
  target/debug/canic nns subnet refresh --format json
  target/debug/canic nns subnet list --format json
  target/debug/canic nns subnet info mf7xa-laaaa-aaaar-qaaaa-cai --format json
  bash scripts/ci/instruction-audit-report.sh --estimate-execution-cycles --estimate-canister-principal mf7xa-laaaa-aaaar-qaaaa-cai
  ```

- `0.60.3` lets instruction-audit execution-cycle estimates use the refreshed
  mainnet subnet catalog as an optional source, while preserving explicit-rate
  and explicit-node-count precedence and omitting catalog-derived estimates
  when the cache is missing, stale, unresolved, or not chargeable.

  ```text
  bash scripts/ci/instruction-audit-report.sh --estimate-execution-cycles --estimate-canister-principal <canister-principal>
  bash scripts/ci/instruction-audit-report.sh --estimate-execution-cycles --estimate-canister-principal <canister-principal> --allow-stale-subnet-catalog
  bash scripts/ci/instruction-audit-report.sh --estimate-execution-cycles --estimate-canister-principal <canister-principal> --subnet-catalog-stale-after <duration>
  ```

- `0.60.2` adds live mainnet subnet catalog refresh through a shared
  protobuf-based NNS registry adapter, with refresh locking, atomic cache
  replacement, dry-run/export support, and previous-cache preservation on
  refresh failure.

  ```text
  canic nns subnet refresh
  canic --network ic nns subnet refresh
  canic nns subnet refresh --dry-run --output <path>
  ```

- `0.60.1` refines the cached subnet inspection command wording, preserving
  the mainnet-only `--network ic` behavior and clarifying the cached-only
  missing-catalog error.

  ```text
  canic nns subnet list
  canic --network ic nns subnet list
  canic nns subnet info <subnet|canister|subnet-prefix|deployment-target>
  ```

- `0.60.0` starts the NNS subnet inspection line with a cached mainnet IC
  subnet catalog schema/resolver and read-only NNS subnet inspection commands
  over local cache data. Live refresh and estimate integration remain
  deferred to later 0.60 patches.

  ```text
  canic nns subnet list
  canic --network ic nns subnet list
  canic nns subnet info <subnet|canister|subnet-prefix|deployment-target>
  ```

## [0.59.x] - 2026-06-03 - Instruction accounting and offline cost estimates

Detailed patch breakdown: [docs/changelog/0.59.md](docs/changelog/0.59.md)

- `0.59.7` centralizes instruction-footprint report status and baseline
  sentinel labels while preserving rendered report output.

- `0.59.6` clarifies direct instruction-audit estimate flag diagnostics by
  separating boolean flag parsing from positive integer parsing.

- `0.59.5` pins the instruction-footprint markdown estimate section wording
  and adds report-rendering coverage for the instructions-only estimate label.

- `0.59.4` pins the remaining offline estimate JSON contract labels as named
  report-support constants while keeping literal-value tests and serialized
  output unchanged.

- `0.59.3` keeps the offline estimate object lean by pinning the repeated
  omitted-cost list as one static contract while preserving the serialized
  JSON shape.

- `0.59.2` restores CI `RUSTUP_TOOLCHAIN` propagation for nested wasm builds,
  removes the noisy ICP-refill macro `compile_fail` doctest from release
  gates while preserving ordinary test coverage of the required missing-guard
  compile-time error, and records the follow-up contract in the 0.59 design
  doc.

- `0.59.1` tightens the 0.59 estimate input contract so direct report
  environment sources cannot be supplied without estimate mode, and fixes CI
  workflow linting/toolchain setup so actionlint catches GitHub Actions context
  errors locally and in CI.

- `0.59.0` starts the instruction-accounting line by hardening
  instruction-footprint artifacts around `performance_counter(1)` semantics,
  preserving message-kind `sample_origin`, and adding opt-in offline execution
  cycle estimates for update rows without introducing NNS/catalog lookup or
  renaming real cycle funding surfaces.

  ```text
  bash scripts/ci/instruction-audit-report.sh --estimate-execution-cycles --estimate-node-count <13|34>
  bash scripts/ci/instruction-audit-report.sh --estimate-execution-cycles --cycles-per-billion-instructions <cycles>
  ```

## [0.58.x] - 2026-06-02 - ICP-to-cycles refill primitive

Detailed patch breakdown: [docs/changelog/0.58.md](docs/changelog/0.58.md)

- `0.58.17` closes out Slice 58 by wiring ICP-refill system canister override
  config into workflow resolution, tightening hub self-refill design notes, and
  cleaning up cycle-tracker top-up dispatch.

- `0.58.16` finishes the ICP-refill record-boundary cleanup by moving notify
  eligibility, stale transfer-window checks, retry validation, and
  stored-record-to-request conversion into storage ops.

- `0.58.15` keeps ICP-refill recovery lookup and status predicates in storage
  ops and shares manual policy preflight input construction across rate-gated
  and non-rate-gated refill paths.

- `0.58.14` keeps the post-refill cleanup behavior-neutral by centralizing
  completed-cycle saturation, direct-child parent checks, and cycles-timer
  in-flight guards across metrics, grant-ledger reuse, and top-up scheduling.

- `0.58.13` reuses the existing child funding grant ledger for successful
  registered direct-child ICP refills, so completed CMC notify totals feed the
  same budget and cooldown accounting as ordinary child grants.

- `0.58.12` wires ICP-refill policy through existing cycles-funding kill
  switch and child cooldown hooks, and leans out CI/setup helper installation
  without adding a separate refill policy island.

- `0.58.11` hardens ICP-refill validation by covering notify retry caps,
  CMC/ledger recovery mappings, and the feature-enabled endpoint macro
  missing-guard compile-fail contract in the fast test lane.

- `0.58.10` adds bounded ICP-refill observability under existing
  `cycles_funding` core metrics without adding a new metrics tier, query API,
  or metric family.

- `0.58.9` turns downstream adoption feedback into a local academic fleet
  runbook, project skill, sourceable canister ID exports, and sharper
  install, metrics, and protected internal-call diagnostics.

- `0.58.8` splits the endpoint macro pipeline into focused directory modules
  and moves access-plan synthesis into its own expansion submodule without
  changing macro behavior.

- `0.58.7` cleans up endpoint attribute parsing by sharing marker helpers and
  access-path decoding while preserving the endpoint macro contract.

- `0.58.6` adds composite query endpoint macro support and endpoint perf
  call-kind labels while continuing the ICP refill and lint-suppression DRY
  cleanup without changing the refill workflow or DTO surface.

- `0.58.5` cleans up the ICP refill core by centralizing repeated infra error
  mapping, refill record status/error mutation helpers, and shared transfer
  retry stale-window handling without changing the workflow or DTO surface.

- `0.58.4` cleans up the cycles conversion CLI module boundary and reduces
  PocketIC CI disk pressure by removing duplicate prebuild work and clearing
  generated wasm target caches between heavy integration suites.

- `0.58.3` exposes the ICP refill primitive through the opt-in facade endpoint
  macro, wires configured hub self-refill into the existing cycle tracker timer,
  and adds the retained thin `canic cycles convert` trigger with local-only
  fabrication labeling.

- `0.58.2` adds the ICP ledger/CMC helper layer, pure refill policy gates, and
  the manual canister-side refill workflow with persisted retry identity,
  duplicate recovery, stale-window protection, bad-fee retry preparation,
  notify attempt capping, and dry-run estimates.

- `0.58.1` starts the ICP refill implementation with passive request/status
  DTOs, MVP `topup.icp_refill` config validation, authoritative refill record
  storage, and generated bootstrap config support.

- `0.58.0` starts the ICP-to-cycles refill primitive line with a scoped design
  for canister-side CMC/ICP-ledger conversion, recovery, local fabrication,
  funding-chain integration, MVP refill config, and bounded metrics without
  adding a parallel identity-funded CLI transfer engine. The design pins the
  hub self-refill hook, separates recovery records from top-up observability,
  and keeps the thin CLI trigger outside the exit gate.

## [0.57.x] - 2026-06-01 - Audit rotation and feedback window

Detailed patch breakdown: [docs/changelog/0.57.md](docs/changelog/0.57.md)

- `0.57.16` cleans up the 0.58-adjacent cycles CLI and runtime funding helper
  surfaces ahead of the ICP refill primitive while preserving existing cycles
  command behavior.

- `0.57.15` continues the behavior-neutral deploy CLI DRY cleanup by
  regularizing command metadata, argument construction, usage rendering, and
  dispatch tests across the deploy command families while preserving command
  behavior.

- `0.57.14` continues the behavior-neutral deploy CLI DRY cleanup by splitting
  passive promote/external command families and top-level deploy command
  construction into focused modules while preserving command behavior.

- `0.57.13` finishes the behavior-neutral deploy CLI DRY cleanup and test
  split, and records the 0.58 ICP-to-cycles conversion design.

- `0.57.12` continues the DRY deploy CLI cleanup by moving authority dry-run
  resume-report, and passive deployment-truth command handling into private
  submodules while preserving command behavior, help text, output defaults, and
  dry-run boundary coverage.

- `0.57.11` removes the completed upstream launcher watch after it flagged a
  newer candidate for manual testing, and follows the DRY consolidation audit
  by splitting deploy command-family glue into private CLI submodules without
  changing command semantics.

- `0.57.10` refreshes and reruns the security-boundary-ordering audit. It
  verifies public delegated-token endpoint auth, protected internal role
  endpoint proof ordering, root RPC replay sequencing, and capability
  attestation cache reuse remain ordered across their trust boundaries.

- `0.57.9` refreshes and reruns the bootstrap-lifecycle-symmetry and
  canonical-auth-boundary audits. It verifies lifecycle hooks remain
  synchronous restore adapters that schedule bootstrap/user work through
  lifecycle timers, and verifies public delegated-token auth plus protected
  internal role predicates still converge on their canonical verification
  boundaries.

- `0.57.8` refreshes and reruns the access-purity audit for the current
  singular delegated-token audience and protected internal endpoint role
  predicate surface. It verifies access remains a thin endpoint boundary and
  removes stale versioned wording from endpoint macro diagnostics.

- `0.57.7` refreshes and reruns the publish-surface audit for the current
  hard-cut eight-crate package posture. It verifies package-local README
  alignment, the `1.91.0` published MSRV contract, the small `canic` default
  feature surface, the `canic-wasm-store` `cdylib`-only posture, retained
  installed/packaged proof guardrails, and package verification for all current
  published crates.

- `0.57.6` refreshes and reruns the ops-purity audit for the current compact
  v1 runtime tree. It keeps ops scoped to runtime primitives, moves topology
  policy-input mapping out of an ops-owned `policy` module path, normalizes
  ops `Principal` imports to Canic's runtime type facade, and removes workflow
  path comment noise from ops storage.

- `0.57.5` refreshes and reruns the layer-violations audit for the current
  post-v1 runtime tree. It verifies that public RPC proof orchestration remains
  workflow-owned, updates the recurring audit definition for the current API
  boundary, and normalizes policy-layer `Principal` imports to Canic's runtime
  type facade.

- `0.57.4` refreshes and reruns the auth-abstraction-equivalence and DRY
  consolidation audits. It verifies the hard-cut singular delegated-token
  audience model, checks compact v1 evidence/provenance/policy/catalog/proof
  surfaces for duplication drift, and consolidates the duplicate
  evidence-envelope command-provenance path normalization helper in the CLI.

- `0.57.3` refreshes the recurring instruction-footprint audit definition for
  the maintained PocketIC runner and runtime probe scope, while explicitly
  keeping host-side evidence commands out of the canister instruction matrix.
  It also restores the explicit audit-only `leaf_probe` runtime config so the
  probe does not fall back to generated compile-only standalone metadata, and
  fixes baseline selection to use the latest prior retained report across days.

- `0.57.2` keeps the recurring Wasm footprint audit aligned with hard-cut
  fleet role/package metadata by resolving Cargo package names from
  `[roles.<role>].package`, passing the selected fleet config into artifact
  shrinking, and removing stale `minimal = N/A` baseline reporting.

- `0.57.1` follows the refreshed layer-boundary audit: capability envelope and
  proof orchestration now lives under workflow, the public RPC API delegates
  thinly, and module-source resolver errors stay on Canic's internal error
  boundary. It also tightens packaged README posture for `canic-cli` and
  `canic-host` around the compact v1 operator surface.

- `0.57.0` starts a maintenance line for rotating recurring audits while
  waiting for real user feedback on the compact v1 surface. It refreshes the
  `publish-surface` recurring audit definition for the post-0.56 packaged
  proof story without adding product surface, commands, DTOs, or mutation
  authority.

## [0.56.x] - 2026-06-01 - V1 packaged downstream proofs

Detailed patch breakdown: [docs/changelog/0.56.md](docs/changelog/0.56.md)

- `0.56.4` closes the packaged downstream proof line with a PASS audit:
  ```text
  docs/audits/release-lines/0.56-closeout.md
  ```
  The audit verifies the installed CLI proof, packaged downstream CLI proof,
  packaged `wasm_store` bootstrap proof, declared Rust `1.91.0` MSRV lane, and
  absence of new product surface or mutation authority.

- `0.56.3` hardens the special packaged downstream `wasm_store` proof:
  ```text
  docs/operations/0.56-packaged-wasm-store.md
  scripts/ci/verify-packaged-downstream-wasm-store.sh
  ```
  The proof now packages and patches same-version Canic sibling crates
  explicitly, rejects repository crate paths and `target/debug/canic`, isolates
  proof execution paths where practical, and verifies that the generated
  bootstrap wrapper points at packaged Canic sources. This remains an internal
  bootstrap/runtime proof, not ordinary downstream dependency guidance.

- `0.56.2` hardens the packaged downstream CLI proof:
  ```text
  docs/operations/0.56-packaged-downstream-cli.md
  scripts/ci/verify-packaged-downstream-cli.sh
  ```
  The proof now rejects repository crate paths and `target/debug/canic` in the
  packaged tool root, isolates proof execution paths where practical, and runs
  current v1 read-only commands against a downstream project. It also packages
  and patches `canic-control-plane` explicitly so local pre-publication
  versions do not pass by resolving that dependency from crates.io.

- `0.56.1` hardens the installed CLI smoke:
  ```text
  docs/operations/0.56-installed-cli-smoke.md
  scripts/ci/verify-installed-canic-cli.sh
  ```
  The proof now asserts it is using the temporary installed binary rather than
  `target/debug/canic`, isolates `HOME`, `CARGO_HOME`, `CARGO_TARGET_DIR`, and
  `TMPDIR`, and runs the maintained v1 readiness smoke through that binary.

- `0.56.0` proposes the tentative packaged downstream proof line and hard-cuts
  retained packaged/installed release probes to current v1 questions:
  ```text
  docs/design/0.56-v1-packaged-downstream-proofs/0.56-design.md
  docs/operations/0.56-v1-release-probes.md
  scripts/ci/verify-installed-canic-cli.sh
  ```
  The installed CLI probe now installs `canic` into a temporary root and runs
  the maintained v1 readiness smoke through that installed binary instead of
  building repo-local roles with the old role-only build shape. The packaged
  downstream CLI fixture now uses current fleet-scoped role declarations. The
  line is scoped to proving installed CLI and packaged downstream behavior
  with current v1 command shapes. It is not a new product feature line and
  keeps deployment groups, signing, locks, registry import, teardown,
  controller mutation, active adoption/import, broad live verification, and
  new stable public DTO families out of scope. Packaged proofs must use package
  archives or unpacked package roots rather than repository path dependencies,
  `target/debug/canic`, unpublished local crates, or repository `.canic` /
  `.icp` state.

## [0.55.x] - 2026-05-31 - V1 stabilization and readiness

Detailed patch breakdown: [docs/changelog/0.55.md](docs/changelog/0.55.md)

- `0.55.5` adds the final post-0.55.4 closeout audit:
  ```text
  docs/audits/release-lines/0.55-final-closeout.md
  ```
  Verdict: PASS. The audit verifies the maintained v1 command surface, local
  smoke proof, heavier operator proof, proof artifacts, docs/help alignment,
  passive/active boundaries, and 0.54 passive-catalog transition.

- `0.55.4` adds the heavier v1 operator proof:
  ```text
  scripts/ci/v1-operator-proof.sh
  docs/operations/0.55-v1-operator-proof.md
  ```
  The proof builds `demo.app` with stable build provenance, registers an
  explicit local deployment target under a temporary proof root, and emits a
  deployment-check envelope that fingerprints the build provenance. The check
  is expected to be `blocked_by_policy` because the proof does not install the
  fleet, verify a live root, or build every fleet artifact.

- `0.55.3` closes the v1 stabilization line with a candidate audit:
  ```text
  docs/audits/release-lines/0.55-closeout.md
  ```
  Verdict: PASS. The audit verifies the compact v1 command
  surface, docs/help alignment, local smoke proof, passive boundaries, and
  absence of new deployment groups, signing, locks, registry import, teardown,
  controller mutation, active adoption/import, or broad live verification.

- `0.55.2` adds a maintained local smoke proof for the safe v1
  setup/catalog/evidence subset:
  ```text
  scripts/ci/v1-readiness-smoke.sh
  docs/operations/0.55-v1-local-smoke.md
  ```
  The smoke runs in a temporary project and proves fleet creation, canister
  scaffold, declared-only inspection, explicit role attachment, empty local
  catalog reporting, and passive evidence-gate evaluation without running
  artifact builds, installs, live deployment checks, controller mutation,
  registry import, teardown, or active adoption/import.

- `0.55.1` adds a maintained v1 readiness checklist and aligns the current
  docs/help surface around the compact operator story:
  ```text
  docs/architecture/v1-readiness-checklist.md
  canic evidence gate --policy <path> --envelope <path>
  canic evidence gate --policy <path> --manifest <path>
  ```
  The checklist records the command set, required files, expected evidence
  outputs, and passive boundaries without adding deployment groups, signing,
  locks, registry import, teardown, controller mutation, or active adoption.

- `0.55.0` starts the v1 stabilization line with a design for proving the
  compact operator story instead of adding another feature layer:
  ```text
  docs/design/0.55-v1-stabilization-readiness/0.55-design.md
  ```
  The line is scoped around docs/help alignment, stale-surface cleanup, a
  practical local smoke proof, and a v1-candidate audit. It deliberately keeps
  deployment groups, signing, locks, registry import, teardown, controller
  mutation, active adoption/import, and broad live verification out of scope.

## [0.54.x] - 2026-05-31 - Passive deployment catalog

Detailed patch breakdown: [docs/changelog/0.54.md](docs/changelog/0.54.md)

- `0.54.2` closes the passive deployment catalog line with a release audit:
  ```text
  docs/audits/release-lines/0.54-closeout.md
  ```
  The audit verifies the catalog commands, local-state-only source, text/JSON
  output, explicit output files, missing/legacy/malformed-state behavior,
  passive boundary, v1 walkthrough, and absence of groups, locks, signing,
  registry import, teardown, controller mutation, topology mutation, install
  authority, or active adoption/import.
  It also resolves the 0.49 design-doc follow-up by aligning the old design
  with shipped hard-cut command shapes and metadata requirements.

- `0.54.1` adds a maintained compact v1 operator walkthrough:
  ```text
  docs/architecture/v1-operator-walkthrough.md
  ```
  The guide connects build provenance, deployment-check envelopes, policy
  gates, and the passive deployment catalog while keeping fleet/role identity
  separate from deployment-target identity and keeping install, controller,
  registry, topology, teardown, signing, locks, groups, and active adoption out
  of scope.

- `0.54.0` adds a passive deployment catalog over existing deployment-target
  local state:
  ```text
  canic deploy catalog list
  canic deploy catalog inspect <deployment>
  ```
  The catalog reads `.canic/<network>/deployments/<deployment>.json`, emits
  text by default or raw `DeploymentCatalogReportV1` JSON with `--format json`,
  and can write the selected format with `--output <path>`. It does not query
  live deployments, create deployment truth, infer deployments from fleet
  names, mutate topology/controllers/state, install Wasm, register artifacts,
  or add deployment groups.

## [0.53.x] - 2026-05-31 - CI policy gates and project evidence manifests

Detailed patch breakdown: [docs/changelog/0.53.md](docs/changelog/0.53.md)

- `0.53.6` closes the CI policy gate line with a release audit:
  ```text
  docs/audits/release-lines/0.53-closeout.md
  ```
  The audit verifies the passive single-envelope gate, build-provenance policy
  rules, project evidence manifests, duplicate manifest-path hardening, CLI
  help, docs, tests, and unchanged passive boundary.

- `0.53.5` hardens project evidence manifests by rejecting duplicate evidence
  paths before policy gate evaluation. This prevents the same saved envelope
  from being evaluated twice under one manifest.

- `0.53.4` adds policy gate architecture guidance and CI examples:
  ```text
  docs/architecture/ci-policy-gates.md
  ```
  The guide documents policy files, project evidence manifests, the current
  `canic evidence gate` command shapes, minimal CI usage, output formats, exit
  classes, and the passive safety boundary.

- `0.53.3` adds project evidence manifests to the passive policy gate:
  ```text
  canic evidence gate --policy <path> --manifest <path>
  ```
  A manifest groups existing evidence envelope files with required/optional
  status, expected payload schema, and target identity. Manifest gates remain
  read-only and still do not run builds, discover live state, mutate evidence
  inputs, or turn policy success into deployment truth.

- `0.53.2` adds optional build-provenance policy rules to the existing passive
  policy gate:
  ```text
  canic evidence gate --policy <path> --envelope <path>
  ```
  Policies may now require clean source evidence, `Cargo.lock` evidence, gzip
  Wasm output, SHA-256 artifact hashes, and package metadata `fleet.role`
  matching the evaluated envelope target. The gate still evaluates one
  existing evidence envelope and remains passive.

- `0.53.1` adds the passive single-envelope policy gate:
  ```text
  canic evidence gate --policy <path> --envelope <path>
  ```
  The command reads one strict `CiPolicyV1` TOML file and one existing
  `EvidenceEnvelopeV1`, evaluates stable envelope fields and payload schema
  identity/stability, and emits stable `PolicyGateReportV1` output. Raw
  `--format json` emits the report; `--format envelope-json` wraps the report
  in a new `EvidenceEnvelopeV1` with policy-file and evaluated-envelope input
  fingerprints. The gate is passive and does not run builds, query live
  deployments, mutate evidence/config/topology/controllers, or turn policy
  success into deployment truth.

- `0.53.0` hard-cuts stale CLI surfaces before the policy-gate work starts:
  ```text
  canic fleet config <fleet>
  canic backup manifest validate --manifest <file>
  ```
  The old top-level `canic config` and `canic manifest` command families are
  removed. Global `--network` forwarding now reaches all deployment-truth
  deploy leaves that consume network selection instead of only the top-level
  check/report leaves.

- Proposed the tentative 0.53 design:
  ```text
  docs/design/0.53-ci-policy-gates-project-manifests/0.53-design.md
  ```
  The line should build on 0.51 evidence envelopes and 0.52 build provenance
  by adding passive CI policy gates for existing evidence. The first policy
  slice is now scoped to a single-envelope gate:
  ```text
  canic evidence gate --policy <path> --envelope <path>
  ```
  It evaluates envelope schema, payload schema identity/stability, exit class,
  and summary evidence state, emits stable `PolicyGateReportV1`, and defers
  build-provenance field rules plus project evidence manifests until the
  single-envelope semantics are proven.
  It defers deployment locks, signing, provider wrappers, registry import,
  controller mutation, topology mutation, active adoption/import, and
  deployment/install authority.

## [0.52.x] - 2026-05-31 - Source, build, and artifact provenance

Detailed patch breakdown: [docs/changelog/0.52.md](docs/changelog/0.52.md)

- `0.52.4` closes the source/build/artifact provenance line with a release
  audit:
  ```text
  docs/audits/release-lines/0.52-closeout.md
  ```
  The audit verifies the stable `canic.build_provenance.v1` payload, explicit
  build provenance output, saved build-provenance evidence inputs, CI/GitOps
  policy docs, and the unchanged deployment/install/topology/controller
  boundary.

- `0.52.3` adds CI/GitOps policy guidance for stable build provenance:
  ```text
  docs/architecture/build-provenance-ci-policy.md
  ```
  The guide explains recommended checks for dirty source state, `Cargo.lock`
  drift, package metadata `fleet.role`, raw/gzip Wasm artifact hashes, and
  saved provenance linkage from passive adoption/deployment-check envelopes.

- `0.52.2` lets passive adoption-report and deployment-check evidence
  envelopes fingerprint saved build provenance evidence:
  ```text
  canic fleet adoption report <fleet> --profile <profile> --format envelope-json --build-provenance <path>
  canic deploy check <deployment> --format envelope-json --build-provenance <path>
  ```
  The file is recorded as a stable `canic.build_provenance.v1` input
  fingerprint only; report generation does not re-run builds, import
  artifacts, mutate topology/controllers, or turn provenance into deployment
  truth. This patch also adapts Canic memory-ledger diagnostics to the locked
  `ic-memory 0.7.0` API.

- `0.52.1` adds explicit build provenance output:
  ```text
  canic build <fleet> <role> --provenance <path>
  ```
  The file is an `EvidenceEnvelopeV1` with stable
  `canic.build_provenance.v1` payload and records source, Cargo, package
  metadata, build profile, and artifact hash evidence after a successful build.

- Proposed the 0.52 design:
  ```text
  docs/design/0.52-source-build-artifact-provenance/0.52-design.md
  ```
  The line should build on 0.51 evidence envelopes by adding stable source,
  Cargo, build, and artifact provenance for:
  ```text
  canic build <fleet> <role> --provenance <path>
  ```
  It defers signing, CI locks, project manifests, provider wrappers, registry
  import, controller mutation, topology mutation, and deployment/install
  authority.

## [0.51.x] - 2026-05-31 - CI/GitOps provenance and stable evidence envelopes

Detailed patch breakdown: [docs/changelog/0.51.md](docs/changelog/0.51.md)

- `0.51.6` marks the historical post-46 CI/GitOps provenance backlog as
  partially superseded by 0.51 and replaces old backlog-only draft names with
  the implemented envelope vocabulary:
  ```text
  EvidenceEnvelopeV1
  ExitClassV1
  ```
  Remaining backlog scope is source/build/artifact provenance, CI locks,
  project manifest semantics, optional signing/attestation, and provider
  wrappers.

- `0.51.5` adds the 0.51 closeout audit:
  ```text
  docs/audits/release-lines/0.51-closeout.md
  ```
  The audit verifies the stable evidence-envelope model, passive adoption and
  deployment-check emitters, shared exit-class and fingerprint behavior,
  envelope comparison, docs, and targeted validation.

- `0.51.4` adds CI/GitOps evidence-envelope pipeline guidance with concrete
  passive artifact examples:
  ```text
  canic fleet adoption report demo --profile minimal --format envelope-json \
    --output artifacts/canic/adoption-envelope.json
  canic deploy check demo-staging --format envelope-json \
    > artifacts/canic/deployment-check-envelope.json
  canic evidence compare \
    --left artifacts/canic/baseline-deployment-check-envelope.json \
    --right artifacts/canic/deployment-check-envelope.json \
    --format json \
    > artifacts/canic/envelope-compare.json
  ```
  The guidance explains which stable envelope fields CI should use, when raw
  JSON is still command-specific, and what envelope artifacts do not prove.

- `0.51.3` adds a CI-friendly stable envelope comparison command:
  ```text
  canic evidence compare --left <path> --right <path>
  ```
  The command compares stable `EvidenceEnvelopeV1` fields while ignoring
  timestamp/version noise and the nested command-specific payload body.

- `0.51.2` centralizes evidence-envelope exit-class precedence, aligns
  adoption-report and deployment-check envelope emitters on the shared
  summary-based classification, and documents CI policy guidance for warnings,
  blockers, missing required evidence, and conflicts.

- `0.51.1` hardens envelope input fingerprints by centralizing file
  fingerprinting in `canic-host`, adding `path_display`, normalizing evidence
  paths relative to the selected root, and redacting absolute paths outside
  that root. It also hard-cuts fleet role declarations so every
  `[roles.<role>]` entry must carry an explicit `package = "<path>"`, and
  workspace governance now rejects package paths that do not contain a real
  `Cargo.toml`. Adoption reports use `undeclared-role` rather than a
  non-package role state for observed-only findings.

- `0.51.0` adds the stable `EvidenceEnvelopeV1` model and envelope JSON output
  for passive adoption reports and deployment checks:
  ```text
  canic fleet adoption report <fleet> --profile <profile> --format envelope-json
  canic deploy check <deployment> --format envelope-json
  ```
  Existing adoption `--format json` remains the raw experimental adoption
  report payload, and existing deployment-check JSON remains raw
  `DeploymentCheckV1`. This patch also tightens release validation fixtures for
  the hard-cut role lifecycle and runs internal Wasm artifact builds with
  Cargo `--locked`.

## [0.50.x] - 2026-05-30 - Adoption profiles and safe onboarding

Detailed patch breakdown: [docs/changelog/0.50.md](docs/changelog/0.50.md)

- `0.50.15` closes the adoption line by updating the implemented 0.50 design,
  keeping JSON output experimental for all of 0.50.x, and adding regression
  coverage for symmetric artifact evidence conflicts, authority-gated
  declaration recommendations, and explicit artifact-manifest precedence.

- `0.50.14` marks conflicting artifact evidence as an adoption
  `evidence-conflict` when supplied artifact manifest and inventory evidence
  disagree about whether the same role is Canic-built or externally supplied.

- `0.50.13` carries unresolved inventory observations and unresolved artifact
  entries from supplied evidence into adoption report
  `missing_or_stale_evidence`, so passive reports show evidence gaps already
  recorded by deployment-truth artifacts.

- `0.50.12` expands text adoption reports so observed canister rows include
  match confidence plus supplied evidence such as controllers, Wasm evidence,
  deployment-target evidence, and warnings.

- `0.50.11` gates observed-only role declaration recommendations on authority
  evidence. Canic-authorized candidates may still get a blocked
  `canic fleet role declare ...` preview, while user-controlled, external, or
  unknown candidates now get an authority-review recommendation first.

- `0.50.10` lets adoption reports consume saved Cargo metadata package
  evidence:
  ```text
  canic fleet adoption report <fleet> --profile <profile> --cargo-metadata <path>
  ```
  The option reads `[package.metadata.canic]` fleet/role metadata from an
  existing `cargo metadata --format-version 1` JSON artifact and rejects
  ambiguous use with `--package-metadata`. Cargo package paths are normalized
  against the selected fleet config so `package = "."` and sibling package
  declarations can match.

- `0.50.9` makes `--deployment-check <path>` also supply saved plan artifact
  evidence from `DeploymentCheckV1.plan.role_artifacts`, unless an explicit
  `--artifact-manifest <path>` is provided.

- `0.50.8` lets adoption reports consume saved deployment-check inventory
  evidence:
  ```text
  canic fleet adoption report <fleet> --profile <profile> --deployment-check <path>
  ```
  The option extracts the `DeploymentCheckV1.inventory` artifact and rejects
  ambiguous use with `--inventory`.

- `0.50.7` adds explicit read-only evidence inputs for adoption reports:
  ```text
  canic fleet adoption report <fleet> --profile <profile> --inventory <path>
  canic fleet adoption report <fleet> --profile <profile> --artifact-manifest <path>
  canic fleet adoption report <fleet> --profile <profile> --package-metadata <path>
  ```
  These options feed existing JSON evidence into the report without live
  discovery or mutation.

- `0.50.6` adds active adoption profile architecture docs, including the
  read-only report boundary, profile vocabulary, lifecycle classifications,
  recommendation previews, blocked actions, and evidence rules.

- `0.50.5` polishes adoption text rendering so suggested actions appear as
  `suggested_action_preview` entries, are marked as not executed by the report,
  and blocked actions are framed as non-executed report output.

- `0.50.4` clarifies hybrid external-Wasm reporting: adoption findings now
  include supplied module-hash and external artifact evidence, warn that
  artifact registry import is outside adoption reporting, and explicitly block
  `artifact registry import`.

- `0.50.3` adds standalone and leaf-only adoption report fixtures. Standalone
  roles remain declared-only without synthesized topology, and leaf-only
  reports keep authority-sensitive observed roles visible without recommending
  role declaration.

- `0.50.2` adds brownfield and partial adoption report fixtures, including
  external-controller, observed-only, and declared-only cases that keep all
  adoption recommendations passive and non-executing.

- `0.50.1` adds the read-only fleet-scoped adoption report CLI:
  ```text
  canic fleet adoption report <fleet> --profile <profile>
  ```
  The command renders text by default, can emit experimental JSON, and writes
  only an explicitly requested report artifact through `--output <path>`.

- `0.50.0` starts the passive adoption foundation with host-side adoption
  profiles, report models, role/resource classifications, and a read-only
  report builder that preserves 0.49 declared-vs-attached role boundaries while
  keeping recommendations non-executing.

## [0.49.x] - 2026-05-29 - Role lifecycle foundation

Detailed patch breakdown: [docs/changelog/0.49.md](docs/changelog/0.49.md)

- `0.49.9` refreshes setup/build docs around the hard-cut role lifecycle:
  package metadata now shows both `fleet` and `role`, examples use
  `canic build <fleet> <role>`, and the current handoff names 0.49 as the
  active line.

- `0.49.8` adds workspace governance for `[package.metadata.canic]` fleet-role
  declarations and fixes generated standalone configs so compile-only
  standalone canisters no longer synthesize role attachment topology.

- `0.49.7` adds fleet-scoped role renaming:
  ```
  canic fleet role rename <fleet> <old-role> <new-role>
  ```
  The command updates the selected fleet config, topology role references, and
  matching package metadata when the declared package manifest is editable.

- `0.49.6` hardens deployment-truth role selection so declared-only roles stay
  visible in lifecycle output but are excluded from deployable role selectors,
  install targets, local artifact manifests, inventories, and local deployment
  plans.

- `0.49.5` hard-cuts visible artifact builds to fleet-scoped role identity:
  ```
  canic build <fleet> <role>
  ```
  The command selects the matching fleet config and rejects declared-only roles
  before starting the Cargo artifact build.

- `0.49.4` adds declared-only canister scaffolding:
  ```
  canic scaffold canister <fleet> <role>
  ```
  Developers can create a new ordinary canister crate and role declaration
  before choosing topology placement.

- `0.49.3` adds direct topology attachment for declared roles:
  ```
  canic fleet role attach <fleet> <role> --subnet <subnet>
  canic fleet role attach <fleet> <role> --subnet <subnet> --kind <kind>
  ```
  Operators can move ordinary roles from declared-only to attached topology
  without editing `canic.toml` by hand.

- `0.49.2` adds config-only fleet role declaration:
  ```
  canic fleet role declare <fleet> <role> --package <path>
  ```
  Operators can declare an ordinary package-backed role before topology
  attachment while root and duplicate declarations still fail closed.

- `0.49.1` adds read-only fleet role lifecycle inspection:
  ```
  canic fleet role list <fleet>
  canic fleet role inspect <fleet> <role>
  ```
  Operators can list declared roles and inspect whether a role is still
  compile-only or attached to topology.

- `0.49.0` starts the role-lifecycle foundation: Canic configs now declare
  fleet-scoped `[roles.<role>]`, package metadata includes `fleet`, and
  `canic::build!` validates package `fleet.role` against declarations while
  tracking attached-vs-declared role state. It also fixes generated
  packaged-downstream `wasm_store` bootstrap metadata, scopes CI artifact
  builds to the selected test fleet config, and makes visible `canic build`
  artifact builds require topology-attached roles.

## [0.48.x] - 2026-05-28 - Clean up & Audits

Detailed patch breakdown: [docs/changelog/0.48.md](docs/changelog/0.48.md)

- `0.48.11` hard-cuts delegated-token audiences to singular role/principal
  targets and sets the published MSRV to Rust `1.91.0` while keeping the
  internal toolchain on Rust `1.96.0`. It also regroups top-level CLI help so
  ICP token/cycles wrappers are presented as wallet commands rather than fleet
  commands, and tightens wallet transfer selectors to use
  `<deployment>/<role-or-canister>` for Canic-resolved recipients.

- `0.48.10` adds ICP-shaped `canic cycles` and `canic token` wrappers with
  explicit Canic deployment/role recipient resolution.

- `0.48.9` reruns the capability-scope-enforcement audit, confirms capability
  and scope checks still run after authentication and identity binding, and
  refreshes the recurring audit hotspot path for endpoint auth ordering. It
  also reruns dependency hygiene and confirms published crates still avoid
  unpublished workspace-member dependencies.

- `0.48.8` reruns the token-trust-chain audit and confirms delegated-token
  verification still requires verifier-local root trust, root-certified shard
  authority, canonical cert/claim hashes, shard signatures, and endpoint guard
  ordering.

- `0.48.7` reruns the audience-target-binding audit and hardens canister
  artifact role resolution so builds require exactly one scoped
  `[package.metadata.canic] role` package under the selected canister root.

- `0.48.6` reruns the oldest recurring auth freshness audits, confirms
  subject-caller binding still holds, and aligns delegated grants plus
  role/internal attestations with Canic's exclusive expiry boundary.

- `0.48.5` clarifies Candid artifact behavior, prevents canister artifact
  crates from exposing Rust library targets, documents delegated-token
  audience binding examples, and aligns the closed 0.41-0.47 handoff docs.

- `0.48.4` raises Canic's published MSRV to Rust `1.96.0`, using the new
  standard assertion and duration helpers for clearer diagnostics and simpler
  scheduling constants.

- `0.48.3` adds a demo `user_hub` / `user_shard` sharding walkthrough and
  adopts `ic-testkit` 0.1.9 helpers to simplify PocketIC setup diagnostics.

- `0.48.2` refreshes active setup, configuration, architecture, and crate docs
  so they consistently describe metadata-driven canister roles, derived
  singleton topology, and the single normal `canic::start!()` startup surface.

- `0.48.1` simplifies downstream canister setup by making
  `[package.metadata.canic] role = "..."` the single source of truth for
  `canic::build!` and `canic::start!()`, while removing the old build/root
  macro variants and redundant checked-in fleet scaffolding.

- `0.48.0` removes authored subnet `auto_create` and `subnet_index` config
  lists, deriving both from configured singleton canister roles so fleet setup
  has one source of truth for stable subnet services.

## [0.47.x] - 2026-05-27 - Verified deployment registration

Detailed patch breakdown: [docs/changelog/0.47.md](docs/changelog/0.47.md)

- `0.47.12` adds source-guard coverage proving explicit root verification
  validates deployment-truth evidence before local-state mutation, writes
  verified state through the compare-and-swap helper, and creates receipts only
  after the guarded write.

- `0.47.11` makes root-verification receipts preserve and validate the source
  report `requested_at` timestamp, including `unix:<seconds>` matching against
  the receipt write timestamp for explicit verify-path receipts.

- `0.47.10` makes root-verification receipts preserve the source report source
  enum in JSON, text, and digest input, keeping standalone receipts explicit
  that accepted evidence came from a deployment-truth check artifact.

- `0.47.9` makes root-verification receipts preserve the source report's
  current root-verification state and validate it against the receipt's
  previous local-state trust state.

- `0.47.8` makes root-verification receipts preserve the source report's
  observed root canister ID and passive state transition, binding standalone
  receipt evidence to the exact report path accepted before local-state write.

- `0.47.7` makes root-verification receipts preserve source report evidence
  status and source root observation source, and makes reports archive
  `observed_root_canister_id` as a first-class evidence field.

- `0.47.6` makes archived root-verification reports carry and render
  `observed_root_observation_source`, and binds the `root_observation_source`
  check row to that field during validation.

- `0.47.5` cleans up 0.47 closeout wording by making `canic deploy root`
  describe inspection and explicit verification, updating the design status to
  reflect that the main root-verification gate landed, and clarifying
  root-verification receipt text as local-state mutation without canister
  execution.

- `0.47.4` hardens archived root-verification evidence by rejecting malformed
  digest fields, forged check rows, unsupported or stale source
  `DeploymentCheckV1` artifacts, and duplicate or unexpected report check rows.

- `0.47.3` tightens root-verification receipt validation so local-state digest
  transitions must match the claimed root-verification state transition, and
  adds JSON shape coverage for the root-verification receipt artifact.

- `0.47.2` hardens root verification by making already verified same-root
  verification a receipt-emitting no-op, preserving local state unchanged, and
  blocking verified root-replacement attempts.

- `0.47.1` adds the explicit receipt-backed state transition for registered
  deployment roots whose deployment-truth evidence is satisfied.

```bash
canic deploy root verify demo-local --from-check deployment-check.json
```

- `0.47.0` starts verified deployment registration with explicit
  deployment-root observation evidence, passive root-verification reports, and
  a read-only root inspection command for digest-bound source-check evidence.

```bash
canic deploy root inspect --request root-verification.json
```

## [0.46.x] - 2026-05-26 - Multi-deployment operations

- `0.46.19` updates `canic-host` package metadata, README, and crate docs so
  the host crate describes deployment and fleet-template ownership instead of
  stale fleet-owned live state wording.

- `0.46.18` removes Canic's direct `pocket-ic` dependency edge: the workspace
  and test fleet now depend on PocketIC only through `ic-testkit`, leaving the
  lockfile with the single transitive `pocket-ic` version owned by that
  package.

- `0.46.17` completes the backup artifact hard cut by renaming the persisted
  manifest file to `deployment-backup-manifest.json`, serializing full non-root
  backup plans as `non-root-deployment`, and updating manifest validation
  errors to deployment member/role wording.

- `0.46.16` hard-cuts the `canic-backup` manifest boundary to deployment
  vocabulary: public Rust types are now `DeploymentBackupManifest`,
  `DeploymentSection`, and `DeploymentMember`, manifest JSON now uses
  `deployment` and `deployment_checks`, and crate metadata/docs plus test-only
  helpers use deployment backup wording.

- `0.46.15` hard-cuts restore and snapshot internals around deployment-target
  vocabulary: snapshot download validates explicit canister selections with
  deployment-membership naming, restore plan JSON now uses
  `deployment_verification_checks`, restore verification summaries use
  `deployment_checks`, restore operation counts use
  `deployment_verifications`, journal operation kinds serialize as
  `verify-deployment`, and command previews describe deployment-root
  verification instead of fleet-root verification.

- `0.46.14` cleans remaining backup/restore manifest help around backup
  artifacts: `canic manifest` / `canic manifest validate` now describe backup
  manifests without presenting them as live fleet-owned state, and
  `canic-cli` package metadata now describes deployment backup/restore
  workflows.

- `0.46.13` tightens snapshot download and restore examples around
  deployment-target backup layout naming: `canic snapshot download` now parses
  an installed deployment target, defaults snapshot backup directories to
  `backups/deployment-...`, reports deployment-root/membership errors with
  deployment wording, and restore help examples use the deployment-prefixed
  layout path.

- `0.46.12` tightens backup create/status/inspect as deployment-target
  surfaces: `canic backup create` now uses deployment identity internally,
  defaults new backup directories to `backups/deployment-...`, renders
  `DEPLOYMENT` in create/inspect tables, serializes dry-run status and inspect
  JSON with `deployment`, and maps the legacy lower-level backup plan `fleet`
  field only at the CLI boundary.

- `0.46.11` tightens live operator surfaces as installed-deployment surfaces:
  `canic info list`, `canic info cycles`, `canic metrics`, and
  `canic backup create` help/output now refer to deployment targets instead of
  deployed fleets, metrics/cycle JSON reports serialize `deployment`, and
  `canic config` remains explicitly fleet-template-facing.

- `0.46.10` aligns installed-deployment recovery and help text with the 0.46
  hard cut, making backup, cycles, metrics, list, medic, `info`, and
  deployment-plan assumptions consistently describe deployment targets and the
  required `canic deploy register ... --allow-unverified` acknowledgement.

- `0.46.9` adds automated coverage for the release-index guard and tightens it
  so release commits fail when the index is empty, includes staged deletions or
  non-release files, or has partially staged release files.

- `0.46.8` adds a release-index guard so `make release-commit` refuses staged
  non-release files or partially staged release files before creating a release
  commit and tag.

- `0.46.7` hardens passive deployment comparison so blocked, warning, stale,
  or tampered `DeploymentCheckV1` inputs cannot render as safe comparison
  evidence, archived targets retain deployment identity, and release-version
  bump scripts no longer run unrelated protocol/install tests internally.

- `0.46.6` cleans installed-deployment CLI wording across backup, cycles,
  metrics, list, status, medic, and top-level help surfaces so missing/lost
  live-state messages consistently describe deployment targets and explicit
  `deploy register` recovery instead of stale fleet-owned placeholders.

- `0.46.5` hardens the deployment-target recovery path so unverified
  registered roots are install safety blockers, not warnings. Legacy
  fleet-state recovery guidance now requires operators to provide the owning
  fleet template explicitly, and source guards keep `canic deploy check` plus
  host check/preflight paths read-only so they cannot silently rewrite
  `root_verification`.

- `0.46.4` tightens deployment-target state as hard-cut state: local state now
  records `created_at_unix_secs` and `updated_at_unix_secs`, stale state with
  the old `installed_at_unix_secs` field fails closed, and explicit recovery
  registration requires `--allow-unverified` before writing a root that has not
  been live-verified.

```bash
canic deploy register demo-local --fleet-template demo --root uxrrr-q7777-77774-qaaaq-cai --allow-unverified
```

- `0.46.3` removes stale fleet-owned naming from the deployment-target
  install-state API and state shape. Local install state now stores
  `deployment_name` and `fleet_template` without a duplicate `fleet` field, the
  shared host lookup boundary is now `installed_deployment`, receipt paths and
  deployment-state readers use deployment-target terminology, and
  deployment-target state that still contains the stale `fleet` field fails
  closed instead of being accepted as current state.

- `0.46.2` makes plan-mediated deploy install and read-only deploy truth
  commands target-explicit. `canic deploy install <deployment> --plan <file>`
  rejects plans whose `deployment_name` does not match the requested target,
  and `canic deploy check <deployment>` resolves registered deployment state to
  the correct fleet-template config without falling back to stale fleet-named
  live state.

```bash
canic deploy install demo-local --plan promoted-plan.json
canic deploy check demo-local
```

- `0.46.1` begins the deployment-target state hard cut: local install state now
  writes under `.canic/<network>/deployments/<deployment>.json`, deployment
  truth reads target-named state instead of fleet-template state, legacy
  `.canic/<network>/fleets/*.json` live state fails closed, supplied install
  plans require exact deployment identity, and `canic deploy register` provides
  the explicit minimal recovery path for known roots. It also refreshes the
  first-install guide, improves missing `canic::finish!()` guidance, and keeps
  first-install execution preflight from blocking on absent prior root
  authority observation.

```bash
canic deploy register demo-local --fleet-template demo --root uxrrr-q7777-77774-qaaaq-cai --allow-unverified
```

- `0.46.0` starts passive multi-deployment comparison with
  `DeploymentComparisonReportV1`, a `canic deploy compare` operator command,
  current upstream ICP CLI/ic-wasm npm tooling instead of repo-pinned versions,
  current scaffold/getting-started docs for the hard-cut fleet shape, and
  post-46 backlog docs that no longer look like approved numbered follow-ons.

```bash
canic deploy compare --left staging-check.json --right prod-check.json
```

See detailed breakdown:
[docs/changelog/0.46.md](docs/changelog/0.46.md)

## [0.45.x] - 2026-05-26 - External lifecycle

- `0.45.9` adds inventory-backed external lifecycle verification checks that
  can derive verification observations from existing `DeploymentCheckV1`
  inventory artifacts, bind deployment-plan, check, inventory, module, config,
  controller-control-class, and protected-call evidence, and keep supplied
  observations as consistency-only evidence that cannot mark external work
  live-verified.

- `0.45.8` adds passive external upgrade completion reports that combine
  proposal, consent-evidence, and verification-check artifacts without
  treating consent or reported external action as completion proof.

```bash
canic deploy external inspect completion --request external-completion.json
```

- `0.45.7` adds passive external upgrade verification checks that evaluate
  supplied observation facts against verification policies and reject archived
  checks with duplicate or internally inconsistent requirement rows.

```bash
canic deploy external inspect verification-check --request external-verification-check.json
```

- `0.45.6` adds passive external upgrade verification-policy artifacts so
  proposals can publish digest-bound live-inventory postconditions before any
  externally reported lifecycle action is treated as complete.

```bash
canic deploy external inspect verification-policy --request external-verification-policy.json
```

- `0.45.5` adds passive external lifecycle check and handoff artifacts so
  operators can summarize direct/pending/blocked lifecycle work and package
  pending external proposals into coordination instructions without delivering
  consent or executing upgrades.

```bash
canic deploy external check demo
canic deploy external handoff demo
```

- `0.45.4` adds passive critical-fix, consent-evidence, and verification
  report artifacts for external lifecycle work, keeping reported consent,
  reported external action, and verified live completion distinct.

```bash
canic deploy external critical-fix --fix-id fix-2026-05 --severity critical demo
canic deploy external inspect consent --request external-consent.json
canic deploy external verify --request external-verification.json
```

- `0.45.3` adds passive pending external lifecycle reports and hardens
  external-upgrade receipts against stale proposal pairing.

```bash
canic deploy external pending demo
```

- `0.45.2` adds the first passive external lifecycle CLI reports for lifecycle
  plans and external-upgrade proposals.

```bash
canic deploy external plan demo
canic deploy external proposals demo
```

- `0.45.1` hardens passive external lifecycle artifacts with deterministic
  report/plan/proposal/receipt validation, source-check linkage, passive text
  rendering, and no-mutation source guards.

- `0.45.0` starts the external/user-owned lifecycle line with passive
  lifecycle authority projection from existing `CanisterControlClassV1`
  deployment truth, central `ExternalLifecyclePlanV1` partitioning, and the
  first passive external-upgrade proposal artifacts that bind current
  observations to target artifact/config facts without granting consent or
  executing upgrades. It also adds external lifecycle receipts that
  structurally record pending, refused, delegated, or externally completed
  outcomes while keeping live inventory as truth.

See detailed breakdown:
[docs/changelog/0.45.md](docs/changelog/0.45.md)

## [0.44.x] - 2026-05-25 - Artifact promotion

- `0.44.16` closes the artifact-promotion line: promotion is represented as
  digest-pinned, authority-preserving `DeploymentPlanV1` transformation and
  passive provenance artifacts, with promoted-plan execution mediated by the
  normal deployment-truth/preflight install runner.

- `0.44.15` emits artifact promotion execution receipt wrappers after
  successful plan-mediated installs from ready `ArtifactPromotionPlanV1`
  envelopes, linking promotion plan/provenance evidence to the nested
  deployment receipt without adding a promotion executor.

- `0.44.14` hardens plan-mediated promotion install with direct coverage for
  ready and blocked promotion plan envelopes, raw deployment plan input, target
  network mismatches, missing root wasm artifacts, and the current-install
  mediation source guard.

- `0.44.13` adds plan-mediated promotion install for supplied
  `DeploymentPlanV1` or `ArtifactPromotionPlanV1` files, routing execution
  through the current install runner/gate path.

```bash
canic deploy install --plan promoted-plan.json
```

- `0.44.12` adds a small passive artifact-promotion CLI surface for planning,
  readiness checks, and transform diffs, while demoting DTO-level promotion
  reports under the advanced inspect namespace.

```bash
canic deploy promote plan --request promotion-plan.json
canic deploy promote check --request promotion-check.json
canic deploy promote diff --request promotion-diff.json
canic deploy promote inspect readiness --request promotion-readiness.json
canic deploy promote inspect artifact-identity --request promotion-artifacts.json
canic deploy promote inspect provenance --request promotion-provenance.json
```

- `0.44.11` digest-pins promotion readiness, transform evidence, and build
  materialization evidence, carrying materialization evidence digests through
  source-build transform, provenance, and execution receipt role rows.

- `0.44.10` digest-pins passive promotion policy, catalog verification,
  provenance, and execution receipt artifacts so archived promotion evidence
  rejects stale policy decisions, catalog observations, report links, and
  receipt drift.

- `0.44.9` adds passive wasm-store catalog verification with deterministic
  role observation digests, provenance linkage, locator-drift blockers, and
  execution receipt preservation of catalog evidence.

- `0.44.8` adds passive promotion execution receipt wrappers and validated
  artifact identity summary counters, tightening promotion evidence without
  introducing a separate promotion executor.

- `0.44.7` adds passive wasm-store, source/build materialization, and
  promotion provenance reports so promotion planning can link staged bytes,
  materialized outputs, and plan evidence without claiming execution.

- `0.44.6` adds passive artifact promotion plan envelopes with target execution
  lineage and deployment-check validation for promoted-plan preflight evidence.

- `0.44.5` adds materialization-linked promotion transforms and lineage digests
  for promoted plans and receipt-backed artifact sources.

- `0.44.4` adds passive role promotion policy checks and readiness integration
  that distinguish sealed-byte-only roles from byte-identical source/build
  promotion roles.

- `0.44.3` adds passive source/build materialization identity and linkage
  evidence for build recipes, target-specific inputs, and output digests.

- `0.44.2` adds passive promotion artifact identity reports that separate
  source locator kind from artifact identity kind and group roles by
  deterministic identity keys.

- `0.44.1` adds passive promoted-plan transformation and transform evidence
  artifacts, including validation and text output that explicitly reports no
  execution occurred.

- `0.44.0` starts the artifact-promotion line with passive role artifact
  source, promotion readiness, and digest-pinned override validation.

See detailed breakdown:
[docs/changelog/0.44.md](docs/changelog/0.44.md)

## [0.43.x] - 2026-05-24 - Backend-agnostic execution

- `0.43.8` completes the current-install runner bridge by routing activation
  phases through a private phase-operation runner and guarding the
  deployment-truth/preflight-before-mutation ordering.

- `0.43.7` moves the remaining current-install root resolution, build,
  manifest, resume, and readiness phases behind narrow operation values and
  aligns execution preflight evidence with the installer receipt phases.

- `0.43.6` adds a testkit preflight context that validates the same
  `DeploymentPlanV1` shape as the current CLI executor, and moves current
  install root wasm installation, root funding, and release-set staging through
  narrow operation values.

- `0.43.5` hardens deployment receipt status semantics so failed execution
  receipts distinguish pre-mutation, post-mutation, and partial application
  from role-phase evidence, and resume checks reject contradictory receipts.

- `0.43.4` adds typed artifact-staging receipts and enriches current-install
  `stage_release_set` evidence from the release-set manifest.

- `0.43.3` removes the standalone `canic-cdk` workspace crate while preserving
  the public `canic::cdk` facade through `canic-core::cdk`.

- `0.43.2` hardens passive execution-preflight evidence with provenance,
  consistency, capability, and JSON-shape validation.

- `0.43.1` expands passive execution-readiness checks and records
  `execution_preflight` receipts before later current-install phases continue.

- `0.43.0` starts the plan-driven execution line with executor context,
  capability evidence, current-CLI backend scaffolding, and passive execution
  preflight readiness.

```bash
canic deploy plan <deployment>
canic deploy check <deployment>
canic deploy authority check <deployment>
```

See detailed breakdown:
[docs/changelog/0.43.md](docs/changelog/0.43.md)

## [0.42.x] - 2026-05-23 - Authority reconciliation

- `0.42.14` hardens the authority closeout boundary without adding controller
  mutation: authority CLI help now documents exit-status scope, authority
  `Safe` is documented as authority-scoped rather than whole-deployment-safe,
  dry-run receipts/evidence are documented as structural self-consistency
  artifacts, and tests now guard authority paths against controller mutation
  primitives while pinning the `Authority*V1` JSON schema shape. The design
  now also records explicit `Authority*V1` schema-governance rules for future
  field, enum, and receipt-surface evolution, and propagates the 0.42.14
  handoff constraints into the 0.43 through 0.46 design docs.

- `0.42.13` closes out the authority reconciliation line with a focused
  closeout audit and status handoff updates confirming 0.42 remains a
  dry-run/report-first authority evidence release.

- `0.42.12` tightens authority receipt-only output and human-facing dry-run
  labels so standalone receipts keep provenance/timestamp guards and every
  authority text/help surface clearly reflects the read-only dry-run boundary.

- `0.42.11` hardens standalone authority receipt construction, separates the
  reusable PocketIC helpers into `ic-testkit`, lowers the declared MSRV to
  Rust 1.88, and removes stale CDK static-canister/wrapper surfaces.

- `0.42.10` tightens authority blocker reporting so unsafe canister authority,
  hard authority findings, external actions, missing observations, and
  automatic dry-run candidates stay distinct in blocked dry-run reports and
  evidence validation.

- `0.42.9` moves authority evidence/report construction and local
  report/receipt/evidence ID generation into `canic-host`, strengthens
  dry-run evidence provenance/timestamp validation, and thins CLI authority
  tests back to parsing, format selection, and host-helper delegation.

- `0.42.8` hardens authority dry-run evidence validation so archived evidence
  rejects schema drift, stale report-derived fields, invalid receipt
  completion state, and mismatched evidence/receipt completion timestamps.

- `0.42.7` adds host-owned human-readable text output for read-only authority
  dry-run commands while preserving JSON as the default automation format.

```bash
canic deploy authority check <deployment> --format text
canic deploy authority evidence <deployment> --format text
canic deploy authority report <deployment> --format text
canic deploy authority receipt <deployment> --format text
```

- `0.42.6` hardens authority dry-run evidence: reports and receipts now carry
  source check/inventory/profile provenance, receipt construction rejects mixed
  report/plan/check data, and complete evidence bundles are validated before
  CLI output.

- `0.42.5` makes authority reports and receipts more self-describing by
  preserving controller deltas, report IDs, inventory IDs, and profile hashes,
  and fixes bootstrap `wasm_store` artifact builds on runners without the
  optional `ic-wasm` binary.

- `0.42.4` tightens dry-run authority report semantics: external-action
  records now contain only actual external authority actions, standalone
  receipts preserve unresolved observation gaps, reports include typed
  apply-readiness blockers, and the 0.42 design/status docs now frame apply,
  pool mutation, remote lock/epoch checks, and post-apply verification as
  promoted-or-later work.

- `0.42.3` tightens break-glass authority reporting: staging/emergency overlap
  with normal controllers now blocks dry-run authority plans, hard findings are
  counted and preserved in receipts, and blocked reports emit specific next
  actions for unsafe canister findings versus hard authority findings.

- `0.42.2` adds evidence-only authority dry-run receipts and read-only
  `canic deploy authority receipt|evidence <deployment>` JSON surfaces, preserving
  controller observations and unresolved external actions without attempting
  controller mutation.

```bash
canic deploy authority receipt <deployment>
canic deploy authority evidence <deployment>
```

- `0.42.1` adds the read-only authority report surface and completes the first
  self-contained dry-run evidence model: reports include status/counts,
  external actions, pool authority cases, automatic action candidates, and
  typed gap/action/control-class breakdowns without applying controller
  changes.

```bash
canic deploy authority report <deployment>
```

- `0.42.0` starts dry-run authority reconciliation with
  `AuthorityReconciliationPlanV1`, a passive planner over the 0.41 deployment
  truth check, and read-only `canic deploy authority check <deployment>` output for
  controller-state classification without IC controller mutation.

```bash
canic deploy authority check <deployment>
```

See detailed breakdown:
[docs/changelog/0.42.md](docs/changelog/0.42.md)

## [0.41.x] - 2026-05-21 - Deployment truth model

- `0.41.18` is a cleanup-only deployment truth report refactor: duplicate
  evidence grouping and diff/finding construction now share local helpers,
  with no intended operator-facing behavior change.

- `0.41.17` hardens receipt and planned verifier evidence handling: duplicate
  phase receipts, duplicate role-phase receipts, and duplicate planned
  verifier role-epoch expectations now warn for exact duplicates and hard-fail
  when evidence conflicts.

- `0.41.16` hardens deployment truth duplicate-evidence handling across
  observed artifacts, verifier role epochs, planned artifacts, planned
  canisters, and planned pool identities: conflicting evidence now hard-fails,
  while exact duplicate evidence warns and is compared only once.

- `0.41.15` tightens passive deployment truth topology validation: enriched
  live-status evidence now feeds pool and controller drift checks, concrete
  canister IDs and observed role identities are checked for contradictions
  across non-pool and pool inventory surfaces, ambiguous role-only matches
  hard-fail, and installed module-hash comparison now targets concrete planned
  canister IDs when available.

- `0.41.14` expands passive deployment truth topology coverage: the implicit
  bootstrap `wasm_store` now participates in planned and observed artifact
  evidence, child registry observations can be enriched with live status,
  controllers, and module hashes, and extra or duplicate non-pool canisters now
  warn as topology drift.

- `0.41.13` fills in passive deployment identity digests from release-set
  manifests, parsed runtime config, topology, artifacts, pools, and authority
  facts, and maps subnet-registry role entries into observed canister facts
  without treating registry-only observations as controller authority.

- `0.41.12` expands current-install deployment truth receipts across the
  existing installer phases, including root resolution, build, manifest,
  mutating activation phases, readiness, final install-state persistence, and
  role-scoped build outcomes.

- `0.41.11` tightens current-install deployment truth by blocking every
  `SafetyReportV1` hard failure, persisting the artifact-gate receipt as local
  JSON, and letting `deploy resume-report` discover the latest local receipt by
  default.

```bash
canic deploy resume-report <deployment>
```

- `0.41.10` expands passive deployment truth coverage for pool and verifier
  readiness facts: configured pool identities now enter plans, installed
  registry entries can populate observed pool inventory, pool/readiness drift is
  diffed, and ambiguous pool observations become typed gaps.

- `0.41.9` adds receipt-aware deployment truth comparison for passive resume
  reporting, prints explicit `Complete` or `FailedBeforeMutation` receipt
  status from the current-install artifact gate, and introduces read-only
  `canic deploy resume-report <deployment> --receipt <file>` to render
  `ResumeSafetyV1` without resuming or mutating state.

```bash
canic deploy resume-report <deployment> --receipt <file>
```

- `0.41.8` extends local deployment truth plans with installed root identity
  from `.canic` state, records missing install state as a non-blocking plan
  assumption, blocks current install when a concrete expected root canister is
  missing from observed inventory, labels gate findings by source
  (`plan`, `inventory`, or `diff`), and adds role-scoped artifact
  materialization receipt evidence without making receipts the gate authority.

- `0.41.7` wires configured deployment controllers into the local deployment
  truth plan, expands the current-install safety gate to block artifact digest
  and observable controller-authority drift, and prints finding codes for
  scriptable gate failures.

- `0.41.6` adds controller authority comparison to the deployment truth diff so
  live root controllers must include expected authority profile controllers,
  unsafe authority-profile overlaps block, undeclared live controllers warn,
  and declared staging/emergency controllers are treated as intentional.

- `0.41.5` realigns deployment truth with the revised 0.41 design by keeping
  raw config hashes separate from canonical deployment identity, adding
  read-only live root status inventory, comparing installed module hashes, and
  extending receipts for partial-execution evidence.

- `0.41.4` adds deployment truth receipt evidence for the current-install
  artifact gate, exposes direct `deploy diff` and `deploy report` JSON views,
  and tightens read-only safety checks around local config and artifact digest
  drift.

```bash
canic deploy diff <deployment>
canic deploy report <deployment>
canic deploy check <deployment>
```

- `0.41.3` adds read-only `canic deploy plan|inventory|check <deployment>` JSON
  surfaces, adapts current install inputs into deployment truth checks, and
  blocks installer continuation after build when configured role artifacts are
  missing.

```bash
canic deploy plan <deployment>
canic deploy inventory <deployment>
canic deploy check <deployment>
```

- `0.41.2` adds the read-only local deployment truth check pipeline: local
  inventory, role artifact manifests, source-tagged artifact hash evidence,
  local plan construction, normalized diffs, safety reports, and per-design
  status logs without changing installer mutation behavior.

- `0.41.1` adds passive `canic-host::deployment_truth` V1 model scaffolding for
  deployment plans, inventories, receipts, diffs, safety reports, role
  artifacts, canister control classifications, verifier-readiness observations,
  and phase postconditions, with JSON round-trip tests and no installer
  behavior changes.

- `0.41.0` starts the deployment-truth design-prep line by framing
  `DeploymentPlanV1`, `DeploymentInventoryV1`, `DeploymentReceiptV1`,
  `DeploymentDiffV1`, and `SafetyReportV1` as the next installer correctness
  boundary after 0.40's attested internal-call hard cut. This slice is
  documentation and architecture preparation only; it does not change runtime
  deployment behavior.

See detailed breakdown:
[docs/changelog/0.41.md](docs/changelog/0.41.md)

---

## [0.40.x] - 2026-05-19 - Attested Canic calls

- `0.40.15` updates Canic to `ic-memory 0.6.1`, removes the last public ledger-codec integration surface from Canic's native memory diagnostic path, and keeps ID 255 rejection delegated to `ic-memory` slot validation.

- `0.40.14` tightens root-signed auth-material time windows: verifiers reject malformed or future role-attestation/internal-invocation proof windows, `CanicCall` avoids caching proof material verifiers will reject, and root issuance now shares one TTL/window path for role attestations and internal invocation proofs.

- `0.40.13` aligns the 0.40 attested-call design and access-contract docs with the current raw-ingress protected wrapper model, makes `CanicCall` dispatch encoded envelopes through raw ingress bytes, validates empty method/role/TTL metadata locally, and strengthens the protected raw-call source guard so multi-line protected method literals/constants are caught without misclassifying `CanicCall::...` usage.

- `0.40.12` makes protected internal endpoint wrappers decode the Canic envelope from raw ingress bytes inside Canic, so malformed raw calls return typed `InternalRpcMalformed` errors instead of failing in the CDK argument decoder.

- `0.40.11` extends the protected internal-call source guard beyond wasm-store methods so shared protocol descriptors and protected endpoint declarations are also treated as raw-call-forbidden Canic internal methods.

- `0.40.10` makes root authoritative for role-attestation epochs, removes the legacy caller-supplied epoch from canonical replay/signature payloads, fixes outbound attestation cache reuse for newer root-signed epochs, and domain-separates the remaining root request/capability metadata nonce streams.

- `0.40.9` adds a real app-style project hub/instance fixture for generated protected clients, proves it through PocketIC, fixes the built-in wasm-store protected client decode path, and domain-separates auth-material root request metadata from other root request IDs.

- `0.40.8` adds `canic_protected_endpoint!` for shared protocol modules to publish protected internal endpoint descriptors consumed by `canic_internal_client!`, covering the cross-canister case where the caller crate cannot depend on the target canister implementation crate, and hardens descriptor construction so protected endpoint metadata cannot omit its method or caller roles.

- `0.40.7` adds the first facade macro for typed protected internal clients and promotes protected endpoint descriptor accessors to the stable generated `canic_internal_endpoint_<endpoint>()` naming shape.

- `0.40.6` starts the app-facing generated-client surface by adding protected internal endpoint descriptors emitted by the endpoint macro, a generic `CanicInternalClient` that calls those descriptors through `CanicCall`, and moving the built-in wasm-store client onto protocol-owned protected endpoint descriptors.

- `0.40.5` removes the transitional AppIndex-only `caller::has_app_role(...)` access predicate from the macro DSL and runtime access evaluator; protected sibling RPC must use root-signed `caller::has_role(...)` or `caller::has_any_role(...)`.

- `0.40.4` starts the built-in internal-client pass by moving root wasm-store calls behind a typed `WasmStoreInternalClient`, adding a private root auth material client for structural proof/key requests, and exposing the wasm-store protected/query manifests through the public `canic::protocol` facade.

- `0.40.3` starts the protected-internal-call guardrail pass by centralizing the protected wasm-store method list, checking the macro and `.did` surfaces against it, classifying built-in internal endpoints under the 0.40 exception model, asserting protected macro wrappers verify the exported method name, and adding a first-party source scan that fails if protected methods are called through raw `Call`/`CallOps` instead of `CanicCall`.

- `0.40.2` moves wasm-store update calls onto protected `CanicCall` envelopes, fixes direct root proof decoding, delegates generic multi-crate memory registration to `ic-memory 0.5.1`, and makes `canic-testkit` standalone from Canic runtime crates by moving Canic-specific PocketIC harness helpers into `canic-testing-internal`.

- `0.40.1` adds an outgoing heap cache for `CanicCall` internal invocation proofs, keyed by the exact root/key/subject/role/audience/method/subnet/TTL call edge and evicted before expiry or when the local role epoch floor has moved past the cached proof, plus typed internal-call auth error codes so `CanicCall` can invalidate cached proof material and retry once for stale epochs or unknown verifier keys.

- `0.40.0` starts the attested internal-call hard cut by adding passive wire DTOs for method-scoped internal invocation proofs and Canic internal-call envelopes, a distinct signing domain for internal invocation proof payloads, root issuance for AppIndex or subnet-registry authorized internal callers, verifier-side method/role proof validation, the first protected update wrapper path for `caller::has_role(...)` endpoints, and the low-level `CanicCall` envelope-sending primitive.

See detailed breakdown:
[docs/changelog/0.40.md](docs/changelog/0.40.md)

---

## [0.39.x] - 2026-05-18 - `ic-memory` extraction

- `0.39.16` delegates `MemoryManager` range authority and native stable-cell allocation persistence to published `ic-memory 0.4.0`, removes the temporary local crate patch, removes Canic-local range/live registry duplication including the stale named application authority range, removes the opt-in live memory registry diagnostic, and hard-cuts Canic allocation persistence to the native `ic-memory` durable ledger instead of preserving the old Canic physical ledger format.

- `0.39.15` points Canic at the standalone crates.io `ic-memory 0.0.1` package and removes the in-tree `ic-memory` workspace member.

- `0.39.14` turns dual-slot protected recovery and diagnostics into `ic-memory` store-trait mechanics and adds protected-slot recovery details to the Canic memory-ledger diagnostic response.

- `0.39.13` moves reusable dual-slot recovery selection into `ic-memory` and makes Canic ledger commits choose the inactive slot from validated recovery state instead of an unprotected header pointer.

- `0.39.12` routes Canic memory bootstrap and opening through `ic-memory` declaration snapshots, allocation-history validation, and validated sessions while keeping Canic namespace/range rules in a production policy adapter.

- `0.39.11` removes the `canic-memory` crate from the workspace by moving its remaining runtime backend into `canic-core` and making `ic-memory` the direct allocation-governance dependency.

- `0.39.10` moves the Canic managed-memory macro surface into `canic-core`, removes the macro surface from `canic-memory`, and confines direct backend references to the temporary core adapter boundary.

- `0.39.9` shrinks the remaining Canic memory boundary by removing direct `canic-memory` dependencies from the facade and control plane, routing managed-memory access through `canic-core` while `canic-memory` continues toward retirement.

- `0.39.8` moves `MemoryManager` slot-shape validation into `ic-memory`, starts making `canic-memory` consume the local extraction crate directly, and reallocates Canic framework IDs so `0-9` belong to future `ic-memory` governance.

- `0.39.7` adds Canic-owned policy adapter coverage for mapping `ic-memory` allocation slots to Canic's MemoryManager ID ranges without wiring publishable runtime crates to the unpublished extraction crate.

- `0.39.6` adds explicit empty-ledger genesis initialization, ledger format and integrity checks, protected commit recovery diagnostics, and generation-scoped reserve/retire bootstrap operations to the local `ic-memory` extraction crate, while corrupt, incompatible, malformed, or partially written stores still fail closed.

- `0.39.5` adds generic protected ledger commit mechanics plus explicit reserve/retire/bootstrap lifecycle primitives to the local `ic-memory` extraction crate.

- `0.39.4` keeps `ic-memory` as unpublished local extraction scaffolding, restores `canic-memory` as a self-contained publishable crate, and blocks publishing when publishable manifests depend on unpublished local crates.

- `0.39.3` was published out of sequence while the local `ic-memory` dependency boundary was still being corrected; use `0.39.4` as the packaging correction.

- `0.39.2` keeps the `ic-memory` extraction local while tightening generic validated-session boundaries and preserving `canic-memory` as a publishable self-contained crate.

- `0.39.1` adds an AppIndex-backed `caller::has_app_role(role)` access predicate for internal sibling checks.

- `0.39.0` starts the standalone `ic-memory` extraction by adding a generic allocation-governance crate built around `stable_key -> allocation_slot forever` while keeping Canic-specific range, namespace, controller, and lifecycle policy in Canic adapters.

See detailed breakdown:
[docs/changelog/0.39.md](docs/changelog/0.39.md)

---

## [0.38.x] - 2026-05-17 - Stable memory ABI hard cut

- `0.38.9` removes the misleading `canic fleet sync` surface, replaces it with `canic fleet check <name>`, and cleans up the hidden control-plane support boundary.

- `0.38.8` stops deriving `icp.yaml` from `canic.toml`, makes `canic status` and the legacy `fleet sync` path check ICP project config read-only, pins the checked-in local ICP network to the current v13 launcher tag, and adds an upstream watch so CI flags the first candidate release after the delegation-certificate fix.

- `0.38.7` defragments the Canic core memory map before the ABI layout is frozen and keeps ICP network settings such as `ii`/`nns` owned by `icp.yaml` during fleet sync.

- `0.38.6` adds persisted ABI ledger `layout_epoch` validation and exposes the epoch through the controller-only ledger diagnostic.

- `0.38.5` makes `canic info cycles` include fresh auto-top-up events immediately and aligns stable-memory ABI documentation around the final Canic-managed memory contract.

- `0.38.4` tightens stable-memory ABI guard coverage across the Canic-managed runtime surface and clarifies declaration/bootstrap/opening phase separation for startup-selected memory slots.

- `0.38.3` moves the controller-only `canic_memory_ledger` recovery diagnostic into the default Canic runtime bundles, including `wasm_store`, while keeping the heavier live `canic_memory_registry` diagnostic opt-in.

- `0.38.2` adds a controller-only `canic_memory_ledger` diagnostic query for opt-in memory observability builds, exposing committed ID `0` ABI ledger state without using normal Canic endpoint dispatch.

- `0.38.1` adds optional schema-version and schema-fingerprint metadata, canonical range authority records, stricter physical-header validation, raw stable-memory bootstrap preflight, and restricted ID `0` diagnostic reads to the stable-memory ABI ledger and diagnostics.

- `0.38.0` starts the stable-memory ABI hard cut by making explicit stable keys mandatory for Canic-managed memory identity, reserving Canic IDs `0-99` and application IDs `100-254`, making ID `255` permanently invalid, adding the canonical ID `0` ledger self-record, splitting startup declaration from memory opening, rejecting duplicate runtime declarations before user ledger mutation, and adding framework guard coverage for implicit memory registration.

See detailed breakdown:
[docs/changelog/0.38.md](docs/changelog/0.38.md)

---

## [0.37.x] - 2026-05-16 - Quis ipsos auditores audit?

- `0.37.2` restores stable-memory ABI tracking by adding a persisted `canic-memory` layout ledger at memory ID `0`, reserving `0-4` for layout metadata, recording every owner range and registered memory ID across upgrades, rejecting historical range or ID reuse, defragmenting framework ownership so `canic-control-plane` owns `5-10` and `canic-core` reserves `11-79`, adding explicit stable-memory ABI keys so Canic-owned identity survives crate/type/label renames, and updating memory-range guidance so full Canic applications use `80-254` while ID `255` remains reserved by `ic-stable-structures`.

- `0.37.1` refreshes the layer boundary, workflow purity, ops purity, access purity, and security-boundary ordering audits, moves module-source runtime ownership below workflow with API compatibility re-exports, removes workflow dependencies on storage canister records, policy definitions, mutable funding ledgers, and DTO conversion helpers, renames delegated-auth certificate validation from an ops-owned policy surface to explicit certificate rules and TTL limits, routes app-mode and whitelist access checks through ops helpers, preserves verifier behavior and the existing `cert_policy` metrics label, adds endpoint macro and RPC attestation-cache ordering guards, splits installation guidance into `INSTALLING.md`, refreshes README command examples, and tightens CI layering guards.

- `0.37.0` starts a cleanup minor by rerunning the bootstrap lifecycle and canonical auth boundary audits, tightening non-root post-upgrade failures so runtime continuation errors return through the lifecycle adapter, refreshing the auth audit definition around the current macro/core verifier boundary, and exporting `DelegatedToken` from the prelude for authenticated endpoint authors.

See detailed breakdown:
[docs/changelog/0.37.md](docs/changelog/0.37.md)

---

## [0.36.x] - 2026-05-15 - Backup/restore proofing

- `0.36.15` adds `canic restore status/run --require-ready` so operators and CI can fail a prepared restore before mutation when the apply journal is blocked or not ready, while still writing the JSON status summary first.

```bash
canic restore status 1 --require-ready --require-no-attention
canic restore run 1 --dry-run --require-ready --require-no-attention
canic restore run 1 --execute --max-steps 1 --require-no-attention
```

- `0.36.14` makes row-reference restore execution fail closed when the prepared apply journal's `backup_root` is missing or points at a different backup directory, preventing copied or stale journals from loading artifacts outside the selected backup row.

```bash
canic restore run 1 --dry-run
canic restore status 1 --require-no-attention
canic restore run 1 --execute --max-steps 1 --require-no-attention
```

- `0.36.13` polishes the row-reference restore operator path by adding prepare/status/run examples to CLI help and docs, making missing prepared plan or apply-journal defaults fail with actionable `canic restore prepare <backup-ref>` guidance, and refreshing the 0.36 restore design flow around backup-list references.

```bash
canic restore help
canic restore prepare 1 --require-verified --require-restore-ready
canic restore status 1 --require-no-attention
canic restore run 1 --execute --max-steps 1 --require-no-attention
```

- `0.36.12` removes the `/tmp` restore choreography by adding `canic restore prepare <backup-ref>`, backup-list row references for restore plan/apply/run/status, default restore plan and apply-journal files inside the backup layout, and `canic restore status <backup-ref>` for completion and attention gates.

```bash
canic restore prepare 1 --require-verified --require-restore-ready
canic restore apply 1 --dry-run
canic restore run 1 --dry-run
canic restore status 1 --require-complete --require-no-attention
```

- `0.36.11` proves the full six-canister non-root fleet restore path from a verified backup through plan, apply, dry-run, max-step execute/resume, completion enforcement, and final readiness, and adds `canic backup prune` so operators can preview and remove failed backups or keep only the newest entries without manual directory deletion.

```bash
canic backup prune --failed --dry-run
canic backup prune --keep 1 --dry-run
canic restore run 1 --execute --max-steps 1 --require-no-attention
```

- `0.36.10` proves the real backup/restore operator path for local subtree and full non-root fleet backups, fixes restore runner ICP snapshot commands so network flags are passed to the leaf command and fresh uploads do not incorrectly request resume state, fixes full fleet backup manifests so disconnected root-omitted branches become separate backup units, normalizes `canic backup list` timestamps for unfinished execution layouts, and makes `canic info cycles` show explicit burn and top-up rates in a compact default table with wider diagnostics behind `--verbose`.

```bash
canic backup create test
canic backup verify 1
canic info cycles test
canic info cycles test --verbose
```

- `0.36.9` moves deployed-fleet read queries under the new `canic info` command group with `info list` and `info cycles` leaves, removing the old top-level `list` and `cycles` aliases.

```bash
canic info list test
canic info list test --subtree user_hub
canic info cycles test
```

- `0.36.8` tightens restore-runner journal loading so terminal restore operations must be backed by the latest matching command receipt attempt with the same state timestamp, and lets `canic list --subtree` and `canic cycles --subtree` resolve unique role names while requiring principals for repeated roles.

- `0.36.7` tightens restore apply-journal command receipts so persisted receipts must keep their timestamp, command, status, and bounded output audit fields, stops stale local replica status metadata from being treated as running unless the configured gateway port is reachable, and keeps `icp.yaml` fleet sync from churning `networks:` below `environments:`.

- `0.36.6` tightens backup execution integrity so terminal operations and timestamps must match the latest durable receipt, persisted restart-required state must match the operation graph, and execution transitions must carry audit timestamps.

- `0.36.5` tightens backup execution receipts so operation outcomes keep audit timestamps and invalid receipts cannot leave partial journal state behind.

- `0.36.4` rejects duplicate restore receipt attempts and adds an active-line changelog width check for root and detailed release notes.

- `0.36.3` starts restore-runner hardening by failing upload steps that do not return the uploaded snapshot id needed by later load steps, and adds explicit failed-operation retry recovery, stricter legacy upload-id parsing, and receipt checks for terminal apply-journal operations.

- `0.36.2` makes execution-backed backup layout errors consistent across status, inspect, and verify, and tightens execution integrity so completed mutating work must have matching receipts.

- `0.36.1` hardens backup resume safety for existing `--out` layouts and makes backup create/list report reused or invalid execution-backed layouts more clearly.

- `0.36.0` starts the Backup/Restore V1 hardening line by documenting the existing execution flow and proving backup create resume, runner retry, list/status reporting, verification, completion gating, and manifest finalization behavior against durable journals.

See detailed breakdown:
[docs/changelog/0.36.md](docs/changelog/0.36.md)

---

## [0.35.x] - 2026-05-13 - Gettin' it workin'

- `0.35.16` removes stale `icp project show` handoff guidance and trims the remaining `icp.yaml` parser surface by deduplicating top-level section scanning and dropping an unused gateway-port wrapper.

- `0.35.15` hardens local replica status parsing, pins gateway discovery to top-level ICP network config, sends live-read probes through query calls, and tightens JSON receipt/response parsing.

- `0.35.14` keeps foreground local-replica installs on the direct gateway path when replica status root-key data is returned as JSON or CBOR.

- `0.35.13` keeps foreground local-replica installs working when ICP CLI reports the local environment stopped by targeting the reachable gateway/root key directly and using JSON root create/status receipts.

- `0.35.12` hard-cuts fleet configs to project-root `fleets/`, keeps generated ICP and Canic state at that discovered root from nested commands, changes CLI wasm size columns to report the uncompressed IC install size first, and makes install continue with the created root principal instead of relying on a later name lookup.

- `0.35.11` keeps foreground local replicas usable when ICP CLI reports them stopped by checking the resolved HTTP status endpoint and surfacing that source in text and JSON status output.

- `0.35.10` aligns install readiness plus fleet-aware read/backup commands with the resolved ICP project root and makes live cycle/metadata reads request ICP CLI JSON explicitly.

- `0.35.9` runs replica, status, and install commands from the ICP project root implied by the resolved fleet config, preventing split-repo `backend/fleets/` layouts from creating repo-root `icp.yaml` / `.icp` split-brain state.

- `0.35.8` makes fleet config discovery work from split repos by checking both `fleets/<fleet>/canic.toml` and `backend/fleets/<fleet>/canic.toml`, adds explicit `--fleets-dir <dir>` / `CANIC_FLEETS_ROOT=<dir>` overrides for nonstandard layouts, and keeps that override out of `.canic` and `icp.yaml` to avoid stale hidden state.

- `0.35.7` shortens the root README getting-started flow, puts `canic-cli` installation first, and updates quick examples around `fleets/<fleet>/canic.toml` plus split-repo paths.

- `0.35.6` makes `canic replica start` auto-sync `icp.yaml` from fleet configs with clearer setup errors, supports split-source `fleets/<fleet>/canic.toml` layouts, fixes the generated bootstrap `wasm_store` wrapper so downstreams do not need their own wrapper crate, and creates the local artifact root before first install builds.

- `0.35.5` adds the installed `canic build <role>` artifact-builder command, adds build/install `--profile` selection plus build `--workspace`, `--icp-root`, and `--config` flags for downstream repos, and removes the internal build-session environment bridge from install builds.

- `0.35.4` removes stale and duplicate root wasm-store endpoints, routes publication through `canic_wasm_store_admin` plus `canic_wasm_store_overview`, controller-gates root state/app-registry/log diagnostics, simplifies `canic_canister_status` to controller-only access, updates wasm-store reconcile coverage to the current managed release roles, records the first `0.35` instruction-footprint performance baseline, and reruns the audience-target-binding invariant audit.

- `0.35.3` adds local replica port visibility and `canic replica start --port <port>`, makes local direct replica queries use the configured gateway port, improves project/environment ownership diagnostics for port conflicts, adds `canic fleet sync` plus automatic `icp.yaml` sync after `canic fleet create <name>`, standardizes install timing table formatting, clarifies default top-up opt-ins with explicit `topup = {}` config blocks, and raises the default top-up amount from `4T` to `5T`.

- `0.35.2` retains the installing/upgrading root controller in the runtime controller set used for newly allocated managed children, renames the test scaling worker role to `scale_replica`, shortens role top-up config from `topup_policy` to `topup`, enables default role top-up policies, and removes the old `scripts/app/build.sh` wrapper now that `icp.yaml` calls the host builder directly.

- `0.35.1` hard-cuts managed child controller policy so every newly allocated child canister is controlled by its configured controllers, root, and its direct parent, and tightens install build output with clearer context, per-canister artifact sizes, and explicit root top-up phase/amount messages.

- `0.35.0` is a shiny new 35.0: it adds executable restore stop/start phases, cleans install build-context output, tightens local ICP network commands, and splits the backup domain planner, manifest, runner, restore planning, execution journal, and snapshot capture code into focused modules.

See detailed breakdown:
[docs/changelog/0.35.md](docs/changelog/0.35.md)

---

## [0.34.x] - 2026-05-10 - Backup/restore rework

- `0.34.6` is a CLI boundary cleanup slice that moves shared ICP response parsing, live registry parsing, and installed-fleet resolution into `canic-host`, removes the backup domain crate's dependency on host plumbing, splits endpoints/cycles/metrics/top-level CLI glue into focused modules, replaces the old `canic-cli::args` helper drawer with focused `canic-cli::cli` and `canic-cli::support` module trees, continues shrinking large CLI command modules such as backup, and clarifies local ICP replica state loss in `canic status`.

```bash
canic list test --subtree user_hub
canic endpoints test app --json
canic backup inspect 1
```

- `0.34.5` splits cycle-balance history from live runtime telemetry, adds timestamped cycle top-up history, tightens list/cycles table rendering, removes install-time local replica autostart, separates version bumps from release staging/commit/push targets, and narrows `canic` facade defaults back to metrics-only so plain app canisters do not carry root/control-plane code.

```bash
canic cycles test --since 1h --limit 5
canic metrics test --kind core --nonzero --role app
canic metrics test --kind runtime --nonzero --json
make release-patch
```

- `0.34.4` fixes the new `canic metrics <fleet>` cycle-tracker path so non-root top-up policy checks cannot issue IC calls from init mode, standardizes delayed background workflow startup on 30 seconds, and separates the standard cycle-tracker endpoint wiring from topology views.

- `0.34.3` makes endpoint output structured for automation, exposes IC module hashes in list/backup surfaces, adds fleet cycle-tracker metrics including the canonical `wasm_store`, removes the raw `canic build` wrapper, centralizes wasm/hash helpers, keeps artifact checksums distinct from module hashes, and adds the first executable `canic backup create` flow.

- `0.34.2` aligns the root replay unauthorized-caller test with the current security metrics surface.

- `0.34.1` lets `canic backup inspect`, `canic backup status`, and `canic backup verify` target a backup by `canic backup list` row number or `BACKUP_ID`, while keeping `--dir <dir>` for explicit paths and returning a typed dry-run rejection from `verify`.

- `0.34.0` starts the topology-aware backup/restore rework by adding the typed backup plan, authority preflight, `Proven`/`Declared`/`Unknown` authority evidence, target-scoped authority receipts with preflight ids and validity windows, topology/quiescence preflight receipts, a full execution preflight receipt bundle with journal-side plan binding, execution journal, persisted plan/execution-journal layout with cross-file integrity checks, quiescence policy, operation, and receipt model in `canic-backup`, plus the first `canic backup create <fleet> --dry-run` CLI planner that writes `backup-plan.json` and `backup-execution-journal.json` without live mutation and makes `canic backup list`/`status`/`inspect` understand plan-only directories as dry-run layouts with numbered list rows.

See detailed breakdown:
[docs/changelog/0.34.md](docs/changelog/0.34.md)

---

## [0.33.x] - 2026-05-08 - dfx -> icp-cli

- `0.33.7` combines `canic_canister_version` and `canic_standards` into `canic_metadata` with package metadata, Canic version, and IC canister version, adds a `CANIC` column to `canic list` using parallel metadata endpoint reads, keeps local root installs funded to at least `100.00 TC`, groups backup/restore command families under their own main-help heading, makes local `canic snapshot download <fleet>` resolve registry targets through the decoded local replica query path, uses quiet ICP snapshot creation output so size units cannot be mistaken for snapshot ids, removes fresh-download misuse of `--resume`, splits `canic list` live registry projection, response parsing, and tree traversal out of the command root, centralizes host/operator table rendering for headers and underlines, splits host root readiness diagnostics out of install orchestration, deduplicates live-list threaded query collection and config-loader error mapping, and adds the 0.34 backup/restore redesign plan.

- `0.33.6` adds fleet-scoped `canic endpoints` for Candid method and argument inspection, makes `--icp <path>` and `--network <name>` top-level-only options, removes low-value list/config/install/endpoints selectors by replacing `canic list --from` with `--subtree` and removing `canic list --root`, `canic config --from`, `canic endpoints --did`, `canic endpoints --role`, and every public `canic install` override so install is now just `canic install <fleet>`, rejects duplicate discovered fleet names and install config identity mismatches, removes the `KIND` column from `canic list`, adds a `CYCLES` column with parallel endpoint reads, hard-cuts Candid finalization to required trailing `canic::finish!()`, restricts generated Candid artifacts/metadata to local builds, moves the `minimal` baseline canister under `canisters/audit`, keeps `icp.yaml` aligned with the test fleet after that move, and makes `canic status` judge local deployment against bootstrap-required roles.

```bash
canic endpoints test app
canic endpoints test tl4x7-vh777-77776-aaacq-cai
```

- `0.33.5` refreshes the module-structure audit after the 0.33.4 cleanup, splits core IC management/provisioning, control-plane publication, and backup/restore runner/apply-journal internals into focused Rust directory modules, and reduces the structural risk readout back to `3/10`.

- `0.33.4` keeps default metrics enabled on all canisters, makes the standard pre-1.0 root/auth/sharding runtime capabilities default on the `canic` facade so canister manifests no longer choose Canic features manually, trims the public metrics selector into tiered surfaces, folds redundant low-level management/provisioning/system rows behind higher-level operator metrics, and compiles role-inferred metrics profiles per canister.

- `0.33.3` cleans up `canic-cli` parser and command-family internals by moving routing and validation further onto Clap, sharing host helpers, and splitting restore CLI tests without changing command shapes.

- `0.33.2` is a cleanup/audit slice that hardens delegated auth, subject-caller binding, lifecycle timer symmetry, layer boundaries, capability proof dispatch, root replay expiry/capacity behavior, and audit baselines, including removal of the public partial `AuthApi::verify_token` helper, reducing the complexity audit residual score to `3/10`, replacing stale lint allowances with checked expectations, and starting the config validation ownership refactor.

- `0.33.1` adds native local replica controls, makes project status detect stale local installs, funds local root creation through ICP CLI, and adds ICP CLI debug/status workflows for local development.

```bash
canic replica start --debug
canic replica status
canic replica stop
canic status
canic --network local status
```

- `0.33.0` hard-cuts Canic from DFX project tooling to ICP CLI project tooling, replacing `dfx.json` with `icp.yaml`, moving live local artifacts to `.icp`, and routing install, list, medic, snapshot, restore, CI, and dev setup through ICP CLI.

```bash
canic install demo
canic --network local list demo
canic medic demo
```

See detailed breakdown:
[docs/changelog/0.33.md](docs/changelog/0.33.md)

---

## [0.32.x] - 2026-05-07 - Canic Executable

- `0.32.6` finishes the positional fleet CLI cleanup for install, medic, and snapshot download commands, with docs and local helper output updated to match.

```bash
canic install test
canic --network local medic test
canic snapshot download test --canister <canister-id> --dry-run
```

- `0.32.5` clarifies repo fleet layout by making `fleets/test` the CI-backed reference fleet, keeping `fleets/demo` as a minimal root-plus-app example, moving isolated test fixture canisters under `canisters/test`, changing fleet-scoped commands to use positional fleet arguments, adding compact capability/top-up and verbose config output, and removing the public `canic network` command plus persisted current-network/current-fleet marker state.
- `0.32.4` removes stale reference/release-set CLI surfaces, tightens CLI/host command boundaries, requires explicit fleet selection on fleet-scoped commands, separates non-fleet canisters back under `canisters/`, moves fleet creation to `canic fleet create`, splits config inspection from deployed canister listing, removes saved default-context reads, and makes role-attestation audiences required at the DTO boundary.
- `0.32.3` records a short release-bookkeeping recovery after a git hiccup; the tree was checked for deleted files and the 0.32 changelog is back in order.
- `0.32.2` adds a pre-commit large-file guard so accidentally staged files over 20 MiB are rejected before they reach the repo.
- `0.32.1` focuses the `canic` executable into a clearer operator tool, with confirmation-guarded project scaffolding, fleet-aware install/list/medic flows, simpler snapshot backup commands, backup discovery, and cleaner help output.
- `0.32.0` makes fleet identity explicit in `canic.toml`, removes install-time fleet defaults, and makes `canic list` plus top-level help clearer for multi-fleet operator workflows.

See detailed breakdown:
[docs/changelog/0.32.md](docs/changelog/0.32.md)

---

## [0.31.x] - 2026-05-06 - Snapshot Cleanup

- `0.31.2` finishes CLI parser and host-tool cleanup by moving command parsing onto shared Clap helpers, adding `canic build` and `canic release-set`, and renaming `canic-installer` to `canic-host`.
- `0.31.1` trims the backup/restore v1 surface to the current snapshot workflow, removes retired report/preflight/assertion commands, and makes restore execution rely on ordered journals, stopped-canister checks, and concrete state markers.
- `0.31.0` starts the snapshot cleanup line with safer snapshot restore planning, `canic install` and fleet-aware listing flows, compact install progress, and standalone build config for sandbox/probe canisters.

See detailed breakdown:
[docs/changelog/0.31.md](docs/changelog/0.31.md)

---

## [0.30.x] - 2026-05-03 - Fleet Snapshot Backups

- `0.30.39` trims the `canic` CLI and root README docs into operator-focused guides, removes duplicated installer detail, drops stale canister-layout wording, adds a full 0.30 release audit, and drafts the 0.31 snapshot cleanup plan.
- `0.30.38` adds `canic list`, `canic backup smoke`, easier `canic` binary installs, trimmed CLI help, groups repo-owned canisters by purpose, and removes the old shared reference-support crate.
- `0.30.37` adds manifest design-conformance reporting plus manifest, preflight, and restore-plan `--require-design-v1` gates so smoke checks can fail closed on topology, unit, quiescence, verification, provenance, or restore-order gaps.
- `0.30.36` adds restore runner batch summaries, delta counters, and fail-closed batch gates so automation can see and require how a native runner batch started, changed, and stopped.
- `0.30.35` lets `canic restore run` accept, echo, and require `--updated-at <text>` markers on runner summaries and receipts so native runner transitions can carry operator-supplied comparable state markers instead of always using `unknown`.
- `0.30.34` adds restore pending-work summaries, runner operation receipts/summaries, and fail-closed progress/stale-pending/receipt gates so automation can require claimed-work freshness and execution audit events without recomputing counters.
- `0.30.33` adds restore apply progress summaries to status, report, and runner output so automation can read remaining, transitionable, attention-needed, and integer completion progress without recomputing counters.
- `0.30.32` persists restore apply journal operation-kind counts and validates supplied counts against concrete journal operations, while keeping older journals readable.
- `0.30.31` makes restore planning expand role-level member verification checks into concrete member operations, honors verification role filters before dry-runs or runner previews are generated, and carries operation-kind counts through dry-runs, apply journals, and runner summaries.
- `0.30.30` makes restore apply dry-runs render declared fleet-level verification checks as final `verify-fleet` operations, so restore plans, operation counts, and runner previews agree before execution.

- `0.30.29` centralizes native restore-runner state strings without changing JSON output, adds generated ingress payload limits for `canic_update` endpoints, and adds a local sandbox canister with `start_local!` for quick manual experiments.

```rust
#[canic_update(payload(max_bytes = 32 * 1024))]
fn import(payload: String) -> Result<usize, Error> {
    Ok(payload.len())
}
```

- `0.30.28` starts runner cleanup by moving `canic restore run` summaries onto typed response structs, adds explicit runner-mode/state/action/count gates for automation, and turns the restore apply script into a mode-aware native-runner wrapper.

```bash
canic restore run \
  --journal restore-apply-journal.json \
  --execute \
  --network local \
  --max-steps 1 \
  --out restore-run.json
```

- `0.30.27` moves guarded restore journal execution into `canic restore run --execute`, keeps `--dry-run` previews, adds pending-operation recovery, writes summaries with `stopped_reason` and `next_action`, adds CI gates, and adds a maintained script wrapper for operators who still want the shell flow.

- `0.30.26` adds `canic restore apply-report` and `--require-no-attention` so operators and CI can summarize restore apply journal outcomes, counts, and attention-needed operations without reading the full journal.

```bash
canic restore apply-report \
  --journal restore-apply-journal.json \
  --out restore-apply-report.json \
  --require-no-attention
```

- `0.30.25` adds restore runner guards for `apply-status --require-ready`, `apply-command --require-command`, `apply-claim --sequence`, `apply-unclaim --sequence`, and `apply-mark --require-pending` so external restore scripts can fail closed when work is blocked, no command is available, the journal moved, or a completion was not claimed first.

```bash
canic restore apply-status \
  --journal restore-apply-journal.json \
  --out restore-apply-status.json \
  --require-ready \
  --require-no-pending \
  --require-no-failed
```

- `0.30.24` adds `canic restore apply-claim` and `canic restore apply-unclaim`, keeping pending operations as the next resumable restore step so external runners can claim work before executing `dfx` commands and recover cleanly after interruption.

```bash
canic restore apply-status \
  --journal restore-apply-journal.json \
  --out restore-apply-status.json \
  --require-no-pending \
  --require-no-failed \
  --require-complete
```

- `0.30.23` makes restore apply journal advancement ordered, adds `canic restore apply-command`, and exposes `ManagementCall` metrics so external runners cannot skip ahead and operators can see which management-canister operation is failing.

```bash
canic restore apply-command \
  --journal restore-apply-journal.json \
  --network local \
  --out restore-apply-command.json
```

- `0.30.22` adds restore apply journal state transitions plus `canic restore apply-next` and `canic restore apply-mark` so external restore runners can fetch the next operation and mark individual operations completed or failed while keeping resumable journal counts consistent, and tightens metrics documentation and facade coverage so every metric family stays visible and documented.

```bash
canic restore apply-next \
  --journal restore-apply-journal.json \
  --out restore-apply-next.json
```

- `0.30.21` adds an initial restore apply journal and `canic restore apply-status` so dry-runs can emit and summarize operation states before any mutating restore execution is enabled, and adds first-class `Provisioning` metrics for create, install, propagation, and upgrade workflow visibility.

```bash
canic restore apply \
  --plan restore-plan.json \
  --status restore-status.json \
  --backup-dir backups/<run-id> \
  --dry-run \
  --out restore-apply-dry-run.json \
  --journal-out restore-apply-journal.json
```


- `0.30.20` lets `canic restore apply --dry-run` validate restore artifacts under a backup directory before any future restore execution path can rely on the plan, and adds first-class `Intent` and `PlatformCall` metrics for reservation and platform-call visibility.

```bash
canic restore apply \
  --plan restore-plan.json \
  --status restore-status.json \
  --backup-dir backups/<run-id> \
  --dry-run \
  --out restore-apply-dry-run.json
```

- `0.30.19` adds `canic restore apply --dry-run` so operators can render ordered upload, load, reinstall, and verification operations from a restore plan before real restore execution exists, and adds first-class `Auth` and `Replay` metrics for session, attestation, and replay-safety visibility.

```bash
canic restore apply \
  --plan restore-plan.json \
  --status restore-status.json \
  --dry-run \
  --out restore-apply-dry-run.json
```

- `0.30.18` adds restore-readiness gates and `canic restore status` so automation can write report, plan, and initial status artifacts before restore execution, exposes feature-gated sharding and delegated-auth outcome metrics, and records runtime canister snapshot/restore calls in `CanisterOps`.

```bash
canic restore plan \
  --backup-dir backups/<run-id> \
  --mapping restore-map.json \
  --out restore-plan.json \
  --require-verified \
  --require-restore-ready
```

```bash
canic restore status \
  --plan restore-plan.json \
  --out restore-status.json
```

- `0.30.17` makes restore dry-run, preflight, and snapshot journals expose explicit mapping, journal operation metrics, provenance, readiness, and reason fields for automation, and adds cascade, pool, scaling, and directory metrics for propagation, reusable-canister, worker-placement, and keyed-placement visibility.
- `0.30.16` adds canister operation and wasm-store metrics for fleet lifecycle visibility, including create allocation source, propagation failure, and targeted lifecycle metric coverage.
- `0.30.15` adds restore identity, verification, and topology ordering summaries, typed query perf samples for local-only instruction audit probes, and lifecycle metrics for init/post-upgrade runtime seeding plus async bootstrap progress.

```rust
Ok(MetricsQuery::sample_query(EnvQuery::snapshot()))
```

- `0.30.14` validates backup unit topology and verification role boundaries, rejects ambiguous backup unit and verification filter declarations, and reports backup-unit topology metadata in manifest validation summaries.
- `0.30.13` was accidentally skipped during patch publishing; no release was cut for that patch number.
- `0.30.12` adds `canic backup provenance`, includes provenance and compact audit status output in preflight bundles, and makes backup verification fail closed when manifest and journal topology receipts drift.

```bash
canic backup provenance \
  --dir backups/<run-id> \
  --out backup-provenance.json \
  --require-consistent
```

- `0.30.11` refreshes the release version and installer surfaces after the 0.30.10 topology/journal inspection line so downstream setup paths resolve the live patch.
- `0.30.10` adds scriptable backup inspection, records topology receipts in journals, rejects manifest/journal artifact path drift, fails snapshot capture if topology changes before the first snapshot is created, and updates runtime `ctor` hooks for the explicit unsafe constructor form.
- `0.30.9` refreshes the release version and installer surfaces after the manifest snapshot checksum line so downstream setup paths resolve the live patch.
- `0.30.8` records durable artifact checksums in manifest snapshot provenance and rejects verified backup layouts when manifest and journal checksums disagree.
- `0.30.7` makes snapshot capture write the canonical backup manifest, adds `canic backup preflight` for the standard no-mutation restore-readiness report bundle, and cleans up the 0.30 changelog example placement.

```bash
canic backup preflight \
  --dir backups/<run-id> \
  --out-dir preflight/<run-id> \
  --mapping restore-map.json
```

- `0.30.6` refreshes the release version and installer surfaces after the 0.30.5 operator reporting line so downstream setup paths resolve the live patch.
- `0.30.5` lets manifest validation write report files, backup status fail on incomplete journals, restore dry-run planning require a verified backup layout, and Access/Perf metrics stay covered end to end.

```bash
canic manifest validate \
  --manifest backups/<run-id>/manifest.json \
  --out manifest-validation.json
```

```bash
canic backup status \
  --dir backups/<run-id> \
  --out backup-status.json \
  --require-complete
```

```bash
canic restore plan \
  --backup-dir backups/<run-id> \
  --mapping restore-map.json \
  --out restore-plan.json \
  --require-verified
```

- `0.30.4` refreshes the release version and installer surfaces after the backup integrity line so downstream setup paths resolve the live patch.
- `0.30.3` adds `canic backup status`, `canic backup verify`, and backup layout integrity reporting so operators can inspect resumable journals and validate a manifest, durable artifact set, and SHA-256 checksums before restore planning.

```bash
canic backup verify \
  --dir backups/<run-id> \
  --out backup-integrity.json
```

- `0.30.2` tightens restore preflight by making restore plans include provenance, target parent mapping, identity, snapshot, and verification metadata while rejecting backup-unit and mapping references that do not exist in the manifest.
- `0.30.1` finishes the publish follow-through for the fleet backup line by including the new backup and CLI crates in release order, adding manifest validation and restore planning commands, removing the remaining endpoint metrics macro hooks, documenting metric row shapes, and refreshing installer/version surfaces.

```bash
canic manifest validate \
  --manifest backups/<run-id>/manifest.json
```

- `0.30.0` adds the first fleet backup foundation with manifest validation, topology hashing, resumable artifact journals, restore dry-run planning, and a `canic` CLI command for downloading snapshots for a canister and its registry-discovered children.

```bash
canic snapshot download \
  --canister <canister-id> \
  --root <root-canister-id> \
  --recursive \
  --out backups/<run-id> \
  --stop-before-snapshot \
  --resume-after-snapshot
```

See detailed breakdown:
[docs/changelog/0.30.md](docs/changelog/0.30.md)

---

## [0.29.x] - 2026-04-28 - Delegated Auth Hard Cut

- `0.29.10` removes unused endpoint outcome counters from `canic_metrics` and keeps child-side auto-topup decision metrics visible for no-policy and above-threshold states.
- `0.29.9` removes high idle drain from delegated-auth, log-retention, intent-cleanup, and pool-reset background timers.
- `0.29.8` fixes delegated-token guards so large authenticated upload payloads, such as image chunks, no longer count against the token safety check.
- `0.29.7` fixes `canic_standards` metadata so canisters report their own crate identity instead of always identifying as `canic-core`.
- `0.29.6` removes the remaining delegated-auth shard public-key stable cache, makes signer startup check key material without persisting it, and tightens active AppIndex/SubnetIndex naming so old directory terminology only remains in historical docs and placement-directory code.
- `0.29.5` removes old shim surfaces from the hard-cut line: authenticated guards require `DelegatedToken`, config uses only `app_index` / `subnet_index` plus the neutral per-canister `auth` table, role-attestation refresh startup is separated from delegated-token signing, auth identifiers and crate names are explicit, the installer exposes only `canic-install-root`, and the testkit process lock requires the structured owner format.
- `0.29.4` tightens the hard-cut delegated-auth model, moves delegated root trust material into cascaded `SubnetState`, removes verifier-side root-key fetch-on-verify, aligns the README/design docs with the current signed shard-key binding and thin-root install flow, and rechecks that proof caches, V2 names, and root-key fallback surfaces are gone.
- `0.29.3` removes the temporary version suffix from delegated-auth DTOs, APIs, endpoint names, and internal modules, and makes stable auth key caches identity-bound so key-name changes cannot reuse stale key material.
- `0.29.2` hard-cuts delegated auth to self-validating tokens: verifier proof caches/fanout/admin repair are removed, guards accept only the current delegated-token shape, and old V1 DTO/API/test surfaces are gone.
- `0.29.1` adds the next Delegated Auth implementation slice: policy helpers, root-key trust resolution, pure verifier logic, pure root proof issuance, internal root signing, pure shard token minting, internal shard signing, internal verifier validation, explicit API helpers, the root delegation endpoint, signer-facing mint helpers, root-key pull-on-verify, current-shape guard validation, delegated signer lifecycle prewarm, root-owned TTL policy, topology catch-up proof-sync removal, and focused auth edge-case coverage.
- `0.29.0` starts the hard-cut Delegated Auth line with a design for self-validating delegated tokens plus the first DTO and canonical-encoding implementation slice.

See detailed breakdown:
[docs/changelog/0.29.md](docs/changelog/0.29.md)

---

## [0.28.x] - 2026-04-27 - Delegation Audience Hard Cut

- `0.28.4` pushes still-valid delegated-auth proofs to newly created verifier canisters, so tokens issued before a topology change keep working on the new verifier.
- `0.28.3` removes obsolete delegated-auth signer-proof and admin verifier-prewarm flows now that signer lifecycle prewarm uses canonical root issuance.
- `0.28.2` adds focused lifecycle-gap regression coverage for verifier proof-cache loss, moves the reinstall/upgrade mechanics into the test harness, and fixes the reconcile root harness so staged releases match configured initial shards.
- `0.28.1` forces delegated signer lifecycle prewarm to refresh verifier fanout even when the signer already has a reusable proof, aligns init/post-upgrade readiness on the same auth bootstrap flow, makes root own verifier fanout derivation, success, and root-local proof caching, and adds a signed-off delegated-auth lifecycle design note: [docs/design/archive/0.28-delegated-auth-lifecycle/0.28-design.md](docs/design/archive/0.28-delegated-auth-lifecycle/0.28-design.md).
- `0.28.0` hard-cuts delegated auth onto `DelegationAudience` and required shard public keys, so stale-audience token refresh and verifier proof installation use explicit, non-optional auth material.

```rust
let token = DelegationApi::ensure_token(
    existing_token,
    DelegationAudience::Roles(vec![CanisterRole::new("project_hub")]),
)
.await?;
```

See detailed breakdown:
[docs/changelog/0.28.md](docs/changelog/0.28.md)

---

## [0.27.x] - 2026-04-13 - Topology Taxonomy & Bug Fixing

- `0.27.21` adds idempotent issuer-side token ensure/reissue helpers, so downstream apps can refresh stale audiences without wallet prompts or silently renewing sessions.
- `0.27.20` restores signed delegated-token extension payloads, so downstream apps can keep carrying app-owned identity context such as `user_id` without moving that data into CANIC-owned auth semantics.
- `0.27.19` refreshes the release metadata and installer references for the late `0.27` line while preserving the prior CI-maintenance changelog backfill.
- `0.27.18` fixes the role-attestation PocketIC baseline by starting attestation fixtures with threshold-key support, so delegated signer proof prewarm completes and CI no longer times out waiting for signer readiness.
- `0.27.17` carries a small CI maintenance fix, keeping the release-line checks aligned before the role-attestation fixture fix in `0.27.18`.
- `0.27.16` wires `actionlint` into dev setup and CI, so GitHub Actions workflow syntax and context errors are caught before they block pull request checks or tag checks.
- `0.27.15` adds `initial_workers` to scaling pool policy, so scaling parents can warm workers during bootstrap while keeping startup size separate from steady-state `min_workers` and bounded by `max_workers`.
- `0.27.14` adds `initial_shards` to sharding pool policy and prewarms delegated signer proof during shard bootstrap, so first account placement can reuse a ready, root-authorized shard instead of paying canister creation and delegation setup on the request path.
- `0.27.13` fixes fresh root bootstrap with large static pool imports by waiting only for the configured initial pool slice and queueing the remaining `pool.import.ic` canisters, so downstream reinstalls no longer sit in `root:init:import_pool` while resetting the entire spare pool.
- `0.27.12` fixes the remaining GitHub Actions toolchain drift by exporting `RUSTUP_TOOLCHAIN` per CI job and installing `wasm32-unknown-unknown` for the matching internal toolchain, so nested bootstrap and test-canister wasm builds stop falling back to the wrong compiler during CI.
- `0.27.11` fixes the nested Cargo build paths used by bootstrap/test canister builds so they reuse the parent CI toolchain selection, which stops the MSRV lane from failing when those nested wasm builds would otherwise miss the installed `wasm32-unknown-unknown` target.
- `0.27.10` fixes the GitHub Actions `dfx` bootstrap lane by replacing the shell-installed `dfxvm` path with the official `dfinity/setup-dfx` action, so CI no longer fails on non-interactive runner shells while installing `dfx`.
- `0.27.9` separates Canic’s published MSRV from its repo-local toolchain pin by declaring Rust `1.91.0` across the workspace crates while keeping internal CI and bootstrap builds on Rust `1.95.0`, so downstream source consumers are not forced onto the newer compiler just because Canic uses it internally.
- `0.27.8` bumps the pinned workspace Rust toolchain to `1.95.0`, aligns CI and the shared developer bootstrap with that compiler, and folds the required new Clippy cleanup into the tree so the standard warning-as-error checks stay green on the newer toolchain.
- `0.27.7` switches `canic-cdk` over to the canonical upstream `icrc-ledger-types` `Account` and `Subaccount` definitions, so downstream code can stay on Canic’s `cdk::types` facade while aligning with the standard ICRC ledger wire types instead of Canic’s local copy.
- `0.27.6` rolls the shared `ctor` dependency back to the earlier `0.8` line after the brief `0.10` upgrade in `0.27.5`, keeping Canic's constructor-macro path on the previously working version while retagging the shared installer/docs to point at the new patch.
- `0.27.5` teaches the shared `install-dev` / `update-dev` bootstrap path to provision Python 3, so local developer setup covers the Python-based helper lane without asking contributors to install it separately first.
- `0.27.4` removes the remaining `derive_more` dependency from the published crate set by replacing a few simple wrapper derives with explicit trait impls, which keeps the public workspace dependency surface smaller and more predictable without changing behavior.
- `0.27.3` hardens `directory` placement under failure by making async create finalization claim-owned, treating missing provisional children as already cleaned during stale recovery, and routing resolve/recover through one shared pending-state classifier so key liveness and repair behavior stop drifting.
- `0.27.2` adds the first full `directory` placement cut: singleton parents can now declare keyed `directory` pools, `instance` children are restricted to those parents, the runtime stores `Pending | Bound` directory entries, and `resolve_or_create` now claims before async create, repairs valid stale provisional children, and never lets stale `Pending` claims block progress forever.
- `0.27.1` carries the full first topology implementation cut: it replaces `tenant` with `instance`, renames the old lookup/export surface from `directory` to `index` across config and runtime APIs, updates the checked-in configs and `.did` surface to the new terms, and leaves only `app_directory` / `subnet_directory` as temporary config parse aliases during migration.
- `0.27.0` starts the topology-taxonomy line by separating structural canister kind from placement family, reserving `directory` for keyed instance placement while renaming the older lookup concept toward `app_index` / `subnet_index`, and making `tenant -> instance` an immediate migration decision instead of a tolerated long-term ambiguity.

See detailed breakdown:
[docs/changelog/0.27.md](docs/changelog/0.27.md)

---

## [0.26.x] - 2026-04-06 - Metrics Baseline

- `0.26.12` finishes another late-line cleanup pass by splitting more oversized installer/test/runtime support seams, isolating the audit target from the full cached-root helper tree so dead-code warnings stop spilling across test binaries, and keeping the focused root/audit verification green without reopening the runtime surface.
- `0.26.11` keeps the late `0.26` line on maintenance-only follow-through, with small cleanup around the installer/test-harness seams, README alignment around the public install-target and PocketIC test surfaces, and another full root-suite verification pass.
- `0.26.10` keeps the late `0.26` line on maintenance follow-through only, with small installer/test-harness cleanup, README alignment around the public install-target and PocketIC test surfaces, and another full root-suite verification pass.
- `0.26.9` hardens the late `0.26` maintenance line by tightening the public PocketIC test wrapper boundary, narrowing cached root-baseline retries to real startup failures, reducing repeated local artifact freshness scans, and splitting installer workspace discovery into a smaller shared seam.
- `0.26.8` corrects the new installer CLI surface by renaming it to `canic-list-install-targets` and making it print the full local install target set, including `root`, so downstream scripts can use the same target list Canic’s own local install path uses.
- `0.26.7` adds a public `canic-list-install-targets` CLI to `canic-installer`, so downstream workspaces can list the local install target set from `canic.toml` without re-owning that parser logic.
- `0.26.6` cleans up the local tooling surface by moving the shared setup script into `scripts/dev/install_dev.sh`, removing stale `Makefile` convenience aliases and old install targets, and keeping the release-facing install URL/tests aligned with that slimmer setup path.
- `0.26.5` fixes a delegated-token timing race during fresh proof provisioning: when a signer has to ask root for a new delegation first, Canic now rebases the token timestamps onto that new proof window so downstream verifiers stop seeing `token issued before delegation` on otherwise valid login flows.
- `0.26.4` keeps the late `0.26` follow-through on the clean side by splitting more `canic-testkit` and runtime ownership seams, making `wasm_store.did` refresh explicit instead of incidental during normal bootstrap builds, fixing the workspace test runner so the PocketIC suites follow their moved `canic-tests` package targets, and finishing the delegated-auth verifier bootstrap fix so root now pushes the delegation public key with the proof and verifier-only canisters do not need their own threshold-ECDSA support for delegation provisioning.
- `0.26.3` makes delegated-auth config fail fast when the build is under-provisioned: root now traps immediately if delegated auth is configured without `auth-crypto`, signer canisters trap if they are built without threshold-ECDSA support, and verifier-only canisters still stay verifier-only.
- `0.26.2` keeps the first `0.26` runtime follow-through on the clean side by simplifying root replay/cycles routing, tightening delegation and verifier-cache paths, and lowering the retained instruction hotspots to `root::canic_response_capability_v1 = 489511` and `root::canic_request_delegation = 1682331` in the latest same-day rerun.
- `0.26.1` restores the supported public `ICRC-21` dispatcher facade at `canic::api::protocol::icrc21::Icrc21Dispatcher`, so downstream canisters no longer need hidden `canic-core` paths after the earlier facade narrowing.
- `0.26.0` establishes the first `0.26` metrics and performance baseline, refreshing the retained wasm and instruction audit reports so the next runtime work can measure drift against a clear starting point instead of the late `0.25` cleanup line.

See detailed breakdown:
[docs/changelog/0.26.md](docs/changelog/0.26.md)

---

## [0.25.x] - 2026-04-05 - Recurring Audit Refresh

- `0.25.11` moves `canic_metrics` off the internal-test build gate and onto a real `canic` `metrics` feature that is enabled by default, so ordinary facade users keep the metrics endpoint by default while still being able to opt out explicitly with Cargo features.
- `0.25.10` cleans up the public `canic-memory` facade by renaming the stable-memory bootstrap and lookup methods toward intent and by hiding the runtime summary type from the public return values, so downstreams use a smaller `MemoryApi` surface instead of substrate-shaped names.
- `0.25.9` extends `canic-memory` with small read-only registration queries, so downstreams can inspect registered memory ids by owner or label through the supported `MemoryApi` facade instead of reading registry/runtime snapshots directly.
- `0.25.8` adds a small read-only `canic-memory` inspection helper so downstreams can ask who owns one memory id, what reserved range it belongs to, and whether that slot already has a registered label, without reaching into registry/runtime internals.
- `0.25.7` adds a supported dynamic-memory API to `canic-memory`, so downstream crates can reserve ranges, register runtime-selected memory IDs, and open `VirtualMemory` handles without importing the hidden `MEMORY_MANAGER` internals directly, while also hardening shared `canic-testkit` PocketIC baseline recovery and continuing the `canic-testkit::pic` cleanup without changing downstream call sites.
- `0.25.6` adds the new recurring `module-structure` audit and uses its first retained pass to tighten structural visibility: `canic-core` now hides more support-only root modules, `canic-memory` no longer root-re-exports backend bootstrap state, and `canic-testkit::pic` is split by ownership so the public PocketIC seam is cleaner without changing downstream call sites.
- `0.25.5` keeps the `0.25` follow-through on the clean side by trimming more shared runtime weight from the default demo surface, removing leftover `wasm_store` carryover endpoints, centralizing the internal test/audit wasm-build path, and landing two small measured runtime cuts that lower sampled `root::canic_request_delegation` from `1768507` to `1726014` local instructions across the retained reruns.
- `0.25.4` finishes the internal canister-boundary cleanup by splitting correctness fixtures from audit probes, moving the `audit_*_probe` crates into a dedicated `audit-canisters` lane, and tightening the default instruction audit so it measures shared runtime and audit-only probe paths instead of demo `create_*` provisioning flows.
- `0.25.3` continues the post-audit runtime trim by cutting more avoidable work out of the delegated-auth and replay paths, including replay payload compaction, cheaper delegation cert hashing, a thinner root signing/cache path for `canic_request_delegation`, and compact cached cycles responses that cut sampled `canic_response_capability_v1` `cycles-request` from `1481137` to `601860` local instructions in the next retained audit rerun.
- `0.25.2` starts the runtime follow-through from the `0.25.0` audit sweep by tightening delegated-auth proof provisioning, threading shard key material through the root install path so verifier setup stops repeating avoidable key lookup work, and trimming repeated proof-install payload encoding in the `canic_request_delegation` hot path while keeping the auth/runtime checks green.
- `0.25.1` follows the audit sweep by splitting the auth/runtime complexity hotspots into smaller modules, moving the `test` role out of the default demo topology into internal test-only canisters, removing root debug helpers so the demo/reference canisters stay closer to real user-facing flows, and making public `canic-testkit` PocketIC setup more ergonomic with fallible startup/install helpers plus temp-root lock-parent creation for repo-local `TMPDIR` paths.
- `0.25.0` refreshes the recurring audit line with retained summary reruns across layering, capability surface, wasm footprint, instruction footprint, lifecycle/change-friction checks, and the auth invariants; the current result is that the invariants still hold while the main remaining pressure is complexity concentrated in the auth/runtime seams.

See detailed breakdown:
[docs/changelog/0.25.md](docs/changelog/0.25.md)

---

## [0.24.x] - 2026-04-04 - Shared Runtime Reduction and Test Boundary Cleanup

- `0.24.8` extends public `canic-testkit` with a generic prebuilt-wasm install path, so downstream PocketIC suites that do not use Canic canisters can still stay fully `canic-testkit`-backed instead of hand-rolling `create_canister` / `add_cycles` / `install_canister` adapters.
- `0.24.7` hardens the `pic_role_attestation` PocketIC suite by rebuilding dead cached baselines automatically after failed restore attempts and by aligning the role-attestation capability tests with the real `signer -> root` cycles caller path instead of the old `root -> root` shortcut.
- `0.24.6` makes `canic-testkit` more useful for downstreams by promoting the generic standalone non-root PocketIC fixture and PocketIC `install_code` retry helpers into the public crate, while keeping Canic-specific root, attestation, and delegation fixtures internal.
- `0.24.5` finishes another test-boundary cleanup pass by moving the local bogus-token auth guard onto the standalone PocketIC lane, sharing the internal `user_hub -> user_shard -> root delegation` fixture plumbing across auth-focused suites, and giving the reconcile tests their own named cached root profile so the remaining root hierarchy entrypoints are explicit instead of generic.
- `0.24.4` keeps hierarchy-heavy testing focused on the cases that really need `root` by moving standalone `app`, `test`, and `scale_hub` checks onto a shared internal PocketIC fixture, keeps heavy internal env/directory queries out of ordinary canister builds behind a test-only flag, and hardens the local tooling path by auto-recovering local `dfx` once and letting the wasm audit build artifacts without depending on a healthy replica first.
- `0.24.3` folds sharding back into `canic-core`, removes the standalone `canic-sharding-runtime` crate and the extra `xxhash-rust` dependency, keeps the `canic` `sharding` feature stable for facade users, switches HRW scoring to `sha2`, and narrows the internal root harness around explicit topology, scaling, and sharding profiles so hierarchy-heavy suites only pay for the roles they actually exercise.
- `0.24.2` follows the first `0.24` auth reductions by reusing cached root response attestations, carrying cycles authorization through replay/capability execution, trimming replay and registry work, and clarifying that query lanes are measured through same-call probe endpoints because query-side perf rows do not persist, while the next dated rerun cuts sampled `root::canic_request_delegation` from `3205866` in `instruction-footprint-20` to `2274445` in the `2026-04-05` instruction audit.
- `0.24.1` follows up the first `0.24` perf pass by warming root auth key material during setup, removing the redundant root-to-signer delegation proof push, and collapsing the root verifier cache path into one auth-state write, which cuts sampled `root::canic_request_delegation` from `4356980` in `instruction-footprint-17` to `3205866` in `instruction-footprint-20`.
- `0.24.0` continues the shared-runtime reduction line by trimming shipped `CandidType` doc bloat, separating the public `canic-testkit` surface from unpublished self-test support, cutting sampled root chunk publication from about `9.7M` to `390k` local instructions, cutting sampled `root::canic_request_delegation` from `5516827` in `instruction-footprint-15` to `4356980` in `instruction-footprint-17`, and hardening the audit and release surfaces around those reductions.

See detailed breakdown:
[docs/changelog/0.24.md](docs/changelog/0.24.md)

---

## [0.23.x] - 2026-04-03 - Deferred Follow-Through

- `0.23.2` removes the checked-in wasm budget layer from the recurring footprint audit, so follow-through work is driven by dated size deltas and hotspot evidence instead of static thresholds.
- `0.23.1` follows up the new parent-to-child cycles test helper with a small `scale` canister cleanup so the `request_cycles_from_parent` endpoint stays warning-free under `make clippy`.
- `0.23.0` starts the follow-through line with checked-in wasm budgets, a dated wasm-footprint rerun, a clearer split between the public `canic-testkit` PocketIC wrapper and the new unpublished `canic-testing-internal` self-test crate, a removal of the unused `*cycles_accept` compatibility endpoint so management-canister cycle deposit stays the only Canic-managed funding path, and a fix for the curlable setup script so its default `canic-installer` version stays aligned with the current Canic release.

See detailed breakdown:
[docs/changelog/0.23.md](docs/changelog/0.23.md)

---

## [0.22.x] - 2026-04-02 - Audits, Wasm Size, and Perf

- `0.22.10` fixes the narrowed local root-install build path so it issues one quiet `dfx build <canister>` call per selected target, matches the real DFX CLI contract, keeps the one-time Canic build context stable across the whole install, restores downstream `make test-canisters` flows after the `0.22.9` targeted-build change, adds a curlable `scripts/install.sh` setup path that bootstraps Rust when needed and installs the pinned Rust/Cargo/Canic toolchain plus `dfx` in one step, and removes the stale duplicate environment-update path so setup docs point at one shared flow.
- `0.22.9` tightens the local thin-root install path by fabricating cycles only when local root is actually short, building only `root` plus the configured release roles from the root-owning subnet, keeping the normal wait loop quieter, and removing the now-redundant DFX dependency edges from the reference `dfx.json`.
- `0.22.8` cleans up the repo-local/downstream output so both the shell wrapper and direct `canic-build-canister-artifact` calls print the workspace/DFX roots once per run, show the selected `debug|fast|release` build profile, add visible spacing between canister builds, log per-canister elapsed time with `0.01s` precision, and render the installer’s end-of-run timing summary as a readable table.
- `0.22.7` lets the installer auto-discover nested canister manifests from Cargo workspace metadata so downstreams no longer need flat alias directories just to match Canic role names.
- `0.22.6` improves local install diagnostics by exposing a typed `canic_bootstrap_status` query, lets the installer fail immediately on root bootstrap errors with phase-aware output and an end-of-install timing summary instead of waiting only on `canic_ready`, fixes the public visible-canister build path so it applies the same `ic-wasm shrink` pass as the hidden bootstrap `wasm_store` builder, and removes committed visible canister `.did` files so generated `.dfx/local/canisters/*/*.did` outputs are the only live source of truth apart from the canonical checked-in `crates/canic-wasm-store/wasm_store.did`.
- `0.22.5` continues the downstream `wasm_store` instruction-limit follow-through by removing a redundant init-time managed-store catalog import after publication, so root no longer snapshots the just-retired rollover store again before bootstrap can finish.
- `0.22.4` continues the downstream `wasm_store` instruction-limit follow-through by removing the managed-store chunk-store preflight during install-source resolution, so root no longer asks a freshly published store to enumerate its whole chunk-hash set again before `install_chunked_code`.
- `0.22.3` finishes the downstream `wasm_store` instruction-limit follow-through by replacing repeated full-store occupied-byte rescans with incremental counters, so each new chunk upload no longer re-serializes every already-stored chunk just to enforce capacity.
- `0.22.2` continues the `wasm_store` publication follow-through by streaming release chunks through the live root/store publication path instead of buffering full releases in memory and switching staged-release payload verification to incremental hashing, further reducing the cost of large downstream bootstrap publication.
- `0.22.1` follows up the audit/perf line by caching the expensive debug small-store reconcile baseline, adding a compact workspace timing summary table, recording the first dated `0.22` instruction-footprint report, hardening the wasm audit runner so missing local `dfx` fails fast, keeping `make publish` viable with the one intentional local `canic-core -> canic-testkit` test-only edge, and trimming managed `wasm_store` publication hot paths so large downstream release sets stop hitting instruction limits during bootstrap.
- `0.22.0` opens the audit/perf line by making `.dfx` artifact reuse aware of build env and profile, moving more reusable PocketIC root-baseline setup into `canic-testkit`, standardizing three wasm build lanes (`debug`, `fast`, `release`) across repo-local and downstream builders, and routing the special small-store reconcile build through the shared root harness so future audit work starts from reproducible inputs instead of stale artifact reuse.

See detailed breakdown:
[docs/changelog/0.22.md](docs/changelog/0.22.md)

---

## [0.21.x] - 2026-04-01 - Implicit Wasm Store and Managed Release Fleet

- `0.21.12` fixes the release lane so `make publish` can resume after partial crates.io uploads, skips already-published workspace crates instead of aborting at the first duplicate, keeps workspace manifest inheritance intact, and unblocks `canic-core` publish preparation by using a targeted `--no-verify` publish exception for its test-only `canic-testkit` edge.
- `0.21.11` stops the local installer from overriding caller-selected build profiles, keeps repo-local smoke installs on the optimized dev wasm path by default, hardcodes Canic wasm staging/install chunks to the IC-safe `1_048_576` bytes with no env or config override surface, adds visible installer plus root-side staging progress, moves reusable root PocketIC baseline setup into `canic-testkit`, front-loads root artifact builds once per workspace test run, and makes the normal `make test` path run with `--nocapture` plus explicit per-suite timings so long PocketIC phases stay visible live.
- `0.21.10` teaches the public `canic-installer` tools to separate Cargo/config discovery from DFX artifact output, so split repos like `backend/` + `frontend/` can keep one real repo-root `.dfx` while pointing Canic at a nested Rust workspace through `CANIC_WORKSPACE_ROOT` and `CANIC_DFX_ROOT`, and the repo-local `make demo-install` / `make test-canisters` smoke path now defaults to optimized dev wasm instead of slower release canister builds.
- `0.21.9` finishes productizing the downstream build/install boundary by publishing `canic-build-canister-artifact` and `canic-install-root`, shrinking the repo-local build/install scripts into thin wrappers, and adding an installed-binary `canic-installer` probe so downstream projects can rely on public Canic tools instead of copying more shell logic.
- `0.21.8` finishes the thin-root cleanup by moving GitHub Actions onto the shared Canic wasm build helper, preferring the public installer binaries in the repo-local wrappers, and publishing the hidden bootstrap `wasm_store` build behind `canic-build-wasm-store-artifact` so downstreams no longer need to re-own that shell logic.
- `0.21.7` hardens the new `canic-installer` path by fixing its false ready-timeout on successful thin-root installs, adding direct coverage for the accepted `canic_ready` JSON shapes, rejecting bad `.wasm.gz` release artifacts before any root staging work begins, opportunistically emitting `root.release-set.json` from the public installer path during normal custom builds, and proving the packaged installer can emit a downstream manifest from normalized package contents.
- `0.21.6` publishes `canic-installer` as the downstream thin-root installer surface, moves the manifest/staging binaries off workspace-private `canic-internal`, and hardens `root.release-set.json` so it only stages roles from the single subnet that actually owns `root`.
- `0.21.4` keeps `root.wasm` thin again by embedding only the bootstrap `wasm_store`, moving ordinary release staging back out to a manifest-driven Rust installer flow in `canic-internal`, removing the hidden `wasm_store` leak from downstream `dfx.json`, and restoring a manual `scripts/app/dfx_start.sh` convenience script without reintroducing auto-started `dfx` into the normal test or install gates.
- `0.21.3` hardens the managed `wasm_store` fleet again by adding root-facing live publication and retired-store status reads, proving the fixed-target and retire/finalize/delete flows under PocketIC, and making lifecycle-boundary tests resilient to PocketIC install throttling instead of failing on transient rate limits.
- `0.21.2` hardens the managed `wasm_store` fleet follow-through by clarifying the root-owned approved-state overview surface and adding PocketIC runtime proofs that exact releases are reused while conflicting duplicate `template_id@version` publications fail closed without mutating fleet state.
- `0.21.1` hardens the first managed-fleet release by scoping and pruning stale approved roles to the current config-driven release set, keeping the implicit `wasm_store` preset downstream-safe without const-only assumptions, tightening the root-owned overview semantics so its headroom flag is clearly approved-state-only, and removing the local `dfx` smoke path from `make test` / `make test-bump` so the normal test gate stays PocketIC/Cargo-driven while manual `dfx` installs still fail fast if the replica is not already running.
- `0.21.0` starts the new managed release-fleet line: `root` now owns the implicit `wasm_store` bootstrap, embeds the build-produced `.wasm.gz` bootstrap and ordinary release artifacts, manages a tracked multi-store fleet with exact-release reuse and post-upgrade reconcile, and lets downstreams build through `canic` without carrying a local `wasm_store` crate or a manual bootstrap script.

```bash
cargo install --locked canic-installer --version <same-version-as-canic>
dfx build --all
canic-install-root root
```

See detailed breakdown:
[docs/changelog/0.21.md](docs/changelog/0.21.md)

---

## [0.20.x] - 2026-03-31 - Cleanup and Optimization

- `0.20.10` turns root publication into a real `wasm_store` fleet manager: it now places releases from the full approved manifest set across the tracked store inventory, reuses exact existing releases instead of duplicating them, creates fresh stores proactively when no current store can accept a release, and stops assuming the current release set lives in one default store.
- `0.20.10` also hardens the fleet follow-through: root post-upgrade now reconciles approved manifests against the exact current release bytes instead of conflicting on older copies in older stores, the root store overview now clearly reports approved-release projections instead of pretending to know live occupancy, ordinary embedded release bundles are gzip-only, and the hidden `wasm_store` build path can synthesize its own wrapper so downstreams do not need to carry extra `wasm_store` config or source.
- `0.20.9` makes root publication multi-store aware by retrying individual releases on a newly promoted `wasm_store` when the current one runs out of capacity, and keeps later installs aligned by importing the catalog from the active publication store instead of assuming the configured default binding always won.
- `0.20.8` publishes the canonical `canic-wasm-store` crate so downstreams can stop carrying a local `wasm_store` canister crate, switches the embedded ordinary root release bundle to `.wasm.gz` payloads, and lets root roll publication across additional `wasm_store` canisters when one store cannot fit the whole bootstrap release set.
- `0.20.6` hardens the embedded `wasm_store` bootstrap contract by rejecting empty or non-wasm `.wasm.gz` artifacts during the root build itself, and expands the bootstrap provenance log to include both the original DFX source path and the copied embedded path so downstream artifact bugs fail early and read clearly.
- `0.20.5` fixes the embedded `wasm_store` bootstrap source so `root` now installs the current DFX-built `.wasm.gz` artifact instead of drifting back to a stale checked-in payload, and logs the exact embedded bootstrap provenance during root init so bootstrap mismatches are visible immediately.
- `0.20.4` makes ordinary child-role publication an internal root bootstrap detail by embedding the release bundle into `root` during the normal `dfx build --all` flow, so reinstalling `root` is sufficient again in local deployments and the old external release-staging scripts are gone.
- `0.20.3` stabilizes the `0.20` perf tooling by turning the instruction audit into a real repeated baseline instead of a one-off harness, adding production `perf!` checkpoints across the critical root/auth/replay/scaling/sharding flows, measuring root template-staging admin updates directly, and hardening the audit/build path so unrelated local `dfx` and Cargo state no longer invalidate the report runner.
- `0.20.2` makes `wasm_store` an internal root bootstrap detail instead of a user-managed reference canister, removes the old `shard` / `shard_hub` reference roles, consolidates the sharding demo and test lane on `user_hub` / `user_shard`, hardens root release staging so stale local `.dfx` artifacts cannot silently republish deleted roles, adds a generic host-side root bootstrap helper that downstream Canic projects can point at their own `canic.toml` and `.dfx` artifacts, and surfaces the staged `template_id@version` through staging, publication, and install logs so operators can see exactly which release root selected.
- `0.20.0` opens the cleanup and optimization line, using recurring wasm-footprint and instruction-footprint audits to drive shared wasm reduction, lower `perf!` and endpoint instruction counts, catch regressions before they spread across the runtime floor, keep publishable crates free of workspace-only integration-test baggage, and round out the `canic` control-plane facade so downstreams can keep dropping direct `canic-control-plane` imports.

See detailed breakdown:
[docs/changelog/0.20.md](docs/changelog/0.20.md)

---

## [0.19.x] - 2026-03-30 - Library Lane Cleanup and Crate Graph Simplification

- `0.19.6` cleans up stale automation by removing the unused `make release` / `check-versioning` paths and obsolete bootstrap helper scripts, fixes CI’s old `template_store` canister list to the current `wasm_store` topology, and adds a recurring instruction-footprint audit definition for `perf!` and endpoint instruction regression tracking.
- `0.19.5` rounds out the downstream facade story by adding a feature-gated `sharding` lane on `canic`, so sharding coordinator canisters can keep using `canic::api::canister::placement::ShardingApi` and `start!()` without depending on `canic-sharding-runtime` directly, while `root` and `wasm_store` continue to use the existing `control-plane` feature.
- `0.19.3` restores a feature-gated `canic` control-plane lane so downstream `root` and `wasm_store` crates can keep using the facade-owned root lifecycle and template/store API paths without making ordinary leaf canisters pull control-plane code by default.
- `0.19.2` simplifies the workspace crate graph by merging the temporary template helper crates into `canic-control-plane`, deleting the dead `canic-dsl` and `canic-utils` crates, and restoring an empty shared `SubnetState` so the generic state cascade shape is `[as ss ad sd]` again without reintroducing root-owned publication inventory into non-root sync.
- `0.19.1` finishes the library/reference split by moving template/store and sharding implementation lanes out of the default `canic` path, compiling `canic.toml` into the canister instead of parsing TOML at runtime, collapsing the temporary template helper crates back into `canic-control-plane`, removing the dead `canic::dsl` / `canic-utils` crates, standardizing debug-only Candid export on `canic::cdk::export_candid_debug!()`, and hardening the staged `wasm_store`/`root` reference install flow behind `make demo-install` once `dfx` is already running.
- `0.19.0` starts the `0.19` line with a clean post-`0.18` audit baseline, recording the release wasm footprint (`minimal`/`app`/`scale`/`shard` at `2489858` bytes, `root` at `3730865`, `wasm_store` at `2823075`) and the refreshed capability-surface baseline before the next reduction pass begins.

```toml
canic = { version = "0.19.5", features = ["control-plane", "sharding"] }
```

See detailed breakdown:
[docs/changelog/0.19.md](docs/changelog/0.19.md)

---

## [0.18.x] - 2026-03-27 - Template Store and Chunked Install Cutover

- `0.18.7` stops stale non-root canisters from spamming root with failed attestation-key refreshes after they fall out of the subnet registry, fixes cached `.did` invalidation so per-canister release builds stop retriggering whole-workspace rebuilds during `dfx build --all`, and compacts shared capability-proof wire payloads behind `CapabilityProofBlob` so non-root interfaces carry less proof-shape fan-out.
- `0.18.6` removes the remaining env-driven eager-init build split, keeps release builds single-pass while caching `.did` files independently of release wasm, stages the full config-defined release set into `root` before local smoke/bootstrap flows continue, adds root-owned bootstrap debug visibility with human-readable wasm sizes, and fixes the local smoke path so it calls the `test` canister that `root` actually created and registered.
- `0.18.5` keeps `ICRC-21` behind role-scoped compile-time gating, trims the shared generated surface by making `canic_app_state` and `canic_subnet_state` root-only, removes embedded release payloads from both `root` and `wasm_store`, and hardens bundle builds so profile-mismatched `.dfx/local` artifacts are no longer silently reused when the AA pipeline stages releases through `root`.
- `0.18.4` gives `root` a single controller-facing `canic_wasm_store_overview` read endpoint built entirely from root-owned state so operators can inspect all tracked wasm stores without direct store queries, consolidates the older split wasm-store status queries into that overview surface, and tightens the local release flow so `make patch` / `make minor` skip PocketIC-heavy tests, rely on an already-running `dfx`, and stop failing plain Cargo/clippy builds when `.dfx` release artifacts have not been generated yet.
- `0.18.3` makes `root` bootstrap its first `wasm_store` automatically again, updates the `canic-memory` eager-init contract so `canic::start!` consumes it seamlessly without extra user wiring, and hardens local `dfx` test flows by starting clean replicas and removing the now-stale manual bootstrap staging step from `make test` and `make patch`.
- `0.18.2` makes the `root` and `wasm_store` release flow fully config-driven from `canic.toml`, moves live wasm-store inventory into runtime subnet state so `root` can create and promote stores dynamically instead of relying on static bindings, and standardizes debug-only Candid export behind `canic::cdk::export_candid!()`.
- `0.18.1` completes the staged `wasm_store` bootstrap follow-up by fixing local `dfx` installs to stage the bootstrap payload before root becomes ready, restoring local compact-config compatibility, and trimming release-only exports so the raw `root` artifact drops further to `3554964` bytes.
- `0.18.0` starts the wasm-store cutover by moving ordinary child payload ownership out of `root`, requiring store-backed chunked install for every role except bootstrap `wasm_store`, reducing the raw release `root` artifact to `4151294` bytes (`delta -10366542` vs `0.17.3`), simplifying setup with one implicit per-subnet `wasm_store` on a fixed 40 MB / 4 MB IC preset, and refreshing the workspace baseline to Rust `1.94.1` with `ctor 0.8` and `sha2 0.11`.

```toml
[subnets.prime]
auto_create = ["app", "user_hub", "scale_hub", "shard_hub"]

[subnets.prime.canisters.app]
kind = "singleton"
```

See detailed breakdown:
[docs/changelog/0.18.md](docs/changelog/0.18.md)

---

## [0.17.x] - 2026-03-25 - Wasm Audit and Endpoint Surface Reduction

- `0.17.3` continues the wasm audit line by tightening `canic_metrics` and `canic_log`, completing the `0.17` root decomposition handoff to `0.18`, and reducing the `minimal` raw release artifact to `2433930` bytes (`delta -26446` vs `0.17.2`).
- `0.17.2` continues the wasm audit line by slimming shared runtime, metrics, and observability paths, bringing the `minimal` raw release artifact down to `2460376` bytes (`delta -100624` vs `0.17.1`) while keeping the intended operator-facing feature set intact.
- `0.17.1` cuts the shared wasm floor again by separating root-only capability verification from the non-root cycles path and by removing the old Canic standards canister-status endpoint, bringing the `minimal` raw release artifact down to `2561000` bytes while keeping the intended runtime feature set intact.
- `0.17.0` starts the wasm audit line with a measured per-canister footprint baseline, renames the canonical baseline canister from `blank` to `minimal`, and trims optional scaling, sharding, delegated-auth, and `ICRC-21` endpoint exports behind compile-time config so disabled features stop inflating every build.

See detailed breakdown:
[docs/changelog/0.17.md](docs/changelog/0.17.md)

---

## [0.16.x] - 2026-03-16 - Delegation Proof Evolution

- `0.16.2` hardens delegated-auth token handling by rejecting malformed or unusable lifetimes at both issuance and verification, making the zero-skew policy explicit, restoring ops-owned proof boundaries, and closing the `0.16` auth/proof line with remaining root/template architecture work handed off to `0.17` and `0.18`.
- `0.16.1` hardens delegated-auth audience binding so verifier proof installs and delegated-session bootstrap reject out-of-scope audiences, while typed auth rollout metrics make prewarm/repair failures easier to track during the `0.16` auth refactor.
- `0.16.0` is reserved as a placeholder minor-line entry for delegation proof evolution follow-up work (deferred from `0.15` Phase 3), with implementation details tracked in the `0.16` design docs.

See detailed breakdown:
[docs/changelog/0.16.md](docs/changelog/0.16.md)

---

## [0.15.x] - 2026-03-12 - Unified Auth Identity Foundation

- `0.15.6` bumps `pocket-ic` to `13.0`, refreshes supporting IC/Rust dependencies, and advances the workspace to `0.15.6` so local and integration tooling stay aligned with the current dependency baseline.
- `0.15.5` fixes CI flakiness in delegation/role-attestation integration builds by making cfg-gated test-material compilation reliably rebuild when `CANIC_TEST_DELEGATION_MATERIAL` changes between runs.
- `0.15.4` completes Tier 1 delegation provisioning guarantees by requiring required verifier fanout success at issuance, adding root-side verifier-target validation and role-labeled provisioning metrics, and validating issuance -> verifier verify -> bootstrap -> authenticated guard success end to end; Phase 3 follow-ups are explicitly deferred to the `0.16` design track.
- `0.15.3` removes unused legacy compatibility shims/fallbacks and records a follow-up `layer-violations` rerun (`3/10`, no hard layer violations).
- `0.15.2` fixes shard token issuance regression by routing non-root delegation requests to root over RPC, so shard-initiated proof refresh works again while root-only authorization stays enforced.
- `0.15.1` finalizes 0.15 release governance docs by recording explicit security sign-off scope/residual risks, freezing the auth-semantic boundary for 0.15, and clarifying canonical release-boundary tracking.
- `0.15.0` hardens delegated-caller behavior into token-gated delegated-session semantics with strict subject binding, TTL clamp, replay/session-binding controls, and auth observability, while keeping raw-caller infrastructure predicates unchanged.

```rust
DelegationApi::set_delegated_session_subject(delegated_subject, bootstrap_token, Some(300))?;
```

See detailed breakdown:
[docs/changelog/0.15.md](docs/changelog/0.15.md)

---

## [0.14.x] - 2026-03-09 - Parent-Funded Cycles Control Plane

- `0.14.4` upgrades recurring architecture/auth audits with normalized risk scoring, structural hotspot tracing, early-warning/fan-in detection, and stronger layer-drift checks so risks are easier to spot before regressions ship.
- `0.14.3` standardizes delegated-token issuance naming on `issue`, adds `DelegationApi::issue_token` as the single app-facing issuance path, and removes legacy `mint` naming from delegation endpoints and metrics labels.
- `0.14.2` consolidates metrics queries under `canic_metrics` (`MetricsRequest`/`MetricsResponse`) and removes the per-metric `canic_metrics_*` endpoint variants.
- `0.14.1` removes `funding_policy` config fields and keeps `topup_policy` as the only cycles config surface, while restoring unbounded request evaluation so oversized requests fail on actual parent balance checks instead of being clamped by config.
- `0.14.0` makes subtree funding parent-only with replay-safe RPC execution, adds an app-level global funding kill switch, and ships parent-emitted cycles funding metrics (totals, per-child, and denial reasons).

```text
canic_metrics(record { kind = variant { RootCapability }; page = record { limit = 100; offset = 0 } })
```

See detailed breakdown:
[docs/changelog/0.14.md](docs/changelog/0.14.md)

---

## [0.13.x] - 2026-03-07 - Distributed Capability Invocation

- `0.13.8` hardens cycles top-up safety validation with stronger config tests, restructures design/audit documentation layout for maintainability, and adds the `0.14` parent-funded cycles control-plane design/status documentation.
- `0.13.7` completed lifecycle boundary follow-up coverage (non-root repeated post-upgrade readiness plus non-root post-upgrade failure-phase checks), tightened root capability metric internals, refreshed replay/audit run guidance for constrained local environments, and fixed intent concurrency capacity checks so `max_in_flight` counts only pending reservations (preventing committed claim intents from permanently blocking later claims for the same caller-scoped key).
- `0.13.6` expanded auth/replay/capability test coverage and aligned root replay integration tests with current duplicate handling, while making the shared root test harness recover cleanly after a failed test.
- `0.13.5` further reduced branching pressure by moving replay commit fully into ops, switching built-in access predicates to evaluator-based dispatch, and replacing monolithic root capability metric events with structured `event_type`/`outcome`/`proof_mode` metrics.
- `0.13.4` simplified proof, replay, and auth internals with pluggable verifiers, a dedicated replay guard path, faster duplicate rejection, and clearer delegated-auth error grouping.
- `0.13.3` finished the auth/control-plane extraction, standardized directory modules with `mod.rs`, and refreshed complexity/velocity audit baselines.
- `0.13.2` continued the module split and moved request/auth helpers behind cleaner facades, reducing coupling between high-traffic code paths.
- `0.13.1` split large RPC/auth workflow files into smaller modules, making the control plane easier to read and change without altering behavior.
- `0.13.0` introduced signed capability envelopes for cross-canister root calls, with built-in replay protection and capability hashing to prevent request reuse/tampering.

```text
same request_id + same payload -> ReplayDuplicateSame (rejected)
same request_id + different payload -> ReplayDuplicateConflict (rejected)
```

See detailed breakdown:
[docs/changelog/0.13.md](docs/changelog/0.13.md)

---

## [0.12.x] - 2026-03-07 - Root Role Attestation Framework

- `0.12.0` adds root-signed role attestations and an attested root dispatch path, so services can authorize callers by signed proof instead of full directory sync.

See detailed breakdown:
[docs/changelog/0.12.md](docs/changelog/0.12.md)

---

## [0.11.x] - 2026-03-07 - Capabilities Arc and Replay Hardening

- `0.11.1` hardens root capability replay/dispatch behavior, improves auth diagnostics, and records each root's local subnet binding in `canic_app_registry`.
- `0.11.0` starts the capability-focused auth line with stronger scope checks and safer account/numeric behavior.

See detailed breakdown:
[docs/changelog/0.11.md](docs/changelog/0.11.md)

---

## [0.10.x] - 2026-02-24 - Delegated Auth Tightening and Runtime Guardrails

- `0.10.5` switched HTTP outcall APIs to raw response bytes, tightened memory-bootstrap safety, and reduced default wasm artifact size.
- `0.10.2` fixed lifecycle ordering so memory bootstrap is guaranteed before env restoration and runtime stable-memory access.
- `0.10.1` added optional scope syntax to `authenticated(...)` while preserving delegated-token verification semantics.
- `0.10.0` moved authenticated endpoints to direct delegated-token verification with explicit root/shard/audience binding and removed relay-style auth envelopes.

```rust
let raw: HttpRequestResult = HttpApi::get(url).await?;
```

See detailed breakdown:
[docs/changelog/0.10.md](docs/changelog/0.10.md)

---

## [0.9.x] - 2026-01-19 - Delegated Auth and Access Hardening

- `0.9.26` exported `SubnetRegistryApi` at the stable public path.
- `0.9.25` expanded network/pool bootstrap logging for clearer operational diagnostics.
- `0.9.24` added root top-up balance checks and safer pool-import bootstrap ordering.
- `0.9.23` renamed canister kinds and sharding query terminology to the current contract.
- `0.9.20` fixed multi-argument delegated-token ingress decoding and removed legacy dev bypass behavior.
- `0.9.18` enforced compile-time validation rules for authenticated endpoint argument shapes.
- `0.9.17` moved local bypass handling into delegated verification so auth paths stay consistent.
- `0.9.16` added a local/dev short-circuit path for delegated auth under controlled conditions.
- `0.9.14` removed delegation rotation/admin/status surfaces as part of shard lifecycle cleanup.
- `0.9.13` added signer-initiated delegation request support through root.
- `0.9.12` completed auth delegation audit follow-up and strengthened view-boundary usage.
- `0.9.11` added delegated-auth rejection counters for better operational visibility.
- `0.9.10` standardized the delegated-auth guard surface as `auth::authenticated()`.
- `0.9.7` cleaned up IC call builders so argument encoding/injection is consistently fallible and explicit.
- `0.9.6` hardened lifecycle/config semantics and normalized app config naming.
- `0.9.5` aligned access predicates into explicit families (`app`, `auth`, `env`) with a cleaner DSL surface.
- `0.9.4` made app init mode config-driven and aligned sync access behavior.
- `0.9.3` made app-state gating default-on for endpoints unless explicitly overridden.
- `0.9.2` moved endpoint authorization to a single `requires(...)` expression model with composable predicates.
- `0.9.1` ran consolidation audits to tighten layering boundaries and consistency rules.
- `0.9.0` established the delegated-auth baseline and runtime architecture for proof-driven endpoint authorization.

See detailed breakdown:
[docs/changelog/0.9.md](docs/changelog/0.9.md)

---

## [0.8.x] - 2026-01-13 - Intent System and API Consolidation

- `0.8.6` raised intent pending-entry storage bounds to safely handle large keys.
- `0.8.5` introduced the stable-memory intent system with reserve/commit/abort flows and contention coverage.
- `0.8.4` cleaned up docs and reduced redundant snapshot/view conversions.
- `0.8.3` exposed protocol surfaces through the public API layer.
- `0.8.1` exported `HttpApi` under `api::ic` alongside call utilities.
- `0.8.0` consolidated the public API surface and hardened error-model consistency.

See detailed breakdown:
[docs/changelog/0.8.md](docs/changelog/0.8.md)

---

## [0.7.x] - 2025-12-30 - Architecture Consolidation and Boundary Cleanup

- `0.7.28` moved macro entrypoints into the `canic` facade crate.
- `0.7.26` cleaned up stale docs and layering inconsistencies.
- `0.7.23` added a fail-fast root bootstrap guard for uninitialized embedded wasm registries.
- `0.7.22` unified internal topology state on authoritative `CanisterRecord`.
- `0.7.21` expanded IC call workflow helpers with argument-aware variants.
- `0.7.15` standardized endpoint-wrapper error conversion into downstream error types.
- `0.7.14` removed DTO usage from ops via ops-local command types.
- `0.7.13` standardized infra error bubbling and structure under ops.
- `0.7.12` switched signature internals to the `ic-certified-map` hash tree path.
- `0.7.11` moved sharding placement to a pure deterministic policy model.
- `0.7.10` moved API instrumentation ownership into `access`.
- `0.7.9` mirrored authentication helpers into `api::access`.
- `0.7.8` aligned topology policy modules under `policy::topology`.
- `0.7.7` split `api/topology` and filled missing surface functions.
- `0.7.6` resynced certified data from the signature map during post-upgrade.
- `0.7.4` expanded `canic-cdk` with additional ckToken support.
- `0.7.3` added a public `api::ic::call` wrapper routed through ops instrumentation.
- `0.7.2` tightened workflow/policy naming and topology lookup contracts.
- `0.7.1` tightened ops-layer boundaries through an explicit audit pass.
- `0.7.0` consolidated architecture/runtime discipline and clarified boundary ownership.

See detailed breakdown:
[docs/changelog/0.7.md](docs/changelog/0.7.md)

---

## [0.6.x] - 2025-12-18 - Runtime Hardening and Pool Evolution

- `0.6.20` added stricter canister-kind validation, typed endpoint identity, and registry/pool hardening.
- `0.6.19` switched endpoint perf accounting to an exclusive scoped stack model.
- `0.6.18` added log entry byte caps and fixed several lifecycle/http/sharding edge cases.
- `0.6.17` added bootstrap-time pool import support (`pool.import.local` / `pool.import.ic`).
- `0.6.16` hardened pool import/recycle/install failure handling and state cascade behavior.
- `0.6.13` made env/config access fallible with clearer lifecycle failure behavior and stronger directory/env semantics.
- `0.6.12` enforced build-time `DFX_NETWORK` validation across scripts and Cargo workflows.
- `0.6.10` improved ICRC-21 error propagation for idiomatic `?` handling.
- `0.6.9` renamed reserve configuration to pool and introduced status-aware import modes.
- `0.6.8` removed mutex-based randomness plumbing and introduced configurable reseed behavior.
- `0.6.7` replaced macro panics with compile errors for unsupported endpoint parameter patterns.
- `0.6.6` restored build-network access and aligned access-policy/runtime wrappers.
- `0.6.0` introduced a major endpoint-protection/runtime refactor and split metrics endpoints.

See detailed breakdown:
[docs/changelog/0.6.md](docs/changelog/0.6.md)

---

## [0.5.x] - 2025-12-05 - Metrics, Lifecycle, and Memory Foundations

- `0.5.22` aligned CI to build deterministic wasm artifacts before lint/test gates.
- `0.5.21` consolidated perf/type paths and improved timer metric labeling.
- `0.5.17` added ops-level HTTP metrics support.
- `0.5.16` fixed CMC top-up reply handling so failed top-ups are not reported as success.
- `0.5.15` simplified reserve-pool lifecycle orchestration.
- `0.5.14` split metrics into ICC and system categories.
- `0.5.13` centralized canister call metric recording through wrapped cross-canister construction.
- `0.5.12` made topology sync branch-targeted with safer fallback behavior.
- `0.5.10` added a wrapper around `performance_counter`.
- `0.5.8` reduced cascade complexity toward near-linear sync behavior.
- `0.5.7` improved create-flow bootstrap diagnostics with caller/parent context logs.
- `0.5.6` unified background timer startup through a single role-aware service entrypoint.
- `0.5.4` hardened reserve import/recycle sequencing and cascade safety.
- `0.5.2` split stable-memory infrastructure into `canic-memory` and re-exported runtime/macro support.
- `0.5.1` moved shared wrappers into `canic-core::types` and slimmed public type exports.
- `0.5.0` introduced the `canic-cdk` facade and stabilized a curated IC integration surface.

See detailed breakdown:
[docs/changelog/0.5.md](docs/changelog/0.5.md)

---

## [0.4.x] - 2025-12-01 - Registry and Signature Stability Passes

- `0.4.12` unified signature verification entrypoints and fixed root child-directory rebuild behavior.
- `0.4.8` tightened memory visibility and removed unused internals.
- `0.4.7` fixed signature verification panic behavior for short principal forms.
- `0.4.6` aligned directory rebuild behavior and added end-to-end consistency coverage.
- `0.4.1` fixed canister registration ordering to avoid phantom entries on install failure.
- `0.4.0` formalized the `endpoints -> ops -> model` layering contract.

See detailed breakdown:
[docs/changelog/0.4.md](docs/changelog/0.4.md)

---

## [0.3.x] - 2025-11-15 - Pagination and Logging Foundations

- `0.3.15` expanded app/subnet directory access across canisters with paginated DTO responses.
- `0.3.0` added paginated subnet-children APIs and introduced configurable bounded log retention.

See detailed breakdown:
[docs/changelog/0.3.md](docs/changelog/0.3.md)

---
## [0.2.x] - 2025-11-10 - PRIME Subnet and Topology Foundations

- `0.2.24` added `cfg(test)`-gated PocketIC helper support under `test/`.
- `0.2.21` fixed nested canister-role validation so invalid deep config is detected correctly.
- `0.2.17` removed the `icrc-ledger-types` dependency in favor of a local implementation.
- `0.2.10` switched sharding structures to string-based IDs and standardized scaling placement on HRW.
- `0.2.9` strengthened recursive config validation, including invalid subnet-directory detection.
- `0.2.7` moved `xxhash` utilities into `canic` for shared sharding usage.
- `0.2.6` continued layer cleanup by splitting memory/ops responsibilities and moving reserve config to per-subnet settings.
- `0.2.3` moved app/subnet directory projections to `SubnetCanisterRegistry` and included directory state in canister init payloads.
- `0.2.2` removed legacy delegation flow and added `ops::signature` for canister-signature creation/verification.
- `0.2.1` shipped early stabilization fixes after the initial topology rollout.
- `0.2.0` introduced prime-subnet topology foundations, including `SubnetRole`, `Env` identity context, and synchronized state+directory snapshots.

See detailed breakdown:
[docs/changelog/0.2.md](docs/changelog/0.2.md)

---

## [0.1.x] - 2025-10-08 - Initial Publish and Early Runtime Foundations

- `0.1.7` added subnet PID capture support with `dfx 0.30.2` for root subnet context tracking.
- `0.1.4` added delegation sync helpers and a more ergonomic `debug!` logging macro.
- `0.1.3` refreshed documentation, including a README rewrite and cleanup of outdated docs.
- `0.1.0` published `canic` to crates.io after the final rename from `icu`.

See detailed breakdown:
[docs/changelog/0.1.md](docs/changelog/0.1.md)
