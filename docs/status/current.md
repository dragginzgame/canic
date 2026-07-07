# Current Status

Last updated: 2026-07-07

## Purpose

This is the compact handoff for new agent sessions. Read this first, then
inspect only the files needed for the current task. Detailed historical status
before this compaction is archived at
`docs/status/archive/2026-06-30-precompact.md`.

## Current Line

- The active line is `0.82.x` boundary hardening. Source of truth:
  `docs/design/0.82-boundary-hardening/0.82-design.md`.

- The current package/release-surface version is `0.82.26`. Earlier in the
  0.82 line, an accidental next-minor workspace/version-surface bump was
  corrected before patch work continued. A local stale next-minor tag was
  observed then, but it has not been deleted.

- The current `0.82.1` working slice makes the pure-policy boundary explicit:
  core policy modules live under `domain::policy::pure`, policy input/decision
  shapes moved out of `view/`, and internal call sites use the explicit pure
  namespace. This is a no-behavior-change slice with no CLI, endpoint, JSON,
  Candid, stable-state, deployment-truth, or evidence/report surface changes.
  The root and detailed `0.82.1` changelog entries are prepared.

- The current `0.82.2` working slice starts with release-safety tooling:
  `make minor` and `make major` require interactive confirmation before they
  run release gates or bump version files; `release-minor` and `release-major`
  inherit the guard.

- The same `0.82.2` slice addresses the ICP refill DTO/view boundary:
  `IcpRefillStatus` and `IcpRefillErrorCode` are now owned by
  `domain::icp_refill`, `dto::icp_refill` re-exports them to preserve public
  Rust paths and Candid shape, and internal view/storage/workflow/metrics code
  imports the values from the domain owner. This has a docs-only hardening
  report at
  `docs/design/0.82-boundary-hardening/0.82-icp-refill-dto-boundary-report.md`.

- The same `0.82.2` slice also moves root runtime subnet identity values to
  `domain::subnet` while preserving `dto::subnet` re-exports for the macro/init
  Candid boundary. Runtime root workflow imports the domain owner directly, and
  the docs-only hardening report is
  `docs/design/0.82-boundary-hardening/0.82-runtime-identity-dto-boundary-report.md`.

- The current 0.82 follow-up slice continues DTO boundary cleanup by moving
  cycle top-up event status ownership to `domain::cycles` while preserving the
  public `dto::cycles::CycleTopupEventStatus` re-export and Candid shape.
  Storage cycle ops now import the domain owner directly, with the docs-only
  report at
  `docs/design/0.82-boundary-hardening/0.82-cycle-topup-dto-boundary-report.md`.

- The same 0.82 follow-up slice moves canister pool status ownership to
  `domain::pool` while preserving the public
  `dto::pool::CanisterPoolStatus` re-export and Candid shape. Pool storage
  mapping and import/recycle workflow decisions now import the domain owner
  directly, with the docs-only report at
  `docs/design/0.82-boundary-hardening/0.82-pool-status-dto-boundary-report.md`.

- The same 0.82 follow-up slice extends the ICP refill DTO boundary cleanup by
  moving `IcpRefillMode` to `domain::icp_refill` while preserving the public
  DTO re-export and request/dry-run Candid shape. Manual, hub, replay, storage,
  and workflow tests now import the mode from the domain owner.

- The same 0.82 follow-up slice moves metrics selector ownership to
  `domain::metrics` while preserving the public `dto::metrics::MetricsKind`
  re-export and Candid shape. Runtime metrics projection, metrics workflow
  query, and lifecycle facade tests now import the domain owner directly, with
  the docs-only report at
  `docs/design/0.82-boundary-hardening/0.82-metrics-kind-dto-boundary-report.md`.

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
  `docs/design/0.82-boundary-hardening/0.82-runtime-failure-severity-dto-boundary-report.md`,
  `docs/design/0.82-boundary-hardening/0.82-runtime-field-visibility-dto-boundary-report.md`,
  `docs/design/0.82-boundary-hardening/0.82-runtime-diagnostic-status-dto-boundary-report.md`.

- The `0.82.5` slice moves memory diagnostic value ownership to
  `domain::memory` while preserving the public
  `dto::memory::MemoryCommitRecoveryErrorResponse`,
  `dto::memory::MemoryRangeAuthorityMode`, and
  `dto::memory::MemoryAllocationState` re-exports and Candid shapes. Runtime
  memory ops now import the domain owner directly, with the docs-only report at
  `docs/design/0.82-boundary-hardening/0.82-memory-diagnostic-dto-boundary-report.md`.

- The `0.82.6` slice moves app mode ownership to `domain::state` while
  preserving the public
  `storage::stable::state::app::AppMode` and `dto::state::AppMode` re-exports,
  Candid shape, and stable app-state serialization. App-state mapping now uses
  the shared domain value directly, with the docs-only report at
  `docs/design/0.82-boundary-hardening/0.82-app-mode-domain-boundary-report.md`.

- The `0.82.7` slice moves canister status and log-visibility ownership to
  `domain::canister` while preserving the public
  `dto::canister::{CanisterStatusType, LogVisibility}` and
  `ops::ic::mgmt::{CanisterStatusType, LogVisibility}` re-exports and Candid
  shapes. Management status DTO projection now uses the shared domain values
  directly, while raw management-canister infra payload types remain separate.
  The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-canister-status-domain-boundary-report.md`.

- The current `0.82.8` working slice moves HTTP method ownership to
  `domain::http` while preserving the public `dto::http::HttpMethod`,
  `ops::ic::http::HttpMethod`, and `ops::runtime::metrics::http::HttpMethod`
  re-exports and Candid method labels. IC HTTP ops and runtime HTTP metrics now
  use the shared domain value directly, while raw management-canister HTTP
  infra payload types remain separate. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-http-method-domain-boundary-report.md`.
  The same slice moves runtime endpoint status ownership to `domain::runtime`
  while preserving the public
  `dto::runtime::{HealthStatus, ReadinessStatus, RuntimeStatus, TimerStatus}`
  re-exports and serialized Candid/Serde shapes. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-runtime-status-domain-boundary-report.md`.

- The current `0.82.9` working slice moves app command status ownership to
  `domain::state` while preserving the public `dto::state::AppStatus`
  re-export and Candid command shape. App-state storage ops now import the
  status value from the domain owner, while app command/response DTOs and
  stable app-state serialization remain unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-app-status-domain-boundary-report.md`.
  The same slice moves feature-gated blob-storage billing status ownership to
  `domain::blob_storage` while preserving the public `dto::blob_storage`
  re-exports and serialized Candid/Serde shapes. Blob-storage billing status
  builders now import the status values from the domain owner, while Cashier
  request/result DTOs and billing behavior remain unchanged. The docs-only
  report is
  `docs/design/0.82-boundary-hardening/0.82-blob-storage-status-domain-boundary-report.md`.
  The same slice moves timer scheduling mode ownership to `domain::runtime`
  while preserving the public `ops::runtime::metrics::timer::TimerMode`
  re-export and projected metric labels. Timer scheduling ops and metrics
  projection now import the mode value from the domain owner, while timer
  recording behavior remains unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-timer-mode-domain-boundary-report.md`.

- The current `0.82.10` working slice moves platform-call metric dimension
  ownership to `domain::metrics` while preserving the public
  `ops::runtime::metrics::platform_call` re-exports and projected metric
  labels. IC call, HTTP, ledger, and management ops now import the metric
  dimension values from the domain owner, while platform-call metric recording
  and operation behavior remain unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-platform-call-metric-domain-boundary-report.md`.

- The current `0.82.11` working slice moves canister-op and management-call
  metric dimension ownership to `domain::metrics` while preserving the public
  `ops::runtime::metrics::canister_ops` and
  `ops::runtime::metrics::management_call` re-exports, canister-op public
  metric labels, and management-call counter behavior. Lifecycle,
  provisioning, and management ops now import the metric dimension values from
  the domain owner, while metric recording and snapshot storage remain
  unchanged. Docs-only reports:
  `docs/design/0.82-boundary-hardening/0.82-canister-ops-metric-domain-boundary-report.md`,
  `docs/design/0.82-boundary-hardening/0.82-management-call-metric-domain-boundary-report.md`.

- The current `0.82.12` working slice moves lifecycle and wasm-store metric
  dimension ownership to `domain::metrics` while preserving the public
  `ops::runtime::metrics::lifecycle`,
  `ops::runtime::metrics::wasm_store`, and `api::lifecycle::metrics`
  re-exports and public metric labels. Install-source resolution now imports
  wasm-store metric dimension values from the domain owner, while lifecycle and
  wasm-store metric recording and snapshot storage remain unchanged. Docs-only
  reports:
  `docs/design/0.82-boundary-hardening/0.82-lifecycle-metric-domain-boundary-report.md`,
  `docs/design/0.82-boundary-hardening/0.82-wasm-store-metric-domain-boundary-report.md`.

- The current 0.82 follow-up slice removes the internal
  `ops::replay::model` compatibility shim after moving replay ops and
  replay-protected workflows to the canonical `model::replay` owner. Hidden
  control-plane support now exposes `CommandKind` through a model-shaped support
  namespace. Replay behavior, stable replay receipt layout, endpoint surfaces,
  CLI behavior, Candid, JSON, deployment truth, and evidence/report schemas are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-replay-model-shim-removal-report.md`.

- The current 0.82 follow-up slice removes the internal
  `ops::replay::slot` legacy root replay adapter after routing root replay
  quota checks, reservation, commit, and purge mechanics through shared replay
  receipt helpers/storage directly. Replay behavior, stable replay receipt
  layout, endpoint surfaces, CLI behavior, Candid, JSON, deployment truth, and
  evidence/report schemas are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-root-replay-slot-adapter-removal-report.md`.

- The same 0.82 follow-up slice removes `dto::rpc` re-exports from
  `ops::rpc::request` so RPC request/response DTOs are imported from their DTO
  owner while request ops keep only dispatch helpers/errors. RPC behavior,
  capability metadata, Candid shapes, endpoint surfaces, CLI behavior, JSON,
  deployment truth, and evidence/report schemas are unchanged. The docs-only
  report is
  `docs/design/0.82-boundary-hardening/0.82-rpc-request-dto-boundary-report.md`.

- The same 0.82 follow-up slice removes the workflow-layer `TimerId` re-export
  so timer handles are imported from `ops::runtime::timer`, while
  `TimerWorkflow` keeps scheduling orchestration. Timer behavior, lifecycle
  facade behavior, runtime timer metric labels, endpoint surfaces, CLI
  behavior, Candid, JSON, deployment truth, and evidence/report schemas are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-timer-id-workflow-boundary-report.md`.

- The current 0.82 follow-up slice tightens the hidden control-plane support
  facade for pool status by exposing `CanisterPoolStatus` through
  `control_plane_support::domain::pool` instead of a DTO-shaped support
  namespace. Public `dto::pool` compatibility, pool behavior, endpoint
  surfaces, CLI behavior, Candid, JSON, deployment truth, and evidence/report
  schemas are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-control-plane-pool-status-support-boundary-report.md`.

- The same 0.82 follow-up slice removes the crate-private
  `support::WasmStoreGcExecutionStats` re-export in `canic-control-plane` so
  the template API imports GC stats from template storage ops directly.
  Wasm-store GC behavior, endpoint surfaces, CLI behavior, Candid, JSON,
  deployment truth, and evidence/report schemas are unchanged. The docs-only
  report is
  `docs/design/0.82-boundary-hardening/0.82-control-plane-template-gc-support-boundary-report.md`.

- The same 0.82 follow-up slice removes the hidden
  `control_plane_support::workflow::prelude` wildcard support path after
  root bootstrap was narrowed to import `Principal` from
  `control_plane_support::cdk::types` directly. Root bootstrap behavior,
  endpoint surfaces, CLI behavior, Candid, JSON, deployment truth, and
  evidence/report schemas are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-control-plane-prelude-support-boundary-report.md`.

- The current 0.82 follow-up slice adds maintained boundary guard tests for
  pure policy and passive DTO ownership. Pure policy modules are now checked
  against forbidden side-effect imports, async/timer/IC call fragments, and
  wire serialization fragments. Non-error DTO trees in `canic-core` and
  `canic-control-plane` are checked against internal behavior-layer imports
  and side-effect fragments, with `dto::error` documented as the public error
  boundary-adapter exception. Runtime behavior, endpoint surfaces, CLI
  behavior, Candid, JSON, deployment truth, and evidence/report schemas are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-policy-dto-boundary-guard-report.md`.

- The same 0.82 follow-up slice adds a maintained lifecycle boundary guard.
  Before-bootstrap lifecycle adapters in `canic-core` and
  `canic-control-plane` are checked to remain synchronous and timer-free, while
  root and non-root async bootstrap schedule helpers are checked to keep their
  explicit zero-delay lifecycle timer boundary. Runtime behavior, lifecycle
  macro behavior, endpoint surfaces, CLI behavior, Candid, JSON, deployment
  truth, and evidence/report schemas are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-lifecycle-boundary-guard-report.md`.

- The same 0.82 follow-up slice hard-cuts runtime introspection enum labels to
  canonical snake_case Candid/Serde labels. Candid supports explicit
  per-variant `serde(rename)` labels but not `rename_all`, so the previous
  `rename_all` plus PascalCase `serde(alias)` workaround has been removed.
  Public Rust re-export paths, endpoint routes, endpoint guards, runtime status
  builder behavior, deployment truth, evidence/report schemas, and stable-state
  layout are unchanged; the serialized runtime enum label surface is
  intentionally changed to snake_case only. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-runtime-enum-label-hard-cut-report.md`.

- The current 0.82 follow-up slice adds a maintained Candid serde boundary
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
  `docs/design/0.82-boundary-hardening/0.82-candid-serde-boundary-guard-report.md`
  and
  `docs/design/0.82-boundary-hardening/0.82-http-method-alias-hard-cut-report.md`.

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
  `docs/design/0.82-boundary-hardening/0.82-hard-cut-compatibility-sweep-report.md`.
  The root and detailed `0.82.17` changelog entries are prepared.

- The current 0.82 follow-up slice hard-cuts the public registry policy error
  codes that still used pre-service-topology singleton names. The public
  `ErrorCode` variants, host direct-query wire decoder, and checked-in
  wasm-store DID now use service-owned names for replica scaling, shard
  sharding, and instance directory policy failures. Registry policy behavior,
  messages, endpoint routes, CLI command surfaces, deployment truth,
  evidence/report schemas, and stable-state layout are unchanged. The docs-only
  report is
  `docs/design/0.82-boundary-hardening/0.82-policy-error-code-hard-cut-report.md`.

- The same 0.82 follow-up slice hard-cuts auth metric compatibility mirroring.
  Auth session, bootstrap, identity-fallback, and role-attestation events now
  record only the canonical Auth metric family instead of also writing older
  Access-family rows. Auth behavior, auth identity resolution, access-expression
  guard metrics, metrics query sorting/pagination, endpoint routes, CLI command
  surfaces, Candid, JSON, deployment truth, evidence/report schemas, and
  stable-state layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-auth-metric-mirror-hard-cut-report.md`.

- The same 0.82 follow-up slice hard-cuts the root bootstrap subnet identity
  fallback. Root bootstrap no longer invents a subnet principal from
  `canister_self()` when registry discovery is unavailable; local/test builds
  use the explicit subnet identity seeded by lifecycle init, while IC builds
  fail the bootstrap phase if NNS registry subnet discovery returns no subnet
  or errors. Root init argument shape, endpoint routes, CLI command surfaces,
  Candid, JSON, deployment truth, evidence/report schemas, and stable-state
  layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-root-bootstrap-subnet-identity-hard-cut-report.md`.
  The root and detailed `0.82.18` changelog entries are prepared.

- The current 0.82 follow-up slice hard-cuts CLI metrics/cycles
  `response_candid` fallback parsing. `canic info metrics` and
  `canic info cycles` now require structured JSON values for metrics, cycle
  tracker, and top-up report pages; text-only `response_candid` payloads and
  malformed structured entries with `response_candid` present are rejected.
  CLI command names/options, successful report output, endpoint Candid
  signatures, deployment truth, evidence/report schemas, and stable-state
  layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-cli-metrics-cycles-response-candid-hard-cut-report.md`.

- The same 0.82 follow-up slice hard-cuts host `canic_metadata`
  `response_candid` fallback parsing. Metadata version discovery now requires
  a structured JSON `canic_version` field; raw Candid text and text-only
  `response_candid` wrapper output are rejected. The `canic_metadata` endpoint
  Candid signature, CLI list command surfaces, successful live-list rendering,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-canic-metadata-response-candid-hard-cut-report.md`.
  The root and detailed `0.82.19` changelog entries are prepared.

- The current 0.82 follow-up slice hard-cuts host cycle-balance
  `response_candid` fallback parsing. ICP CLI `canic_cycle_balance` output now
  requires a structured JSON `Ok` value, while the local replica fast path
  still decodes typed Candid bytes directly. Raw Candid text and text-only
  `response_candid` wrapper output are rejected. The endpoint Candid
  signature, CLI list/cycles command surfaces, successful live-list rendering,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-cycle-balance-response-candid-hard-cut-report.md`.

- The same 0.82 follow-up slice hard-cuts root bootstrap-readiness
  `response_candid` fallback parsing. ICP CLI `canic_bootstrap_status` output
  now requires a structured JSON status record or wrapped `Ok` record, while
  the local replica fast path still decodes typed Candid bytes directly. The
  bootstrap-status endpoint Candid signature, root bootstrap lifecycle
  behavior, install command surfaces, deployment truth, evidence/report
  schemas, and stable-state layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-bootstrap-readiness-response-candid-hard-cut-report.md`.
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
  `docs/design/0.82-boundary-hardening/0.82-inspect-response-candid-metadata-hard-cut-report.md`.

- The same `0.82.21` slice hard-cuts deployment-truth artifact
  observation across network roots. Non-local deployment-truth/deploy-plan
  artifact observation now requires `.icp/<network>/canisters` and no longer
  falls back to `.icp/local/canisters`; missing selected-network artifacts are
  reported through the existing `local_artifacts.root` gap. Deployment truth,
  deploy plan, evidence, Candid, and stable-state schemas are unchanged. The
  docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-artifact-root-network-fallback-hard-cut-report.md`.

- The same `0.82.21` slice hard-cuts deployment-truth local config
  fleet-name fallback. When local config cannot resolve a fleet name,
  deployment-truth root observations now report the existing
  `local_config.fleet_name` gap and use `fleet_template = "unknown"` instead
  of copying the deployment target name into fleet-template identity. Schemas,
  command surfaces, evidence, Candid, and stable-state layout are unchanged.
  The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-local-config-fleet-name-fallback-hard-cut-report.md`.

- The same `0.82.21` slice hard-cuts the deployment catalog's active
  legacy fleet-state warning. Catalog reports now read only current
  `.canic/<network>/deployments` state and no longer probe removed
  `.canic/<network>/fleets` paths to emit `catalog.legacy_fleet_state_ignored`.
  Current catalog schema, command surfaces, deployment truth, evidence, Candid,
  and stable-state layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-deployment-catalog-legacy-fleet-warning-hard-cut-report.md`.

- The same `0.82.21` slice hard-cuts install-root legacy fleet-state
  lookup. `read_deployment_install_state` now reads only current
  `.canic/<network>/deployments/<deployment>.json` state and returns no state
  when that file is absent; it no longer probes removed
  `.canic/<network>/fleets/<name>.json` paths. Deployment registration help now
  describes the current deployment-target boundary without 0.46 legacy recovery
  language. Schemas and command surfaces are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-install-root-legacy-fleet-state-hard-cut-report.md`.
  The root and detailed `0.82.21` changelog entries include these hard cuts.

- The current `0.82.22` working slice removes CLI anti-resurrection tests for
  removed command aliases and obsolete hard-cut forms while preserving current
  positive parser, help, JSON, report, and exit-code coverage. Command
  behavior, command surfaces, endpoint surfaces, Candid, JSON, deployment
  truth, evidence/report schemas, and stable-state layout are unchanged. The
  slice also removes negative help assertions that mentioned the retired
  `canic info medic` route and renames endpoint macro guard-grammar coverage
  away from compatibility-alias wording. The auth verifier legacy
  root-proof-mode rejection test remains because it protects an active
  security/config invariant. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-cli-anti-resurrection-test-cleanup-report.md`.
  The root and detailed `0.82.22` changelog entries are prepared.

- The current 0.82 follow-up slice removes hidden
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
  `docs/design/0.82-boundary-hardening/0.82-control-plane-core-owner-support-boundary-report.md`.
  The root and detailed `0.82.23` changelog entries are prepared.

- The current 0.82 follow-up slice removes the broad hidden
  `control_plane_support::cdk` mirror. Control-plane code now imports public
  CDK types directly from `canic_core::cdk::types`, while support facades remain
  reserved for crate-private core mediation. Runtime template publication,
  root bootstrap behavior, endpoint surfaces, CLI behavior, Candid, JSON,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-control-plane-cdk-support-boundary-report.md`.

- The same 0.82 follow-up slice removes the hidden
  `control_plane_support::protocol` mirror. The control-plane wasm-store
  template client and protocol manifest tests now import public endpoint-name
  constants directly from `canic_core::protocol`, while endpoint names,
  endpoint classifications, endpoint surfaces, CLI behavior, Candid, JSON,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-control-plane-protocol-support-boundary-report.md`.

- The same 0.82 follow-up slice cleans stale release-line wording out of
  active CLI help and error text for state manifest, deploy plan, and inspect
  output. The commands now describe the current command contracts without
  implying those surfaces are tied to their original 0.79-0.81 release lines.
  Command parsing, accepted/rejected forms, exit codes, JSON/report fields,
  Candid, deployment truth, evidence/report schemas, and stable-state layout
  are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-active-cli-release-wording-cleanup-report.md`.
  The root and detailed `0.82.24` changelog entries are prepared.

- The current 0.82 follow-up slice narrows
  `control_plane_support::format` to the single formatting helper used by
  `canic-control-plane`. The hidden support namespace now exports only
  `byte_size`; host-side `cycles_tc` and `truncate` usage remains on its
  existing support path. Control-plane byte-size labels, endpoint surfaces,
  CLI behavior, Candid, JSON, deployment truth, evidence/report schemas, and
  stable-state layout are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-control-plane-format-support-boundary-report.md`.

- The same 0.82 follow-up slice cleans stale release-line labels out of active
  medic source comments and lint-expectation reasons. Medic report categories,
  exit-code behavior, endpoint surfaces, CLI behavior, Candid, JSON,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-active-source-release-comment-cleanup-report.md`.
  The root and detailed `0.82.25` changelog entries are prepared.

- The current 0.82 follow-up slice hard-cuts unused wasm-store Rust API facade
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
  `docs/design/0.82-boundary-hardening/0.82-wasm-store-api-facade-hard-cut-report.md`.
  The root and detailed `0.82.26` changelog entries are prepared.

- The current 0.82 follow-up slice hard-cuts unused wasm-store bootstrap Rust
  helpers. Root-specific direct staging helpers, their manifest normalization
  code, the unused bootstrap binding constant, and the direct staged-release
  publication support wrapper are removed. Lifecycle-used embedded release-set
  helpers, endpoint-used bootstrap helpers, endpoint method names, Candid,
  JSON, deployment truth, evidence/report schemas, stable-state layout,
  wasm-store storage behavior, publication workflow behavior, and lifecycle
  behavior are unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-wasm-store-bootstrap-helper-hard-cut-report.md`.
  The root and detailed `0.82.27` changelog entries are prepared.

- The current 0.82 follow-up slice removes the private wasm-store
  `LocalWasmStoreApi` pass-through helper and collapses the remaining
  crate-private template support module into private template API helpers.
  `WasmStoreCanisterApi` now calls private template helpers directly, while
  root bootstrap and publication APIs keep the same public method surfaces.
  Endpoint surfaces, CLI behavior, Candid, JSON, deployment truth,
  evidence/report schemas, stable-state layout, wasm-store storage behavior,
  publication workflow behavior, bootstrap behavior, and GC behavior are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-wasm-store-template-support-cleanup-report.md`.
  The root and detailed `0.82.28` changelog entries are prepared.

- The current 0.82 follow-up slice narrows workflow prelude usage in the pool
  and IC workflow clusters. Pool import/recycle/reset/scheduler/query/admin,
  IC call/ledger/management, provisioning, and ICP refill workflow modules now
  import boundary values from concrete `cdk`, `ids`, and `log` owners instead
  of `workflow::prelude::*`. The stale `workflow::prelude::Account` Rust
  re-export is removed in favor of the canonical `cdk::types::Account` owner.
  Operator command surfaces, endpoint names, Candid, JSON, deployment truth,
  evidence/report schemas, stable-state layout, pool behavior, IC call
  behavior, ledger behavior, ICP refill behavior, and provisioning behavior are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-workflow-prelude-boundary-report.md`.
  The root and detailed `0.82.29` changelog entries are prepared.

- The current 0.82 follow-up slice finishes the workflow prelude hard cut.
  Env, runtime, auth, cascade, lifecycle, RPC request, bootstrap,
  topology-index, placement-scaling, and cycle-tracking workflow modules now
  import passive values from concrete `cdk`, `ids`, and `log` owners instead
  of `workflow::prelude::*`. The unused `workflow::prelude` module is removed.
  Operator command surfaces, endpoint names, Candid, JSON, deployment truth,
  evidence/report schemas, stable-state layout, runtime startup behavior,
  auth renewal behavior, timer behavior, cascade behavior, canister lifecycle
  behavior, RPC behavior, scaling behavior, and cycle tracking behavior are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-workflow-prelude-hard-cut-report.md`.
  The root and detailed `0.82.30` changelog entries are prepared.

- Pre-1.0 hard-cut policy is now explicit in `AGENTS.md`: do not add aliases,
  shims, compatibility wrappers, legacy fallback paths, backwards-compatibility
  layers, or anti-resurrection tests unless the maintainer explicitly asks.

- The current `0.82.31` slice hard-cuts two unused Rust fallback surfaces.
  `access::expr::requires` is removed in favor of the canonical
  `access::expr::all` Rust helper, while endpoint macro `requires(...)`
  grammar remains unchanged. `CanicMetadataApi::metadata` and its core-package
  constants are removed so metadata construction stays on
  `CanicMetadataApi::metadata_for(...)`, which is the path used by the
  endpoint metadata macro with exporting-canister package metadata. Operator
  commands, endpoint method names, Candid, JSON, metadata response fields,
  deployment truth, evidence/report schemas, and stable-state layout are
  unchanged. The docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-access-metadata-fallback-hard-cut-report.md`.
  The root and detailed `0.82.31` changelog entries are prepared.

- The current `0.82.32` slice removes remaining host/CLI `response_candid` and
  raw Candid parser fixtures from active tests and hard-cuts
  `replica_query::parse_ready_json_value` so Candid text strings such as
  `"(true)"` and unrelated truthy object fields no longer count as readiness
  success. Maintained structured JSON boolean and explicit `Ok` shapes plus
  typed response-byte decoding remain covered. Operator
  commands, endpoint method names, Candid, JSON report schemas, deployment
  truth, evidence/report schemas, and stable-state layout are unchanged. The
  docs-only report is
  `docs/design/0.82-boundary-hardening/0.82-response-candid-test-fixture-hard-cut-report.md`.
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

- The current `0.80.7` working slice returns to the stable-state design by
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

- The current `0.80.8` working slice tightens state-audit upgrade-window
  validation so a domain whose `min_supported_version` is zero or greater than
  its current `version` fails with `state_domain_invalid_support_window`
  instead of being treated as a no-migration case. The same slice rejects
  invalid or duplicate migration declarations before checking required
  migration edges, and fails duplicate state-domain names within one canister
  role. The 0.80.8 changelog entries are staged in the root ledger and
  detailed 0.80 notes.

- The current `0.80.9` working slice starts by making the top-level state
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
  `canic deploy plan help` and `canic deploy plan --help` so the planning
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
  `canic medic project help` and `canic medic deployment help` render medic
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

- Continue 0.80 by expanding Rust-authored state declarations beyond the first
  root-family slice, then add more precise `*Data` snapshot declarations and
  migration coverage metadata. Do not add migration execution, stable-memory
  inspection, state dump/explore commands, generated manifest writes, runtime
  introspection endpoints, or mutation semantics.

- Before release preparation, run the focused gates for touched surfaces and
  broaden to the release matrix as needed. Do not assign a new patch version or
  change Cargo package versions unless the maintainer explicitly asks for
  release preparation.

## Useful Validation

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
