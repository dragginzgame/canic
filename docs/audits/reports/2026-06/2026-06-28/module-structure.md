# Module Structure Audit - 2026-06-28

## Report Preamble

- Definition path: `docs/audits/recurring/system/module-structure.md`
- Scope: `crates/canic`, `crates/canic-backup`, `crates/canic-cli`,
  `crates/canic-core`, `crates/canic-control-plane`, `crates/canic-host`,
  `crates/canic-macros`, `crates/canic-wasm-store`,
  `crates/canic-testing-internal`, `crates/canic-tests`, sibling
  `../ic-testkit`, `fleets/**`, `canisters/test/**`, `canisters/audit/**`,
  and `canisters/sandbox/**`.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/module-structure.md`
- Code snapshot identifier: `b140a86c` with dirty worktree.
- Method tag/version: `module-structure-current`.
- Comparability status: `comparable`, with path-adjusted notes for the
  root-renewal directory-module split and the post-cleanup blob-storage API
  split.
- Exclusions applied: generated target outputs, `.icp` runtime cache,
  historical audit reports outside the compared baseline, broad style-only
  hygiene, and test files for production layer-reference scans unless the row
  explicitly describes test seam containment.
- Notable methodology changes vs baseline: none.
- Auditor: `codex`.
- Run timestamp: `2026-06-28T13:52:03Z`.
- Worktree: `dirty`; unrelated dirty root-renewal and audit-report edits were
  preserved.

Verification status: **PASS**.

No High or Critical structural violation was confirmed. The current risk is
lower than the June 19 module-structure run because the root-renewal and
blob-storage API parents now delegate to focused child modules. Residual
pressure remains in broad public auth DTO/API surfaces and the host
deployment-truth support contract.

## Structural Hotspots

| File / Module | Exposed or Coupled Item | Visibility Scope | Direction / Exposure Impact | Risk |
| --- | --- | --- | --- | --- |
| `crates/canic-core/src/dto/auth.rs` | delegated-token, root proof batch, root-renewal, issuer policy, and role-attestation DTOs | `pub` DTOs under public `canic_core::dto` and facade DTO reachability | Passive Candid/protocol boundary data with public fields; no storage or execution ownership found. | Medium |
| `crates/canic-core/src/api/auth/mod.rs` | `AuthApi` delegated-token, root proof, root-renewal, role-attestation methods | public API methods on public `AuthApi` | Endpoint/API boundary maps DTOs into ops/workflow and environment guards; no stable record ownership found. | Medium |
| `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/*` | renewal template/provisioner facade, scheduler, retrieval, install, identity, view conversion | private child modules; outward functions are `pub(super)` or `pub(in crate::ops::auth::delegation)` | Correct ops owner for deterministic state transitions; split lowers parent-hub pressure but the lifecycle remains multi-axis. | Medium |
| `crates/canic-core/src/api/blob_storage/*` | hash, lifecycle, gateway, billing endpoint helpers | private child modules behind public `BlobStorageApi` | API facade now maps endpoint input through ops conversion/lifecycle/funding; direct production API-to-model conversion was removed. | Low |
| `crates/canic-core/src/access/auth/identity.rs` | delegated-session identity fallback and invalid-subject cleanup | `pub(super)` under public access auth helpers | Accepted access-boundary exception from access-purity: narrow `AuthStateOps` read/clear and metrics, not workflow or proof provisioning. | Low |
| `crates/canic-host/src/deployment_truth/mod.rs` | broad host deployment-truth `pub use` surface | public host support module and many `pub use` groups | Role-owned host/operator support API; broad but not re-exported through the `canic` runtime facade. | Medium |
| `../ic-testkit/crates/ic-testkit/src/pic/mod.rs` | generic PocketIC support | public generic support crate | Canic-free test infrastructure; Canic-specific helpers remain in `canic-testing-internal`. | Low |

## Public Surface Map

| Item | Kind | Path | Publicly Reachable From Root? | Classification | Visibility Scope | Exposure Impact | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Canic facade roots | module family | `crates/canic/src/lib.rs` (`access`, `api`, `dto`, `ids`, `prelude`, `protocol`) | yes | intended external API | `pub mod` / `pub use` | Public facade remains the broad user surface; no storage/model root re-export found. | Low |
| Canic hidden support roots | module family | `crates/canic/src/lib.rs` (`__internal`, `__build`) | yes, hidden | macro/build support | `#[doc(hidden)] pub mod` | Required macro/build plumbing; no product API claim. | Low |
| Core public and crate-private roots | module family | `crates/canic-core/src/lib.rs` | yes for API/DTO/support; no for ops/storage/workflow/model | lower-level support API | mixed `pub mod`, `#[doc(hidden)] pub mod`, `pub(crate) mod` | Runtime internals remain crate-private; `dto`, `api`, `ids`, protocol, memory, and support roots stay public. | Medium |
| Auth DTO family | DTO family | `crates/canic-core/src/dto/auth.rs` | yes | stable protocol DTOs | `pub struct` / `pub enum` with public fields | Required protocol/Candid shapes; broad fan-in but passive. | Medium |
| Root-renewal ops children | module family | `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/*` | no | crate-internal ops implementation | private modules, `pub(super)`, `pub(in crate::ops::auth::delegation)` | Split exposes no new public root path; outward access remains inside auth delegation ops. | Low |
| Blob-storage API children | module family | `crates/canic-core/src/api/blob_storage/*` | only through `BlobStorageApi` methods | API boundary implementation | private child modules under public `BlobStorageApi` | Split does not widen public reachability; it narrows implementation ownership. | Low |
| Host deployment truth | support module family | `crates/canic-host/src/deployment_truth/mod.rs` | yes through `canic_host::deployment_truth` | host/operator support API | `pub mod` plus broad `pub use` | Broad public support contract remains role-aligned and outside the canister runtime facade. | Medium |
| Generic testkit | support module family | `../ic-testkit/crates/ic-testkit/src/lib.rs`, `pic/mod.rs` | yes | public generic test infrastructure | `pub mod`, `pub use`, public helpers | Generic PocketIC surface contains no Canic runtime dependency. | Low |

## Subsystem Dependency Graph

| Subsystem / Crate | Depends On | Depended On By | Direction Assessment | Risk |
| --- | --- | --- | --- | --- |
| `canic` | `canic-core`, `canic-macros`, optional control-plane support | fleet/test/audit/sandbox canisters, examples, tests | Facade direction remains clean; hidden roots are macro/build plumbing. | Low |
| `canic-core` | IC/CDK/storage/memory/runtime dependencies | `canic`, `canic-control-plane`, `canic-host`, tests | Public roots expose API/DTO/support; ops/storage/workflow/model remain crate-private. | Low |
| `canic-control-plane` | `canic-core` and support crates | `canic` control-plane feature and root/store canisters | Runtime support stays below the facade. | Low |
| `canic-host` | `canic-core`, serialization, filesystem/process support | `canic-cli` | Host/operator support remains facade-free; deployment-truth surface is broad but host-owned. | Medium |
| `canic-cli` | `canic-core`, `canic-host`, `canic-backup` | binary entrypoint | CLI owns UX and dispatch; auth command module remains a size watchpoint, not public API leakage. | Medium |
| `canic-testing-internal` | `ic-testkit`, `canic`, `canic-core`, `canic-control-plane` | `canic-tests` | Repo-only test harness remains one-way and unpublished. | Low |
| `../ic-testkit` | `pocket-ic`, `candid`, generic support crates | Canic tests and downstreams | No `canic_core`, `canic-testing-internal`, or `canic::` source references found. | Low |

## Circularity Findings

| Subsystem A | Subsystem B | Real Cycle? | Evidence | Risk |
| --- | --- | --- | --- | --- |
| `canic` | `canic-core` | no | `canic` depends on `canic-core`; `canic-core` does not depend on `canic`. | Low |
| `canic-host` | `canic-cli` | no | CLI depends on host; host has no CLI dependency. | Low |
| `canic-testing-internal` | `../ic-testkit` | no | internal harness depends on `ic-testkit`; `ic-testkit` has no Canic references. | Low |
| root-renewal API/ops/workflow/storage cluster | multiple layers | no | Direction remains endpoint/API -> ops/workflow -> ops/storage, with DTOs as passive boundary data. | Low |
| blob-storage API/ops/model/storage cluster | multiple layers | no | API calls ops conversion/lifecycle/funding; production API-to-model references were removed. | Low |

## Visibility Hygiene Findings

| Item | Path | Current Visibility | Narrowest Plausible Visibility | Why Narrower Seems Valid / Invalid | Risk |
| --- | --- | --- | --- | --- | --- |
| auth DTOs | `crates/canic-core/src/dto/auth.rs` | `pub` | keep current | Protocol and Candid tests pin these boundary shapes; public fields are DTO contract fields, not storage internals. | Medium |
| root-renewal child modules | `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/*` | private modules; selected `pub(super)` / restricted `pub(in ...)` functions | keep current | API and parent ops module need bounded access; no root-public path exists. | Low |
| blob-storage API child modules | `crates/canic-core/src/api/blob_storage/*` | private modules behind `BlobStorageApi` | keep current | Generated endpoint helpers still call the public API type; child modules do not widen reachability. | Low |
| access delegated-session fallback | `crates/canic-core/src/access/auth/identity.rs` | `pub(super)` helpers | keep current | Existing access-purity exception for endpoint-boundary identity resolution; not proof lifecycle or workflow ownership. | Low |
| host deployment-truth re-exports | `crates/canic-host/src/deployment_truth/mod.rs` | public `pub use` groups from private submodules | keep current until support contract is intentionally split further | Broad host support API is role-aligned and not re-exported by `canic`. | Medium |
| core execution roots | `crates/canic-core/src/lib.rs` (`config`, `domain`, `infra`, `lifecycle`, `model`, `ops`, `storage`, `view`, `workflow`) | `pub(crate) mod` | keep current | Execution/storage/workflow internals remain outside public root reachability. | Low |

### Test Leakage

| Item | Location | Leakage Type | Build Impact | Risk |
| --- | --- | --- | --- | --- |
| generic testkit use in product/runtime crates | product source and manifest scan | none found | `ic-testkit` appears in `canic-testing-internal`, `canic-tests`, and one test canister dev dependency, not runtime products. | Low |
| Canic-specific PocketIC helpers | `crates/canic-testing-internal/src/pic/**` | repo-only test harness | `canic-tests` depends on it; generic `ic-testkit` does not. | Low |
| audit canister names in host/CLI | host/CLI/fleet scan | test fixture only | audit canister path appears in a `canic-host` release-set test fixture, not production fleet dependency. | Low |

## Layering Violations

| Layer / Rule | Upward Dependency Found? | Description | Risk |
| --- | --- | --- | --- |
| `storage` must not depend on workflow/policy/ops | no | reverse scan found no `crate::workflow` references from `storage`, `ops`, `access`, or `domain`. | Low |
| `ops` must not depend on workflow | no | reverse scan found no production `crate::workflow` references in `ops`. | Low |
| pure domain policy must not depend on ops/storage/runtime effects | no | `domain/**` scan found no `crate::ops`, `crate::storage`, `crate::workflow`, `crate::model`, or `crate::api` references. | Low |
| `workflow` and API should not bypass ops into storage/model | no production breach after cleanup | production scan found no workflow/API/facade storage/model references; only `api::blob_storage::tests` uses stable store reset helpers. | Low |
| access endpoint-boundary auth may use narrow session ops | accepted watchpoint | `access/auth/identity.rs` reads/clears delegated sessions and records metrics; the 2026-06-22 access-purity audit treats this as endpoint-boundary identity fallback, not policy ownership. | Low |
| DTOs must remain passive | no | auth DTOs define Candid/Serde shapes and public fields only; no mutation, async, storage, or runtime calls were found in inspected context. | Low |

## Structural Pressure Areas

| Area | Pressure Type | Why This Is Pressure, Not Yet Violation | Drift Sensitivity | Risk |
| --- | --- | --- | --- | --- |
| auth DTO/API surface | public protocol and API breadth | root proof, root renewal, role attestation, delegated sessions, and delegated tokens share `dto::auth` and `api::auth`; DTOs are passive and API delegates. | medium when adding auth protocol shapes | Medium |
| root-managed renewal ops cluster | multi-axis lifecycle state | schedule, retrieval, install, template/provisioner, identity, and view conversion now have separate private modules; no new public surface. | medium when adding renewal outcomes | Medium |
| blob-storage API facade | recent split and feature-gated billing | parent is 41 LOC and children are responsibility-specific; watch that new billing/lifecycle behavior stays in child modules. | medium when adding upload/billing behavior | Low |
| `canic-host::deployment_truth` | broad public host support API | many host model/helper re-exports remain public, but ownership is host/operator support, not runtime facade. | medium when adding deployment phases | Medium |
| `crates/canic-cli/src/auth/mod.rs` | command hub size | CLI auth command dispatch is large but not externally reusable API and does not leak through runtime crates. | medium if renewal CLI grows further | Medium |
| `access::auth` delegated-session fallback | accepted endpoint-boundary side effect | narrow session read/clear and metrics use are covered by access-purity; no root proof or workflow orchestration in access. | low if kept narrow | Low |

## Hub Module Pressure

| Hub Module | Top Imported Sibling Subsystems | Unique Sibling Subsystems Imported | Cross-Layer Dependency Count | Delta vs Previous Report | HIP | Pressure Band | Risk |
| --- | --- | ---: | ---: | --- | ---: | --- | --- |
| `crates/canic-core/src/api/auth/mod.rs` | `dto::auth`, `ops::auth`, `ops::config`, `ops::ic`, `ops::runtime`, `workflow::runtime::auth` | 5 | 2 | path-comparable; root-renewal methods added before this run remain present | 0.40 | moderate | Medium |
| `crates/canic-core/src/dto/auth.rs` | `dto::prelude`, `dto::rpc`, public Candid auth DTOs | 2 | 0 | path-comparable; passive public protocol breadth remains | 0.00 | low by HIP, medium by public protocol breadth | Medium |
| `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/*` | `domain::policy::auth`, `dto::auth`, `ops::storage::auth`, `ops::runtime::metrics`, auth delegation helpers | 5 | 3 | path-adjusted improvement from larger parent to private schedule/retrieval/install/view children | 0.60 | moderate | Medium |
| `crates/canic-core/src/api/blob_storage/*` | `dto::blob_storage`, `ops::blob_storage`, `ops::cashier`, `ops::ic`, `dto::error` | 5 | 2 | path-adjusted improvement from one large `api/blob_storage.rs` facade to focused children | 0.40 | moderate at family level, low per parent | Low |
| `crates/canic-core/src/access/expr/mod.rs` | `access`, `ids`, `log`, `cdk` | 4 | 1 | path-comparable; still central endpoint access expression model | 0.25 | low | Low |
| `../ic-testkit/crates/ic-testkit/src/pic/mod.rs` | generic PocketIC helper modules | 10 | 0 | path-comparable; still Canic-free | 0.00 | low | Low |
| `crates/canic-testing-internal/src/pic/mod.rs` | Canic fixture helpers layered on `ic-testkit` | 7 | 1 | path-comparable; repo-only harness seam | 0.14 | low | Low |
| `crates/canic-host/src/deployment_truth/mod.rs` | authority, executor, lifecycle, model, multi, observe, plan, promotion, receipt, report, root, text | 12 | 1 | path-comparable; still broad public host support | 0.08 | low by HIP, medium by public surface breadth | Medium |

## Dependency Fan-In Pressure

| Module / Symbol Group | Direct Files | Subsystems Referencing | Pressure Level |
| --- | ---: | --- | --- |
| root-renewal terms | 30 files in direct scan | facade macros, facade protocol tests, canic-tests, dto, domain policy, access predicates, storage, ops, API, workflow | Medium architectural gravity |
| blob-storage terms | 39 files in direct scan | facade macros/API, CLI, test canisters, canic-tests, protocol, view/model/ops/storage/dto/API | Medium feature surface |
| access auth identity/verifier lane | 9 files in direct scan | macro expansion, API session bootstrap, access expression/auth modules | Low-medium endpoint-boundary pressure |
| host deployment-truth surface | broad host/CLI direct scan | host model/validation/text/promotion/lifecycle/report plus CLI deploy commands | Broad support API |
| generic `ic-testkit::pic` public support | public generic helper family | generic PocketIC support and Canic internal tests | Normal |

## Early Warning Signals

| Signal | Location | Evidence | Risk |
| --- | --- | --- | --- |
| auth DTO gravity | `crates/canic-core/src/dto/auth.rs` | delegated-token, root proof batch, root-renewal, issuer policy, and role-attestation protocol shapes share one public file. | Medium |
| auth API convergence | `crates/canic-core/src/api/auth/mod.rs` | root renewal, root proof, active proof, role-attestation, delegated token, and session bootstrap methods live on one public API type. | Medium |
| root-renewal lifecycle spread | `ops/auth/delegation/root_issuer_renewal/*` | private split exists, but schedule/retrieval/install/status still move together for new lifecycle outcomes. | Medium |
| host deployment-truth breadth | `crates/canic-host/src/deployment_truth/mod.rs` | public host module still re-exports many model/helper groups. | Medium |
| blob-storage split regression risk | `crates/canic-core/src/api/blob_storage/*` | parent is now small; new behavior should not move back into `mod.rs`. | Low |
| module-layout escapes | workspace Rust files | no `#[path]` usage and no `foo.rs` plus `foo/mod.rs` duplicates found; only build-time `include!(env!(...))` macros remain in `macros/start.rs`. | Low |

### Enum Shock Radius

| Enum | Defined In | Reference Surface | Risk |
| --- | --- | --- | --- |
| `RootIssuerRenewalOutcome` | `crates/canic-core/src/dto/auth.rs` | DTO, domain policy, ops renewal state, storage mapper, tests | Medium |
| `RootDelegationProofInstallOutcome` | `crates/canic-core/src/dto/auth.rs` | DTO, workflow provisioning, ops install recorders, protocol tests | Medium |
| `ActiveDelegationProofStatus` | `crates/canic-core/src/dto/auth.rs` | API, ops active proof status, macro endpoints, protocol tests | Medium |
| `DelegationAudience` / `DelegatedRoleGrant` | `crates/canic-core/src/dto/auth.rs` | token, proof, root issuer policy, domain policy conversion | Medium |

### Cross-Layer Struct Spread

| Struct | Defined In | Layers Referencing | Risk |
| --- | --- | --- | --- |
| `RootIssuerRenewalTemplate*` / `RootIssuerRenewalStatus*` | `crates/canic-core/src/dto/auth.rs` | dto/api/ops/storage/domain/tests | Medium |
| `RootDelegationRenewal*` DTOs | `crates/canic-core/src/dto/auth.rs` | dto/api/ops/access/storage/workflow/tests | Medium |
| `RootDelegationProofBatch*` DTOs | `crates/canic-core/src/dto/auth.rs` | dto/api/workflow/ops/macros/tests | Medium |
| `BlobStorageStatus*` / billing DTOs | `crates/canic-core/src/dto/blob_storage.rs` | dto/api/ops/CLI/macros/tests | Medium |

### Growing Hub Modules

| Module | Current Size / Surface | Recent Churn | Risk |
| --- | ---: | --- | --- |
| `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/*` | parent 255 LOC; production children 102/541/232/394/151 LOC | high in 0.74 root-managed renewal | Medium |
| `crates/canic-core/src/api/blob_storage/*` | parent 41 LOC; production children 22/122/67/460 LOC | high in 0.70/0.71 and split during this cleanup | Low |
| `crates/canic-core/src/dto/auth.rs` | public auth protocol DTO file | auth proof/renewal protocol growth | Medium |
| `crates/canic-host/src/deployment_truth/mod.rs` | public re-export root with many groups | broad host support API remains | Medium |
| `crates/canic-cli/src/auth/mod.rs` | large CLI auth command module | 0.74 CLI/auth split left this as a command hub | Medium |

## Known Intentional Exceptions

| Exception | Why Intentional | Scope Guardrail | Still Valid This Run? |
| --- | --- | --- | --- |
| `canic` facade re-exports stable DTO/API/protocol support | primary user-facing crate owns facade ergonomics | do not re-export storage/model/workflow internals | yes |
| `dto` public fields | Candid/Serde boundary contracts require data-only public shapes | DTOs must remain passive and not own mutation, async, storage, or policy | yes |
| `access::auth` delegated-session read/clear and access metrics | endpoint-boundary identity resolution and fallback hygiene | keep proof provisioning, root issuer policy mutation, workflow, and broad storage conversion out of access | yes |
| `canic-host::deployment_truth` public support surface | host/CLI operator planning APIs are intended support contracts | keep outside `canic` runtime facade and avoid leaking canister storage internals | yes |
| sibling `../ic-testkit` public PocketIC helpers | generic test infrastructure for downstreams | no Canic-specific harness or runtime semantics in generic testkit | yes |

## Delta Since Baseline

| Delta Type | Item / Subsystem | Previous | Current | Impact |
| --- | --- | --- | --- | --- |
| module split | `ops/auth/delegation/root_issuer_renewal/*` | root-renewal logic concentrated in parent/test-heavy module during 0.74 work | private child modules for identity, install, retrieval, schedule, view, and tests | lowers parent-hub pressure without widening public reachability |
| module split | `api/blob_storage/*` | single large `api/blob_storage.rs` facade in the complexity baseline | parent `mod.rs` plus private hash, lifecycle, gateway, billing, and test children | lowers API parent pressure and keeps direct model conversion behind ops |
| boundary cleanup | blob root hash conversion | production API directly called model `BlobRootHash::into_string` before cleanup | API uses `BlobStorageConversionOps` canonical text/bytes helpers | removes production API-to-model conversion reference |
| public surface | crate roots | broad but stable public roots | no new storage/model/workflow root exposure through `canic` found | no new public-internal leak |
| module layout | directory-module policy | no `#[path]` or duplicate module pair violations | still no `#[path]` and no `foo.rs` plus `foo/mod.rs` duplicates | invariant holds |

## Risk Score

Risk Score: **3 / 10**.

Score basis:

- `+0` for confirmed High/Critical structural violations: none found.
- `+1` for broad auth DTO/API public protocol and helper surface.
- `+1` for root-managed renewal fan-in across DTO/API/access/domain/ops/
  storage/workflow/tests, even after the healthier private child-module split.
- `+1` for broad host deployment-truth public support surface and CLI auth
  command hub watchpoints.
- `+0` for blob-storage API after cleanup: parent-hub and direct model
  conversion pressure were reduced.

Verdict: low structural risk with remaining medium watchpoints; no confirmed
structural failure.

## Structural Risk Index

| Category | Risk Index (1-10, lower is better) | Basis |
| --- | ---: | --- |
| Public Surface Discipline | 3 | Public auth DTO/API and host deployment-truth surfaces are broad but role-aligned. |
| Layer Directionality | 2 | Layering guards and direct scans found no production storage/workflow/API bypass after cleanup. |
| Circularity Safety | 1 | Cargo metadata resolved and no crate/subsystem cycles were found. |
| Visibility Hygiene | 3 | Split modules use private/restricted visibility; host deployment-truth remains intentionally broad. |
| Facade Containment | 2 | `canic` does not expose storage/model/workflow roots; host support remains outside runtime facade. |

Overall Structural Risk Index: **3 / 10**.

## Drift Sensitivity Summary

| Growth Vector | Affected Subsystems | Why Multiple Layers Would Change | Drift Risk |
| --- | --- | --- | --- |
| new root-renewal lifecycle outcome | DTO, domain policy, ops schedule/retrieval/install/status, storage mapper, workflow install, tests/docs | outcome and state transitions must stay synchronized across passive DTOs and ops-owned state. | Medium |
| new auth proof endpoint | macro endpoints, `api/auth`, ops auth, replay policy, protocol tests | endpoint guard, API mapping, proof helper, replay classification, and Candid surface move together. | Medium |
| new blob-storage billing/status capability | API child module, ops blob/cashier, DTO, CLI render/tests | endpoint helper and operator output must preserve ops-owned conversion/lifecycle boundaries. | Medium |
| new deployment-truth phase | `canic-host::deployment_truth`, CLI deploy commands, host reports/tests | public host support contract needs model/validation/text/report pieces. | Medium |
| new generic PocketIC helper | `../ic-testkit`, `canic-testing-internal`, `canic-tests` | generic helpers must remain Canic-free while Canic topology helpers stay internal. | Low |

## Verification Readout

| Check / Command | Status | Notes |
| --- | --- | --- |
| recurring definition review | PASS | reviewed `docs/audits/recurring/system/module-structure.md`; no definition changes were required. |
| baseline review | PASS | compared against `docs/audits/reports/2026-06/2026-06-19/module-structure.md`. |
| `git rev-parse --short HEAD` | PASS | code snapshot identifier `b140a86c`. |
| `date -u +%Y-%m-%dT%H:%M:%SZ` | PASS | timestamp `2026-06-28T13:52:03Z`. |
| `cargo metadata --locked --no-deps --format-version 1` | PASS | workspace crate graph resolved without dependency-cycle failure. |
| public root scan over public-facing crate roots and sibling `ic-testkit` | PASS | no accidental storage/model/workflow root exposure through `canic`; root auth DTOs and host support surfaces are intentional. |
| `rg -n "#\\[path\\s*=|include!\\(" crates canisters fleets ../ic-testkit -g '*.rs'` | PASS | no `#[path]`; only build-time `include!(env!(...))` calls in `crates/canic/src/macros/start.rs`. |
| `find crates canisters fleets -type f -name '*.rs' -print \| sed 's#\\.rs$##; s#/mod$##' \| sort \| uniq -d` | PASS | no `foo.rs` plus `foo/mod.rs` duplicate module pairs. |
| reverse workflow-reference scan from `ops`, `storage`, `access`, and `domain` | PASS | no `crate::workflow` or `canic_core::workflow` matches. |
| domain lower-layer scan | PASS | no `crate::ops`, `crate::storage`, `crate::workflow`, `crate::model`, or `crate::api` matches in `domain/**`. |
| workflow/API/facade storage/model scan | PASS | production scan found no storage/model bypass; only blob-storage API tests reset stable stores. |
| test/fleet seam scan for `ic-testkit` / `canic-testing-internal` | PASS | `ic-testkit` remains generic; Canic-specific helpers stay in repo-only test crates. |
| audit/fleet seam scan | PASS | audit canister path appeared only in a host release-set test fixture. |
| `bash scripts/ci/run-layering-guards.sh` | PASS | executable layering guard passed. |
| `cargo test --locked -p canic-core --lib ops::auth::delegation -- --nocapture` | PASS | 46 tests passed, covering root-renewal schedule/retrieval/install split. |
| `cargo test --locked -p canic-core blob_storage --lib --features blob-storage-billing -- --nocapture` | PASS | 49 tests passed, covering blob-storage split and ops conversion cleanup. |
| `cargo test --locked -p canic --test workspace_manifest -- --nocapture` | PASS | 6 tests passed. |
| `cargo test --locked -p canic --test protocol_surface -- --nocapture` | PASS | 17 tests passed. |
| `cargo test --locked -p canic --test protocol_inventory_gate -- --nocapture` | PASS | 24 tests passed. |

## Follow-up Actions

- Keep the root-renewal schedule/retrieval/install/view child modules private
  and route new lifecycle behavior to the matching owner.
- Keep blob-storage hash, lifecycle, gateway, and billing behavior in the
  matching child module, with canonical root-hash conversion behind
  `ops::blob_storage::conversion`.
- Watch `crates/canic-core/src/dto/auth.rs`, `crates/canic-core/src/api/auth/mod.rs`,
  `crates/canic-host/src/deployment_truth/mod.rs`, and
  `crates/canic-cli/src/auth/mod.rs` for renewed hub growth.
