# Audit Summary - 2026-06-19

## Run Contexts

| Report | Type | Scope | Status |
| ---- | ---- | ---- | ---- |
| `access-purity.md` | Recurring system | access auth/session/token predicates, access expression evaluation, access metrics facade, endpoint macro access lowering | PASS |
| `audience-target-binding.md` | Recurring invariant | delegated-token audience/grant binding, root issuer policy proof-prepare binding, role-attestation audience binding, root capability target hash binding | PASS |
| `auth-abstraction-equivalence.md` | Recurring invariant | macro-generated authenticated endpoint expansion, access-expression dispatch, delegated-token verifier parity, delegated-session identity lanes, canister/subnet/project audience binding, root-proof provisioning integration | PASS |
| `bootstrap-lifecycle-symmetry.md` | Recurring system | start macros, lifecycle API wrappers, init/post-upgrade adapters, root control-plane lifecycle scheduling, runtime continuation, lifecycle boundary tests | PASS |
| `capability-scope-enforcement.md` | Recurring invariant | endpoint delegated-token verify/bind/scope ordering, delegated-token local-role scope enforcement, structural root capability proof routing, root capability authorization/replay ordering | PASS |
| `capability-surface.md` | Recurring system | endpoint macro bundles, retained-fleet DID surface, protocol constants, RPC/capability DTO variants, root proof provisioning endpoints, issuer-local delegated-token endpoints | PASS |
| `canonical-auth-boundary.md` | Recurring invariant | macro-generated authenticated endpoint expansion, access-expression dispatch, delegated-token endpoint verification, private token-material helper boundaries, signed role-attestation verification, root proof provisioning endpoints, issuer-local delegated-token issuance surfaces | PASS |
| `canic-cli-cli-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/cli/` | PASS |
| `canic-cli-evidence-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/evidence.rs` | PASS |
| `canic-cli-evidence-support-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/evidence_support.rs` | PASS |
| `canic-cli-info-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/info.rs` | PASS |
| `canic-cli-list-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/list/` | PASS |
| `canic-cli-output-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/output/` | PASS |
| `canic-cli-support-candid-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/support/candid.rs` | PASS |
| `change-friction.md` | Recurring system | current `0.68` root proof provisioning feature slices, release/hygiene sweep filtering, CAF/locality, boundary leakage, enum shock radius, and gravity-well pressure | PASS |
| `complexity-accretion.md` | Recurring system | `canic-core` conceptual growth, root capability contraction, root proof provisioning lifecycle spread, large-file pressure, branch-density hotspots | PASS |
| `dependency-hygiene.md` | Recurring system | workspace Cargo manifests, public/support crates, internal fixtures, fleets | PASS |
| `dry-consolidation.md` | Recurring system | CLI/host/backup ownership, evidence/report builders, command-family glue, release scripts, root proof provisioning lifecycle split | PASS |
| `expiry-replay-single-use.md` | Recurring invariant | delegated-token freshness, active proof install/status, root proof batch replay/expiry, replay policy, root replay | PASS |
| `instruction-footprint.md` | Recurring system | PocketIC instruction matrix, query probes, root proof batch prepare, verifier-side delegated-token confirmation, replay/cycles, template admin updates | PASS |
| `module-structure.md` | Recurring system | public crate roots, canic-core layers, root proof provisioning cluster, host/test seams, module layout | PASS |
| `ops-purity.md` | Recurring system | `canic-core` ops purity, root proof provisioning split, public-error boundary, policy mapper naming, runtime metrics/IC/RPC/auth hotspots | PASS |
| `publish-surface.md` | Recurring system | eight published crates, package READMEs/docs metadata, default features, binary/example/bench surfaces, installed/packaged proof scripts, package validation docs | PASS |
| `security-boundary-ordering.md` | Recurring system | endpoint delegated-token ordering, macro access sequencing, root proof provisioning prepare/get/install, issuer-local token prepare/get, root replay and capability proof ordering | PASS |
| `subject-caller-binding.md` | Recurring invariant | delegated-token endpoint subject binding, identity lanes, endpoint macro access expansion, delegated-session resolution, root-proof provisioning principal separation | PASS |
| `token-trust-chain.md` | Recurring invariant | delegated-token root/issuer trust chain, configured root canister/root-key verifier config, canonical cert/claims hashes, active proof install, role-attestation proof verification | PASS |
| `wasm-footprint-2.md` | Recurring system | release and wasm-debug canister artifacts for app, user_hub, user_shard, scale_hub, scale_replica, root; root bundle outlier; same-day baseline comparison; twiggy/ic-wasm attribution | PASS |
| `workflow-purity.md` | Recurring system | `canic-core` workflow orchestration, replay/cost/intent sequencing, root proof provisioning install, delegated-token prepare, pool/ICP refill recovery | PASS |

## Risk Index Summary

| Report | Risk | Notes |
| ---- | ----: | ---- |
| `access-purity.md` | 2 / 10 | Access remains a thin endpoint boundary; the only cleanup was refreshing the audit definition's workflow and auth-state scans. |
| `audience-target-binding.md` | 3 / 10 | No audience/target binding break found; the definition was refreshed for current root issuer policy and capability hash ownership, with residual fan-in pressure around delegated auth/root provisioning DTOs. |
| `auth-abstraction-equivalence.md` | 3 / 10 | Generated authenticated endpoints still converge through `AccessContext`, `eval_access`, `delegated_token_verified`, and `AuthOps::verify_token`; residual risk is expected auth/provisioning hotspot pressure. |
| `bootstrap-lifecycle-symmetry.md` | 2 / 10 | Lifecycle hooks remain thin restore-and-schedule adapters; audit definition was tightened around root timer scheduling and grouped lifecycle imports. |
| `capability-scope-enforcement.md` | 3 / 10 | No scope-as-identity or authorization-before-authentication break found; the definition was refreshed away from stale delegated-grant capability proof paths, with residual capability DTO and replay/authorization fan-in pressure. |
| `capability-surface.md` | 4 / 10 | No hard scoping failure found; endpoint definitions grew through root proof provisioning and issuer-local auth while default memory-ledger DID surface and old RPC/proof variants contracted. |
| `canonical-auth-boundary.md` | 3 / 10 | Authenticated endpoints still converge through macro access lowering, `eval_access`, `delegated_token_verified`, and `AuthOps::verify_token`; residual risk is auth/provisioning boundary adjacency. |
| `canic-cli-cli-module-surface-hardening.md` | 2 / 10 | Internal CLI helper surface is retained with owner; one one-caller help-rendering wrapper was inlined. |
| `canic-cli-evidence-module-surface-hardening.md` | 3 / 10 | Evidence dispatcher is retained with owner; compare/gate execution bodies were moved into focused helpers. |
| `canic-cli-evidence-support-module-surface-hardening.md` | 2 / 10 | Evidence command-provenance helper is retained with owner; optional path input was narrowed to `Option<&Path>`. |
| `canic-cli-info-module-surface-hardening.md` | 2 / 10 | Read-only info dispatcher is retained with owner; duplicate subcommand remapping was removed. |
| `canic-cli-list-module-surface-hardening.md` | 3 / 10 | Read-only deployment registry and fleet config listing is retained with owner; no high-confidence cleanup was found. |
| `canic-cli-output-module-surface-hardening.md` | 2 / 10 | Shared CLI output helpers are retained with owner; one plain-filename parent-directory edge case was hardened. |
| `canic-cli-support-candid-module-surface-hardening.md` | 1 / 10 | Local Candid sidecar helper is retained with owner; focused tests now cover existing sidecars, absent roots, and missing registry roles. |
| `change-friction.md` | 4 / 10 | Change friction remains moderate and manageable; routine `0.68` root-proof provisioning slices are narrower than the prior sample, no boundary leakage regression was found, and broad hygiene/release sweeps are classified separately. |
| `complexity-accretion.md` | 3 / 10 | Capability variants contracted after the hard cuts, and root proof provisioning pressure was reduced by splitting `ops/auth/delegation/mod.rs` into focused local owners. |
| `dependency-hygiene.md` | 2 / 10 | Published crates still avoid unpublished runtime edges; `canic` defaults remain narrow; auth/control-plane/sharding features remain explicit. |
| `dry-consolidation.md` | 3 / 10 | No High or Medium duplicate owner found; deploy pressure is much lower after the split and root proof provisioning has clear layer owners. |
| `expiry-replay-single-use.md` | 3 / 10 | No expiry/replay break found; root proof batch prepare/get/install and active proof freshness are covered, with residual fan-in pressure around delegated auth and provisioning DTOs. |
| `instruction-footprint.md` | 2 / 10 | PocketIC measurement passed after reinstalling pinned `icq`; query probes and checkpoint capture work, with root proof auth flow checkpoints still a coverage gap. |
| `module-structure.md` | 4 / 10 | No High/Critical structural violation found; root proof provisioning and host deployment-truth are pressure areas, not direction or facade leaks. |
| `ops-purity.md` | 3 / 10 | No ops/workflow or policy ownership violation remains; root proof provisioning split is correct after renaming the root issuer policy mapper module and tightening a broad `Principal` import. |
| `publish-surface.md` | 2 / 10 | Published package contract remains healthy across the eight-crate surface; only intentional lower-level docs thinness and special `wasm_store` packaging remain as pressure. |
| `security-boundary-ordering.md` | 3 / 10 | No ordering bypass found; the audit definition now targets current root proof provisioning, direct root query retrieval, issuer-local active proof install, and retired internal proof paths. |
| `subject-caller-binding.md` | 3 / 10 | No subject/caller break or bearer fallback found; residual risk is identity-lane and root-proof provisioning terminology pressure. |
| `token-trust-chain.md` | 3 / 10 | No trust-chain break found; audit definition was updated from stale root/shard wording to the current root/issuer canister-signature model. |
| `wasm-footprint-2.md` | 3 / 10 | Same-day rerun found zero shrunk-size drift against the baseline report; root remains the expected bundle outlier, but broad default scope no longer raises risk by itself. |
| `workflow-purity.md` | 3 / 10 | Workflow is back to orchestration ownership; root capability hash encoding moved to ops, with residual pressure in large replay/auth/cycle orchestration files. |

## Method / Comparability Notes

- `access-purity.md` uses `access-purity-current` and is non-comparable with
  the 2026-06-01 report because the live audit definition now filters
  comment-only workflow scan matches and explicitly scans the current
  delegated-session/token-use auth-state surface.
- `audience-target-binding.md` uses `audience-target-binding-current` and is
  partially comparable with the 2026-06-13 report because the delegated-token,
  role-attestation, and capability target-binding outcomes remain comparable,
  while the live audit definition now names current root issuer policy proof
  prepare checks and `ops/rpc/capability.rs` hash ownership instead of retired
  delegated-grant and stale capability paths.
- `auth-abstraction-equivalence.md` uses
  `auth-abstraction-equivalence-current` and is non-comparable with the
  2026-06-01 report because the live audit definition now targets the current
  canister/subnet/project audience model, split endpoint macro modules, and
  direct scan/test evidence after the old trust-chain guard script was
  intentionally retired.
- `bootstrap-lifecycle-symmetry.md` uses
  `bootstrap-lifecycle-symmetry/current` and is partially comparable with the
  2026-06-01 report because the lifecycle contract is unchanged, while the live
  audit definition now scans root `TimerApi::set_lifecycle_timer`, grouped
  lifecycle imports, and `crates/canic/tests` coverage explicitly.
- `capability-scope-enforcement.md` uses
  `capability-scope-enforcement-current` and is partially comparable with the
  2026-06-13 report because endpoint auth ordering and root capability
  authorization behavior remain comparable, while the live audit definition now
  targets the current structural-only capability proof path instead of retired
  delegated-grant capability proof names.
- `canic-cli-cli-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `capability-surface.md` uses `capability-surface-current` and is partially
  comparable with the 2026-05-31 report because endpoint/protocol/DTO/DID
  counts remain comparable, while the refreshed audit definition now explicitly
  scans current root proof provisioning and issuer-local delegated-token
  endpoint families.
- `canonical-auth-boundary.md` uses `canonical-auth-boundary/current` and is
  non-comparable with the 2026-06-01 report because the live audit definition
  now targets the current delegated-token, signed role-attestation, and root
  proof provisioning surfaces instead of retired internal-invocation,
  protected caller-role predicate, and role/principal audience paths.
- `canic-cli-evidence-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-evidence-support-module-surface-hardening.md` uses `MSH-2.0` and
  is non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-info-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-list-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-output-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `canic-cli-support-candid-module-surface-hardening.md` uses `MSH-2.0` and is
  non-comparable because it is the first targeted MSH run for this module.
- `change-friction.md` uses
  `change-friction-current-root-proof-provisioning` and is partially
  comparable with the 2026-06-13 report because CAF/locality, boundary
  leakage, enum shock radius, gravity-well pressure, and release-sweep
  filtering remain comparable, while the current sample targets the `0.68`
  root-proof provisioning and hygiene line rather than the earlier post-`0.65`
  auth cleanup plus host decomposition line.
- `complexity-accretion.md` uses
  `Method V4.3 / root-proof provisioning map refresh` and is partially
  comparable with the 2026-05-31 report because file, LOC, enum, and
  large-file counts remain comparable, while the live audit definition now
  recognizes `model/`, `replay_policy/`, hard-cut capability proof semantics,
  and root proof provisioning as a first-class auth lifecycle.
- `dependency-hygiene.md` uses `dependency-hygiene-current` and is
  non-comparable with the 2026-05-29 report because the live audit definition
  now requires standard recurring structural-hotspot, hub-pressure,
  fan-in-pressure, early-warning, and `Risk Score` sections.
- `dry-consolidation.md` uses
  `DRY Consolidation V6 / root-proof provisioning split` and is partially
  comparable with the 2026-06-02 report because the core CLI, host, backup,
  evidence, and release-script scans remain comparable, while the live audit
  definition now explicitly scans root proof provisioning ownership and the
  current deploy module split.
- `expiry-replay-single-use.md` uses `Method V4.4` and is non-comparable with
  the 2026-05-29 report because the live audit definition now covers the root
  proof provisioning model, active proof status, request-id replay/idempotency,
  and verifier-local bearer-token statelessness checks.
- `instruction-footprint.md` uses `Method V2` and is partially comparable with
  the 2026-06-04 report because canonical row semantics and scenario keys are
  retained, while the audit definition and generated wording now name the
  current root proof provisioning and issuer-local delegated-token surfaces.
- `module-structure.md` uses `module-structure-current` and is non-comparable
  with the 2026-05-29 report because the live audit definition now uses
  standard recurring headings and explicitly checks the root proof provisioning
  module cluster and directory-module policy.
- `ops-purity.md` uses `ops-purity/current-root-proof-provisioning` and is
  partially comparable with the 2026-06-01 report because the core ops
  invariant is unchanged, while the live audit definition now explicitly scans
  root proof provisioning split, root issuer policy mapping, and public-error
  boundary behavior.
- `publish-surface.md` uses `publish-surface-current-v2` and is comparable
  with the 2026-06-01 report. The only methodology change is additive: the
  live audit definition now names the non-versioned release package/install
  validation docs as the current entry point, with retained versioned probe
  docs as supporting inventories.
- `security-boundary-ordering.md` uses
  `security-boundary-ordering/current-root-proof-provisioning` and is
  non-comparable with the 2026-06-01 report because the live audit definition
  now targets current root proof provisioning, configured root/issuer
  canister-signature verification, direct root query retrieval, and retired
  protected internal proof paths instead of the old root/shard and internal
  invocation proof model.
- `subject-caller-binding.md` uses `subject-caller-binding-current` and is
  non-comparable with the 2026-05-29 report because the live audit definition
  now explicitly covers delegated-session identity lanes, active proof
  install/status, and root-proof provisioning principal separation.
- `token-trust-chain.md` uses `token-trust-chain-current` and is
  non-comparable with the 2026-05-29 report because the live audit definition
  now covers the configured root/issuer canister-signature trust chain,
  canonical cert/claims hashes, active proof install, and role-attestation
  proof verification instead of the old root/shard trust-chain model.
- `wasm-footprint-2.md` uses `wasm-footprint-v2` / runner `Method V2` and is
  partially comparable with the 2026-06-08 Method V1 report because release
  built/shrunk size fields remain comparable, while the report schema now
  captures `wasm-debug` built artifacts and debug-vs-release deltas.
- `workflow-purity.md` uses `workflow-purity-v3` and is partially comparable
  with the 2026-06-06 report because the core workflow-purity invariant is
  unchanged, while the live audit definition now covers root proof
  provisioning, active/pending proof records, model/DTO boundary comparison,
  and direct-query root proof retrieval invariants.

## Key Findings

- Workflow no longer carries the prior `IcpRefillRecord` / `CanisterRecord`
  production leaks, and root proof provisioning install remains
  orchestration-only through `EnvOps`, `AuthOps`, and `CallOps`.
- Root capability proof-binding Candid hash encoding moved from
  `workflow::rpc::capability` to `ops::rpc::capability`, leaving workflow with
  a delegating API wrapper for existing callers/tests.
- Stale delegated-token and role-attestation variants were removed from
  `workflow::rpc::RpcWorkflowError`; current auth provisioning errors now live
  with the delegated-auth workflow/ops surfaces.
- Access remains limited to caller/authenticated-subject resolution,
  delegated-token first-argument decoding, access expression evaluation, and
  metrics facade recording.
- Generated authenticated endpoints still route through
  `AccessContext -> eval_access -> AuthenticatedEvaluator ->
  access::auth::delegated_token_verified -> AuthOps::verify_token`.
- The live delegated-token audience shape remains `Canister`,
  `CanicSubnet`, or `Project`; stale plural role/principal audience scans found
  only historical changelog text.
- `AuthApi::verify_token_material(...)` remains private and used by delegated
  session bootstrap only, not as endpoint auth.
- The PocketIC sharding suite passed issuer-local delegated-token verification
  against a generated authenticated endpoint after active root proof install.
- Lifecycle macros still delegate pre-bootstrap runtime restoration
  synchronously, then schedule bootstrap/user work through lifecycle timers.
- Root lifecycle scheduling remains split intentionally: `canic-core` owns
  runtime restoration and `canic-control-plane` schedules root bootstrap timers.
- The lifecycle layering scan now catches grouped `ops::...` imports; only
  expected runtime env/timer/trap imports were found, with no stable-storage or
  domain-policy imports.
- `start_local!` and `start_wasm_store!` remain explicit special runtime modes;
  no active public `start_root!` macro surface was found.
- `access/auth/identity.rs` still uses only narrow `AuthStateOps`
  delegated-session read/clear calls for endpoint-boundary fallback behavior.
- Endpoint macro access lowering still authenticates, builds an access context,
  evaluates access expressions, and delegates without hiding workflow or
  topology mutation.
- The access-purity audit definition now filters comment-only workflow scan
  hits and scans delegated-session plus verifier-local token-use names
  explicitly.
- Audience-target binding still holds across delegated-token verification,
  issuer-local delegated-token preparation, root issuer policy proof-prepare
  preflight, role-attestation endpoint verification, and root capability target
  hash binding.
- The audience-target audit definition no longer names retired delegated-grant
  verifier commands or stale `api/rpc/capability/*` paths; it now tracks root
  issuer policy and `ops/rpc/capability.rs` ownership directly.
- The first PocketIC audience-target endpoint attempt was blocked by sandboxed
  local server binding, and the unsandboxed retry passed both
  role-attestation and structural capability endpoint checks.
- Capability-scope enforcement still holds across endpoint delegated-token
  verify/bind/scope ordering, delegated-token local-role scope checks, root
  structural capability proof routing, and root replay/authorization ordering.
- The capability-scope audit definition was refreshed away from stale
  delegated-grant capability proof hotspots and now lists current targeted
  unit plus PocketIC verification commands.
- Capability DTO/proof/envelope names appear in 16 Rust files across API, DTO,
  ops, workflow, tests, macros, and test canisters; this is medium fan-in
  pressure, not a current enforcement break.
- Change friction remains at 4 / 10: no direct workflow/access/API
  storage/model references or reverse ops/storage/access workflow references
  were found, while root proof provisioning remains a real cross-layer feature
  axis.
- The sampled routine `0.68` feature slices average 17 files, down from 36.20
  in the prior sample; broad `0844ddb7` and `2fb69455` hygiene/release sweeps
  are tracked separately from routine feature friction.
- `ops/auth/delegation.rs` has moved to a directory-module owner with active,
  batch, pending, root issuer policy, error, and test responsibilities split
  into focused files; this is classified as structural improvement despite
  broad sweep churn.
- The `cli/` module remains the shared command-construction boundary for
  `canic-cli` parsing, defaults, global option forwarding, and top-level help.
- The capability-surface audit definition now scans current root proof
  provisioning, issuer-local delegated-token, role-attestation, and retired
  delegation-set endpoint families.
- Retained fleet artifacts were refreshed for `app`, `user_hub`,
  `user_shard`, `scale_hub`, `scale_replica`, and `root`.
- Root proof provisioning endpoints appeared only on `root`; issuer-local
  delegated-token endpoints appeared only on `user_shard`; role-attestation
  prepare/get appeared only on `root`.
- Endpoint definitions grew from `52` to `58`, core protocol constants grew
  from `30` to `37`, and facade-only protocol constants stayed at `24`.
- Root capability RPC request/response/family variants contracted from `6` to
  `4`, and `CapabilityProof` variants contracted from `3` to `1`.
- No retained DID exposed `canic_memory_ledger`; the default memory-ledger
  diagnostic surface is cfg-gated and covered by protocol-surface tests.
- The canonical-auth-boundary audit definition was refreshed before execution
  to distinguish public delegated-token endpoint auth, explicit signed
  role-attestation verification, and root/issuer provisioning surfaces.
- Active Rust scans found no retired `verify_internal_invocation_proof`,
  `InternalInvocationProof`, `caller::has_role`, or `caller::has_any_role`
  endpoint-auth paths.
- The private `AuthApi::verify_token_material(...)` helper remains unavailable
  as public endpoint authorization, and no `AuthApi::verify_token`-style
  endpoint-auth shortcut was found.
- Authenticated endpoint macro validation still requires a first
  `DelegatedToken` argument, and expansion still evaluates access before
  handler dispatch.
- Signed role-attestation verification passed unit and PocketIC proof/rejection
  tests, while root proof provisioning endpoints remain controller/root or
  issuer-local operational surfaces.
- No ICP, DFX, deployment mutation, stable storage, backup/recovery, wasm, or
  generated-boundary authority was found in the inspected module.
- The `render_help` wrapper had one caller and was inlined into `render_usage`.
- The positive-number parser helpers are retained because they are consumed as
  clap `ValueParser` function items by cycles, metrics, and restore options.
- The `evidence.rs` dispatcher now delegates leaf execution to focused
  `run_compare` and `run_gate` helpers while preserving report-before-error
  behavior.
- The `evidence_support.rs` helper remains a passive command-provenance adapter.
- `push_optional_path_arg` now takes `Option<&Path>` and its focused tests cover
  absent optional paths plus outside-root redaction.
- The `info.rs` dispatcher no longer reclassifies already-validated clap
  subcommands in `parse_info_command`; dispatch remains in `run`.
- The `list/` module remains the read-only `canic info list` and
  `canic fleet config` boundary. It retains internal parsing, config loading,
  live registry observation, and table rendering helpers with owner; no
  high-confidence cleanup was found.
- The `output/` module remains a passive file/stdout IO helper boundary.
- Plain relative output filenames now skip empty parent-directory creation,
  while nested output paths still create parents.
- Output helper visibility was left as `pub` because the parent module is
  private and clippy treats `pub(crate)` as redundant there.
- The `support/candid.rs` module remains a passive CLI adapter over host-owned
  local Candid sidecar discovery.
- Focused support Candid tests now pin existing-sidecar lookup, absent-root
  handling, and missing-registry-role handling.
- The complexity-accretion audit definition was refreshed before execution so
  current root-proof provisioning, active proof install/status, and hard-cut
  capability proof semantics are measured instead of stale proof-mode axes.
- Root capability RPC variants contracted from `6` to `4`, and
  `CapabilityProof` contracted from `3` to `1`.
- Total `canic-core` runtime files grew from `448` to `486`, runtime logical
  LOC grew from `45126` to `56641`, and non-test files above `600` LOC grew
  from `0` to `6`.
- `ops/auth/delegation/mod.rs` was reduced from `810` logical LOC to a
  `70`-LOC facade over focused `active`, `batch`, `pending`, `policy`, and
  `errors` modules.
- The complexity-accretion result is low residual risk after cleanup and is not
  a current 0.68 MVP blocker.
- The dependency-hygiene run found no High or Critical package-boundary
  violation.
- `canic` remains the only broad public facade; `default = ["metrics"]` still
  excludes control-plane, sharding, and delegated-auth proof features.
- `canic-core` canister-signature creation, certification, and verification
  dependencies remain optional and auth-feature-gated.
- `canic-cli`, `canic-host`, and `canic-backup` keep one-way operator package
  direction without a `canic-cli -> canic` facade edge.
- Auth/control-plane-enabled fixture and fleet canisters remain
  `publish = false`.
- The dry-consolidation audit definition now scans root proof provisioning
  prepare/get/install, active proof status, pending proof metadata, install
  outcome, and verifier configuration ownership.
- The old `deploy/mod.rs` DRY hotspot is materially resolved: it is now a
  focused facade, deploy production submodules are under 500 lines in the
  sampled inventory, and deploy output-format parsing has a local owner.
- Evidence envelope assembly remains command-specific for deployment check,
  fleet adoption, and evidence gate, while stable DTO/schema/hash ownership
  remains centralized in `canic-host::evidence_envelope`.
- Backup/snapshot registry traversal remains command-specific, but registry
  parsing and ICP registry query transport still have host-owned helpers.
- Root proof provisioning has distinct owners for endpoint guards, workflow
  broadcast, ops metadata/proof helpers, stable records, replay policy, and DTO
  boundary shapes; no duplicate lifecycle owner was found.
- The instruction-footprint audit definition was refreshed before execution so
  root proof provisioning and issuer-local delegated-token auth are first-class
  runtime measurement surfaces.
- The canonical PocketIC instruction runner passed after reinstalling pinned
  `icq 0.2.23`; the first attempt failed with local `icq 0.2.26`.
- Query probe rows remain visible through same-call `QueryPerfSample` probes,
  and checkpoint capture recorded four non-zero template-publication
  checkpoint deltas.
- The highest sampled endpoint remains `app:canic_log:empty-page` at
  `297827` average local instructions; root proof batch prepare, root
  capability cycles, and template admin update rows remained endpoint-zero in
  the persisted update-row matrix.
- Root proof provisioning and issuer delegated-token issuance/verification are
  now explicitly tracked as a checkpoint coverage gap under
  `workflow/runtime/auth`.
- Delegated-token verification remains TTL-bounded and verifier-stateless; no
  stale direct `now > expires_at` boundary checks were found in core freshness
  paths.
- Active root proof install/status checks cover not-yet-valid, expired,
  valid, refresh-needed, and expired-status behavior.
- Root proof batch prepare is request-id and fingerprint protected; conflicting
  request-id reuse rejects, while same-request replay returns cached metadata.
- Root proof batch get/install reject expired pending metadata, proof
  mismatches, and stale pending metadata.
- Replay policy inventory, root replay capacity ordering, root replay expiry
  boundary, and capability replay metadata expiry/skew checks all passed.
- The module-structure run found no High or Critical structural violation.
- Root proof provisioning direction remains macro endpoint -> API -> ops or
  workflow -> ops/storage, with DTOs remaining passive boundary data.
- `ops/auth/delegation/mod.rs` and `dto/auth.rs` are the main current root
  proof provisioning structural pressure points, but neither is a
  public/internal seam leak.
- The root proof delegation ops module now uses directory-module layout, with
  the production owner in `ops/auth/delegation/mod.rs` and its focused tests in
  `ops/auth/delegation/tests.rs`.
- Module layout checks found no `foo.rs` plus `foo/mod.rs` duplicates and no
  production `#[path = "..."]` module-layout escapes.
- `canic-host::deployment_truth` remains a broad public host support surface,
  but it does not leak canister-runtime internals through the `canic` facade.
- Generic `ic-testkit` remains Canic-free; Canic-specific PocketIC helpers stay
  in unpublished `canic-testing-internal`.
- The ops-purity audit definition was refreshed before execution so current
  root proof provisioning and public-error boundary behavior are scanned
  directly.
- No production `canic-core/src/ops` code imports workflow.
- `ops/auth/delegation/policy.rs` was renamed to
  `ops/auth/delegation/root_issuer_policy.rs`; the module maps boundary and
  storage shapes while pure issuer policy decisions stay in `domain/policy/auth`.
- `ops/topology/index/builder.rs` now imports `Principal` through the runtime
  type facade instead of the broader Candid path.
- Root proof broadcast remains in `workflow/runtime/auth/provisioning`; ops
  owns bounded batch metadata/proof operations and issuer-local active proof
  verification/storage.
- The ops public-error scan found only accepted hotspots: remote RPC wire-error
  preservation and the typed root data-certificate unavailable protocol error.
- The publish-surface audit definition now names the non-versioned release
  package/install validation docs as the current entry point, while retaining
  the versioned probe docs as supporting inventories.
- The published crate count remains eight: `canic`, `canic-backup`,
  `canic-cli`, `canic-control-plane`, `canic-core`, `canic-host`,
  `canic-macros`, and `canic-wasm-store`.
- All eight published crates retain package-local README posture, docs.rs
  metadata, repository/homepage metadata, and inherited `rust-version = 1.91.0`.
- `canic` remains the main facade and its default feature set remains the
  documented small `metrics` default.
- `canic-cli` remains the installed `canic` operator binary surface, while
  `canic-host` and `canic-backup` remain role-specific support crates.
- `canic-wasm-store` still packages as a special `cdylib` canister artifact
  source and does not expose an ordinary reusable `rlib` dependency surface.
- Installed/packaged proof scripts still create package or install roots first
  and guard against repository crate paths and `target/debug/canic` shortcuts.
- `cargo package` packaged and verified all eight published crates on the
  current dirty worktree with `--allow-dirty`.
- The security-boundary-ordering audit definition was refreshed away from
  stale protected internal proof wrapper and root/shard wording.
- Endpoint delegated-token auth still verifies token material, binds the
  verified subject to the caller, and enforces required scope before dispatch.
- Access auth scans found no verifier-local delegated-token use store or
  consume/update path.
- Generated endpoint macros still evaluate access before dispatch, and active
  Rust scans found no retired protected internal proof or caller-role predicate
  paths.
- Root proof provisioning now follows the intended order: root prepare update,
  direct root get query, root install update, issuer-local active proof
  verification/storage, then issuer-local delegated-token prepare/get.
- Missing root query data certificates map to the typed
  `RootDataCertificateUnavailable` public error.
- Root batch install preflights submitted proof material against pending
  metadata before issuer calls and does not assemble proofs in the install
  update.
- Root replay-first capability handling still aborts fresh replay reservations
  on authorization or execution failure and commits only after successful
  execution.
- The subject-caller-binding audit separately confirmed that delegated-token
  endpoint auth keeps raw transport caller and authenticated-subject lanes
  distinct.
- Endpoint macro expansion still builds `AccessContext` from
  `resolve_authenticated_identity(...)`, preserving raw transport caller and
  authenticated-subject lanes.
- Caller/topology predicates still use the raw transport caller, while
  authenticated predicates use the resolved authenticated subject.
- `AuthApi::verify_token_material(...)` remains private and documented as
  incomplete for endpoint authorization without caller binding.
- Root proof provisioning principals such as `issuer_pid` and `installed_by`
  remain issuer/provisioning authority terms, not delegated-token end-user
  auth subjects.
- The token-trust-chain audit definition was refreshed away from stale
  root/shard signature and delegated root-key helper wording.
- Delegated-token verification remains rooted in explicit
  `AuthProofVerifierConfig` trust anchors: configured root canister id plus raw
  IC root public key.
- Root canister-signature proofs, issuer canister-signature proofs, canonical
  cert hashes, canonical claims hashes, and issuer proof binding hashes are
  all recomputed or verified before token acceptance.
- Positive-cache hits still rerun local canonical, audience, grant, subject,
  and scope checks.
- Active proof install verifies root proof material and local issuer canister
  binding before storing active proof state.
- PocketIC root batch provisioning and role-attestation proof verification
  paths passed.
- The wasm-footprint runner/report now captures `wasm-debug` built artifacts
  during normal release audit runs and records debug-vs-release deltas in both
  Markdown and JSON artifacts.
- The 2026-06-19 release shrunk sizes are `app` 2996944,
  `user_hub` 3154906, `user_shard` 3095441, `scale_hub` 3028777,
  `scale_replica` 3006586, and `root` 4914820 bytes.
- The wasm-footprint rerun is low residual risk at 3 / 10; same-day baseline
  deltas are `+0` for every canister, and `root` remains a tracked bundle
  outlier rather than a leaf-canister peer.

## Verification Readout Rollup

| Report | PASS | FAIL | BLOCKED |
| ---- | ----: | ----: | ----: |
| `access-purity.md` | 9 | 0 | 0 |
| `audience-target-binding.md` | 10 | 0 | 0 |
| `auth-abstraction-equivalence.md` | 18 | 0 | 0 |
| `bootstrap-lifecycle-symmetry.md` | 15 | 0 | 0 |
| `capability-scope-enforcement.md` | 10 | 0 | 0 |
| `capability-surface.md` | 11 | 0 | 0 |
| `canonical-auth-boundary.md` | 21 | 0 | 0 |
| `canic-cli-cli-module-surface-hardening.md` | 4 | 0 | 0 |
| `canic-cli-evidence-module-surface-hardening.md` | 4 | 0 | 0 |
| `canic-cli-evidence-support-module-surface-hardening.md` | 6 | 0 | 0 |
| `canic-cli-info-module-surface-hardening.md` | 4 | 0 | 0 |
| `canic-cli-list-module-surface-hardening.md` | 6 | 0 | 0 |
| `canic-cli-output-module-surface-hardening.md` | 4 | 0 | 0 |
| `canic-cli-support-candid-module-surface-hardening.md` | 5 | 0 | 0 |
| `change-friction.md` | 8 | 0 | 0 |
| `dependency-hygiene.md` | 19 | 0 | 0 |
| `dry-consolidation.md` | 15 | 0 | 0 |
| `expiry-replay-single-use.md` | 15 | 0 | 0 |
| `instruction-footprint.md` | 10 | 0 | 0 |
| `module-structure.md` | 19 | 0 | 0 |
| `ops-purity.md` | 14 | 0 | 0 |
| `publish-surface.md` | 12 | 0 | 0 |
| `security-boundary-ordering.md` | 14 | 0 | 0 |
| `subject-caller-binding.md` | 21 | 0 | 0 |
| `token-trust-chain.md` | 16 | 0 | 0 |
| `wasm-footprint-2.md` | 5 | 0 | 0 |

## Follow-up Actions

- Continue the CLI tree with the next focused module, avoiding backup/restore
  recovery surfaces unless a Tier 2 pass is explicitly desired.
- Defer broad `&Path` cleanup around restore IO to a dedicated restore/io pass,
  because it crosses into backup/recovery-adjacent authority.
- Keep `canic` defaults narrow and keep auth/control-plane-enabled fixture
  canisters unpublished.
- Keep DRY cleanup domain-first. Revisit evidence envelope helpers only if
  another emitter appears or two emitters converge on the same
  output/fingerprint behavior.
- Keep the instruction-footprint runner pinned to `icq 0.2.23` until
  `tool-versions.env` changes, and add first auth-flow `perf!` checkpoints
  before treating root proof provisioning as fully stage-attributed.
- Keep release package/install validation docs non-versioned as the current
  entry point; use retained versioned probe docs only as supporting inventories.
- Monitor `ops/auth/delegation/mod.rs` and `dto/auth.rs` fan-in while the root
  proof provisioning slice stabilizes.
- Clean up the non-fatal delegated-auth metrics lint-expectation warnings in a
  focused lint/hygiene pass if they still reproduce under clippy `-D warnings`.
- Keep root proof provisioning DTO/API/ops/workflow responsibilities separated
  while that slice stabilizes.
- Re-run ops-purity if root proof provisioning grows broadcast, retry,
  scheduler, or provisioning-loop behavior near `ops/auth/delegation`.
- Keep endpoint macros thin and avoid moving proof lifecycle policy into macro
  emission code.
- Keep capability DTOs passive and preserve explicit token verify, subject
  binding, scope enforcement, capability proof validation, replay reservation,
  authorization, execution, and replay commit ordering when root capability
  behavior changes.
- Treat future root proof provisioning behavior changes as coordinated
  cross-layer slices, and keep broad code-hygiene/release sweeps separated from
  routine feature-friction trend metrics.
- Keep auth abstraction reports using direct scans/tests now that the old
  trust-chain guard shell script is intentionally retired.
- Add a PocketIC nested issuer-to-root retrieval negative test if the root
  proof retrieval surface changes again, so failure is pinned to the
  data-certificate/direct-query invariant rather than incidental ACL setup.
- Keep tracking `root` separately from leaf canisters in wasm-footprint runs so
  bundle behavior does not get conflated with shared runtime floor drift.
