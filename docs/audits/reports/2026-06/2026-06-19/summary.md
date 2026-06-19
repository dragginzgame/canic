# Audit Summary - 2026-06-19

## Run Contexts

| Report | Type | Scope | Status |
| ---- | ---- | ---- | ---- |
| `access-purity.md` | Recurring system | access auth/session/token predicates, access expression evaluation, access metrics facade, endpoint macro access lowering | PASS |
| `auth-abstraction-equivalence.md` | Recurring invariant | macro-generated authenticated endpoint expansion, access-expression dispatch, delegated-token verifier parity, delegated-session identity lanes, canister/subnet/project audience binding, root-proof provisioning integration | PASS |
| `bootstrap-lifecycle-symmetry.md` | Recurring system | start macros, lifecycle API wrappers, init/post-upgrade adapters, root control-plane lifecycle scheduling, runtime continuation, lifecycle boundary tests | PASS |
| `capability-surface.md` | Recurring system | endpoint macro bundles, retained-fleet DID surface, protocol constants, RPC/capability DTO variants, root proof provisioning endpoints, issuer-local delegated-token endpoints | PASS |
| `canonical-auth-boundary.md` | Recurring invariant | macro-generated authenticated endpoint expansion, access-expression dispatch, delegated-token endpoint verification, private token-material helper boundaries, signed role-attestation verification, root proof provisioning endpoints, issuer-local delegated-token issuance surfaces | PASS |
| `canic-cli-cli-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/cli/` | PASS |
| `canic-cli-evidence-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/evidence.rs` | PASS |
| `canic-cli-evidence-support-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/evidence_support.rs` | PASS |
| `canic-cli-info-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/info.rs` | PASS |
| `canic-cli-list-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/list/` | PASS |
| `canic-cli-output-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/output/` | PASS |
| `canic-cli-support-candid-module-surface-hardening.md` | Modular MSH | `crates/canic-cli/src/support/candid.rs` | PASS |
| `dependency-hygiene.md` | Recurring system | workspace Cargo manifests, public/support crates, internal fixtures, fleets | PASS |
| `expiry-replay-single-use.md` | Recurring invariant | delegated-token freshness, active proof install/status, root proof batch replay/expiry, replay policy, root replay | PASS |
| `module-structure.md` | Recurring system | public crate roots, canic-core layers, root proof provisioning cluster, host/test seams, module layout | PASS |
| `subject-caller-binding.md` | Recurring invariant | delegated-token endpoint subject binding, identity lanes, endpoint macro access expansion, delegated-session resolution, root-proof provisioning principal separation | PASS |
| `token-trust-chain.md` | Recurring invariant | delegated-token root/issuer trust chain, configured root canister/root-key verifier config, canonical cert/claims hashes, active proof install, role-attestation proof verification | PASS |

## Risk Index Summary

| Report | Risk | Notes |
| ---- | ----: | ---- |
| `access-purity.md` | 2 / 10 | Access remains a thin endpoint boundary; the only cleanup was refreshing the audit definition's workflow and auth-state scans. |
| `auth-abstraction-equivalence.md` | 3 / 10 | Generated authenticated endpoints still converge through `AccessContext`, `eval_access`, `delegated_token_verified`, and `AuthOps::verify_token`; residual risk is expected auth/provisioning hotspot pressure. |
| `bootstrap-lifecycle-symmetry.md` | 2 / 10 | Lifecycle hooks remain thin restore-and-schedule adapters; audit definition was tightened around root timer scheduling and grouped lifecycle imports. |
| `capability-surface.md` | 4 / 10 | No hard scoping failure found; endpoint definitions grew through root proof provisioning and issuer-local auth while default memory-ledger DID surface and old RPC/proof variants contracted. |
| `canonical-auth-boundary.md` | 3 / 10 | Authenticated endpoints still converge through macro access lowering, `eval_access`, `delegated_token_verified`, and `AuthOps::verify_token`; residual risk is auth/provisioning boundary adjacency. |
| `canic-cli-cli-module-surface-hardening.md` | 2 / 10 | Internal CLI helper surface is retained with owner; one one-caller help-rendering wrapper was inlined. |
| `canic-cli-evidence-module-surface-hardening.md` | 3 / 10 | Evidence dispatcher is retained with owner; compare/gate execution bodies were moved into focused helpers. |
| `canic-cli-evidence-support-module-surface-hardening.md` | 2 / 10 | Evidence command-provenance helper is retained with owner; optional path input was narrowed to `Option<&Path>`. |
| `canic-cli-info-module-surface-hardening.md` | 2 / 10 | Read-only info dispatcher is retained with owner; duplicate subcommand remapping was removed. |
| `canic-cli-list-module-surface-hardening.md` | 3 / 10 | Read-only deployment registry and fleet config listing is retained with owner; no high-confidence cleanup was found. |
| `canic-cli-output-module-surface-hardening.md` | 2 / 10 | Shared CLI output helpers are retained with owner; one plain-filename parent-directory edge case was hardened. |
| `canic-cli-support-candid-module-surface-hardening.md` | 1 / 10 | Local Candid sidecar helper is retained with owner; focused tests now cover existing sidecars, absent roots, and missing registry roles. |
| `dependency-hygiene.md` | 2 / 10 | Published crates still avoid unpublished runtime edges; `canic` defaults remain narrow; auth/control-plane/sharding features remain explicit. |
| `expiry-replay-single-use.md` | 3 / 10 | No expiry/replay break found; root proof batch prepare/get/install and active proof freshness are covered, with residual fan-in pressure around delegated auth and provisioning DTOs. |
| `module-structure.md` | 4 / 10 | No High/Critical structural violation found; root proof provisioning and host deployment-truth are pressure areas, not direction or facade leaks. |
| `subject-caller-binding.md` | 3 / 10 | No subject/caller break or bearer fallback found; residual risk is identity-lane and root-proof provisioning terminology pressure. |
| `token-trust-chain.md` | 3 / 10 | No trust-chain break found; audit definition was updated from stale root/shard wording to the current root/issuer canister-signature model. |

## Method / Comparability Notes

- `access-purity.md` uses `access-purity-current` and is non-comparable with
  the 2026-06-01 report because the live audit definition now filters
  comment-only workflow scan matches and explicitly scans the current
  delegated-session/token-use auth-state surface.
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
- `dependency-hygiene.md` uses `dependency-hygiene-current` and is
  non-comparable with the 2026-05-29 report because the live audit definition
  now requires standard recurring structural-hotspot, hub-pressure,
  fan-in-pressure, early-warning, and `Risk Score` sections.
- `expiry-replay-single-use.md` uses `Method V4.4` and is non-comparable with
  the 2026-05-29 report because the live audit definition now covers the root
  proof provisioning model, active proof status, request-id replay/idempotency,
  and verifier-local bearer-token statelessness checks.
- `module-structure.md` uses `module-structure-current` and is non-comparable
  with the 2026-05-29 report because the live audit definition now uses
  standard recurring headings and explicitly checks the root proof provisioning
  module cluster and directory-module policy.
- `subject-caller-binding.md` uses `subject-caller-binding-current` and is
  non-comparable with the 2026-05-29 report because the live audit definition
  now explicitly covers delegated-session identity lanes, active proof
  install/status, and root-proof provisioning principal separation.
- `token-trust-chain.md` uses `token-trust-chain-current` and is
  non-comparable with the 2026-05-29 report because the live audit definition
  now covers the configured root/issuer canister-signature trust chain,
  canonical cert/claims hashes, active proof install, and role-attestation
  proof verification instead of the old root/shard trust-chain model.

## Key Findings

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
- Delegated-token endpoint auth still verifies token material, binds
  `VerifiedDelegatedToken.subject` to the authenticated subject, and then
  enforces required scope.
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

## Verification Readout Rollup

| Report | PASS | FAIL | BLOCKED |
| ---- | ----: | ----: | ----: |
| `access-purity.md` | 9 | 0 | 0 |
| `auth-abstraction-equivalence.md` | 18 | 0 | 0 |
| `bootstrap-lifecycle-symmetry.md` | 15 | 0 | 0 |
| `capability-surface.md` | 11 | 0 | 0 |
| `canonical-auth-boundary.md` | 21 | 0 | 0 |
| `canic-cli-cli-module-surface-hardening.md` | 4 | 0 | 0 |
| `canic-cli-evidence-module-surface-hardening.md` | 4 | 0 | 0 |
| `canic-cli-evidence-support-module-surface-hardening.md` | 6 | 0 | 0 |
| `canic-cli-info-module-surface-hardening.md` | 4 | 0 | 0 |
| `canic-cli-list-module-surface-hardening.md` | 6 | 0 | 0 |
| `canic-cli-output-module-surface-hardening.md` | 4 | 0 | 0 |
| `canic-cli-support-candid-module-surface-hardening.md` | 5 | 0 | 0 |
| `dependency-hygiene.md` | 19 | 0 | 0 |
| `expiry-replay-single-use.md` | 15 | 0 | 0 |
| `module-structure.md` | 19 | 0 | 0 |
| `subject-caller-binding.md` | 21 | 0 | 0 |
| `token-trust-chain.md` | 16 | 0 | 0 |

## Follow-up Actions

- Continue the CLI tree with the next focused module, avoiding backup/restore
  recovery surfaces unless a Tier 2 pass is explicitly desired.
- Defer broad `&Path` cleanup around restore IO to a dedicated restore/io pass,
  because it crosses into backup/recovery-adjacent authority.
- Keep `canic` defaults narrow and keep auth/control-plane-enabled fixture
  canisters unpublished.
- Monitor `ops/auth/delegation/mod.rs` and `dto/auth.rs` fan-in while the root
  proof provisioning slice stabilizes.
- Clean up the non-fatal delegated-auth metrics lint-expectation warnings in a
  focused lint/hygiene pass if they still reproduce under clippy `-D warnings`.
- Keep root proof provisioning DTO/API/ops/workflow responsibilities separated
  while that slice stabilizes.
- Keep endpoint macros thin and avoid moving proof lifecycle policy into macro
  emission code.
- Keep auth abstraction reports using direct scans/tests now that the old
  trust-chain guard shell script is intentionally retired.
