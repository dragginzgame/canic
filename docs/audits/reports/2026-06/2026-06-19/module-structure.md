# Module Structure Audit - 2026-06-19

## Report Preamble

- Definition path: `docs/audits/recurring/system/module-structure.md`
- Scope: `crates/canic`, `crates/canic-backup`, `crates/canic-cli`,
  `crates/canic-core`, `crates/canic-control-plane`, `crates/canic-host`,
  `crates/canic-macros`, `crates/canic-wasm-store`,
  `crates/canic-testing-internal`, `crates/canic-tests`, sibling
  `../ic-testkit`, `fleets/**`, `canisters/test/**`, `canisters/audit/**`,
  and `canisters/sandbox/**`.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-29/module-structure.md`
- Code snapshot identifier: `16894709` with dirty worktree.
- Method tag/version: `module-structure-current`.
- Comparability status: `non-comparable`: the live audit definition now uses
  standard recurring headings and explicitly checks the post-0.68 root proof
  provisioning module cluster and directory-module policy. The old host/CLI
  pressure baseline is also structurally stale because current local sources
  show split deployment command modules.
- Exclusions applied: generated target outputs, `.icp` runtime cache,
  historical audit reports outside the compared baseline, broad style-only code
  hygiene, and unrelated dirty Rust edits outside the inspected structural
  seams.
- Notable methodology changes vs baseline: added focused inspection of root
  proof provisioning DTO/API/ops/workflow/macro ownership, added standard
  `Structural Hotspots`, `Hub Module Pressure`, `Dependency Fan-In Pressure`,
  `Early Warning Signals`, and `Risk Score` sections, and retained directory
  module plus `#[path]` checks.
- Auditor: `codex`.
- Run timestamp: `2026-06-19`.
- Worktree: `dirty`; unrelated source edits were left untouched.

Verification status: **PASS**.

No High or Critical structural violation was confirmed. The current risk is
bounded coordination pressure in the root proof provisioning cluster and host
deployment-truth public support surface, not a dependency-direction breach,
public/internal seam leak, or module-discovery problem.

## Structural Hotspots

| File / Module | Exposed or Coupled Item | Visibility Scope | Direction / Exposure Impact | Risk |
| --- | --- | --- | --- | --- |
| `crates/canic-core/src/dto/auth.rs` | `RootDelegationProofBatch*`, `RootIssuerPolicy*`, `ActiveDelegationProof*`, `AuthRequestMetadata` | `pub` DTOs under public `canic_core::dto` and facade DTO reachability | Passive boundary data for Candid/protocol contracts; no side-effect or policy ownership found. | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | `AuthApi::{prepare,get,install}_delegation_proof_batch_root`, active proof install/status, issuer policy upsert | public API methods on public `AuthApi` | API boundary maps endpoint DTOs into ops/workflow and root/issuer environment guards; no storage schema ownership found. | Medium |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | pending batch metadata, replay/idempotency helpers, active proof state helpers | `pub(crate)` methods on crate-private ops root | Correct owner for deterministic proof metadata/state transitions; large hub with recent churn. | Medium |
| `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs` | `install_delegation_proof_batch_root` | crate-private workflow method | Orchestrates root broadcast install and calls ops preflight; does not own proof verification or storage schema. | Low |
| `crates/canic/src/macros/endpoints/root.rs` | root proof batch endpoint emission | exported macros | Thin endpoint surface delegates to `AuthApi`; controller guard remains at endpoint layer. | Low |
| `crates/canic/src/macros/endpoints/nonroot.rs` | active proof install/status endpoint emission | exported macros | Thin issuer endpoint surface delegates to `AuthApi`; install is controller-gated, status is query. | Low |
| `crates/canic-host/src/deployment_truth/mod.rs` | broad `pub use` support surface | `pub mod deployment_truth`, many `pub use` re-exports | Host-owned deployment support remains a broad public support API; no canister runtime facade leak confirmed. | Medium |
| `crates/canic-core/src/access/expr/mod.rs` | endpoint access expression model | public through hidden/core access support | Central auth expression model remains policy/access-owned; no storage/workflow dependency found. | Low |
| `../ic-testkit/crates/ic-testkit/src/pic/mod.rs` | generic PocketIC helpers | public generic support crate | Canic-free test infrastructure; Canic-specific helpers stay in `canic-testing-internal`. | Low |

## Public Surface Map

| Item | Kind | Path | Publicly Reachable From Root? | Classification | Visibility Scope | Exposure Impact | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Canic facade modules | module family | `crates/canic/src/lib.rs` (`access`, `api`, `dto`, `ids`, `prelude`, `protocol`) | yes | intended external API | `pub mod` | primary user facade remains the broad public surface. | Low |
| Canic hidden macro/build support | module family | `crates/canic/src/lib.rs` (`__internal`, `__build`) | yes, hidden | macro/build support | `#[doc(hidden)] pub mod` | required for macro expansion and build scripts; not normal user API. | Low |
| Core public and hidden roots | module family | `crates/canic-core/src/lib.rs` | yes | lower-level support API | mixed `pub mod`, `#[doc(hidden)] pub mod`, `pub(crate) mod` | DTO/API/ID/CDK/memory/protocol/replay-policy roots remain public; execution/storage/workflow roots remain crate-private. | Medium |
| Root proof provisioning DTOs | DTO family | `crates/canic-core/src/dto/auth.rs` | yes through `canic_core::dto` and facade DTO paths | stable protocol DTOs | `pub struct` / `pub enum` with public fields | required protocol/Candid boundary shapes; inspected context shows no execution ownership. | Medium |
| Root proof provisioning endpoint macros | macro family | `crates/canic/src/macros/endpoints/root.rs`, `nonroot.rs` | yes through exported macros | public endpoint emission | `#[macro_export]` | endpoint wrappers call API methods directly and keep guards at the endpoint boundary. | Low |
| Host deployment truth | support module family | `crates/canic-host/src/deployment_truth/mod.rs` | yes through `canic_host::deployment_truth` | host/operator support API | `pub mod` plus broad `pub use` | still broad but role-owned; not an alternate canister-runtime facade. | Medium |
| CLI deploy command module | binary support module | `crates/canic-cli/src/deploy/mod.rs` | no external command submodules; public library entry remains compact | CLI dispatch/support | private modules plus narrow `pub use`/`pub fn` | current file is a dispatcher, not the old large deployment hub. | Low |
| Generic testkit | support module family | `../ic-testkit/crates/ic-testkit/src/lib.rs`, `pic/mod.rs` | yes | public generic test infrastructure | `pub mod`, `pub use`, `pub struct`, `pub fn` | no Canic dependency or Canic-specific semantics found in the generic testkit. | Low |

## Subsystem Dependency Graph

| Subsystem / Crate | Depends On | Depended On By | Direction Assessment | Risk |
| --- | --- | --- | --- | --- |
| `canic` | `canic-core`, `canic-control-plane` behind feature, `canic-macros` | fleet/test/audit/sandbox canisters, examples, tests | facade direction remains clean; hidden roots are macro/build plumbing. | Low |
| `canic-core` | IC/CDK/storage/memory/runtime dependencies | `canic`, `canic-control-plane`, `canic-host`, tests | public roots expose API/DTO/support; ops/storage/workflow internals remain crate-private. | Low |
| `canic-control-plane` | `canic-core` and support crates | `canic` control-plane feature and root/store canisters | control-plane runtime support stays below the facade. | Low |
| `canic-host` | `canic-core`, serialization/filesystem/process support | `canic-cli` | host/operator support remains facade-free; deployment-truth surface is broad but host-owned. | Medium |
| `canic-cli` | `canic-core`, `canic-host`, `canic-backup` | binary entrypoint | CLI owns UX and dispatch; deploy module is no longer a large monolith in current sources. | Low |
| `canic-backup` | serialization/hash/time support | `canic-cli` | backup remains independent from canister runtime/facade crates. | Low |
| `canic-macros` | `syn`, `quote`, `proc_macro2` | `canic` | proc macro crate has no runtime-internal dependency. | Low |
| `canic-wasm-store` | `canic` runtime facade | installed as special canister | runtime artifact only; no Rust library reuse surface. | Low |
| `canic-testing-internal` | `ic-testkit`, `canic`, `canic-core`, `canic-control-plane` | `canic-tests` | repo-only test harness remains one-way and unpublished. | Low |
| `../ic-testkit` | `pocket-ic`, `candid`, generic support crates | Canic internal tests and downstreams | generic testkit does not encode Canic runtime semantics. | Low |

## Circularity Findings

| Subsystem A | Subsystem B | Real Cycle? | Evidence | Risk |
| --- | --- | --- | --- | --- |
| `canic` | `canic-core` | no | `canic` depends on `canic-core`; `canic-core` does not depend on `canic`. | Low |
| `canic-host` | `canic-cli` | no | CLI depends on host; host has no CLI dependency. | Low |
| `canic-backup` | `canic-cli` | no | CLI depends on backup; backup has no CLI dependency. | Low |
| `canic-testing-internal` | `../ic-testkit` | no | internal harness depends on `ic-testkit`; `ic-testkit` has no Canic references. | Low |
| root proof provisioning DTO/API/ops/workflow/macro cluster | multiple layers | no | direction is macro endpoint -> API -> ops/workflow -> ops/storage, with DTO as passive boundary data. | Low |

## Visibility Hygiene Findings

| Item | Path | Current Visibility | Narrowest Plausible Visibility | Why Narrower Seems Valid / Invalid | Risk |
| --- | --- | --- | --- | --- | --- |
| root proof provisioning DTOs | `crates/canic-core/src/dto/auth.rs` | `pub` | keep current | protocol/Candid tests pin these boundary shapes; public fields are DTO contract fields, not storage internals. | Medium |
| root proof provisioning ops methods | `crates/canic-core/src/ops/auth/delegation/mod.rs` | `pub(crate)` on crate-private ops root | keep current | API and workflow both need crate-local access; not root-public through `canic`. | Medium |
| root proof provisioning endpoint macros | `crates/canic/src/macros/endpoints/root.rs`, `nonroot.rs` | exported macros | keep current | macro API owns endpoint emission and delegates immediately to API methods. | Low |
| host deployment truth re-export surface | `crates/canic-host/src/deployment_truth/mod.rs` | `pub use` from private submodules | keep current until support contract is split further | host support API is broad but role-aligned and not exposed through `canic`. | Medium |
| CLI deploy command internals | `crates/canic-cli/src/deploy/mod.rs` | private submodules plus narrow public helpers | keep current | current dispatcher delegates by command family; old monolith pressure has been reduced. | Low |
| core execution roots | `crates/canic-core/src/lib.rs` (`config`, `domain`, `infra`, `lifecycle`, `model`, `ops`, `storage`, `view`, `workflow`) | `pub(crate) mod` | keep current | execution/storage/workflow internals are not root-public. | Low |

### Test Leakage

| Item | Location | Leakage Type | Build Impact | Risk |
| --- | --- | --- | --- | --- |
| generic testkit use in runtime/product crates | product crates scan | none found | `ic-testkit` appeared only in a test canister dev dependency and a manifest test reference. | Low |
| Canic-specific PocketIC helpers | `crates/canic-testing-internal/src/pic/mod.rs` | repo-only test harness | `publish = false`; no public product dependency found. | Low |
| root proof provisioning protocol tests | `crates/canic/tests/protocol_surface.rs` | public protocol pinning | integration test surface only. | Low |
| instruction audit support references | `crates/canic-tests/tests/instruction_audit_support/**` | test-only reporting | no runtime/fleet dependency impact. | Low |

## Layering Violations

| Layer / Rule | Upward Dependency Found? | Description | Risk |
| --- | --- | --- | --- |
| `storage` must not depend on workflow/policy/ops | no | scan found storage-local preludes and stable auth records only. | Low |
| `ops` may depend on storage but not workflow | no production breach | matches are ops-local references, ops-to-storage access, metrics, or tests; no workflow dependency was confirmed. | Low |
| `domain`/policy must not mutate or call runtime side effects | no | `domain/policy/auth/root_provisioning.rs` imports `Principal` and `CanisterRole`, validates inputs, and returns decisions without ops/storage/runtime calls. | Low |
| `workflow` should use ops instead of storage internals | no high-confidence breach | one existing workflow reference to `crate::ops::storage::state::app::AppStateOps::cycles_funding_enabled()` is ops-mediated state access, not storage bypass. | Low |
| DTOs must remain passive | no | root proof provisioning DTOs define Candid/Serde shapes and public fields only; no mutation, async, storage, or runtime calls were found in inspected context. | Low |
| endpoint macros must marshal/delegate | no | root and non-root proof macros call `AuthApi` methods directly and keep guard declarations at endpoint emission. | Low |
| `../ic-testkit` must not encode Canic runtime semantics | no | no Canic references found in the generic sibling testkit source or manifest. | Low |

## Structural Pressure Areas

| Area | Pressure Type | Why This Is Pressure, Not Yet Violation | Drift Sensitivity | Risk |
| --- | --- | --- | --- | --- |
| root proof provisioning cluster | cross-layer auth/proof surface | spans DTO, API, domain policy, ops, storage mapper, workflow, endpoint macros, protocol tests, and internal test harness; direction remains correct. | high while 0.68 stabilizes | Medium |
| `ops/auth/delegation/mod.rs` | large ops hub | 942 production lines after moving the 720-line test body into `delegation/tests.rs`; it owns pending metadata and active-proof state transitions but is crate-private. | high when adding proof lifecycle states | Medium |
| `dto/auth.rs` | public protocol DTO gravity | 498 lines with active proof, root proof batch, issuer policy, token, and attestation DTOs; passive data only. | medium when adding auth protocol shapes | Medium |
| `canic-host::deployment_truth` | broad host public support surface | 321-line root module with 17 `pub use` groups; role-owned host API and not a facade leak. | medium when adding deployment phases | Medium |
| `access/expr/mod.rs` | endpoint auth expression model | 667-line central access model; imports access/ids/log/CDK support and no storage/workflow internals. | medium when adding predicates | Low |
| `canic-testing-internal::pic` | Canic-specific test harness seam | re-exports Canic topology/proof helpers but remains unpublished and layered above generic `ic-testkit`. | low if kept repo-only | Low |

## Hub Module Pressure

| Hub Module | Top Imported Sibling Subsystems | Unique Sibling Subsystems Imported | Cross-Layer Dependency Count | Delta vs Previous Report | HIP | Pressure Band | Risk |
| --- | --- | ---: | ---: | --- | ---: | --- | --- |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | `domain::policy::auth`, `dto::auth`, `ops::storage::auth`, `ops::ic`, `ops::auth::delegated` | 5 | 2 | new 0.68 root proof provisioning hub; not comparable to baseline host/CLI pressure | 0.40 | moderate | Medium |
| `crates/canic-core/src/dto/auth.rs` | `dto::prelude`, `dto::rpc`, public Candid auth DTOs | 2 | 0 | larger public auth DTO set after root proof provisioning; passive DTO layer | 0.00 | low by HIP, medium by public protocol breadth | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | `dto::auth`, `ops::auth`, `ops::config`, `ops::ic`, `ops::runtime`, `workflow::runtime::auth` | 5 | 2 | expanded root proof provisioning API surface since baseline | 0.40 | moderate | Medium |
| `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs` | `dto::auth`, `ops::auth`, `ops::ic`, `ops::runtime`, `protocol`, `workflow::prelude` | 5 | 2 | new focused workflow module; not comparable to baseline | 0.40 | moderate | Low |
| `crates/canic-core/src/access/expr/mod.rs` | `access`, `ids`, `log`, `cdk` | 4 | 1 | path-comparable; remains central access boundary model | 0.25 | low | Low |
| `../ic-testkit/crates/ic-testkit/src/pic/mod.rs` | `baseline`, `calls`, `diagnostics`, `errors`, `lifecycle`, `process_lock`, `runtime`, `snapshot`, `standalone`, `startup` | 10 | 0 | path-comparable; still Canic-free | 0.00 | low | Low |
| `crates/canic-testing-internal/src/pic/mod.rs` | `artifacts`, `attestation`, `audit`, `canic`, `delegation`, `lifecycle`, `root` | 7 | 1 | path-comparable; still repo-only harness seam | 0.14 | low | Low |
| `crates/canic-host/src/deployment_truth/mod.rs` | `authority`, `executor`, `lifecycle`, `model`, `multi`, `observe`, `plan`, `promotion`, `receipt`, `report`, `root`, `text` | 12 | 1 | path-comparable; still broad public host support | 0.08 | low by HIP, medium by public surface breadth | Medium |

## Dependency Fan-In Pressure

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| root proof provisioning broad group | 27 files | facade macros, canic tests, canic-tests, dto, domain policy, api, ops, storage, workflow, replay policy | Architectural gravity well |
| root proof provisioning core API/DTO/status group | 14 files | facade macros, facade API, dto, domain policy, api, ops, storage, workflow | Hub forming |
| host deployment truth root re-export surface | 17 `pub use` groups | host model/validation/text/promotion/report/root support | Broad support API |
| core public root modules | 18 public/hidden roots plus 2 test-only macros in scan | api/dto/access/bootstrap/cdk/control_plane_support/dispatch/error/ids/ingress/log/memory/perf/protocol/replay_policy/shared_support/test | Stable broad support surface |
| generic `ic-testkit::pic` public support | 7 `pub use` groups plus public builder/runtime helpers | generic PocketIC support only | Normal |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| root proof provisioning fan-in | `dto/auth.rs`, `ops/auth/delegation/mod.rs`, `api/auth/mod.rs`, macro endpoints, storage mapper, workflow provisioning | broad root proof scan found 27 files after test split | Medium |
| large ops auth provisioning hub | `crates/canic-core/src/ops/auth/delegation/mod.rs` | 942 production lines plus 720 adjacent test lines; repeated commits across `0.68.7` through `0.68.23` | Medium |
| public auth DTO gravity | `crates/canic-core/src/dto/auth.rs` | active proof, root proof batch, issuer policy, delegated token, and attestation DTOs share one public file | Medium |
| broad host support re-export surface | `crates/canic-host/src/deployment_truth/mod.rs` | 17 `pub use` groups in a public host module | Medium |
| module-layout escapes | workspace Rust files | no `#[path]` usage and no `foo.rs` plus `foo/mod.rs` duplicates found | Low |
| non-fatal lint expectation warnings during focused test | `crates/canic-core/src/ops/runtime/metrics/delegated_auth.rs` | workflow provisioning unit test emitted four `unfulfilled_lint_expectations` warnings | Low for structure; separate lint hygiene concern |

### Enum Shock Radius

| Enum | Defined In | Reference Surface | Risk |
| --- | --- | --- | --- |
| `RootDelegationProofInstallOutcome` | `crates/canic-core/src/dto/auth.rs` | DTO, workflow provisioning, protocol tests, replay policy tests | Medium |
| `ActiveDelegationProofStatus` | `crates/canic-core/src/dto/auth.rs` | DTO, ops active proof status, API, macro endpoint, protocol tests | Medium |
| `DelegationAudience` / `DelegatedRoleGrant` | `crates/canic-core/src/dto/auth.rs` | token, proof, root issuer policy, domain policy conversion | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `RootDelegationProofBatchPrepareRequest` / `Response` | `crates/canic-core/src/dto/auth.rs` | dto/api/ops/macros/tests | Medium |
| `RootDelegationProofBatchInstallRequest` / `Response` | `crates/canic-core/src/dto/auth.rs` | dto/api/workflow/ops/macros/tests | Medium |
| `RootIssuerPolicyUpsertRequest` / `View` | `crates/canic-core/src/dto/auth.rs` | dto/api/ops/domain policy/tests | Medium |
| `ActiveDelegationProof` / status DTOs | `crates/canic-core/src/dto/auth.rs` | dto/api/ops/storage/workflow/macros/tests | Medium |

### Growing Hub Modules

| Module | Current Size / Surface | Recent Churn | Risk |
| --- | ---: | --- | --- |
| `crates/canic-core/src/ops/auth/delegation/mod.rs` | 942 production lines, tests split to `delegation/tests.rs` | repeated commits across 0.68 root proof provisioning slices | Medium |
| `crates/canic-core/src/dto/auth.rs` | 498 lines | new root proof batch and active proof protocol shapes | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | 215 lines | new root proof provisioning API methods | Medium |
| `crates/canic-host/src/deployment_truth/mod.rs` | 321 lines, 17 `pub use` groups | broad host support API remains | Medium |

## Risk Score

Risk Score: **4 / 10**.

Score basis:

- `+0` for confirmed High/Critical structural violations: none found.
- `+1` for broad public auth DTO surface required by root proof provisioning.
- `+1` for `ops/auth/delegation/mod.rs` remaining production size and recent
  edit pressure, even after splitting adjacent tests into `delegation/tests.rs`.
- `+1` for cross-layer root proof provisioning fan-in across DTO/API/ops,
  workflow, storage mapper, macros, and tests.
- `+1` for broad host deployment-truth public support re-export surface.

Verdict: moderate structural pressure, no confirmed structural failure.

## Amplification Drivers

- Root proof provisioning is an auth protocol slice with necessarily shared
  DTOs, endpoint macros, API methods, ops state helpers, workflow broadcast
  orchestration, storage mapping, replay policy inventory, and protocol tests.
- `ops/auth/delegation/mod.rs` owns several related but distinct proof lifecycle
  concerns: active proof install/status, root issuer policy conversion, batch
  prepare/get/install metadata, replay/idempotency, and cleanup.
- `dto/auth.rs` is the stable Candid/protocol surface for multiple auth
  families, so new auth variants naturally widen the same public file.
- `canic-host::deployment_truth` remains intentionally public and broad; future
  host phases should keep implementation files separated behind that support
  contract.

## Drift Sensitivity Summary

| Growth Vector | Affected Subsystems | Why Multiple Layers Would Change | Drift Risk |
| --- | --- | --- | --- |
| new root proof lifecycle state | DTO, ops auth delegation, workflow provisioning, macro endpoints, protocol tests | new status/outcome shape would touch boundary DTOs plus install/preflight behavior. | Medium |
| new auth proof endpoint | macro endpoints, `api/auth`, `ops/auth`, replay policy, protocol tests | endpoint guard, API mapping, state/proof helper, replay classification, and Candid surface move together. | Medium |
| new auth DTO family | `dto/auth.rs`, facade DTO exports, protocol tests, ops conversion | public DTO file is the stable Candid boundary for auth features. | Medium |
| new deployment-truth phase | `canic-host::deployment_truth`, CLI deploy subcommands, host reports/tests | public host support contract would need new model/validation/text/report pieces. | Medium |
| new generic PocketIC helper | `../ic-testkit`, `canic-testing-internal`, `canic-tests` | generic helpers must remain Canic-free while Canic topology helpers stay internal. | Low |

## Verification Readout

| Check / Command | Status | Notes |
| --- | --- | --- |
| recurring definition review | PASS | reviewed and updated `docs/audits/recurring/system/module-structure.md` for standard recurring headings and current 0.68 focus. |
| baseline review | PASS | compared against `docs/audits/reports/2026-05/2026-05-29/module-structure.md`; current run marked non-comparable. |
| `git rev-parse --short HEAD` | PASS | code snapshot identifier `16894709`. |
| `git status --short` | PASS | worktree dirty; unrelated source edits left untouched. |
| public root scan with `rg` over public crate roots and sibling `ic-testkit` | PASS | no accidental storage/workflow root exposure through `canic`; root proof DTOs are public protocol data. |
| `cargo metadata --locked --no-deps --format-version 1` | PASS | workspace crate graph resolved without dependency-cycle failure. |
| `rg -n "#\\[path\\s*=|include!\\(" crates canisters fleets ../ic-testkit -g '*.rs'` | PASS | no `#[path]` usage; only build-generated `include!(env!(...))` calls in `crates/canic/src/macros/start.rs`. |
| `find crates canisters fleets -type f -name '*.rs' -print \| sed 's#\\.rs$##; s#/mod$##' \| sort \| uniq -d` | PASS | no `foo.rs` plus `foo/mod.rs` duplicates found. |
| root proof provisioning `rg -l` scan | PASS | broad group found 27 files after the test split; treated as fan-in pressure, not violation. |
| layer-reference scan over `domain`, `storage`, `ops`, and `workflow` | PASS | no storage-to-workflow, policy-to-ops, or workflow-to-storage-internal breach confirmed. |
| test/fleet seam scan for `ic-testkit` / `canic-testing-internal` | PASS | `ic-testkit` appeared only in a test canister dev dependency and manifest test context; generic `ic-testkit` source has no Canic references. |
| `cargo test --locked -p canic --test workspace_manifest -- --nocapture` | PASS | 5 tests passed. |
| `cargo test --locked -p canic --test protocol_surface -- --nocapture` | PASS | 11 tests passed, including root proof batch and active proof surface pins. |
| `cargo test --locked -p canic --test protocol_inventory_gate -- --nocapture` | PASS | 23 tests passed. |
| `cargo test --locked -p canic-core --lib ops::auth::delegation -- --nocapture` | PASS | 26 tests passed after moving inline delegation tests into `delegation/tests.rs`. |
| `cargo test --locked -p canic-core --lib workflow::runtime::auth::provisioning -- --nocapture` | PASS | 2 tests passed; emitted non-fatal delegated-auth metrics lint-expectation warnings. |
| `rustfmt --edition 2024 --check crates/canic-core/src/ops/auth/delegation/mod.rs crates/canic-core/src/ops/auth/delegation/tests.rs crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs crates/canic-core/src/dto/auth.rs` | PASS | touched files are formatted. |
| required recurring heading scan | PASS | `Report Preamble`, `Structural Hotspots`, `Hub Module Pressure`, `Dependency Fan-In Pressure`, `Early Warning Signals`, `Risk Score`, and `Verification Readout` headings are present. |
| `git diff --check` | PASS | no whitespace errors. |

## Follow-up Actions

1. Keep root proof provisioning DTOs passive; policy decisions should stay in
   `domain/policy/auth`, state transitions in `ops/auth`, and orchestration in
   `workflow/runtime/auth`.
2. Monitor `ops/auth/delegation/mod.rs` as the 0.68 proof lifecycle stabilizes; do
   not move endpoint guards or workflow broadcast logic into it.
3. Keep endpoint macros thin: guard, call `AuthApi`, return DTO.
4. Keep `canic-host::deployment_truth` role-owned; split implementation files
   before adding another broad deployment phase family.
5. Clean up the delegated-auth metrics lint-expectation warnings in a separate
   lint/hygiene pass if they still reproduce under clippy `-D warnings`.
