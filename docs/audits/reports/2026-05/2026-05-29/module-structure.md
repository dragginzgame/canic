# Module Structure Audit - 2026-05-29

## 0. Run Metadata + Comparability Note

- Scope: `crates/canic`, `crates/canic-backup`, `crates/canic-cli`,
  `crates/canic-core`, `crates/canic-control-plane`, `crates/canic-host`,
  `crates/canic-macros`, `crates/canic-wasm-store`,
  `crates/canic-testing-internal`, `crates/canic-tests`,
  sibling `../ic-testkit`, `fleets/**`, `canisters/test/**`,
  `canisters/audit/**`, and `canisters/sandbox/**`.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-10/module-structure.md`
- Code snapshot identifier: `613b4a76`
- Method tag/version: `module-structure-current`
- Comparability status: non-comparable: the prior report still included
  removed in-repo support crates and pre-deployment-truth host structure.
  This run uses the current crate map, sibling `../ic-testkit`, and explicit
  classifications for `domain`, `view`, `memory`, `lifecycle`,
  `control_plane_support`, `cdk`, `bootstrap`, `dispatch`, `ingress`,
  `format`, and `shared_support`.
- Exclusions applied: generated target outputs, `.icp` runtime cache, and
  historical audit reports other than the compared baseline.
- Notable methodology changes vs baseline: exact line-count and delta
  comparisons are not used as trend claims; pressure comparisons are noted only
  where the current owner and path are clear.

Verification status: **PASS**.

No High or Critical structural violation was confirmed. Current risk is
coordination pressure in host/CLI modules, not a dependency-direction breach,
public/internal seam leak, or module-discovery problem.

## 1. Public Surface Map

| Item | Kind | Path | Publicly Reachable From Root? | Classification | Visibility Scope | Exposure Impact | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Canic facade modules | module family | `crates/canic/src/lib.rs` (`access`, `api`, `dto`, `ids`, `prelude`, `protocol`) | yes | intended external API | `pub mod` | normal user facade remains the primary public surface. | Low |
| Canic hidden macro/build support | module family | `crates/canic/src/lib.rs` (`__internal`, `__build`) | yes | macro/build support | `#[doc(hidden)] pub mod`, build support cfg-gated off wasm | required for macro expansion and build scripts; not presented as normal API. | Low |
| Canic macros | macro family | `crates/canic/src/macros/*.rs` | yes | public macro API plus hidden helpers | `#[macro_export]` | `start!`, `start_local!`, `start_wasm_store!`, `finish!`, endpoint, timer, log, and build macros are root-reachable; hidden `__canic_*` helpers are implementation support. | Low |
| Core public and hidden roots | module family | `crates/canic-core/src/lib.rs` | yes | lower-level support API | mixed `pub mod`, `#[doc(hidden)] pub mod`, `pub(crate) mod` | stable DTO/API/ID/CDK/memory roots are public; execution/storage/workflow roots remain crate-private. | Medium |
| Control-plane runtime support | module family | `crates/canic-control-plane/src/lib.rs` | yes | control-plane support API | `pub mod api/dto/ids/runtime/schema`; internals `pub(crate)` | root/store runtime support stays below `canic` and does not expose storage/workflow roots directly. | Low |
| Host library | module family | `crates/canic-host/src/lib.rs` | yes | operator support API | `pub mod` | broad host support surface including build, ICP, install, release, deployment truth, registry, response parsing, and table helpers. | Medium |
| CLI library entry | functions/types | `crates/canic-cli/src/lib.rs` | yes | binary support API | `pub enum`, `pub fn`, `pub const fn`, one `pub use` | programmatic CLI surface is compact despite large private command modules. | Low |
| Backup library | module family | `crates/canic-backup/src/lib.rs` | yes | backup/restore package API | `pub mod` | broad but domain-owned backup, plan, restore, runner, snapshot, manifest, and persistence surface. | Low |
| Proc macros | attribute macros | `crates/canic-macros/src/lib.rs` | yes | public macro API | `#[proc_macro_attribute] pub fn` | only `canic_query` and `canic_update` are exported. | Low |
| Wasm store canister | canister runtime | `crates/canic-wasm-store/src/lib.rs` | no Rust library target | special canister artifact | `cdylib` package, `start_wasm_store!()` | runtime-only wasm-store surface, not an alternate Rust facade. | Low |
| Internal Canic harness | module family | `crates/canic-testing-internal/src/lib.rs` | workspace-reachable only | internal test harness | `publish = false`, `pub mod canister/pic` | intentionally centralizes Canic-specific PocketIC setup outside `ic-testkit`. | Low |
| Generic testkit | module family | `../ic-testkit/crates/ic-testkit/src/lib.rs`, `src/pic/mod.rs` | yes | public generic test infrastructure | `pub mod`, `pub use`, `pub struct`, `pub fn` | generic PocketIC helpers remain Canic-free; no Canic dependency found. | Low |

## 2. Subsystem Dependency Graph

| Subsystem / Crate | Depends On | Depended On By | Lower-Layer Dependencies | Same-Layer Dependencies | Upward Dependency Found? | Direction Assessment | Risk |
| --- | --- | --- | ---: | ---: | --- | --- | --- |
| `canic` | `canic-core`, `canic-control-plane` behind feature, `canic-macros` | fleet/test/audit/sandbox canisters, examples, tests | 3 | 0 | no | facade direction remains clean; hidden support roots are macro/build plumbing. | Low |
| `canic-core` | IC/CDK/storage/memory/runtime dependencies | `canic`, `canic-control-plane`, `canic-host`, tests | many third-party/platform edges | 0 crate edges | no | core runtime remains below facade and above storage/infra internally. | Low |
| `canic-control-plane` | `canic-core`, storage/serialization/hash support | `canic` feature and root/store canisters | 1 Canic edge | 0 | no | control-plane runtime support stays below the facade. | Low |
| `canic-host` | `canic-core`, serialization/filesystem/process support | `canic-cli` | 1 Canic edge | 0 | no | operator mechanics remain facade-free and host-owned. | Medium |
| `canic-cli` | `canic-core`, `canic-host`, `canic-backup` | binary entrypoint | 3 Canic package edges | 0 | no | CLI owns UX and dispatch; private command modules call host/backup support. | Medium |
| `canic-backup` | serialization/hash/time support | `canic-cli` | 0 Canic runtime edges | 0 | no | backup domain remains independent from canister runtime/facade crates. | Low |
| `canic-macros` | `syn`, `quote`, `proc_macro2` | `canic` | 0 Canic edges | 0 | no | proc macro crate does not depend on runtime internals. | Low |
| `canic-wasm-store` | `canic` runtime facade | installed as special canister | 1 facade edge | 0 | no | runtime artifact only; no Rust library reuse surface. | Low |
| `canic-testing-internal` | `ic-testkit`, `canic`, `canic-core`, `canic-control-plane` | `canic-tests` | test-only edges | 0 | no product edge | repo-only test harness remains one-way. | Low |
| `../ic-testkit` | `pocket-ic`, `candid`, generic support crates | `canic-testing-internal`, Canic tests | 0 Canic edges | 0 | no | generic testkit does not encode Canic runtime semantics. | Low |
| fleets/test/audit/sandbox canisters | `canic`; selected fixtures use test-only support | tests/install tooling | facade/runtime edges only | 0 | no | canister categories stay separate and explicitly unpublished. | Low |

## 3. Circularity Findings

| Subsystem A | Subsystem B | Real Cycle? | Evidence | Risk |
| --- | --- | --- | --- | --- |
| `canic` | `canic-core` | no | `canic` depends on `canic-core`; `canic-core` does not depend on `canic`. | Low |
| `canic-host` | `canic-cli` | no | CLI depends on host; host has no CLI dependency. | Low |
| `canic-backup` | `canic-cli` | no | CLI depends on backup; backup has no CLI dependency. | Low |
| `canic-testing-internal` | `../ic-testkit` | no | internal harness depends on `ic-testkit`; `ic-testkit` has no Canic references. | Low |
| fleets/test/audit/sandbox | product crates | no | manifests keep canister artifacts as consumers of `canic`, not providers to product crates. | Low |

## 4. Visibility Hygiene Findings

| Item | Path | Current Visibility | Narrowest Plausible Visibility | Why Narrower Seems Valid | Risk |
| --- | --- | --- | --- | --- | --- |
| core hidden support roots | `crates/canic-core/src/lib.rs` (`access`, `bootstrap`, `control_plane_support`, `dispatch`, `error`, `ingress`, `shared_support`, `__reexports`) | `#[doc(hidden)] pub mod` | keep current | macro expansion, facade support, and control-plane integration need stable root paths. | Low |
| core execution roots | `crates/canic-core/src/lib.rs` (`config`, `domain`, `infra`, `lifecycle`, `ops`, `storage`, `view`, `workflow`) | `pub(crate) mod` | keep current | execution/storage/workflow internals are not root-public. | Low |
| host deployment truth API | `crates/canic-host/src/deployment_truth/mod.rs` | many `pub use` re-exports from private submodules | keep current until the deployment-truth contract is split or stabilized | the API is intentionally host-owned but now very broad. | Medium |
| CLI deploy command internals | `crates/canic-cli/src/deploy/mod.rs` | private module | split when active | not public, but one private command file coordinates many deployment-truth flows. | Medium |
| backup root modules | `crates/canic-backup/src/lib.rs` | `pub mod` | keep current | backup/restore is a package contract; no runtime/facade leak was confirmed. | Low |

### Under-Containment Signals

| Area | Signal | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |
| host deployment truth | large host-owned public seam | `deployment_truth/mod.rs = 312` lines of re-export surface; implementation files include `promotion.rs = 5279`, `lifecycle.rs = 4017`, `text.rs = 2613`, `report.rs = 2416`, `model.rs = 2307` | Pressure: role-aligned but broad; no direction breach. | Medium |
| CLI deploy | large private command hub | `crates/canic-cli/src/deploy/mod.rs = 6594` lines and imports a wide `canic_host::deployment_truth` surface | Pressure: private UX orchestration, but high change friction. | Medium |
| host install root | large private/public host install module | `crates/canic-host/src/install_root/mod.rs = 3009`; tests are larger but test-only | Pressure: host-owned install mechanics, no facade leak. | Medium |
| core access expression | central access expression model | `crates/canic-core/src/access/expr/mod.rs = 623`; imports `access`, `ids`, `log`, and CDK `Principal` | Pressure: endpoint auth boundary model; no storage/workflow dependency. | Low |
| control-plane storage/template ops | large control-plane storage support | `ops/storage/template/chunked.rs = 827`, `ops/storage/template/mod.rs = 726`, `workflow/bootstrap/root.rs = 962` | Pressure: control-plane-owned mechanics, no public facade leak. | Medium |

### Test Leakage

| Item | Location | Leakage Type | Build Impact | Risk |
| --- | --- | --- | --- | --- |
| Canic internal harness | `crates/canic-testing-internal` | workspace-only test crate | `publish = false`; only `canic-tests` depends on it | Low |
| generic testkit | `../ic-testkit` | public generic test infra | no `canic` or `canic-testing-internal` references found | Low |
| runtime probe dev dependency | `canisters/test/runtime_probe/Cargo.toml` | dev-only use of `ic-testkit` | confined to test canister dev surface | Low |
| audit/test canister paths in tests | `crates/canic-tests/tests/instruction_audit_support/report.rs` | test report references | test-only reporting, not runtime/fleet dependency | Low |

## 5. Layering Violations

| Layer / Rule | Upward Dependency Found? | Description | Risk |
| --- | --- | --- | --- |
| `storage` must not depend on workflow/policy/ops | no | scans found storage-local prelude and storage-local references only. | Low |
| `ops` may depend on storage but not workflow | no production breach | ops references storage records and stable stores; no workflow dependency was found outside same-ops/test references. | Low |
| `domain`/policy must not mutate or call runtime side effects | no | `domain/policy` imports value/view shapes, not ops/workflow/storage side effects. | Low |
| `workflow` should use ops instead of storage internals | no high-confidence breach | one production reference to `AppStateOps::cycles_funding_enabled()` in `workflow/rpc/request/handler/nonroot_cycles.rs`; this is ops-mediated state access, not storage bypass. | Low |
| `../ic-testkit` must not encode Canic runtime semantics | no | no Canic references found in `../ic-testkit/crates/ic-testkit`. | Low |
| facade crates must not expose storage/replay internals accidentally | no | `canic` exposes stable facade DTO/API/ID/CDK/memory paths plus hidden macro support; `storage` and `workflow` are not public through `canic`. | Low |

## 6. Structural Pressure Areas

| Area | Pressure Type | Why This Is Pressure (Not Yet Violation) | Drift Sensitivity | Risk |
| --- | --- | --- | --- | --- |
| `crates/canic-host/src/deployment_truth/*` | public host coordination surface | many model/report/lifecycle/promotion artifacts are public through one host module, but ownership is explicit and host-only. | high when adding new deployment-truth phases or receipt families | Medium |
| `crates/canic-cli/src/deploy/mod.rs` | private CLI UX hub | the module imports and dispatches most deployment-truth command families; it is private and does not widen public API. | high when adding deploy subcommands | Medium |
| `crates/canic-host/src/install_root/mod.rs` | host install mechanics hub | root install flow remains in host, not CLI or backup, but the module is large. | medium when adding install flags/state transitions | Medium |
| `crates/canic-core/src/api/ic/canic.rs` | core API facade file | large API facade over IC call helpers; public but core-owned. | medium when adding IC API methods | Medium |
| `crates/canic-core/src/access/expr/mod.rs` | endpoint auth expression model | central by design and imports only access/value/log surfaces. | medium when adding auth predicates | Low |

### Hub Import Pressure

| Hub Module | Top Imported Sibling Subsystems | Unique Sibling Subsystems Imported | Cross-Layer Dependency Count | Delta vs Previous Report | HIP | Pressure Band | Risk |
| --- | --- | ---: | ---: | --- | ---: | --- | --- |
| `crates/canic-core/src/access/expr/mod.rs` | `access`, `ids`, `log`, `cdk` | 4 | 1 | path-comparable; still a central access boundary model | 0.25 | low | Low |
| `../ic-testkit/crates/ic-testkit/src/pic/mod.rs` | `baseline`, `calls`, `diagnostics`, `errors`, `lifecycle`, `process_lock`, `runtime`, `snapshot`, `standalone`, `startup` | 10 | 0 | non-comparable path vs old in-repo `canic-testkit`; current module is Canic-free | 0.00 | low | Low |
| `crates/canic-testing-internal/src/pic/mod.rs` | `artifacts`, `attestation`, `audit`, `canic`, `delegation`, `lifecycle`, `root` | 7 | 1 | current Canic-only harness seam; no public product API | 0.14 | low | Low |
| `crates/canic-host/src/deployment_truth/mod.rs` | `authority`, `executor`, `lifecycle`, `model`, `multi`, `observe`, `plan`, `promotion`, `receipt`, `report`, `root`, `text` | 12 | 1 | new broad 0.47/0.48 host surface, not comparable to baseline counts | 0.08 | low by HIP, medium by public-surface breadth | Medium |
| `crates/canic-cli/src/deploy/mod.rs` | `cli`, `canic_host::deployment_truth`, `canic_host::install_root`, host config/build | 4 ownership roots | 2 | new broad deployment UX hub, not comparable to baseline counts | 0.50 | moderate | Medium |

## 7. Drift Sensitivity Summary

| Growth Vector | Affected Subsystems | Why Multiple Layers Would Change | Drift Risk |
| --- | --- | --- | --- |
| new deployment-truth phase or receipt family | `canic-host::deployment_truth`, `canic-cli::deploy`, changelog/docs/tests | host model/validation/text/rendering plus CLI dispatch are currently coupled through a broad public module. | Medium |
| new deploy subcommand | `canic-cli::deploy`, `canic-host` | command parsing, JSON/text rendering, and host validation helpers tend to land in one private CLI file. | Medium |
| new install-root state transition | `canic-host::install_root`, `canic-cli::install/deploy`, tests | host install state and CLI invocation may need coordinated changes. | Medium |
| new endpoint auth predicate | `canic-core::access/expr`, endpoint macro generation, auth tests | predicate model, evaluator, metrics/log labels, and endpoint macro guards move together. | Low |
| new generic PocketIC helper | `../ic-testkit`, `canic-testing-internal`, `canic-tests` | generic helpers must remain Canic-free while Canic-specific topology/bootstrap stays internal. | Low |

## 8. Structural Risk Index

| Category | Risk Index | Basis |
| --- | ---: | --- |
| Public Surface Discipline | 4 / 10 | `canic` facade and core roots are intentional; host deployment-truth public surface is broad. |
| Layer Directionality | 2 / 10 | no upward crate dependency, storage/workflow bypass, or policy side-effect breach confirmed. |
| Circularity Safety | 1 / 10 | no real crate or subsystem cycle found. |
| Visibility Hygiene | 4 / 10 | large private/public host and CLI hubs create pressure, but no accidental public leak was confirmed. |
| Facade Containment | 2 / 10 | `canic` remains the facade; `canic-host`, `canic-backup`, and `ic-testkit` do not become canister-runtime facades. |

Overall structural risk index: **4 / 10**.

This is moderate pressure, not structural failure. The score is driven by
large deployment-truth/deploy/install coordination files and broad host public
re-exports. It is not driven by dependency direction, circularity, module
discovery, or product/test seam leakage.

## 9. Verification Readout

| Check | Status | Notes |
| --- | --- | --- |
| recurring definition review | PASS | reviewed `docs/audits/recurring/system/module-structure.md`. |
| baseline review | PASS | compared against `docs/audits/reports/2026-05/2026-05-10/module-structure.md`; exact deltas marked non-comparable. |
| crate-root public surface scan | PASS | scanned public roots for `canic`, `canic-core`, `canic-control-plane`, `canic-host`, `canic-cli`, `canic-backup`, `canic-macros`, `canic-wasm-store`, `canic-testing-internal`, and sibling `ic-testkit`. |
| module-discovery scan | PASS | no `foo.rs` plus `foo/mod.rs` duplicates found; no `#[path = ...]` usage found. |
| include/path scan | PASS | only build-generated `include!(env!(...))` calls in `crates/canic/src/macros/start.rs` were found. |
| dependency direction scan | PASS | `cargo metadata --no-deps --format-version 1`; no cycle or reverse product/test dependency found. |
| test/fleet/audit seam scan | PASS | `ic-testkit` appears only in `canic-testing-internal`, `canic-tests`, and a test canister dev dependency; no runtime/product leak found. |
| layer-reference scan | PASS | no storage-to-workflow, policy-to-ops, or workflow-to-storage-internal breach confirmed. |
| hub line-count scan | PASS | line-count scan identified pressure files but no High/Critical violation. |

Commands run:

- `sed -n '1,260p' docs/audits/recurring/system/module-structure.md`
- `sed -n '260,620p' docs/audits/recurring/system/module-structure.md`
- `sed -n '620,980p' docs/audits/recurring/system/module-structure.md`
- `sed -n '1,260p' docs/audits/reports/2026-05/2026-05-10/module-structure.md`
- `git rev-parse --short HEAD`
- `git status --short`
- `rg "canic-testkit|canic-cdk|canic-memory" docs/audits/recurring/system/module-structure.md docs/audits/reports/2026-05/2026-05-10/module-structure.md docs/audits/reports/2026-05/summary.md`
- `rg "^members|canic-testkit|canic-cdk|canic-memory|ic-testkit" Cargo.toml crates -g 'Cargo.toml' -n`
- `find crates canisters fleets -name '*.rs' -print`
- `rg "#\\[path\\s*=|include!|pub\\s+mod|pub\\s+use|macro_rules!|#\\[macro_export\\]" crates canisters fleets -g '*.rs' -n`
- `find crates canisters fleets -type f -name '*.rs' -exec wc -l {} +`
- `rg "#\\[path\\s*=|include!\\(" crates canisters fleets ../ic-testkit -g '*.rs' -n`
- `find crates canisters fleets -type f -name '*.rs' -exec wc -l {} + | sort -nr | sed -n '1,80p'`
- `find crates canisters fleets -type f -name '*.rs' -print | sed 's#\\.rs$##; s#/mod$##' | sort | uniq -d`
- `cargo metadata --no-deps --format-version 1`
- `rg "canic-testing-internal|canic_tests|canic-tests|canisters/audit|canisters/test|canisters/sandbox|fleets/demo|fleets/test" crates canisters fleets -g 'Cargo.toml' -g '*.rs' -n`
- `rg "crate::(workflow|ops|storage|infra)|super::(workflow|ops|storage|infra)|canic_core::(workflow|ops|storage|infra)" crates/canic-core/src/domain crates/canic-core/src/storage crates/canic-core/src/ops crates/canic-core/src/workflow -g '*.rs' -n`
- `rg "^(#\\[doc\\(hidden\\)\\]\\s*)?pub (mod|use|struct|enum|trait|type|const|fn)|^#\\[macro_export\\]|^#\\[proc_macro" crates/canic/src/lib.rs crates/canic-core/src/lib.rs crates/canic-control-plane/src/lib.rs crates/canic-host/src/lib.rs crates/canic-cli/src/lib.rs crates/canic-backup/src/lib.rs crates/canic-macros/src/lib.rs crates/canic-wasm-store/src/lib.rs crates/canic-testing-internal/src/lib.rs ../ic-testkit/crates/ic-testkit/src/lib.rs ../ic-testkit/crates/ic-testkit/src/pic/mod.rs -n`
- `rg "canic_testing_internal|canic-testing-internal|canic_tests|ic_testkit|ic-testkit" crates/canic crates/canic-core crates/canic-control-plane crates/canic-host crates/canic-cli crates/canic-backup crates/canic-macros crates/canic-wasm-store canisters fleets -g 'Cargo.toml' -g '*.rs' -n`
- `rg "crate::(workflow|ops|storage)|super::(workflow|ops|storage)|canic_core::(workflow|ops|storage)" crates/canic-core/src/domain crates/canic-core/src/storage crates/canic-core/src/ops -g '*.rs' -n`
- `rg "crate::storage|canic_core::storage|crate::ops::storage" crates/canic-core/src/workflow -g '*.rs' -n`

## Follow-up Actions

1. Host maintainers: split `canic-host::deployment_truth` implementation files
   before adding another broad phase family; keep model, validation, text, and
   promotion responsibilities in separate owner files.
2. CLI maintainers: split `crates/canic-cli/src/deploy/mod.rs` before adding
   more deployment-truth commands, preferably by command family or rendering
   boundary.
3. Host maintainers: keep install-root state transitions in `canic-host` and
   avoid moving host mechanics into CLI command modules.
4. Testkit maintainers: keep `../ic-testkit` Canic-free; Canic topology,
   readiness, and bootstrap semantics belong in `canic-testing-internal`.
