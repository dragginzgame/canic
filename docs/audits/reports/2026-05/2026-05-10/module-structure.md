# Module Structure Audit - 2026-05-10

## Report Preamble

- Scope: `crates/canic`, `crates/canic-core`, `crates/canic-control-plane`,
  `crates/canic-wasm-store`, `crates/canic-cdk`, `crates/canic-memory`,
  `crates/canic-testkit`, `crates/canic-testing-internal`,
  `crates/canic-tests`, `crates/canic-host`, `crates/canic-cli`,
  `crates/canic-backup`, `fleets/**`, `canisters/test/**`,
  `canisters/audit/**`, and `canisters/sandbox/**`.
- Compared baseline report path:
  `docs/audits/reports/2026-04/2026-04-06/module-structure.md`
- Code snapshot identifier: `7e0ec893` plus current 0.33.5 cleanup worktree
- Method tag/version: `module-structure-v2`
- Comparability status: non-comparable: scope expanded to include the 0.33
  operator crates (`canic-host`, `canic-cli`, `canic-backup`) and top-level
  `fleets/**` after the ICP CLI hard cut.
- Exclusions applied: generated target outputs, `.icp` runtime cache, historical
  audit reports except the compared baseline, and test internals except
  explicit test/fleet/audit seam checks.
- Notable methodology changes vs baseline: expanded package/crate-root
  inspection to the operator crates because they are now published package
  surfaces rather than incidental local helpers.

## Structural Hotspots

| Area | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- |
| `canic-control-plane` publication workflow phase files | `publication/mod.rs` is now `153` lines and the old `publication/release.rs = 845`, `publication/fleet.rs = 704`, and `publication/lifecycle.rs = 540` files are split. Current largest production files are `release/managed.rs = 275`, `lifecycle/binding.rs = 206`, `lifecycle/gc.rs = 194`, and `fleet/placement.rs = 191`. | Remediated: release, fleet placement, and lifecycle/GC coordination now use focused modules with ordinary Rust module discovery. | Low |
| `canic-core` provisioning and IC management splits | The old `workflow/ic/provision.rs = 697` and `infra/ic/mgmt.rs = 612` files are gone. Current largest split files are `provision/allocation.rs = 193`, `provision/install.rs = 138`, `infra/ic/mgmt/types.rs = 296`, and `infra/ic/mgmt/lifecycle.rs = 186`. | Remediated: planned directory-module split completed with ordinary Rust module discovery and no new public API shape. | Low |
| `canic-backup` restore runner/apply journal splits | The old `restore/runner/mod.rs = 1204` file is split into runner modules topped by `types.rs = 525` and `execute.rs = 337`. `restore/apply/journal/mod.rs` is down to `422` lines, with operation/error types in `types.rs = 353` and counters in `counts.rs = 109`. | Remediated: restore runner execution, preview, IO/locking, status, and preconditions now use normal focused modules; apply journal root now owns state transitions instead of all operation DTOs. | Low |
| `canic` macro/build support splits | The old `macros/endpoints.rs` and `build_support.rs` hubs are gone. Current roots are dispatch-only: `macros/endpoints/mod.rs = 6` and `build_support/mod.rs = 10`. | Remediated: hidden macro/build support remains root-reachable but is now decomposed into focused files. | Low |
| `canic-host` operator phase files | `install_root/mod.rs = 793`, `release_set/mod.rs = 741`, `icp.rs = 600`, `release_set/stage.rs = 540`, and `release_set/config.rs = 527`. | Pressure: host package surface is role-aligned and facade-free, but its install/release helpers are now the main operator-code hubs. | Medium |
| Auth access boundary state touch | `crates/canic-core/src/access/auth/identity.rs` resolves delegated sessions through `AuthStateOps`, clears invalid sessions, records metrics, and reads `EnvOps`. | Pressure: this is an intentional endpoint-auth boundary, but it mixes access evaluation with lower-layer state cleanup and should not spread to general policy modules. | Medium |

## Hub Module Pressure

| Hub Module | Top Imported Sibling Subsystems / Surfaces | Unique Sibling Subsystems Imported | Cross-Layer Dependency Count | Delta vs Previous Report | HIP | Pressure Band | Risk |
| --- | --- | ---: | ---: | --- | ---: | --- | --- |
| `crates/canic-core/src/lib.rs` | public `api`, `dto`, `ids`, `log`, `perf`, `protocol`; hidden `access`, `bootstrap`, `dispatch`, `error`, `ingress`; `canic_memory` re-exports | 6 | 1 | Stable/improved shape from the April report: support roots remain `#[doc(hidden)]`, internal roots remain `pub(crate)`. | 0.25 | low | Low |
| `crates/canic/src/lib.rs` | facade modules `access`, `api`, `dto`, `ids`, `prelude`, `protocol`; hidden `__internal`, `__build`; `cdk`, `memory`, macros, `Error` re-exports | 6 | 1 | Build and endpoint macro support are now decomposed under hidden roots; root surface remains intentional facade API. | 0.25 | low | Low |
| `crates/canic-testkit/src/pic/mod.rs` | `baseline`, `errors`, `process_lock`, `readiness`, `startup`, `standalone` re-exports; `candid`, `canic`, `pocket_ic` imports | 6 | 1 | Improved vs April: root file is now `285` lines, down from `349`; still the intended public PocketIC seam. | 0.17 | low | Low |
| `crates/canic-host/src/lib.rs` | public modules `canister_build`, `format`, `icp`, `install_root`, `release_set`, `replica_query`, `table`; private artifact/workspace helpers | 7 | 1 | New in scope for this method version; host is now a real published operator support library. | 0.42 | medium | Medium |
| `crates/canic-backup/src/lib.rs` | public modules `artifacts`, `discovery`, `journal`, `manifest`, `persistence`, `restore`, `snapshot`, `timestamp`, `topology` | 9 | 1 | New in scope for this method version; restore runner/apply-journal internals are now decomposed, while the package root remains broad by domain. | 0.33 | medium | Low |

## Public Surface Map

| Item | Kind | Path | Publicly Reachable From Root? | Classification | Visibility Scope | Exposure Impact | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Canic facade modules | module family | `crates/canic/src/lib.rs` (`access`, `api`, `dto`, `ids`, `prelude`, `protocol`) | yes | intended external API | `pub mod` | primary canister-facing facade remains broad but intentional. | Low |
| Canic macro/build support | hidden module family | `crates/canic/src/lib.rs` (`__internal`, `__build`) | yes | macro/build support | `#[doc(hidden)] pub mod` | required for generated code and build scripts; hidden from ordinary docs. | Low |
| Core published roots | module family | `crates/canic-core/src/lib.rs` (`api`, `dto`, `ids`, `log`, `perf`, `protocol`) | yes | lower-level support API | `pub mod` | still a secondary public surface below `canic`, but ordinary internals are not root-published. | Medium |
| Core hidden support roots | module family | `crates/canic-core/src/lib.rs` (`access`, `bootstrap`, `dispatch`, `error`, `ingress`, `__control_plane_core`) | yes | facade/macro/build support | `#[doc(hidden)] pub mod` | root-reachable for macro/control-plane support, not presented as normal public API. | Low |
| Memory support crate | module family | `crates/canic-memory/src/lib.rs` (`api`, `registry`, `serialize`; hidden `manager`, `runtime`) | yes | stable-memory support API | mixed `pub mod` and hidden `pub mod` | public API remains small; backend manager/runtime stay hidden. | Low |
| Testkit PocketIC facade | module family | `crates/canic-testkit/src/lib.rs`, `src/pic/mod.rs` | yes | public test infrastructure | `pub mod` and re-exports | generic testkit remains separate from Canic-only internal harnesses. | Low |
| Operator host library | module family | `crates/canic-host/src/lib.rs` | yes | operator support API | `pub mod` | now a published host/fleet/install support library; surface is broad but role-aligned. | Medium |
| Operator CLI library | public functions/types | `crates/canic-cli/src/lib.rs` (`CliError`, `run`, `run_from_env`, `top_level_command`, `version_text`) | yes | binary support API | `pub enum`, `pub fn`, `pub const fn` | compact programmatic entry surface for the installed `canic` binary. | Low |
| Backup library | module family | `crates/canic-backup/src/lib.rs` | yes | backup/restore package API | `pub mod` | domain-broad but package role is explicit. | Low |

## Public Field Exposure

| Type | Public Fields? | Representation Leakage? | Stable DTO/Facade Contract? | Exposure Impact | Risk |
| --- | --- | --- | --- | --- | --- |
| DTO and ID families | yes, by design | no | yes | public transfer contracts remain grouped under `dto`/`ids` rather than storage/workflow modules. | Low |
| `canic-core::bootstrap::EmbeddedRootBootstrapEntry` | yes | mild | support contract | hidden bootstrap module exposes build-produced artifact metadata for host/bootstrap support. | Low |
| Metrics/config DTOs added in active 0.33 work | yes | no | yes | `MetricsProfile` and related config schema are config contracts, not storage internals. | Low |
| `canic-backup` manifest/journal/topology records | yes | no confirmed leak in this run | yes | backup package owns these persistence/manifest contracts. | Low |

No high-risk public field exposure was confirmed. The current pressure is broad
root surfaces and hub concentration, not accidental storage/replay record
publication.

## Subsystem Dependency Graph

| Subsystem / Crate | Depends On | Depended On By | Lower-Layer Dependencies | Same-Layer Dependencies | Upward Dependency Found? | Direction Assessment | Risk |
| --- | --- | --- | ---: | ---: | --- | --- | --- |
| `canic` | `canic-core`, `canic-cdk`, `canic-memory`, optional `canic-control-plane`, `canic-macros` | fleets, probes, test canisters, and testkit consumers | 5 | 0 | no | facade direction remains clean. | Low |
| `canic-core` | `canic-cdk`, `canic-memory`; build-time `proc-macro2`/`quote`/`toml`; dev-only test tools | `canic`, `canic-control-plane`, `canic-testing-internal`, `canic-tests` | 2 runtime | 0 | no runtime reverse edge | runtime graph remains downward; build-only config rendering is contained. | Low |
| `canic-control-plane` | `canic-core`, `canic-cdk`, `canic-memory` | optional `canic` feature and root/store canisters | 3 | 0 | no | root/store control-plane support layer is correctly below the facade. | Low |
| `canic-host` | `canic-core`, host serialization/compression crates | `canic-cli` | 1 Canic edge | 0 | no | operator-host support now stays on the narrower core/data dependency set. | Low |
| `canic-cli` | `canic-core`, `canic-backup`, `canic-host` | installed binary entrypoint | 3 | 0 | no | CLI owns UX and routes to core/host/backup without linking the canister facade. | Low |
| `canic-backup` | `candid`, `serde`, `serde_json`, `sha2`, `thiserror` | `canic-cli` | 0 Canic runtime edges | 0 | no | backup domain stays independent of canister runtime crates. | Low |
| `canic-testkit` | `canic`, `pocket-ic`, `candid`, `serde` | `canic-testing-internal`, `canic-tests`, dev consumers | 1 Canic facade edge | 0 | no | public generic testkit does not depend on internal harnesses. | Low |
| `canic-testing-internal` | `canic-testkit`, `canic`, `canic-core`, `canic-control-plane` | `canic-tests` | 4 | 0 | no | internal-only harness remains one-way and `publish = false`. | Low |
| fleets/test/audit/sandbox canisters | `canic` facade; selected test stubs depend on `canic-core`/`canic-control-plane` | tests/install tooling | 1-2 | 0 | no product reverse edge | non-product canister categories remain separated. | Low |

## Circularity Findings

| Subsystem A | Subsystem B | Real Cycle? | Evidence | Risk |
| --- | --- | --- | --- | --- |
| `canic` | `canic-core` | no | `canic` depends on `canic-core`; `canic-core` has no dependency on `canic`. | Low |
| `canic-testkit` | `canic-testing-internal` | no | `canic-testing-internal` depends on `canic-testkit`; `canic-testkit` has no reverse dependency. | Low |
| `canic-cli` | `canic-host` | no | CLI depends on host; host has no CLI dependency. | Low |
| fleet canisters | audit/test/sandbox canisters | no | manifests keep `fleets/**`, `canisters/test/**`, `canisters/audit/**`, and `canisters/sandbox/**` separate; no fleet manifest depends on audit/test canisters. | Low |

No real crate-level or subsystem-level cycle was confirmed.

## Implementation Leakage

| Violation | Location | Dependency | Description | Directional Impact | Risk |
| --- | --- | --- | --- | --- | --- |
| no confirmed implementation leak above pressure level | `crates/canic-core/src/lib.rs`, `crates/canic/src/lib.rs`, `crates/canic-host/src/lib.rs` | hidden support roots and explicit public modules | public roots are broad, but exposed support seams are either hidden macro/build support or package-owned operator surfaces. | no confirmed direction breach | Low |

Notable pressure:

- `crates/canic-core/src/access/auth/identity.rs` directly uses
  `AuthStateOps`, `SubnetRegistryOps`, `EnvOps`, metrics, and `IcOps` while
  resolving authenticated identity. This is still inside the endpoint access
  boundary, but it should remain isolated there and should not become a pattern
  for general policy modules.
- `crates/canic/src/build_support/metrics.rs` imports
  `canic_core::bootstrap::compiled::MetricsProfile` for build-time metrics
  profile cfg emission. That is acceptable hidden build support, and the
  surrounding `__build` root now delegates through focused modules.

## Visibility Hygiene

### Overexposure

| Item | Path | Current Visibility | Narrowest Plausible Visibility | Why Narrower Seems Valid | Risk |
| --- | --- | --- | --- | --- | --- |
| core hidden support roots | `crates/canic-core/src/lib.rs` | `#[doc(hidden)] pub mod` | keep current | macro/build/control-plane support still needs root reachability. | Low |
| memory backend roots | `crates/canic-memory/src/lib.rs` | `#[doc(hidden)] pub mod manager/runtime` | keep current | macros/bootstrap require paths; ordinary root re-exports remain absent. | Low |
| host operator modules | `crates/canic-host/src/lib.rs` | `pub mod canister_build`, `icp`, `install_root`, `release_set`, `replica_query`, `table` | review only after package docs settle | CLI consumes these as a host library; no narrower call graph judgment without a package-contract decision. | Medium |
| control-plane publication workflow | `crates/canic-control-plane/src/workflow/runtime/template/publication/mod.rs` | internal module, decomposed phase files | keep current | release, fleet, and lifecycle coordination are now split by concern; visibility is not public. | Low |

### Under-Containment Signals

| Area | Signal | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |
| `canic-control-plane` publication | release, fleet, and lifecycle publication phases split into focused modules | largest production files are `publication/release/managed.rs = 275`, `publication/lifecycle/binding.rs = 206`, `publication/lifecycle/gc.rs = 194`, and `publication/fleet/placement.rs = 191` | Remediated | Low |
| `canic-core` provisioning | install/create/register propagation flow split into phase modules | largest provision files are `allocation.rs = 193` and `install.rs = 138` | Remediated | Low |
| `canic-core` IC management | management canister adapter split into request types plus lifecycle/cycles/snapshot/status/settings modules | largest management files are `types.rs = 296` and `lifecycle.rs = 186` | Remediated | Low |
| `canic` endpoint macro bundle | generated endpoint macro support split into directory modules | root `macros/endpoints/mod.rs = 6`; implementation files remain hidden macro support | Remediated | Low |
| `canic-host` package surface | host root exposes seven public modules | `crates/canic-host/src/lib.rs` public module scan | Pressure | Medium |
| `canic-backup` restore state machine | restore runner/apply-journal hubs split into focused modules | largest implementation files are now `restore/runner/types.rs = 525`, `restore/apply/journal/mod.rs = 422`, and `restore/runner/execute.rs = 337` | Remediated | Low |
| `canic-testkit` public PocketIC seam | public root remains centralized but smaller than baseline | `crates/canic-testkit/src/pic/mod.rs = 285` lines, down from `349` in the April report | Pressure | Low |

### Test Leakage

| Item | Location | Leakage Type | Build Impact | Risk |
| --- | --- | --- | --- | --- |
| `canic-core::test` | `crates/canic-core/src/lib.rs` | test-only namespace | `#[cfg(test)] pub mod test` only | Low |
| internal harness crate | `crates/canic-testing-internal/Cargo.toml` | intentionally unpublished harness | `publish = false`, depended on by `canic-tests` | Low |
| runtime probe dev dependency | `canisters/test/runtime_probe/Cargo.toml` | test canister depends on `canic-testkit` as dev-only | confined to `canisters/test/**` | Low |

No non-test runtime import of `canic-testing-internal` or `canic-tests` was
confirmed.

## Layering Violations

| Layer / Rule | Upward Dependency Found? | Description | Risk |
| --- | --- | --- | --- |
| crate dependency direction | no | `cargo metadata --no-deps` and manifest scans show facade/operator/test dependencies remain one-way. | Low |
| runtime storage bypass | no high-confidence violation | production workflow references to storage are limited; one replay handler imports a pure `ReplaySlotKey` stable key type, and nonroot cycles reads app-state through `AppStateOps`. | Low |
| policy/access side effects | no generalized policy breach, but auth boundary pressure exists | `access/auth/identity.rs` uses ops/storage/env/metrics while resolving delegated sessions; acceptable only because it is endpoint access-boundary behavior. | Medium |
| DTO/value boundaries | no | DTO/ID/config roots remain value/support layers rather than execution owners. | Low |

## Early Warning Signals

| Signal | Current Evidence | Risk |
| --- | --- | --- |
| enum shock radius | active metrics-profile work added `MetricsProfile` and build cfg routing, but the endpoint Candid enum stayed stable; no broad enum fan-out beyond metrics/build/render/config was observed. | Low |
| cross-layer struct spread | `MetricsProfile` appears in config schema, bootstrap render/re-export, and build support; this is expected config/build flow, not storage/workflow leakage. | Low |
| hub growth | largest current production files are now template storage, backup manifest/snapshot planning, and `canic-host` install/release helpers; `canic-core` IC management/provisioning, `canic` macro/build support, control-plane publication, and backup restore runner/apply-journal internals were split. | Medium |
| capability surface growth | no new capability endpoint family was observed in this module-structure run. | Low |
| operator package surface growth | `canic-host`, `canic-cli`, and `canic-backup` are now visible published package roots and should stay in future module-structure scope. | Medium |

## Dependency Fan-In Pressure

| Surface | Fan-In Evidence | Assessment | Risk |
| --- | --- | --- | --- |
| `canic` facade | depended on by fleets, probes, test canisters, `canic-testkit`, and workspace tests | intended central facade; pressure is normal for the primary public crate. | Low |
| `canic-core` | depended on by `canic`, `canic-control-plane`, `canic-host`, `canic-cli`, internal test harnesses, and selected test fixtures | lower-level support API remains broad; keep resisting convenience root exports. | Medium |
| `canic-host` | depended on by `canic-cli` and owns install/build/list projection helpers | host-library surface now matters for CLI/package boundaries. | Medium |
| `canic-testkit` | depended on by `canic-testing-internal`, `canic-tests`, and selected dev/test fixtures | correct generic test-infrastructure fan-in. | Low |

## Risk Score

| Category | Risk Index | Basis |
| --- | ---: | --- |
| Public Surface Discipline | 4 / 10 | core/facade support roots remain hidden, but operator support crates are now in published scope and need continued package-boundary discipline. |
| Layer Directionality | 2 / 10 | no crate cycle or clear upward dependency was confirmed. |
| Circularity Safety | 1 / 10 | no real crate/subsystem cycle found. |
| Visibility Hygiene | 3 / 10 | broad but mostly intentional public roots; host phase files and backup manifest/snapshot planning are the main containment pressure after the `canic-core`, `canic-control-plane`, and `canic` splits. |
| Facade Containment | 2 / 10 | `canic` hidden macro/build support is decomposed, `canic-core` IC provisioning adapters are contained, and operator host/CLI code avoids linking the canister facade. |

Overall structural risk index: **3 / 10**.

Interpretation:

- no high or critical structural violation was confirmed
- risk is back at the April `3 / 10` level after the module splits, despite the
  expanded 0.33 operator-crate scope
- the main structural pressure is now host phase-file containment and remaining
  backup manifest/snapshot planning, not dependency direction or core/control-plane
  provisioning shape

## Known Intentional Exceptions

| Exception | Why Intentional | Scope Guardrail | Still Valid This Run? |
| --- | --- | --- | --- |
| `canic::__internal` and `canic::__build` | macro/build expansion requires root-reachable support namespaces | `#[doc(hidden)]`; do not treat as downstream contract | yes |
| hidden `canic-core` support roots | facade/build/macro/control-plane support needs stable expansion paths | keep hidden; avoid adding ordinary public convenience exports | yes |
| hidden `canic-memory::manager/runtime` | macros and bootstrap need backend paths | keep backend state out of ordinary root re-exports | yes |
| `canic-testkit::pic` public surface | generic PocketIC support is a public testing contract | keep Canic-only root harnesses in `canic-testing-internal` | yes |
| `canic-host` public modules | CLI and operator workflows need host-side library ownership | keep UX in `canic-cli`; keep filesystem/build/install mechanics in `canic-host` | yes |
| `canic-backup` public modules | backup/restore manifests and journals are the package contract | do not absorb CLI UX or host install mechanics | yes |

## Delta Since Baseline

| Delta Type | Item / Subsystem | Previous | Current | Impact |
| --- | --- | --- | --- | --- |
| scope expansion | operator crates | excluded or not materially assessed | `canic-host`, `canic-cli`, `canic-backup` included | method changed; future module-structure runs should keep them in scope |
| retained containment | `canic-core` root | hidden support roots after April cleanup | hidden support roots still present; internal roots remain `pub(crate)` | no regression |
| reduced testkit hub | `canic-testkit/src/pic/mod.rs` | `349` lines | `285` lines | lower public testkit hub pressure |
| remediated build-support pressure | `canic/src/build_support.rs` | not a highlighted hotspot | split into `build_support/mod.rs`, `config.rs`, `metrics.rs`, and `bootstrap.rs` | hidden build support remains focused |
| new operator-surface pressure | `canic-host/src/lib.rs` | not in baseline | seven public modules | package-boundary watchpoint for future CLI/host work |
| remediated control-plane phase pressure | `canic-control-plane` publication | `publication/release.rs = 845`, `publication/fleet.rs = 704`, `publication/lifecycle.rs = 540` | split into normal directory modules with max production file `release/managed.rs = 275` | publication workflow is now decomposed by release, placement, binding, creation, inventory, and GC concerns |
| remediated IC management/provisioning pressure | `canic-core` IC workflow/infra | `workflow/ic/provision.rs = 697`, `infra/ic/mgmt.rs = 612` | split into normal directory modules, max focused file `296` lines in management types and `193` lines in provisioning allocation | planned 0.33 addendum cleanup completed |
| remediated backup restore pressure | `canic-backup` restore runner/apply journal | `restore/runner/mod.rs = 1204`, `restore/apply/journal/mod.rs = 868` | split into normal directory modules; runner max focused file is `types.rs = 525`, apply journal root is `422`, and all backup crate tests still pass | operator backup/restore domain remains broad but the runner and journal state machine are easier to review |

## Verification Readout

| Check | Status | Notes |
| --- | --- | --- |
| recurring definition review | PASS | `docs/audits/recurring/system/module-structure.md` and audit how-to reviewed before the run. |
| baseline review | PASS | compared against `docs/audits/reports/2026-04/2026-04-06/module-structure.md`. |
| crate root public surface scan | PASS | scanned root `pub mod`/`pub use` surfaces for facade, core, memory, cdk, testkit, internal testing, control-plane, wasm-store, host, CLI, and backup crates. |
| manifest/dependency direction scan | PASS | inspected workspace manifests and `cargo metadata --no-deps`; no reverse dependency cycle found. |
| hub size scan | PASS | `wc -l` over runtime/support Rust files identified current hotspots. |
| cross-layer import scan | PASS | searched access/domain/config/workflow imports for storage/ops/infra pressure; no high-confidence production layer violation found. |
| test/fleet/audit seam scan | PASS | searched manifests and source references for `canic-testing-internal`, `canic-testkit`, `fleets`, `canisters/test`, `canisters/audit`, and `canisters/sandbox`. |
| build verification | PASS | `cargo check -p canic-core`, `cargo clippy -p canic-core --all-targets -- -D warnings`, and `cargo test -p canic-core --lib -- --nocapture` after the `infra/ic/mgmt` and `workflow/ic/provision` splits; `cargo check -p canic-control-plane`, `cargo clippy -p canic-control-plane --all-targets -- -D warnings`, and `cargo test -p canic-control-plane --lib -- --nocapture` after both the publication release split and the publication fleet/lifecycle split; `cargo check -p canic-backup`, `cargo clippy -p canic-backup --all-targets -- -D warnings`, and `cargo test -p canic-backup --lib -- --nocapture` after the backup restore runner/apply-journal split. |

## Follow-up Actions

1. Control-plane maintainers: keep new publication behavior in the focused
   `publication/release/*`, `publication/fleet/*`, and `publication/lifecycle/*`
   modules instead of rebuilding broad phase files.
2. Core/runtime maintainers: keep new management canister calls inside
   `infra/ic/mgmt/*` by concern, and keep root provisioning behavior in the
   focused `workflow/ic/provision/*` phase modules instead of rebuilding a
   monolithic workflow file.
3. Facade/build maintainers: keep metrics/config build helpers contained behind
   hidden `__build`; do not promote build-support types into ordinary public
   API.
4. Operator maintainers: keep `canic-cli` UX logic out of `canic-host`, and keep
   `canic-host` filesystem/build/install mechanics out of `canic-backup`;
   keep restore runner state transitions inside the focused runner/apply-journal
   modules instead of rebuilding monolithic restore helpers.
5. Auth maintainers: keep delegated-session cleanup and metrics side effects
   isolated to the endpoint access boundary; do not copy that pattern into
   general policy modules.
