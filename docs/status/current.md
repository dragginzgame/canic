# Current Status

Last updated: 2026-05-23

## Purpose

This file is the compact handoff for new agent sessions. Read it first, then
inspect only the files needed for the current task.

## Current Line

- Active minor: `0.42.x` authority reconciliation.
- Theme: turn 0.41 deployment truth and observed controller evidence into a
  dry-run authority reconciliation plan before any controller mutation.
- Current release-work area: passive authority reconciliation model/planner,
  exact external-action reporting, and CLI/report integration.
- Design started at
  `docs/design/0.42-authority-reconciliation/0.42-design.md`; the core issue is
  that Canic should prove controller state is correct or explain exactly why it
  cannot make it correct.

## Recent Work

- Unreleased 0.42 development: authority actions, automatic-action candidates,
  external-action records, and dry-run receipt observations now carry typed
  controller deltas so consumers can read exact add/remove controller sets
  without recomputing them.
- Unreleased 0.42 development: authority dry-run receipts now include the
  source authority report ID, making standalone receipt provenance explicit
  without requiring the full evidence bundle.
- Unreleased 0.42 development: authority reports now carry the inventory ID
  and authority profile hash from the reconciliation plan, making standalone
  report output self-describing.
- Unreleased 0.42 development: bootstrap `wasm_store` artifact builds now treat
  missing `ic-wasm` metadata embedding as optional, matching the shrink pass and
  avoiding CI failures on runners that do not install the auxiliary binary.
- `0.42.4` tightens dry-run authority readiness. External-action records now
  contain only actual external authority actions, standalone receipts preserve
  unresolved observation gaps, reports include typed apply-readiness blockers,
  and the 0.42 design/status docs now frame apply, pool mutation, remote
  lock/epoch checks, and post-apply verification as promoted-or-later work.
- `0.42.3` tightens break-glass authority reporting. Authority reconciliation
  now blocks staging/emergency principal overlap with normal expected
  controllers as `authority_profile_overlap` hard failures, reports count hard
  findings, receipts preserve them, and blocked reports emit specific next
  actions for unsafe canister findings versus hard authority findings.
- `0.42.2` adds passive authority dry-run receipts and read-only
  `canic deploy authority receipt|evidence <fleet>` JSON output. Receipts
  preserve verified controller observations and unresolved external actions
  while explicitly recording that no controller mutations were attempted.
- `0.42.1` adds the read-only authority report/evidence surface. It includes
  `AuthorityReportV1`, `AuthorityReportCountsV1`, self-contained
  external-action records, pool authority cases, explicit
  `AuthorityAutomaticActionV1` records, typed observation gaps, action-count
  breakdowns, control-class breakdowns, and next-action guidance without
  applying controller changes.
- Started `0.42.0` authority reconciliation with a passive
  `AuthorityReconciliationPlanV1` model, dry-run planner over the existing
  `DeploymentCheckV1`, and read-only
  `canic deploy authority check <fleet>` JSON output. The first planner
  classifies already-correct controller sets, deployment-controlled controller
  deltas that could be applied automatically later, external-action cases for
  non-exclusive control classes, and unsafe unknown canisters, without mutating
  IC state.
- `0.41.18` was a cleanup-only deployment truth report refactor. Duplicate
  evidence grouping and diff/finding construction now share local helpers, and
  verifier readiness no longer uses a panic-shaped `expect("checked above")`
  path. No operator-facing behavior change was intended.
- Deployment diffs now detect duplicate planned verifier role-epoch
  expectations: conflicting minimum epochs hard-fail, while exact duplicate
  planned epoch requirements warn and compare only once.
- Receipt-aware deployment diffs now detect duplicate phase receipts:
  conflicting postcondition evidence hard-fails resume, while exact duplicate
  phase receipts warn without changing the resumable phase set.
- Receipt-aware deployment diffs now detect duplicate role-phase receipts:
  conflicting role-scoped phase evidence hard-fails resume, while exact
  duplicate role-phase receipts warn without changing the resumable phase set.
- Deployment diffs now detect duplicate observed artifact evidence by role:
  conflicting artifact observations hard-fail, while exact duplicate artifact
  observations warn instead of being collapsed by role-indexed lookup.
- Deployment diffs now detect duplicate verifier role-epoch observations:
  conflicting epoch evidence hard-fails, while exact duplicate epoch evidence
  warns instead of being collapsed by role-indexed lookup.
- Deployment diffs now detect duplicate planned artifact entries by role:
  conflicting planned artifact evidence hard-fails, while exact duplicate
  planned entries warn and compare only once.
- Deployment diffs now detect duplicate planned canister declarations:
  conflicting role-to-ID assignments hard-fail, while exact duplicate planned
  canister entries warn and compare only once.
- Deployment diffs now detect duplicate planned pool declarations:
  conflicting pool identity-to-ID assignments hard-fail, while exact duplicate
  planned pool entries warn and compare only once.
- Observed pool canister control classes now reuse enriched child live-status
  evidence, so pool safety reports can reflect live controller drift rather
  than only registry parentage.
- Controller drift checks now treat `subnet_registry+icp_canister_status`
  observations as live status evidence, so enriched child observations with
  missing expected controllers fail as controller drift instead of registry-only
  uncertainty.
- Deployment diffs now hard-fail when a concrete expected canister ID is
  observed with a different role assignment, making ID/role topology drift
  explicit.
- Deployment diffs now detect duplicate observed canister IDs: conflicting role
  assignments hard-fail, while exact duplicate observations warn as suspicious
  inventory evidence.
- Deployment diffs now apply the same duplicate-ID guard to pool canisters:
  conflicting pool identities for one canister ID hard-fail, while exact
  duplicate pool observations warn.
- Deployment diffs now hard-fail when a canister appears in both non-pool and
  pool observations with conflicting role identities, making cross-surface
  topology contradictions explicit.
- Deployment diffs now hard-fail when an expected non-pool role has no
  concrete planned canister ID and multiple observed canisters claim that role,
  avoiding first-match ambiguity in passive inventory reports.
- Installed module-hash comparison now targets the concrete planned canister ID
  when available, and hard-fails ambiguous role-only module-hash evidence
  instead of letting duplicate role observations decide the hash check.
- Local deployment truth now treats the implicit bootstrap `wasm_store` role as
  part of the passive role set. Plans expect it, local artifact manifests and
  inventories observe its `.wasm.gz` artifact when present, and missing
  bootstrap store artifacts remain typed gaps rather than installer mutation.
- Installed child canister inventory now enriches subnet-registry role
  observations with read-only live status/controllers/module hashes when those
  status reads succeed. Failed child status reads remain typed observation gaps
  and do not erase the registry-derived role fact.
- Deployment diffs now warn on extra observed non-pool canister roles so
  unexpected registry/live topology is visible in reports without blocking
  current installer continuation.
- Duplicate observed canisters for an otherwise planned non-pool role are
  reported through the same extra-canister warning class rather than being
  hidden by the expected role name.
- Local deployment truth plans and inventories now populate
  `deployment_manifest_digest` from the observed root release-set manifest file
  when it exists. Missing manifests remain typed assumptions or observation
  gaps instead of installer authority.
- Local deployment truth plans and inventories now populate canonical runtime
  config digests from the parsed `ConfigModel`, keeping raw config SHA-256 as
  separate local consistency evidence.
- Local deployment truth identities now include stable set digests for planned
  authority, expected/observed topology, artifact sets, and pool identities
  where those passive facts are available.
- Local deployment inventories now map live subnet-registry role entries into
  observed canister facts. Registry-derived observations satisfy role
  existence and module-hash evidence without pretending controller authority
  was observed.
- Current install now persists additional deployment receipts for release-set
  manifest emission, successful root canister resolution, local artifact build,
  the IC-mutating root install/funding/staging/bootstrap phases, and observed
  `wait_ready` evidence, plus the final local install-state write. The build
  receipt now carries role-scoped artifact outcomes for configured build targets
  when those roles are present in the deployment truth plan.
- Current-install deployment truth gates now treat every
  `SafetyReportV1.hard_failures` entry as a blocker instead of maintaining a
  hand-picked blocker-code allowlist. Warnings remain report-only.
- Current-install deployment truth gates now persist the lightweight
  `DeploymentReceiptV1` artifact-gate receipt as machine-readable JSON under
  `.canic/<network>/deployment-receipts/<fleet>/` before any installer mutation.
- `canic deploy resume-report <fleet>` can now discover the latest persisted
  local deployment receipt automatically; `--receipt <file>` remains available
  for explicit comparisons.
- Added passive pool-canister comparison to deployment truth diffs. Planned
  pool identities now produce `pool_diff` entries, missing concrete pool
  canisters or mismatched pool IDs block, unsafe observed pool control classes
  block, and undeclared observed pool canisters warn without changing installer
  execution.
- Tightened passive verifier-readiness diffs so required role epochs are
  compared against observed epochs: stale observed epochs block and missing
  required role-epoch observations warn.
- Local deployment plans now populate `expected_pool` from configured
  scaling, sharding, and directory pool identities, so pool expectations appear
  in passive deployment truth reports instead of staying empty.
- Local deployment inventory can now map installed fleet registry entries into
  `observed_pool` for configured pool roles. Ambiguous role-to-pool mappings
  are reported as observation gaps rather than guessed.
- Added receipt-aware deployment truth comparison for resume reporting. It
  evaluates plan, inventory, and prior receipt identity together, reports
  blockers for mismatched plans, roots, failed commands, or unverified
  postconditions, and only marks phases resumable after live truth and receipt
  postconditions agree.
- Current-install deployment truth gates now construct and print a lightweight
  `DeploymentReceiptV1` with explicit `Complete` or `FailedBeforeMutation`
  operation status for the artifact materialization gate.
- Added read-only `canic deploy resume-report <fleet> --receipt <file>` to
  print passive `ResumeSafetyV1` JSON from the current deployment truth check
  and a prior `DeploymentReceiptV1`, without resuming or mutating state.
- Extended local deployment truth plans with installed root identity from
  `.canic` state, so the plan records the current root trust anchor and
  concrete expected root canister when available. The current-install safety
  gate now blocks when that expected root is missing from observed inventory.
- Fresh local deployment truth plans now record missing install state as an
  explicit non-blocking plan assumption, and deployment truth reports surface
  plan assumptions as warning findings.
- Current-install gate output now prefixes findings with stable source labels
  (`plan`, `inventory`, or `diff`) and subjects, making plan assumptions
  distinguishable from live observation gaps.
- Current-install artifact receipts now include role-scoped materialization
  evidence. Each configured role records whether its artifact was verified or
  failed, while the deployment truth check remains the gate authority.
- Wired configured deployment controllers into the local deployment truth plan
  so controller drift checks compare live root status against `canic.toml`
  authority intent.
- Promoted the current-install deployment truth gate beyond missing artifacts:
  materialized artifact digest drift and observable controller-authority drift
  now block before manifest emission, install, or staging.
- Blocked current-install deployment truth gates now print their summary,
  receipt postcondition, and machine-readable blocker codes before returning
  the install error.
- Deployment truth gate errors and warning output now include finding codes so
  failed current installs remain scriptable without parsing prose.
- Added controller authority comparison to the deployment truth diff. Live
  root controllers must include the expected authority profile controllers;
  authority-profile overlaps block as unsafe; undeclared live controllers warn;
  declared staging and emergency controllers are treated as intentional
  authority rather than unexplained drift.
- Corrected the 0.41 config identity model after the design update: raw local
  config SHA-256 values are now raw evidence only, while
  `deployment_manifest_digest` remains reserved for the canonical deployment
  manifest identity. Raw config drift still blocks as a local consistency
  finding.
- Started live inventory expansion for installed roots: when local install
  state identifies a root canister, deployment truth now attempts a read-only
  ICP status observation and records live controllers, module hash, and status
  when available. Failed live reads become typed observation gaps.
- Added installed module-hash comparison to the normalized diff so planned
  role module identity can be checked against live root status observations.
- Aligned `DeploymentReceiptV1` with the revised partial-execution design by
  adding operation status and role-scoped phase receipt fields. Current
  installer receipts still populate this lightly; richer per-role outcomes
  remain future execution work.
- Added lightweight deployment truth receipt helpers for the current-install
  artifact materialization gate. The install path now constructs a
  `materialize_artifacts` phase receipt from live check evidence, but the gate
  still makes decisions from the deployment truth check, not from receipt trust.
- Clarified the deployment roadmap/design contract that execution is partial,
  not atomic: receipts must preserve per-role/per-phase outcomes, while
  recovery starts with re-inventory and resume analysis rather than implicit
  rollback.
- Clarified the promotion roadmap/design contract that sealed wasm promotion
  and source/build promotion are separate role-scoped modes. Source/build
  recipe identity is distinct from target-specific materialization input and
  target materialization result because embedded config can intentionally
  change output bytes.
- Added `canic deploy diff <fleet>` and `canic deploy report <fleet>` so the
  normalized deployment diff and safety report are directly inspectable without
  parsing the full deployment check JSON.
- Added local deployment config SHA-256 evidence to the deployment truth plan
  and inventory, and made the diff fail closed when the observed deployment
  manifest digest disagrees with the plan.
- Made `canic deploy check <fleet>` usable as a read-only automation gate: it
  still prints the full `DeploymentCheckV1` JSON, but now exits non-zero when
  the derived `SafetyReportV1` is blocked.
- Tightened local artifact consistency checks: if the plan and inventory both
  observe a `.wasm.gz` file digest for the same role, a mismatch becomes a
  blocking deployment truth finding.
- Added a read-only current-install deployment truth preflight helper. It
  adapts `InstallRootOptions` into the existing local deployment truth check
  pipeline without calling installer mutation steps.
- Added `canic deploy plan|inventory|check <fleet>` as the first read-only
  operator-facing deployment truth commands. They print local deployment truth
  JSON and do not replace `canic install`.
- Added the first current-install deployment truth safety gate. After the build
  phase, the installer now refuses to continue when the deployment truth check
  proves configured role artifacts are missing.
- Added changelog governance coverage so `## Unreleased` remains root-only and
  detailed minor changelog files stay versioned.
- Added per-design-line `status.md` logs to the 0.41-0.50 design directories.
  These files are now the durable place to record what actually landed, what
  drifted from the design, and what remains open for each minor.
- Clarified the deployment roadmap ladder without changing the hard cut:
  0.41 is truth/report groundwork and current-install safety checks, 0.42 is
  report-first dry-run authority reconciliation, and 0.43 owns full
  plan-driven deploy-install execution unless explicitly promoted earlier.
- Added a read-only local deployment plan builder that produces
  `DeploymentPlanV1` from resolved fleet config and the local role artifact
  manifest. It records unresolved assumptions instead of querying IC state or
  changing installer mutation behavior.
- Added a read-only local deployment check wrapper that ties together plan
  construction, inventory collection, diffing, and safety-report rendering.
  This is the first usable shape for a future current-install safety gate, but
  it still does not mutate deployment state.
- Added local `.wasm.gz` file SHA-256 observations to deployment truth
  inventory and role-artifact manifests. These are recorded as explicit
  `ObservedFileDigest` evidence and remain separate from release-set payload
  hashes so observation does not turn release-set metadata into live truth.
- Split `canic-host::deployment_truth` into focused module files before adding
  more behavior: `mod.rs` owns public exports and the schema version,
  `model.rs` owns passive V1 DTOs, `observe.rs` owns local inventory and
  artifact observation, `report.rs` owns diff/report classification, and
  `tests.rs` owns the focused host-side coverage.
- Added a read-only local role artifact manifest builder for
  `RoleArtifactManifestV1`. It maps configured roles and materialized
  `.wasm.gz` files into deployment truth artifact records, reusing
  release-set payload hashes when available and recording missing artifact
  facts as observation gaps.
- Added the first passive deployment truth evaluator. It compares
  `DeploymentPlanV1` and `DeploymentInventoryV1` into `DeploymentDiffV1`, then
  renders `SafetyReportV1` findings for missing artifacts, unsafe control
  classes, identity mismatches, config drift, verifier-readiness gaps, and
  inventory observation gaps without changing installer behavior.
- Added the first read-only local deployment inventory collector. It maps
  configured fleet roles, local install-state root identity, and materialized
  `.wasm.gz` artifacts into `DeploymentInventoryV1`, while missing config or
  artifacts become explicit observation gaps rather than installer errors.
- Added passive host-side deployment truth V1 model scaffolding under
  `canic-host::deployment_truth`. The new types cover plans, inventories,
  receipts, diffs, safety reports, role artifacts, canister control classes,
  verifier readiness, and phase postconditions, with JSON round-trip tests but
  no installer behavior changes.
- Started `0.41.0` as a design-prep slice for the deployment truth model. This
  line follows the 0.40 attested-call hard cut and focuses on making intended
  deployment state, observed inventory, phase receipts, diffs, and safety
  reports explicit before deployment mutation.
- Reframed tentative `0.41` as a deployment truth model at
  `docs/design/0.41-deployment-truth-model/0.41-design.md`. The 0.41 line now
  centers `DeploymentPlanV1`, `DeploymentInventoryV1`,
  `DeploymentReceiptV1`, and `DeploymentDiffV1` / `SafetyReportV1`, with
  receipts treated as evidence rather than truth. The roadmap now continues
  through 0.42 authority reconciliation, 0.43 backend-agnostic execution,
  0.44 artifact promotion, 0.45 external/user-owned lifecycle, and 0.46
  multi-deployment operations.
- Started `0.40.0` by adding the passive Candid DTOs for the protected
  internal-call wire ABI:
  `CanicInternalCallEnvelopeV1`, `CanicInternalCallHeaderV1`,
  `InternalInvocationProofRequest`, `InternalInvocationProofPayloadV1`, and
  `SignedInternalInvocationProofV1`. The first slice also adds the
  `CANIC_INTERNAL_INVOCATION_PROOF_V1` signing domain and hash helper so
  method-scoped invocation proofs cannot share the generic role-attestation
  signing domain.
- Continued `0.40.0` by adding root issuance for method-scoped internal
  invocation proofs. Root now accepts `InternalInvocationProofRequest` through
  the root capability workflow and direct auth endpoint, authorizes the subject
  role from either AppIndex or subnet registry ownership, verifies that the
  audience is known, rejects empty method bindings, signs the proof with the
  internal invocation proof domain, and chooses the signed epoch from root
  config rather than caller input.
- Continued `0.40.0` by adding verifier-side internal invocation proof checks
  and the first generated protected update wrapper path. `caller::has_role(...)`
  and `caller::has_any_role([...])` are now parsed and validated as attested-role
  predicates, update-only in V1, and protected wrappers decode
  `CanicInternalCallEnvelopeV1` inside Canic, verify the proof against
  caller/audience/method/role/subnet/TTL/epoch bindings, then decode original
  Candid args only after authorization succeeds. Mixed non-attested access
  predicates are rejected for this protected wrapper path so no existing
  `requires(...)` condition is silently dropped.
- Continued `0.40.0` by adding the low-level `CanicCall` primitive through
  `canic::api::ic` and the prelude. `CanicCall` keeps raw `Call` unchanged,
  encodes original endpoint args, requests a root-signed method-scoped proof for
  the caller role, builds the internal-call envelope, and dispatches it to the
  protected endpoint. The first cut is correctness-only: no outgoing proof cache
  and no retry-on-stale-material path yet.
- Started `0.40.1` by adding a heap-only outgoing internal-invocation proof
  cache for `CanicCall`. The cache reuses only exact root/key/subject/role/
  subnet/audience/method/TTL call-edge proofs, evicts near-expiry entries, and
  rejects cached proofs below the local role epoch floor; callee verification
  remains the authority.
- Continued `0.40.1` by adding coarse protected internal-call auth error codes
  and a narrow `CanicCall` repair path: if the callee returns stale role-epoch
  material or unknown verifier-key material, the caller invalidates its cached
  proof, obtains fresh root-signed material, and retries the protected call
  once. Expired proofs, malformed envelopes, authorization failures, and domain
  handler errors are not retried.
- Started `0.40.2` by migrating the local wasm-store update surface onto the
  protected internal-call protocol. Wasm-store update endpoints now require
  `caller::has_role("root")`, while root control-plane calls to those update
  methods use `CanicCall`.
  Catalog/status queries remain structural root-query exceptions until a
  protected-query design exists. The same slice aligned direct root auth RPC
  decoding for role attestations and internal invocation proofs so callers
  decode the signed proof payload returned by the direct endpoint instead of
  the local root capability response envelope. Reconcile coverage now asserts
  that old raw update tuples fail against protected wasm-store updates.
- Continued `0.40.2` by consuming `ic-memory` for generic multi-crate
  static range and memory declaration registration. Canic now declares its core
  and control-plane ranges through `ic-memory`, delegates declaration/opening
  macros to the generic runtime, removes the stale Canic-local declaration
  registry, and keeps only the Canic-owned eager TLS touch queue for framework
  storage wrappers.
- Continued `0.40.2` by making `canic-testkit` standalone from Canic runtime
  crates. The published testkit now keeps only generic PocketIC/artifact/call
  helpers, uses its own transport error type, and leaves Canic-specific
  role/init/readiness fixtures in unpublished `canic-testing-internal`.
- Started `0.40.3` by adding protected-internal-call guardrails. The protected
  wasm-store update method list now lives in `canic-core::protocol`, the
  control-plane caller path consumes that canonical classifier, and a source
  guard test rejects first-party raw `Call`/`CallOps` usage for those protected
  method names.
- Extended the same guardrail slice so the wasm-store macro declarations and
  checked-in `wasm_store.did` are tested against the protected-update and
  structural-query manifests, preventing the protected ABI list from drifting
  away from exported endpoint shape.
- Tightened those manifest checks so they are exact-set comparisons in both
  directions: listed methods must appear with the expected ABI, and newly
  envelope-protected or structural-query wasm-store methods cannot appear
  without a manifest update.
- Added the first internal endpoint classification manifest for 0.40. The guard
  parses Canic's built-in macro-emitted internal endpoints and fails if any are
  missing an explicit protected/bootstrap/query-exception/capability/discovery/
  operator classification.
- Added a focused macro expansion regression for protected internal endpoints
  with `name = "..."` exports. The generated wrapper must compare the envelope
  target method and verify the invocation proof against the exported wire name.
- Started `0.40.4` by adding a typed `WasmStoreInternalClient` for the root
  control-plane publication path. Template source resolution, prepare/chunk/
  stage calls, and store-local GC calls now go through one client that selects
  `CanicCall` for protected updates and keeps catalog/status as structural raw
  query exceptions.
- Extended `0.40.4` by giving the wasm-store client an explicit endpoint table
  tested against the protected/query manifests, re-exporting those manifests
  through `canic::protocol`, and adding a private `RootAuthMaterialClient` so
  delegation, role-attestation, internal-invocation-proof, and key-set refresh
  requests use one structural bootstrap client boundary. Both clients now keep
  explicit endpoint tables with focused manifest/classification tests.
- Started `0.40.5` by removing the transitional AppIndex-only
  `caller::has_app_role(...)` path from the macro DSL and runtime access
  evaluator. Protected sibling Canic RPC now has one supported role surface:
  root-signed `caller::has_role(...)` / `caller::has_any_role(...)` envelopes.
- Started `0.40.6` by adding the first generated-client metadata surface for
  protected internal endpoints. The endpoint macro now emits a hidden
  `ProtectedInternalEndpoint` descriptor for every root-signed role-protected
  internal endpoint, and `CanicInternalClient` can call those descriptors through
  `CanicCall` without duplicating method names or accepted-role metadata.
- Extended `0.40.6` by adding protocol-owned protected descriptors for the
  built-in wasm-store update methods and routing `WasmStoreInternalClient`
  through `CanicInternalClient`, leaving only structural catalog/status queries
  on the raw call path.
- Tightened the same `.6` client surface with
  `ProtectedInternalEndpoint::required_single_role()`, so generated clients for
  single-role protected endpoints can derive the caller role from endpoint
  metadata and reserve explicit role selection for multi-role endpoints.
- Started `0.40.7` by turning protected endpoint descriptor accessors into a
  stable generated symbol shape, `canic_internal_endpoint_<endpoint>()`, and
  adding the first `canic_internal_client!` facade macro for typed protected
  update clients backed by those descriptors and `CanicInternalClient`.
- Extended `0.40.7` so `canic_internal_client!` supports explicit
  `role = ...` method clauses for multi-role protected endpoints while keeping
  single-role descriptors as the ergonomic default.
- Extended the `.7` client surface with `CanicInternalCallOptions` and generated
  client `with_*` transport controls for wait mode, attached cycles, and proof
  TTL, so typed clients do not need to drop down to raw `CanicCall` for those
  settings.
- Added integration coverage for the actual downstream flow: a protected
  `#[canic_update(... caller::has_role(...))]` endpoint emits
  `canic_internal_endpoint_<endpoint>()`, and `canic_internal_client!` consumes
  that generated descriptor directly.
- Started `0.40.8` by adding `canic_protected_endpoint!` so shared protocol
  modules can publish `ProtectedInternalEndpoint` descriptors for
  cross-canister generated clients without depending on the target canister
  implementation crate.
- Tightened the `.8` descriptor boundary so protected endpoint descriptors
  reject missing method names, empty accepted-role sets, empty caller roles, and
  duplicate caller roles, while shared protocol descriptor macros reject
  `roles = []` at compile time.
- Started `0.40.9` by adding a real project hub/instance fixture for generated
  protected clients: a test-only shared protocol crate owns the instance
  descriptor, the instance exposes a `caller::has_role("project_hub")`
  protected endpoint, and the hub calls it through `canic_internal_client!`.
- Extended the `.9` fixture into PocketIC coverage: the project hub provisions
  a project instance, calls its protected endpoint through the generated client,
  and a raw direct call to the protected target is rejected.
- Fixed two runtime bugs found by that coverage: the built-in wasm-store
  protected client now decodes the endpoint payload type instead of a
  double-nested `Result`, and auth-material root request metadata is
  domain-separated from provisioning/cycles request metadata so independent
  per-canister counters cannot collide in the same second.
- Started `0.40.10` by making role-attestation issuance use the root's current
  role epoch instead of copying the caller-supplied request epoch, matching the
  internal invocation proof model. The same slice removes the ignored epoch
  field from replay and capability proof payload identity, adds a canonical
  root-capability request payload helper, and domain-separates the remaining
  root request/capability metadata nonce streams. Outbound root-response
  attestation caching now treats the local role epoch as a minimum floor so
  newer root-signed epochs remain reusable while stale cached proofs are still
  rejected.
- Started `0.40.11` by extending the protected internal-call raw-call source
  guard beyond the wasm-store manifest. The guard now also discovers protected
  method names from shared `canic_protected_endpoint!` descriptors and
  protected `#[canic_update(... caller::has_role ...)]` declarations, while
  ignoring macro definitions and doc-comment examples.
- Started `0.40.12` by moving protected internal endpoint envelope decoding
  inside the Canic wrapper. Protected wrappers now read raw ingress bytes,
  decode `CanicInternalCallEnvelopeV1`, verify the proof, and only then decode
  the original endpoint arguments, so malformed raw calls return typed
  `InternalRpcMalformed` errors instead of failing in CDK argument decoding.
  The checked-in wasm-store DID and guard tests now reflect that protected
  updates expose a no-argument raw-ingress wrapper in Candid while `CanicCall`
  sends the envelope bytes directly.
- Followed up after `0.40.12` by aligning the 0.40 design notes and this
  handoff with the raw-ingress protected wrapper model. Historical implementation
  entries should now be read as current raw-ingress behavior rather than typed
  envelope Candid arguments.
- Continued that follow-up by making `CanicCall` encode the internal-call
  envelope explicitly and dispatch those bytes through `with_raw_args(...)`,
  matching the no-argument protected wrapper ABI at the public call boundary.
  A source guard now rejects a regression back to typed envelope-argument
  dispatch. The same low-level call boundary now rejects empty target methods
  and zero effective proof TTLs locally before requesting root proof material.
  Protected endpoint descriptors and handwritten `CanicCall` role selection
  treat whitespace-only method/role metadata as invalid.
- Final closeout pass is aligning the 0.40 design doc with the implemented
  raw-ingress wrapper, descriptor/generated-client, root issuance, heap-only
  cache, and endpoint-classification state.
- Started the next 0.40.13-sized hardening slice by strengthening the protected
  raw-call source guard. It now scans raw call expressions instead of only
  single lines, catches multi-line protected method literals/constants, and
  keeps external calls plus structural query exceptions allowed. The same guard
  now bracket-matches endpoint attributes so nested `caller::has_any_role([...])`
  role arrays do not hide protected methods from discovery. Raw-call pattern
  matching now avoids treating allowed `CanicCall::...` usage as forbidden raw
  `Call::...` usage.
- Started the next 0.40 hardening slice by making verifier-side auth material
  time windows explicit. Role attestations and internal invocation proofs now
  reject malformed windows where `expires_at <= issued_at`, reject future
  `issued_at` values, and map not-yet-valid internal invocation proofs to the
  non-retryable `AuthProofExpired` public class. The outgoing `CanicCall` proof
  cache also refuses malformed or future proof windows before retaining proof
  material. Root-issued role attestations and internal invocation proofs now
  share the same TTL/window construction path, keeping zero TTL, over-limit TTL,
  and expiry-overflow rejection consistent across both auth-material families.
  Internal invocation proof payload construction also rejects blank
  `audience_method` values, matching the authorization preflight guard.
- Started `0.39.1` by adding an AppIndex-backed
  `caller::has_app_role(role)` internal access predicate, giving app hubs and
  shards a first-class way to trust canonical sibling app canisters without
  relying on root-only subnet-registry checks.
- Started `0.39.2` by hardening the local `ic-memory` extraction crate while
  keeping `canic-memory` self-contained and publishable until `ic-memory` has
  an explicit publish order.
- Tightened the `ic-memory` capability boundary so sealed declaration snapshots
  and validated allocation sets cannot be fabricated by public struct literal,
  and runtime fingerprints now flow into staged generation diagnostics.
- Added a generic `ic-memory` diagnostic-export builder while deferring any
  `canic-memory` compile-time dependency on `ic-memory` until the standalone
  crate is ready to be published first.
- Started `0.39.4` as the packaging correction after `0.39.3` was published out
  of sequence: `ic-memory` is path-only local extraction scaffolding, and
  `canic-memory` is self-contained for crates.io publishing until `ic-memory`
  has an explicit publish order.
- Started `0.39.5` as the next local extraction slice for generic allocation
  lifecycle mechanics inside `ic-memory`.
- Added the first generic `ic-memory` physical commit model: dual protected
  generation slots with marker/checksum validation, highest-valid recovery,
  corrupt-newer-slot tolerance, and a native `LedgerCommitStore` boundary for
  allocation-ledger recovery and commits.
- Added generic `ic-memory` lifecycle mechanics for generation-scoped
  reservations, explicit retirements, `reserved -> active` activation, and an
  `AllocationBootstrap` pipeline that recovers, validates, stages, commits, and
  publishes validated allocations without owning framework endpoint policy.
- Started `0.39.6` with explicit genesis recovery boundaries:
  `ic-memory` can initialize from a supplied genesis ledger only when the
  protected commit store is physically empty, exposes commit-slot recovery
  diagnostics, validates `ledger_schema_version`/`physical_format_id`
  compatibility and allocation-history integrity before recovery or commit, and
  still fails closed on corrupt, incompatible, malformed, or partially written
  stores.
- Extended the same `0.39.6` slice so explicit reservation and retirement
  operations go through generic bootstrap helpers and the protected commit
  protocol instead of requiring adapters to hand-roll recover/stage/commit
  sequencing.
- Started `0.39.7` by adding Canic-owned policy adapter coverage in the
  unpublished `canic-tests` crate. The tests prove Canic's
  `MemoryManagerId(u8)` rules against `ic-memory` traits without adding a
  runtime/build dependency from publishable crates to the unpublished local
  extraction crate.
- Started `0.39.8` by moving generic `MemoryManager` slot-shape validation
  into `ic-memory`: known substrate marker, descriptor version,
  `MemoryManagerId`, usable IDs `0-254`, and ID `255` as the invalid sentinel.
  Canic namespace and range ownership still live in the Canic policy adapter.
- Extended `0.39.8` so `canic-memory` now directly depends on local
  `ic-memory` for stable-key grammar and schema-metadata validation. Packaging
  `canic-memory` as an independent crate is intentionally not the active
  constraint while this extraction converges.
- Continued `0.39.8` by making the Canic namespace/range policy an explicit
  adapter module in `canic-memory`, moving the temporary ID `0` self-record key
  to `ic_memory.ledger.v1`, reserving `0-9` for `ic-memory`, narrowing
  `canic-core` to `11-79`, and moving control-plane stores to `80-85`.
- Moved the CBOR serializer and `impl_storable_*` macros from `canic-memory`
  to `canic-cdk`; `canic-memory` now only re-exports them as compatibility
  glue while the memory crate is being retired.
- Started `0.39.9` by removing direct `canic-memory` dependencies from the
  top-level `canic` facade and `canic-control-plane`. `canic-core` is now the
  remaining Canic runtime boundary that directly owns `canic-memory` bootstrap
  glue while the extraction continues toward deleting the compatibility crate.
- Started `0.39.10` by moving the Canic managed-memory macro surface into
  `canic-core`: explicit-key memory declarations, range reservations, and
  eager-init helpers now expand through the core adapter, while the legacy
  implicit `ic_memory!` macro is not part of the core surface. The duplicated
  macro module has also been removed from `canic-memory`, leaving that crate as
  temporary backend glue.
- Started `0.39.11` by removing the `canic-memory` crate from the workspace.
  Its remaining backend modules now live under `canic-core::memory`, and
  `canic-core` depends directly on `ic-memory` for allocation-governance
  primitives.
- Started `0.39.12` by routing Canic runtime memory declarations through
  `ic-memory::DeclarationSnapshot`, adding a production Canic
  `AllocationPolicy` adapter, projecting the existing Canic physical ABI ledger
  into `ic-memory::AllocationLedger`, and running generic allocation-history
  validation during bootstrap without changing the persisted ledger format.
  The validated allocation set is now published from bootstrap, and Canic memory
  opening uses `ic-memory::AllocationSession` over the current MemoryManager
  substrate.
- Started `0.39.13` by moving reusable dual-slot protected recovery selection
  into `ic-memory`, making Canic's physical ledger recovery call the generic
  selector, and making Canic generation commits choose the inactive slot from
  validated recovery state instead of the unprotected `committed_slot` header
  field.
- Started `0.39.14` by adding `ic-memory::DualProtectedCommitStore` and making
  both `ic-memory::DualCommitStore` and Canic's physical ABI ledger record use
  the same trait-provided authoritative-slot recovery and inactive-slot
  selection mechanics.
- Extended `0.39.14` so protected commit recovery diagnostics are generated
  from the same generic `ic-memory` store trait and surfaced through Canic's
  ledger snapshot response.
- Started `0.39.15` by pointing Canic's workspace dependency at the standalone
  crates.io `ic-memory 0.0.1` package and removing the stale in-tree
  `ic-memory` workspace member/source copy.
- Removed the remaining current `canic-memory` references from README and the
  packaged-downstream publish verification scripts; historical changelog/audit
  references still describe older releases.
- Added a workspace manifest guard so explicitly publishable crates cannot add
  runtime or build dependencies on workspace crates marked `publish = false`.
- Wired the same manifest-boundary guard into `scripts/ci/publish-workspace.sh`
  before any publish attempt.
- Started `0.39.16` by moving the current `ic-memory` governance-slot range
  and ledger self-record metadata behind the standalone `ic-memory` API; Canic
  consumes that authority descriptor instead of defining the range itself.
- Canic now targets published `ic-memory 0.4.0` and consumes its generic
  `MemoryManagerRangeAuthority`, native stable-cell ledger record, CBOR ledger
  codec, and stable-structures re-export. Downstream application IDs are no
  longer modeled as a named Canic authority range; they are accepted when
  `ic-memory` validates the slot shape and the ID does not collide with a
  reserved range. The temporary local crates.io patch to the sibling checkout
  has been removed; `Cargo.lock` resolves the crate from crates.io with a
  registry checksum.
- Continued `0.39.16` by thinning `canic-core::memory`: macro-backed memory
  opens now validate by explicit stable key through `ic-memory::AllocationSession`,
  the old implicit-key declaration/registration helpers are gone, and
  `memory::api` is reduced to the ledger diagnostic facade.
- Removed the old per-crate range-reservation runtime path from
  `canic-core::memory`; Canic now keeps range concepts only as policy/ledger
  authority diagnostics, not as a registration prerequisite.
- Replaced Canic-local range DTOs in the memory diagnostic internals with
  `ic-memory` authority records and added the authority `mode` to the
  controller ledger diagnostic response.
- Collapsed the remaining live Canic memory registry duplication. Macro-backed
  stable-memory slots now register immutable `ic-memory::AllocationDeclaration`
  values before bootstrap, ad hoc pre-bootstrap registration remains a small
  pending queue, runtime bootstrap validates and commits a sealed
  `DeclarationSnapshot` through the native `ic-memory` ledger, and diagnostics
  are derived from native `ic-memory` state rather than a second authoritative
  registry map.
- Tightened the physical ledger writer hard cut: Canic now records entries only
  when they are present in an `ic-memory::ValidatedAllocations` set, and the
  old Canic-local key/ID historical conflict scanner has been removed from the
  writer path.
- Hard-cut Canic allocation persistence to the native `ic-memory` durable
  ledger: `crates/canic-core/src/memory/ledger.rs` is now a small stable-cell
  adapter over `ic_memory::LedgerCommitStore`, old Canic physical ledger
  records/projection/writer/checksum ownership are gone, and old Canic physical
  ledger bytes fail closed with an explicit hard-cut error.
- Removed Canic's direct `ic-stable-structures` workspace dependency; memory
  and `canic-cdk::structures` now use `ic_memory::stable_structures` so Canic
  does not drift from the storage substrate version selected by `ic-memory`.
- Drafted the proposed 0.40 attested Canic-call hard cut at
  `docs/design/0.40-attested-canic-calls/0.40-design.md`, replacing
  AppIndex-only sibling authorization with root-signed caller-role envelopes
  for Canic-to-Canic internal endpoints.
- Moved the backup/restore design track forward to
  `docs/design/0.35-backup-restore/0.35-design.md` and marked the old 0.34
  draft as superseded.
- Added the 0.35.2 controller-policy follow-up: root init and post-upgrade now
  retain the installing or upgrading root controller in the runtime controller
  set used for newly allocated managed children.
- Added the 0.35.3 changelog entry covering local replica port visibility,
  `canic replica start --port <port>`, configured-port local queries, ownership
  diagnostics, `canic fleet sync`, automatic `icp.yaml` sync after
  `canic fleet create <name>`, explicit `topup = {}` default top-up config
  blocks, and the default top-up amount change from `4T` to `5T`.
- Started the 0.35.4 endpoint cleanup by removing stale root wasm-store
  bootstrap upload endpoints, controller-gating root state/app-registry/log
  diagnostics, simplifying `canic_canister_status` to controller-only access,
  and updating wasm-store reconcile coverage to current managed release roles.
- Collapsed the root wasm-store endpoint surface by removing the duplicate
  publish-to-current shortcut plus split publication/retired status endpoints;
  current publication uses `canic_wasm_store_admin` and controller reads use
  `canic_wasm_store_overview`.
- Ran the 2026-05-13 recurring `instruction-footprint` performance audit as
  the first `0.35` baseline. It reports risk `3 / 10`; root delegation is the
  highest sampled endpoint at `800834` average local instructions, and the
  first-run baseline deltas are intentionally `N/A`.
- Reran the 2026-05-13 recurring `audience-target-binding` invariant audit. It
  reports risk `3 / 10` and confirms role-attestation, delegated-token,
  delegated-grant, and capability-proof audience/target binding still fails
  closed.
- Reran the 2026-05-14 oldest latest-run recurring audit,
  `token-trust-chain`, at
  `docs/audits/reports/2026-05/2026-05-14/token-trust-chain.md`. It reports
  risk `4 / 10`, finds no trust-chain correctness break, and leaves only
  structural watchpoints around `dto::auth` fan-in plus runtime verifier/guard
  edit pressure.
- Reran the next oldest latest-run recurring audit,
  `auth-abstraction-equivalence`, at
  `docs/audits/reports/2026-05/2026-05-14/auth-abstraction-equivalence.md`.
  It reports risk `3 / 10`, finds no abstraction bypass, and the recurring
  definition now uses current `crates/canic-macros` paths, targeted auth scans,
  and the auth trust-chain guard as required evidence.
- Promoted the repeated ad hoc `dry-consolidation` audit into the recurring
  system suite and reran it at
  `docs/audits/reports/2026-05/2026-05-14/dry-consolidation.md`. It reports
  risk `4 / 10`, down from May 12 after installed-fleet resolution, registry
  parsing, response parsing primitives, and major CLI command modules gained
  clearer owners.
- Applied a small dry-consolidation follow-up: `snapshot download` now uses the
  host installed-fleet resolver/cache for installed fleets, and `medic` reads
  installed-fleet state through the host installed-fleet boundary.
- Added the proposed 0.36 backup/restore v1 design at
  `docs/design/0.36-backup-restore/0.36-design.md`. The 0.36 release focus is
  proving and hardening the existing backup/restore execution code into a full
  operator-working backup and in-place restore flow with durable journals,
  receipts, resume/retry behavior, and status/verify gates.
- Started the first pushable 0.36.0 proof slice by adding backup runner tests
  for max-step resume without replaying completed/preflight work and failed
  snapshot retry from the recorded failed operation.
- Kept backup resume proof at the runner/test layer instead of exposing a public
  manual pause flag for `canic backup create`; 0.36 should start with the
  smallest operator surface that works.
- Added backup status coverage for execution layouts so durable
  plan/journal/manifest state reports `running`, `failed`, and `complete`
  without introducing new operator flags.
- Tightened `canic backup status --require-complete` to require the complete
  execution layout, including the finalized manifest, instead of accepting a
  completed execution journal by itself.
- Tightened `canic backup verify` for execution-backed backups so manifest and
  artifact verification also requires the persisted backup plan and execution
  journal to match and be complete.
- Changed backup create persistence to preserve an existing output layout and
  its progressed execution journal, so the CLI wrapper now honors the same
  resume boundary that the backup runner already supported.
- Changed `canic backup list` to surface execution-backed manifest state
  (`running`, `complete`, `failed`, or invalid plan/journal) instead of
  reporting all manifest-bearing layouts as `ok`.
- Started `0.36.1` by tightening `canic backup create --out <dir>` resume
  safety: existing layouts are preserved only when the stored plan matches the
  requested fleet, network, root, scope, target set, and operation graph.
- Extended backup create resume compatibility to authority and quiescence
  policy fields so dry-run layouts are not reused as executable backup layouts.
- Added a `LAYOUT` column to `canic backup create` output so fresh and resumed
  output layouts are visible to operators.
- Tightened `canic backup list` so manifest-plus-plan layouts with no execution
  journal report `invalid-plan-journal`, not `dry-run`.
- Tightened `canic backup create --out <dir>` so manifest-backed layouts with a
  missing execution journal are treated as incomplete instead of having a new
  journal synthesized.
- Tightened backup status, inspect, and verify so manifest-backed layouts with
  missing execution journals use the same incomplete-layout error instead of
  falling through to raw file-read failures.
- Tightened backup execution integrity so terminal mutating operations require
  matching operation receipts; preflight-completed validation operations remain
  receiptless as intended.
- Started `0.36.3` restore-runner hardening by making upload-snapshot commands
  fail if successful output does not include the uploaded snapshot id required
  by later load-snapshot operations.
- Added explicit `canic restore run --retry-failed` recovery so failed restore
  operations can be moved back to ready after inspection without hand-editing
  the apply journal.
- Tightened legacy restore upload-id parsing so only uploaded-snapshot-labelled
  text can satisfy a successful upload command without structured JSON.
- Tightened restore-runner journal loading so completed or failed operations
  must have matching command receipts before any runner mode proceeds.
- Started `0.36.4` by rejecting duplicate restore operation receipt attempts
  and adding an active-line changelog width check for root and detailed notes.
- Started `0.36.5` by requiring backup execution operation receipts to carry
  `updated_at` so terminal outcomes stay auditable in persisted journals.
- Tightened backup execution receipt recording so invalid receipts roll back
  the attempted operation transition instead of leaving partial in-memory
  state.
- Adjusted the changelog check so root `CHANGELOG.md` patch bullets stay on
  one line while detailed changelog notes keep the 88-column prose wrap.
- Started `0.36.6` by making backup execution integrity compare terminal
  mutating operation state with the latest matching receipt, so stale retry
  history cannot hide a hand-edited journal state mismatch.
- Folded persisted backup execution `restart_required` validation into the
  `0.36.6` slice so edited journals cannot hide a required restart window.
- Tightened `0.36.6` further by requiring backup execution transition
  timestamps before mutation and rejecting persisted pending or terminal
  operation states without `state_updated_at`.
- Added `0.36.6` persistence integrity coverage that rejects terminal backup
  operation timestamp drift from the latest durable operation receipt.
- Started `0.36.7` by requiring restore apply-journal command receipts to keep
  their update timestamp, command preview, exit status, and bounded
  stdout/stderr audit payloads.
- Folded stale local-replica status handling into `0.36.7`: ICP CLI local
  status is now treated as stale unless the configured gateway port is
  actually reachable, so `canic replica start` no longer reports a dead
  configured port as already running.
- Started `0.36.8` by tightening restore-runner journal loading so terminal
  restore operations must be backed by the latest matching command receipt
  attempt with the same state timestamp.
- Folded a `canic list --subtree` role-anchor fix into `0.36.8`: unique role
  names now resolve to their canister principal, while repeated roles require a
  concrete principal.
- Extended the same role-or-principal subtree selector to
  `canic cycles --subtree`, filtering the registry before cycle history,
  balance, and top-up queries run.
- Started `0.36.9` by adding the `canic info` read-only command group with
  `info list` and `info cycles` leaves, then removed the old top-level
  `canic list` and `canic cycles` aliases.
- Started `0.36.10` by proving the local `test` fleet `app` subtree
  backup/restore operator path end to end. The run exposed and fixed restore
  runner ICP command generation: network flags now sit on the concrete leaf
  command, and fresh snapshot uploads no longer pass `--resume`.
- Extended `0.36.10` cycle reporting so `canic info cycles` shows explicit
  burn and top-up rates alongside net cycle movement in a compact default
  table, keeps wider diagnostics behind `--verbose`, and includes JSON fields
  for the derived burn and top-up per-hour values.
- Fixed full non-root fleet backup manifest finalization so root-omitted
  sibling branches are emitted as separate consistency units. The deployed
  local `test` fleet now completes `canic backup create test` with six
  non-root targets, and the resulting layout passes status and verification.
- Normalized `canic backup list` timestamps for unfinished execution layouts:
  failed/running rows use unix markers from execution journals when available,
  legacy run-id stamps are converted to unix markers before display, and local
  stale backup artifact directories were removed so only the verified complete
  `test` backup remains.
- Started `0.36.11` by proving the full six-canister `test` fleet restore path
  from backup row `1`: verify backup, plan with readiness gates, apply journal,
  dry-run, one-step execute/resume, full execute, require-complete, and final
  `canic info list test` readiness.
- Added `canic backup prune` for explicit operator cleanup of backup
  directories. The first selectors are `--failed` and `--keep <count>`, with
  `--dry-run` previews and backup-list ordering.
- Started `0.36.12` by removing the `/tmp` restore choreography: restore
  plan/apply/run now accept backup-list row references, `restore prepare`
  writes default plan and apply-journal files inside the backup layout, and
  `restore status` exposes completion/attention gates for prepared restores.
- Started `0.36.13` by polishing the restore row-reference operator path:
  command help and docs now lead with `restore prepare/status/run <backup-ref>`,
  and missing prepared plan or apply-journal defaults fail with explicit
  `canic restore prepare <backup-ref>` guidance instead of raw file IO errors.
- Started `0.36.14` by making row-reference restore run/status verify that the
  prepared apply journal's `backup_root` points back at the selected backup
  directory, so copied or stale journals cannot silently read restore artifacts
  from a different backup layout.
- Started `0.36.15` by adding `restore status/run --require-ready`, giving
  operators and CI a pre-mutation guard that writes the normal JSON summary and
  then fails if the prepared apply journal is blocked or not ready.
- Closed the active 0.36 implementation track after the `0.36.15` readiness
  guard. Further backup/restore work should be bug fixes or changes proven by
  real operator use, not additional v1 scope expansion.
- Started `0.37.0` by rerunning the refreshed `bootstrap-lifecycle-symmetry`
  audit at
  `docs/audits/reports/2026-05/2026-05-16/bootstrap-lifecycle-symmetry.md` and
  fixed the non-root post-upgrade continuation path so config/auth continuation
  failures return typed errors through the lifecycle adapter instead of
  panicking inside workflow runtime.
- Refreshed and reran the next oldest recurring audit,
  `canonical-auth-boundary`, at
  `docs/audits/reports/2026-05/2026-05-16/canonical-auth-boundary.md`. It found
  no boundary bypass and now explicitly checks current macro/core auth paths,
  required scopes, update replay consumption, and private token-material helper
  limits.
- Exported `DelegatedToken` from `canic::prelude` so normal authenticated
  endpoint modules do not need a separate DTO import.
- Added a config-schema regression proving obsolete per-canister delegated-auth
  verifier tables are rejected instead of accepted through compatibility shims.
- Updated the internal audit scaling probe to use `scale_replica` and
  `policy.initial_workers = 0` so the dry-run planning probe no longer tries
  to allocate startup workers in a standalone PocketIC scenario.
- Refreshed and reran the layer boundary audit at
  `docs/audits/reports/2026-05/2026-05-16/layer-boundary.md`. It found and
  fixed two core layering drifts: workflow no longer imports module-source
  runtime types from the API layer, and cycles authorization no longer depends
  on storage `CanisterRecord` shapes. The CI layering guard now catches both
  regression classes.
- Added and ran the workflow purity audit at
  `docs/audits/reports/2026-05/2026-05-16/workflow-purity.md`. It moved
  cycles-funding policy into `domain/policy`, moved the mutable funding ledger
  into ops, moved HTTP and management DTO conversion helpers into ops, and
  added a layering guard against workflow-defined policy types.
- Added and ran the ops purity audit at
  `docs/audits/reports/2026-05/2026-05-16/ops-purity.md`. It renamed delegated
  auth certificate validation from an ops-owned policy surface to explicit
  certificate rules and documented RPC, auth, metrics, and atomic storage ops
  as accepted hotspots with watchpoints.
- Added and ran the access purity audit at
  `docs/audits/reports/2026-05/2026-05-16/access-purity.md`. It moved stable
  app-mode facts and whitelist config reads behind ops helpers, added an
  access storage/stable-type layering guard, and documented delegated-token
  boundary decode plus delegated-session cleanup as watchpoints.
- Added and ran the security-boundary ordering audit at
  `docs/audits/reports/2026-05/2026-05-16/security-boundary-ordering.md`. It
  found no critical ordering violation and added guards for authenticated
  endpoint macro access-before-dispatch ordering plus cached root response
  attestation payload subject binding.
- Started `0.37.2` by restoring stable-memory ABI tracking in `canic-memory`:
  ID `0` now stores a persisted layout ledger, and historical range or ID drift
  is rejected even if the old declaration is removed from the current binary.
- Started the `0.38.0` hard-cut by making ID `0` the canonical ledger
  self-record, treating IDs `1-99` as Canic framework expansion budget, and
  widening `canic-core` to `11-99`. The later `0.39` hard cut removed the
  named downstream application authority range from Canic policy.
- Added explicit stable-memory ABI keys for Canic-managed memory declarations
  so package, module, type, or label renames do not silently allocate new
  stable memories or strand old ones.
- Started the 0.38 stable-memory ABI design at
  `docs/design/0.38-stable-memory-abi/0.38-design.md` so this work can move as
  an urgent minor instead of remaining a patch-level cleanup note.
- Added current declaration-snapshot validation so duplicate memory IDs,
  duplicate stable keys, and exact duplicate declarations fail before user
  ledger records are committed during bootstrap.
- Added historical-ledger preflight for pending bootstrap claims so failed
  bootstrap validation cannot persist earlier user claims from the same
  snapshot before a later historical conflict is discovered.
- Reworked the persisted layout ledger into a generation-framed store with two
  committed slots, generation checksums, header metadata, and highest-valid
  generation selection.
- Ledger mutation, validation, and diagnostic snapshots now fail closed if no
  committed generation validates.
- Tightened namespace enforcement so non-Canic crates cannot claim `canic.*`
  stable keys even if they choose IDs inside the framework range.
- Split public `MemoryApi` declaration from opening: startup code can declare
  explicit-key slots before bootstrap, and post-bootstrap calls only open
  already-validated slots instead of creating new ledger claims.
- Split `ic_memory_key!` macro declaration from opening as well: constructors
  queue declaration descriptors before registry validation, and eager stable
  stores open virtual memory only after the runtime registry is validated.
- Made the macro open guard target-independent and added host-test bootstrap
  hooks for core and control-plane tests so unit tests validate before opening
  stable-store handles.
- Added `MemoryApi::ledger_snapshot()` as a first diagnostic read path over
  persisted ABI ledger history that does not depend on current registry
  reconstruction.
- Started the post-`0.38.0` ABI diagnostics follow-up by adding optional
  `schema_version` and `schema_fingerprint` metadata to managed memory
  declarations, registry DTOs, and ledger declaration history. Metadata remains
  informational and is not part of allocation identity.
- Added canonical allocation authority records to the old ABI ledger for the
  previous Canic framework/application boundary, exposed through
  `MemoryApi::ledger_snapshot()` diagnostics. The current native `ic-memory`
  path now reports only reserved infrastructure ranges owned by Canic policy.
- Tightened ABI ledger physical-header validation so invalid magic, format,
  schema version, header length, or committed slot metadata fails closed during
  bootstrap instead of being repaired.
- Added raw stable-memory preflight before declaration-snapshot mutation:
  brand-new memory may initialize the genesis ledger, while foreign or corrupt
  raw memory and existing `MemoryManager` state without a valid ID `0` Canic
  ABI ledger fail closed.
- Tightened the wasm `MemoryApi::ledger_snapshot()` diagnostic path so it
  decodes only the ID `0` ABI ledger from raw stable memory and does not depend
  on normal runtime registry reconstruction.
- Started `0.38.2` by adding a controller-only `canic_memory_ledger`
  diagnostic query for opt-in memory observability builds. It bypasses normal
  Canic endpoint dispatch and exposes committed ID `0` ledger header fields,
  the authoritative committed generation, authorities, ranges, and memory
  records through a dedicated DTO.
- Started `0.38.3` by moving `canic_memory_ledger` into the default Canic
  runtime endpoint bundles, including the canonical `wasm_store` surface, while
  keeping the heavier live `canic_memory_registry` diagnostic opt-in.
- Started `0.38.4` by extending the source-level stable-memory ABI guard across
  the Canic-managed runtime surface, including the canonical `wasm_store`, and
  clarifying `canic-memory` documentation around declaration, bootstrap, and
  post-validation opening phases.
- Started `0.38.5` by aligning current stable-memory ABI documentation around
  the final Canic-managed memory contract and clarifying that IDs `1-4` are
  range-protected metadata expansion budget, not canonical per-ID reserved
  records.
- Folded a `canic info cycles` freshness fix into `0.38.5`: when live cycle
  balance data is available, cycle summaries now derive deltas and rates
  through the live balance timestamp so post-sample auto-top-up events are
  visible before the next hourly tracker sample.
- Started `0.38.6` by adding persisted ABI ledger `layout_epoch` validation
  and exposing the compiled epoch through `MemoryApi::ledger_snapshot()`, core
  memory DTOs, `canic_memory_ledger`, and the canonical `wasm_store` DID.
- Started `0.38.7` by hard-cut reallocating `canic.core.app_state.v1` from ID
  `62` to ID `18`, colocating app runtime state with core env and subnet state
  before the 0.38 stable-memory ABI layout is treated as frozen.
- Reworked the PR #8 topology direction for `0.38.7`: local ICP network
  settings such as `ii` and `nns` remain in `icp.yaml`; the later `0.38.8`
  cleanup made Canic's ICP project config checks read-only.
- Started `0.38.8` by stopping Canic from deriving or rewriting `icp.yaml` from
  `canic.toml`, making `canic status` check ICP project config read-only,
  pinning the checked-in local ICP network launcher to
  `v13.0.0-2026-05-07-04-27`, and adding an upstream watch workflow that fails
  when a newer launcher tag appears, prompting a test for the delegation
  certificate fix from upstream `dfinity/ic` commit `17524c56`.
- Started `0.38.9` after `0.38.8` was published by removing the misleading
  `canic fleet sync` command and replacing it with `canic fleet check <name>`.
- Folded hidden-support cleanup into `0.38.9`: renamed the hidden `canic-core`
  `__control_plane_core` bridge to `control_plane_support`, moved neutral
  formatting to hidden `shared_support::format`, and removed the broad
  `core_support` caller aliases from `canic-control-plane`.
- Started `0.39.0` by adding the root `ic-memory` crate as the future
  standalone repository boundary. The first slice includes generic stable-key
  parsing, allocation-slot descriptors, schema metadata, declaration
  collection/sealing, policy and substrate traits, validated allocation
  sessions, generation/ledger data shapes, and diagnostic export shapes without
  depending on Canic or `canic-cdk`.
- Extended the `0.39.0` generic crate with allocation-history validation and
  pure logical generation staging. Current declarations are now checked against
  policy, stable-key history, slot history, and retired allocation tombstones,
  while omitted historical records remain owned and active.
- Added a source-level guard test that rejects implicit registration, direct
  raw stable-memory APIs, independent `MemoryManager` access, and
  `RestrictedMemory` carve-outs in Canic-managed runtime crates.
- Split root install guidance into `INSTALLING.md` and refreshed README
  examples around the current `canic info list` command group.
- Renamed the test fleet scaling worker role from `scale` to `scale_replica`,
  changed role cycle config from `topup_policy` to `topup`, and enabled explicit
  default `topup = {}` policy blocks for the main test app, hub, shard, and
  scaling roles.
- Slimmed the ICP build hook path: `icp.yaml` now invokes
  `cargo run -q -p canic-host --example build_artifact -- <role>` directly,
  the Rust builder owns `ICP_WASM_OUTPUT_PATH` copying, and the old
  `scripts/app/build.sh` wrapper has been removed.
- Tightened local replica ownership checks so `canic replica start --background`
  and `canic status` use project-scoped ICP network status instead of broad
  local ping, while `canic replica stop` distinguishes "this project is already
  stopped" from "port 8000 is owned by a different ICP network/project".
- Added configured local gateway port output to `canic status` and
  `canic replica status`, plus `canic replica start --port <port>` to update
  this project's `icp.yaml` `gateway.port` before starting.
- Hard-cut the managed child controller policy for 0.35.1: newly allocated
  non-root canisters now receive configured controllers, root, and their direct
  parent as controllers; pool reuse updates the controller set before install.
- Tightened `canic install <fleet>` build output by hiding unset requested
  profile noise, using operator labels for build context, omitting duplicate
  ICP root context, adding `WASM_GZ` sizes to the build table, and making
  local root top-up output show the checkpoint phase, exact amount, and target.
- Added explicit restore-run stop/start phases so apply journals now schedule
  snapshot upload, target stop, snapshot load, target start, and verification
  operations instead of depending on manual canister state changes.
- Completed the 0.33 ICP CLI hard cut: `icp.yaml`, `.icp`, ICP CLI install/list/
  medic/snapshot/restore flows, native replica controls, and project status.
- Removed default fleet/network state and the old public `canic network`
  command; fleet-scoped commands take positional fleet names.
- Made the standard pre-1.0 `canic` facade capabilities default so fleet
  canisters no longer choose Canic feature flags manually.
- Trimmed the public metrics surface into role-inferred profiles and tiered
  selectors while keeping metrics enabled by default before 1.0.
- Added `canic endpoints` with Candid method/argument output and changed
  generated Candid finalization to require a trailing `canic::finish!()`.
- Made `canic endpoints` fleet-scoped and moved `--icp <path>` and
  `--network <name>` to top-level-only CLI options; command-local placement is
  hard-rejected instead of kept as a hidden compatibility path.
- Removed low-value list/config selectors: `canic list --root` is gone,
  `canic list --from` is now `canic list --subtree`, and `canic config --from`
  is gone.
- Removed `canic endpoints --did`; endpoint lookup now uses fleet metadata and
  known local role `.did` artifacts only, and registered principals infer their
  fallback role from the fleet registry instead of taking `--role`.
- Removed `KIND` from the live `canic list` table, added `CYCLES` in `0.33.6`,
  and added `CANIC` in `0.33.7`; version and cycle balances now use parallel
  `icp canister call canic_metadata` and `canic_cycle_balance` reads.
- Replaced the separate generated `canic_canister_version` and
  `canic_standards` endpoints with a single `canic_metadata` endpoint that
  includes package metadata, Canic version, and IC canister version.
- Local root installs keep a `100.00 TC` root ready target, including
  pre-bootstrap and post-ready top-up checkpoints for reused local root
  canisters.
- Grouped `snapshot`, `backup`, `manifest`, and `restore` under a dedicated
  backup/restore section in the top-level `canic help` output.
- Fixed local `canic snapshot download <fleet>` target discovery to use decoded
  local replica registry queries instead of parsing the ICP CLI transport JSON
  wrapper.
- Fixed real snapshot-download id extraction to use
  `icp canister snapshot create --quiet` and hex-only parsing, preventing table
  units such as `MiB` from being treated as snapshot ids.
- Removed `--resume` from fresh snapshot downloads and documented the 0.34
  backup/restore redesign around root-stays-up subtree backup phases.
- Centralized byte-size and TC cycle formatting through shared format helpers
  so list and config output use the same labels.
- Removed public install overrides: `canic install` is now just
  `canic install <fleet>` with fleet config, root target, and readiness timeout
  owned by Canic.
- Added hard fleet identity checks: duplicate discovered `[fleet].name` values
  fail config discovery, and install requires the config identity to match the
  requested fleet directory.
- Moved the `minimal` shared-runtime baseline under `canisters/audit` and made
  `canic status` compare local deployments against bootstrap-required roles.
- Refreshed the module-structure audit and reduced the current structural risk
  readout to `3/10`.
- Split current 0.33 hotspots in `canic-core` IC management/provisioning,
  `canic-control-plane` publication, and `canic-backup` restore
  runner/apply-journal internals into normal directory modules.
- Ran the oldest outstanding recurring audit, `change-friction`, against the
  current 0.33 line. It reports medium friction risk at `5/10`: the broad
  DFX-to-ICP CLI hard cut raised patch radius, but no cross-layer leakage was
  confirmed. The rerun after reloading ICP CLI used `icp 0.2.6`, clean snapshot
  `09f5d238`, and included the committed `0.33.7` metadata/list slice.
- Started remediating the change-friction follow-up by splitting `canic list`
  live registry projection into `crates/canic-cli/src/list/live.rs`, reducing
  the command root from the audited `902` lines to `506` lines.
- Deduplicated `canic list` table width/separator/alignment rendering through
  `crates/canic-cli/src/list/table.rs` for both config and registry tables.
- Deduplicated the live-list threaded query collector used by local readiness,
  `canic_metadata` version reads, and `canic_cycle_balance` reads.
- Centralized list config-loader host-config error mapping so adding config
  table columns does not repeat install-state conversion boilerplate.
- Split list endpoint response parsing into `crates/canic-cli/src/list/parse.rs`
  so metadata and cycle-balance response-shape tests live beside the parsers
  rather than the live transport code.
- Promoted table rendering to `canic-host::table` and routed list, status,
  fleet-list, backup-list, medic, and install config-choice tables through one
  host/operator header/underline/spacing/alignment helper.
- Split deployed-registry tree traversal into `crates/canic-cli/src/list/tree.rs`
  so `list/render.rs` no longer owns hierarchy selection and presentation at
  the same time.
- Split host root readiness polling and diagnostics into
  `crates/canic-host/src/install_root/readiness.rs`, reducing
  `install_root/mod.rs` from `901` to `586` lines while preserving the install
  orchestration flow.
- Started the 0.34 backup/restore rework by adding `canic-backup::plan` with
  typed backup plans, targets, operations, authority/read preflights,
  quiescence policy, and operation receipts. This is a model-only slice; live
  snapshot execution is unchanged.
- Split backup plan validation from execution readiness: plans can represent
  `Proven`, `Declared`, or `Unknown` control/read authority for dry-run output,
  while mutating backup execution requires proven authority for every selected
  target.
- Added target-scoped control and snapshot-read authority preflight receipts so
  future execution can upgrade a plan only after proof covers every selected
  target.
- Added typed authority preflight request DTOs derived from `BackupPlan`, giving
  root coordination and host-side authority adapters a stable input contract.
- Added typed topology and quiescence preflight request/receipt DTOs plus
  execution-gate validation for topology drift, target-set changes, policy
  mismatches, and rejected quiescence.
- Added a full execution preflight receipt bundle so future backup execution can
  apply authority receipts and validate topology/quiescence gates through one
  typed boundary.
- Added `preflight_id`, `validated_at`, and `expires_at` to preflight receipts
  and the execution preflight bundle so stale or cross-preflight evidence cannot
  authorize later mutation.
- Added `canic-backup::execution` with a model-only backup execution journal
  built from `BackupPlan` phases, including preflight acceptance, ordered
  operation transitions, durable operation receipts, retryable failures, resume
  summaries, and `restart_required` tracking after stops.
- Added typed preflight receipt-bundle acceptance to the execution journal so
  mutation cannot be unblocked by a bundle from a different plan.
- Added `BackupLayout` read/write support for
  `backup-execution-journal.json`, keeping phase execution progress separate
  from the existing artifact download journal.
- Added `BackupLayout` read/write support for `backup-plan.json` so future
  backup runners can resume against the exact validated plan instead of
  reconstructing the operation graph.
- Added execution-layout integrity verification that rejects a persisted
  execution journal when its plan/run ids or operation graph no longer match
  the stored `backup-plan.json`.
- Added the first `canic backup create <fleet> --dry-run` CLI path, including
  optional `--subtree <role-or-principal>` planning, installed-fleet registry
  discovery, persisted `backup-plan.json`, persisted
  `backup-execution-journal.json`, and a compact dry-run summary table while
  keeping real mutation disabled.
- Made `canic backup list` include plan-only dry-run directories as
  `STATUS=dry-run`, using the persisted plan id as `BACKUP_ID` and planned
  target count as `MEMBERS`.
- Made `canic backup status --dir <dry-run-dir>` understand dry-run
  `backup-plan.json` plus `backup-execution-journal.json` layouts and report
  execution-journal progress while `--require-complete` still rejects them as
  non-backups.
- Added `canic backup inspect --dir <dry-run-dir>` with table and JSON output
  for plan metadata, selected targets, authority evidence, operation order, and
  execution-journal state.
- Added a `#` column to `canic backup list` so operators can refer to visible
  backup rows by a short ordinal as well as by `BACKUP_ID`.
- Made `canic backup inspect`, `canic backup status`, and
  `canic backup verify` accept either the `canic backup list` row number or
  `BACKUP_ID` as a positional backup reference, with `--dir <dir>` kept for
  explicit paths and ambiguous backup ids rejected fail-closed.
- Made `canic backup verify` reject dry-run plan layouts with the typed
  `DryRunNotComplete` error instead of falling through to a missing-manifest
  filesystem error.
- Added registry-backed backup plan construction for explicit subtrees and
  non-root fleet scopes, including top-down stop/snapshot phases, bottom-up
  start phases, and post-restart download/verify/finalize phases.
- Added backup selector resolution for explicit principals and unambiguous
  roles, rejecting missing or ambiguous role selectors before planning.
- Reran the oldest latest-run lightweight recurring audit, `publish-surface`,
  at `docs/audits/reports/2026-05/2026-05-11/publish-surface.md`. It reports
  package-surface risk `3/10`: all 11 publishable crates package and verify.
- Completed the publish-surface follow-up by aligning `crates/canic/README.md`
  with the default facade features and refreshing the recurring audit's
  canonical published crate map.
- Ran the full-codebase DRY consolidation audit for 2026-05-12. It reports
  medium consolidation risk at `5/10`, with installed-fleet resolution and
  large CLI command modules as the highest-value follow-ups.
- Added `canic-host::installed_fleet` with `InstalledFleetResolution`,
  `InstalledFleetSource`, `InstalledFleetRegistry`, and
  `ResolvedFleetTopology`, then routed `canic list`, `canic cycles`,
  `canic metrics`, and `canic endpoints` through the shared installed-fleet
  resolver.
- Split `canic endpoints` into command orchestration, endpoint model, Candid
  parsing, transport, and rendering modules while keeping behavior unchanged.
- Split `canic cycles` into command orchestration, options, response parsing,
  transport/report collection, rendering, and model modules while keeping
  behavior unchanged.
- Split `canic metrics` into command orchestration, options, response parsing,
  transport/report collection, rendering, and model modules while keeping
  behavior unchanged.
- Split top-level CLI command catalog/help rendering and global option
  forwarding out of `canic-cli::lib`, leaving the root focused on command
  dispatch and error mapping.
- Moved shared ICP response parsing primitives from `canic-cli` to
  `canic-host::response_parse`, and switched CLI list/cycles/metrics parsers to
  import the host-owned helpers directly.
- Moved the live subnet registry DTO/parser from `canic-backup::discovery` to
  `canic-host::registry`.
- Promoted the shared installed-fleet resolver to
  `canic-host::installed_fleet`; CLI list/cycles/metrics/endpoints now consume
  host-owned install-state lookup, local replica preference, ICP CLI fallback,
  registry parsing, and topology projection.
- Split the old `canic-cli::args` module into the `canic-cli::cli` directory
  with `clap`, `defaults`, `help`, and `globals` modules, removing the broad
  argument-helper drawer while preserving command behavior.
- Moved `path_stamp` and `registry_tree` under `canic-cli::support` to keep the
  `canic-cli` crate root focused on command families and explicit support
  modules.
- Split `canic-cli::backup` command-family help and report rendering into
  `backup::command` and `backup::render`; `backup::mod` is down to about
  `1050` lines.

## Current Memory Boundary

- Canic no longer maintains a live local allocation registry. Macro/static
  declarations and the small ad hoc pending queue are declaration inputs only.
- Runtime bootstrap collects declarations, validates and commits them through
  the native `ic-memory` durable ledger with Canic policy, publishes
  `ValidatedAllocations`, and only then opens stable-memory handles.
- `ic-memory` owns generic allocation validation: stable-key grammar, schema
  metadata bounds, `MemoryManager` ID shape and ID `255` rejection, duplicate
  declaration keys/slots, historical stable-key movement rejection, physical
  slot reuse rejection, and retired/tombstone rejection when represented in the
  native ledger.
- Canic still owns `canic.*` namespace policy, framework reserved IDs,
  rejection of application claims against reserved ranges, lifecycle ordering,
  eager TLS touches, and diagnostic DTO shaping. `ic-memory` owns
  `ic_memory.*` authority checks, declaring-crate/range composition, and
  validated handle opening.
- Canic no longer preserves the old Canic physical allocation ledger format.
  There is no projection bridge or dual-read compatibility path in the current
  hard cut; old allocation-ledger bytes require a separate migration or
  destructive reset tool before a future compatible boot.
- The opt-in live `canic_memory_registry` endpoint and DTOs have been removed.
  `canic_memory_ledger` is the single supported memory diagnostic surface.

## Validation Recently Run

- `cargo fmt --all --check`
- `cargo test -p canic-core memory --lib`
- `cargo test -p canic-core`
- `cargo clippy -p canic-core --all-targets -- -D warnings`
- `cargo test -p canic-core memory::policy -- --nocapture`
- `cargo check --workspace`
- `cargo test -p canic --test protocol_surface`
- `git diff --check`
- `cargo fmt --all`
- `bash -n scripts/ci/build-ci-wasm-artifacts.sh scripts/ci/wasm-audit-report.sh`
- `cargo check -p canic-host --examples`
- `cargo check -p canic-host --examples -p canic-tests`
- `cargo test -p canic-host canister_build -- --nocapture`
- `cargo clippy -p canic-host --examples -- -D warnings`
- `cargo check -p canic-core -p canic-host -p canic-testing-internal -p canister_scale`
- `cargo test -p canic build_support -- --nocapture`
- `cargo test -p canic-core config::schema -- --nocapture`
- `cargo test -p canic-core config::schema::subnet -- --nocapture`
- `cargo test -p canic-host release_set -- --nocapture`
- `cargo test -p canic-host install_root::tests::config_selection -- --nocapture`
- `cargo test -p canic-cli list::tests -- --nocapture`
- `cargo clippy -p canic -p canic-core -p canic-host -p canic-testing-internal --all-targets -- -D warnings`
- `git diff --check`
- `cargo test -p canic-core workflow::ic::provision::allocation -- --nocapture`
- `cargo check -p canic-core`
- `cargo clippy -p canic-core --all-targets -- -D warnings`
- `cargo check -p canic-host`
- `cargo test -p canic-host install_root -- --nocapture`
- `cargo clippy -p canic-host --all-targets -- -D warnings`
- `git diff --check`
- `cargo test -p canic-backup restore -- --nocapture`
- `cargo test -p canic-cli restore -- --nocapture`
- `cargo check -p canic-backup -p canic-cli`
- `cargo clippy -p canic-backup -p canic-cli --all-targets -- -D warnings`
- `cargo test -p canic-cli list::tests -- --nocapture`
- `cargo test -p canic-cli snapshot -- --nocapture`
- `cargo test -p canic-cli replica -- --nocapture`
- `cargo test -p canic-cli status -- --nocapture`
- `cargo test -p canic-host icp -- --nocapture`
- `cargo test -p canic-host icp_config -- --nocapture`
- `cargo test -p canic-host replica_query -- --nocapture`
- `cargo clippy -p canic-cli -p canic-host --all-targets -- -D warnings`
- `cargo run -p canic-cli -- status`
- `cargo run -p canic-cli -- replica status`
- `cargo test -p canic-host snapshot_id -- --nocapture`
- `cargo test -p canic-host snapshot -- --nocapture`
- `cargo test -p canic-backup discovery -- --nocapture`
- `cargo test -p canic-backup snapshot -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo check -p canic-host`
- `cargo test -p canic-host cycle -- --nocapture`
- `cargo clippy -p canic-host --all-targets -- -D warnings`
- `cargo build -p canic-cli --bin canic`
- `time target/debug/canic list test`
- `target/debug/canic list test`
- `target/debug/canic install demo`
- `target/debug/canic list demo`
- `target/debug/canic snapshot download demo --dry-run`
- `cargo run -q -p canic-cli --bin canic -- endpoints test app`
- `cargo run -q -p canic-cli --bin canic -- endpoints test app --json`
- `cargo check -p canic-core`
- `cargo clippy -p canic-core --all-targets -- -D warnings`
- `cargo test -p canic --test canic_metadata -- --nocapture`
- `cargo check -p canic`
- `cargo clippy -p canic --all-targets -- -D warnings`
- `cargo check -p canic-wasm-store`
- `cargo test -p canic-core --lib -- --nocapture`
- `cargo test -p canic-core --lib workflow::ic -- --nocapture`
- `cargo test -p canic-core --lib ops::ic -- --nocapture`
- `cargo check -p canic-control-plane`
- `cargo clippy -p canic-control-plane --all-targets -- -D warnings`
- `cargo test -p canic-control-plane --lib -- --nocapture`
- `cargo check -p canic-backup`
- `cargo clippy -p canic-backup --all-targets -- -D warnings`
- `cargo test -p canic-backup --lib -- --nocapture`
- `cargo test -p canic-backup plan -- --nocapture`
- `cargo test -p canic-backup execution -- --nocapture`
- `cargo test -p canic-backup persistence -- --nocapture`
- `cargo test -p canic-cli backup -- --nocapture`
- `cargo test -p canic-cli command_family_help_returns_ok -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo run -q -p canic-cli --bin canic -- backup create demo --dry-run --out /tmp/canic-backup-plan-demo`
- `cargo run -q -p canic-cli --bin canic -- backup create demo --subtree app --dry-run --out /tmp/canic-backup-plan-demo-app`
- `cargo run -q -p canic-cli --bin canic -- backup list`
- `cargo package -p canic -p canic-backup -p canic-cdk -p canic-cli -p canic-control-plane -p canic-core -p canic-host -p canic-macros -p canic-memory -p canic-testkit -p canic-wasm-store --locked --allow-dirty`
- `cargo metadata --no-deps --format-version 1`
- `cargo run -q -p canic-cli --bin canic -- backup status --dir backups/fleet-demo-20260510-222116`
- `cargo test -p canic-cli endpoints -- --nocapture`
- `cargo test -p canic-cli cycles::tests -- --nocapture`
- `cargo test -p canic-cli metrics::tests -- --nocapture`
- `cargo test -p canic-cli usage_lists_command_families -- --nocapture`
- `cargo test -p canic-cli command_family_help_returns_ok -- --nocapture`
- `cargo test -p canic-cli version_flags_return_ok -- --nocapture`
- `cargo test -p canic-cli global_ -- --nocapture`
- `cargo test -p canic-host install_root -- --nocapture`
- `cargo test -p canic-cli list::parse -- --nocapture`
- `cargo clippy -p canic-host -p canic-cli --all-targets -- -D warnings`
- `cargo test -p canic-cli installed_fleet -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo check -p canic-host -p canic-backup -p canic-cli`
- `cargo test -p canic-host registry -- --nocapture`
- `cargo test -p canic-host installed_fleet -- --nocapture`
- `cargo test -p canic-backup --lib -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-host -p canic-backup -p canic-cli --all-targets -- -D warnings`
- `cargo check -p canic-cli`
- `cargo test -p canic-cli command_family_help_returns_ok -- --nocapture`
- `cargo test -p canic-cli --lib -- --nocapture`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `git diff --check`
- `cargo test -p canic-cli backup -- --nocapture`
- `cargo run -q -p canic-cli --bin canic -- backup inspect --dir backups/fleet-demo-20260510-222116`
- `cargo run -q -p canic-cli --bin canic -- backup inspect --dir backups/fleet-demo-20260510-222116 --json`
- `cargo run -q -p canic-cli --bin canic -- backup list`
- `cargo run -q -p canic-cli --bin canic -- backup inspect 1`
- `cargo run -q -p canic-cli --bin canic -- backup status 1`
- `cargo run -q -p canic-cli --bin canic -- backup verify 1`
- `cargo run -q -p canic-cli --bin canic -- backup inspect plan-demo-20260510-222116 --json`
- `cargo run -q -p canic-cli --bin canic -- backup status plan-demo-20260510-222116`
- `git show --stat --name-only --format=fuller 8a5814fd`
- `git show --stat --name-only --format=fuller cf24f77e`
- `git show --stat --name-only --format=fuller 53476764`
- `git show --stat --name-only --format=fuller 6ea85fdb`
- `git show --stat --name-only --format=fuller 5b474986`
- `icp --version`
- `git show --stat --name-only --format=fuller 09f5d238`
- `cargo test -p canic-cli list:: -- --nocapture`
- `cargo check -p canic-cli`
- `cargo clippy -p canic-cli --all-targets -- -D warnings`
- `cargo test -p canic-host install_root::tests -- --nocapture`
- `cargo check -p canic-host`
- `cargo clippy -p canic-host --all-targets -- -D warnings`
- `bash scripts/ci/instruction-audit-report.sh`
- `cargo test -p canic-core --lib verify_root_delegated_grant_claims_rejects_audience_mismatch -- --nocapture`
- `cargo test -p canic-core --lib verify_delegated_token_rejects_audience_subset_drift -- --nocapture`
- `cargo test -p canic-core --lib verify_delegated_token_rejects_missing_local_role_for_role_audience -- --nocapture`
- `cargo test -p canic-core --lib mint_delegated_token_rejects_audience_expansion -- --nocapture`
- `cargo test -p canic-core config::schema::subnet::tests::canister_config_rejects_legacy_delegated_auth_table -- --nocapture`
- `cargo test -p canic-core config::schema -- --nocapture`
- `cargo check -p canic-control-plane -p canic -p canic-tests --tests`
- `cargo test -p canic-control-plane publication -- --nocapture`
- `cargo test -p canic-tests --test root_wasm_store_reconcile -- --test-threads=1 --nocapture`
- `cargo test -p canic-tests --test pic_role_attestation role_attestation_verification_paths -- --test-threads=1 --nocapture`
- `cargo test -p canic-tests --test pic_role_attestation capability_endpoint_role_attestation_proof_paths -- --test-threads=1 --nocapture`
- `cargo fmt --all --check`
- `cargo check -p canic-tests --tests`
- `git diff --check`

## Known Worktree Notes

- The worktree is intentionally dirty during active slice work.
- Do not revert unrelated edits.
- Agents must not stage, commit, push, bump versions, or run release targets.

## Cost-Control Rules

- Prefer scoped searches over broad repo searches.
- Avoid searching `docs/changelog/**`, `docs/audits/reports/**`, and generated
  outputs unless the task is specifically about those files.
- Write detailed findings to files; summarize only the high-signal result in
  chat.
- Keep final responses concise and include validation commands actually run.

## Good Next Tasks

1. Finish 0.41 closeout: reconcile `CHANGELOG.md`, `docs/changelog/0.41.md`,
   `docs/design/0.41-deployment-truth-model/status.md`, and this handoff before
   moving the active line to 0.42.
2. Run a focused release-readiness validation pass for the 0.41 deployment
   truth surface: `canic-host` deployment truth tests, install truth tests,
   `canic-cli deploy` tests, `canic-host` clippy, and `git diff --check`.
3. Audit 0.41 against its exit criterion: Canic can state what it plans to do,
   what exists, what differs, and whether it is safe to continue.
4. Keep any remaining 0.41 work scoped to stale-doc cleanup, validation, and
   closeout findings. New executor, authority-reconciliation, promotion, or
   consent workflow work belongs in later design lines.
