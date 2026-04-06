# Module Structure Audit - 2026-04-06

## 0. Run Metadata + Comparability Note

- Scope: `crates/canic`, `crates/canic-core`, `crates/canic-cdk`, `crates/canic-memory`, `crates/canic-testkit`, `crates/canic-testing-internal`, `crates/canic-tests`, `canisters/**`, `crates/canic-core/test-canisters/**`, and `crates/canic-core/audit-canisters/**`
- Compared baseline report path: same-day earlier retained run at this path before audit-backed narrowing and `pic` module cleanup
- Code snapshot identifier: `fe9b4d85`
- Method tag/version: `module-structure-v1`
- Comparability status: `comparable`
- Exclusions applied: `#[cfg(test)]` internals except explicit test-leakage checks, generated `.dfx` artifacts, packaged outputs, and non-runtime scripts
- Notable methodology changes vs baseline: no method change; this rerun also reflects the later `canic-testkit::pic` split into `diagnostics`, `snapshot`, `calls`, and `lifecycle`

## 1. Public Surface Map

### 1A. Crate Root Enumeration

| Item | Kind | Path | Publicly Reachable From Root? | Classification | Visibility Scope | Exposure Impact | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- |
| structured facade surface | module family | `crates/canic/src/lib.rs` (`dto`, `ids`, `api`, `access`, `protocol`, `prelude`) | yes | intended external API | `pub mod` | stable facade grouping for downstream canisters | Low |
| macro/build support namespaces | module family | `crates/canic/src/lib.rs` (`__internal`, `__build`) | yes | macro-support item | `pub mod` + `#[doc(hidden)]` | hidden but intentionally root-reachable for macro/build support | Low |
| facade re-exports | re-export family | `crates/canic/src/lib.rs` (`cdk`, `memory`, `Error`, `canic_query`, `canic_update`, memory macros) | yes | intended external API | `pub use` | stable facade surface, no representation leak observed here | Low |
| published core root modules | module family | `crates/canic-core/src/lib.rs` (`api`, `dto`, `ids`, `log`, `perf`, `protocol`) | yes | intended lower-level API / support surface | `pub mod` | broad alternate facade alongside `canic`, but materially narrower than the prior baseline | Medium |
| hidden core support namespaces | module family | `crates/canic-core/src/lib.rs` (`access`, `bootstrap`, `dispatch`, `error`, `__control_plane_core`) | yes | facade-support, macro/build, or endpoint-support item | `pub mod` + `#[doc(hidden)]` | root-reachable support surface remains, but ordinary published exposure was intentionally narrowed | Low |
| generic testkit surface | module family | `crates/canic-testkit/src/lib.rs` (`artifacts`, `pic`, `Fake`) | yes | intended external API | `pub mod`, `pub struct` | public surface is narrow and clearly test-infra-oriented | Low |
| curated IC SDK facade | module/re-export family | `crates/canic-cdk/src/lib.rs` (`candid`, `ic_cdk`, `mgmt`, `timers`, `env`, `spec`, `structures`, `types`, `utils`, `export_candid_debug!`) | yes | intended external API | `pub use`, `pub mod`, `#[macro_export]` | stable facade over upstream IC SDK crates | Low |
| stable-memory support modules | module family | `crates/canic-memory/src/lib.rs` (`macros`, `registry`, `serialize`) | yes | intended external API | `pub mod` | current public surface matches the crate’s stated registry/serialization role | Low |
| memory bootstrap internals | module family | `crates/canic-memory/src/lib.rs` (`manager`, `runtime`, `__reexports`) | yes | macro-support / bootstrap-support item | `pub mod` + `#[doc(hidden)]` | hidden support paths remain for macros/bootstrap, but root runtime/backend re-exports were removed | Low |
| internal test harness root | module | `crates/canic-testing-internal/src/lib.rs` (`pic`) | yes inside crate graph, not published | internal-only harness surface | `pub mod` in `publish = false` crate | acceptable internal exposure; not an external API commitment | Low |

### 1B. Exposure Classification

| Item | Location | Dependency / Exposed Item | Visibility Scope | Exposure Impact | Risk |
| --- | --- | --- | --- | --- | --- |
| `canic-core::dispatch` is no longer an ordinary published root module | `crates/canic-core/src/lib.rs`, `crates/canic-core/src/dispatch/mod.rs` | `#[doc(hidden)] pub mod dispatch` | hidden `pub mod` | endpoint adapter plumbing remains reachable for macro expansion, but the earlier ordinary published exposure was removed | Low |
| `canic-core::bootstrap` is now correctly treated as build/bootstrap support | `crates/canic-core/src/lib.rs` | `#[doc(hidden)] pub mod bootstrap` | hidden `pub mod` | build-time config compilation remains available without presenting `bootstrap` as a normal runtime facade | Low |
| `canic-core::access` is now hidden behind the facade | `crates/canic-core/src/lib.rs`, `crates/canic/src/lib.rs` | `#[doc(hidden)] pub mod access` | hidden `pub mod` | access helpers remain available for the `canic` facade and macro support without advertising `canic-core::access` as a normal downstream entry path | Low |
| `canic-core::error` is now treated as support surface | `crates/canic-core/src/lib.rs`, `crates/canic-core/src/error.rs` | `#[doc(hidden)] pub mod error` | hidden `pub mod` | internal error types remain reachable for lower-level support crates like `canic-control-plane`, but no longer present as a normal published root module | Low |
| `canic-memory` no longer root-re-exports backend state | `crates/canic-memory/src/lib.rs`, `crates/canic-memory/src/manager.rs` | removed `pub use manager::MEMORY_MANAGER` and `pub use runtime::init_eager_tls` | root re-exports removed; module paths hidden | previous representation-heavy backend leak is gone from the published root | Low |
| `canic-testkit::pic` remains intentionally public but is structurally narrower | `crates/canic-testkit/src/pic/mod.rs`, `baseline.rs`, `process_lock.rs`, `startup.rs`, `diagnostics.rs`, `snapshot.rs`, `calls.rs`, `lifecycle.rs`, `standalone.rs` | public PocketIC wrapper, locking, cached baselines, install/call helpers | `pub mod` with re-exported submodule items | still a central public seam, but implementation ownership is now split by concern instead of being concentrated in one or two files | Low |

### 1C. Public Field Exposure

| Type | Public Fields? | Representation Leakage? | Stable DTO/Facade Contract? | Exposure Impact | Risk |
| --- | --- | --- | --- | --- | --- |
| `MemoryRange` | yes | no | yes | simple value type for memory reservations | Low |
| `MemoryRegistryEntry`, `MemoryRangeEntry`, `MemoryRangeSnapshot` | yes | mild | mostly yes | exposes substrate registry ownership labels and ranges directly, but these are the crate’s stated registry contract | Low |
| `CachedPicBaseline<T>` | yes | no | yes | public testkit fixture contract exposing `pic`, snapshots, and metadata intentionally | Low |

No medium/high public field leak was confirmed in the current run. The earlier pressure was root/module exposure, not field-level leakage.

## 2. Subsystem Dependency Graph

### 2A. Dependency Direction

| Subsystem / Crate | Depends On | Depended On By | Lower-Layer Dependencies | Same-Layer Dependencies | Upward Dependency Found? | Direction Assessment (Pressure/Violation) | Risk |
| --- | --- | --- | ---: | ---: | --- | --- | --- |
| `crates/canic` | `canic-core`, `canic-cdk`, `canic-memory`, optional `canic-control-plane` | demo canisters, installer, downstreams | 4 | 0 | no | facade direction is clean | Low |
| `crates/canic-core` | `canic-cdk`, `canic-memory`; test-only dev-deps on `canic-testkit` and `canic-testing-internal` | `canic`, `canic-control-plane`, `canic-testing-internal`, reference-support | 2 | 0 | no runtime upward dependency | runtime direction is clean; publication breadth is pressure, not a dependency violation | Low |
| `crates/canic-testkit` | `canic`, `pocket-ic`, `candid`, `serde` | `canic-testing-internal`, `canic-tests`, test canisters | 3 | 0 | no | public test infrastructure depends downward on facade/runtime contracts, not on internal harnesses | Low |
| `crates/canic-testing-internal` | `canic-testkit`, `canic`, `canic-core`, `canic-control-plane`, `canic-internal` | `canic-tests` and test-only `canic-core` consumers | 4 | 0 | no | correct internal consumer layer; publish boundary contains the broad seam | Low |
| demo canisters (`canisters/**`) | `canic`, `canic-internal` support constants | root harness / local builds | 2 | 0 | no | demo surface stays on facade/reference support only | Low |
| test canisters (`crates/canic-core/test-canisters/**`) | `canic`, `canic-testkit` as needed | internal tests and harness code | 2 | 0 | no | ownership is separate from demo canisters | Low |
| audit canisters (`crates/canic-core/audit-canisters/**`) | `canic`, `canic-control-plane` as needed | `canic-testing-internal`, `instruction_audit` | 2 | 0 | no | ownership is separate from demo canisters; audit-only consumers are explicit | Low |
| `canic-core` runtime layers | `workflow -> policy -> ops -> storage`, `dto/ids/config` support seams | internal runtime consumers | 4 | 0 | no | runtime direction remains aligned with the layering audit | Low |

### 2B. Circularity Findings

| Subsystem A | Subsystem B | Real Cycle? | Evidence | Risk |
| --- | --- | --- | --- | --- |
| `canic-testkit` | `canic-testing-internal` | no | `crates/canic-testkit/Cargo.toml` has no dependency on `canic-testing-internal`; only the internal crate depends on testkit | Low |
| `canic` | `canic-core` | no | `crates/canic/Cargo.toml` depends on `canic-core`; `crates/canic-core/Cargo.toml` has no runtime dependency on `canic` | Low |
| demo canisters | audit canisters | no | audit probe installation is routed through `crates/canic-testing-internal/src/pic/audit.rs`; no audit-canister dependency appears under `canisters/**` in the scan | Low |

### 2C. Implementation Leakage

| Violation | Location | Dependency | Description | Directional Impact | Risk |
| --- | --- | --- | --- | --- | --- |
| no confirmed current implementation leak requiring severity above low | `crates/canic-core/src/lib.rs`, `crates/canic-memory/src/lib.rs`, `crates/canic-testkit/src/pic/*.rs` | hidden support modules only | the earlier medium findings (`dispatch`, `bootstrap`, `MEMORY_MANAGER`, `init_eager_tls`) were narrowed or removed from ordinary published root exposure | no current directional or exposure breach above residual pressure | Low |

No medium/high implementation-leak finding remains after the current narrowing pass.

## 3. Circularity Findings

No real subsystem-level or crate-level cycle was confirmed.

Residual pressure remains in the public/internal test seam, but the direction stays one-way:

- `canic-testkit` is public and generic
- `canic-testing-internal` is `publish = false` and depends on `canic-testkit`
- `canic-core` only references testing crates through `dev-dependencies`

## 4. Visibility Hygiene Findings

### 4A. Overexposure

| Item | Path | Current Visibility | Narrowest Plausible Visibility | Why Narrower Seems Valid | Risk |
| --- | --- | --- | --- | --- | --- |
| published lower-level core facade breadth | `crates/canic-core/src/lib.rs` | several ordinary `pub mod` roots | keep current roots, but continue resisting new convenience exports | the obvious internal/support roots (`access`, `bootstrap`, `dispatch`, `error`, `domain`) were already narrowed; remaining published roots are actively consumed by `canic`, `canic-control-plane`, test crates, and support crates | Medium |
| hidden memory support modules | `crates/canic-memory/src/lib.rs` -> `manager`, `runtime`, `__reexports` | hidden `pub mod` | keep current hidden visibility | macros/bootstrap support still need root-reachable paths, but ordinary root re-exports were already removed | Low |
| `canic-testkit::pic` seam breadth | `crates/canic-testkit/src/pic/mod.rs` | `pub mod` with re-exported helpers | keep public surface, but continue splitting by ownership if it grows again | the crate’s job is a public PocketIC surface; the remaining pressure is coordination, not accidental exposure | Low |

### 4B. Under-Containment Signals

| Area | Signal | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |
| `crates/canic-testkit/src/pic/mod.rs` | public coordination hub | still owns the public `Pic` type and root-level entry surface, but cached baselines, process locking, startup classification, diagnostics, snapshots, calls, and lifecycle helpers now live in sibling modules; root file dropped from `1324` lines to `349` | Pressure | Low |
| `crates/canic-testing-internal/src/pic/mod.rs` | internal barrel module | now a 25-line re-export seam rather than a logic hub; it mainly names fixture ownership boundaries (`attestation`, `audit`, `delegation`, `lifecycle`, `root`) | Pressure | Low |
| `crates/canic-core/src/lib.rs` | published root breadth | the root is materially smaller than baseline because `domain` is now `pub(crate)` and `access`/`bootstrap`/`dispatch`/`error` are hidden, but `canic-core` still functions as a lower-level public facade alongside `canic` | Pressure | Medium |

### 4C. Test Leakage

| Item | Location | Leakage Type | Build Impact | Risk |
| --- | --- | --- | --- | --- |
| `test` module in `canic-core` | `crates/canic-core/src/lib.rs` | test-only namespace is explicitly gated | `#[cfg(test)] pub mod test;` does not leak into non-test builds | Low |

No runtime module importing test utilities or leaking test helper re-exports into non-test builds was confirmed in the scanned scope.

## 5. Layering Violations

### 5A. No Upward References

| Layer / Rule | Upward Dependency Found? | Description | Risk |
| --- | --- | --- | --- |
| `canic-core` runtime layers must remain downward-only | no | this run found no evidence contradicting the current `layer-violations` report; root module and Cargo inspection did not surface an upward runtime dependency | Low |
| `canic-testkit` must not depend on `canic-testing-internal` | no | `crates/canic-testkit/Cargo.toml` has no dependency on `canic-testing-internal`; only the internal crate depends on testkit | Low |
| demo canisters must not depend on test or audit canisters | no | scan found audit probe usage only in `canic-testing-internal` and `instruction_audit`; no demo-canister dependency on audit/test canisters was found | Low |

### 5B. Workflow / Policy / Ops Separation

| Separation Rule | Breach Found? | Evidence | Risk |
| --- | --- | --- | --- |
| `policy` decides but does not act | no | no new evidence of runtime-side policy leakage surfaced beyond the already-green layering audit | Low |
| `workflow` orchestrates but does not own storage schema | no | this run did not surface a workflow/storage schema exposure breach | Low |
| `ops` remains execution-focused rather than business-policy owning | no | no contrary evidence was found in the inspected structural seams | Low |
| `dto` remains transfer-oriented | no | public facade and core exports still route DTOs through explicit `dto` namespaces instead of action-owned modules | Low |

### 5C. Facade Containment

| Facade Item | Leak Type | Exposure Impact | Risk |
| --- | --- | --- | --- |
| `canic::__internal` | hidden macro-support seam | externally reachable but clearly scoped to macro expansion and hidden from docs | Low |
| `canic-testkit::pic` | none confirmed | public generic PocketIC helper surface remains free of Canic-only root harness exports | Low |
| `canic-memory` root | none confirmed beyond hidden support modules | earlier backend state re-exports were removed from the root facade | Low |
| `canic-core` root | low residual breadth only | `access`, `dispatch`, `bootstrap`, and `error` remain reachable only as hidden support roots, and `domain` is no longer published | Low |

## 6. Structural Pressure Areas

| Area | Pressure Type | Why This Is Pressure (Not Yet Violation) | Drift Sensitivity | Risk |
| --- | --- | --- | --- | --- |
| `crates/canic-core/src/lib.rs` | broad published root | core is still a lower-level public facade alongside `canic`, even though the most clearly internal/support roots were already narrowed | medium if new convenience exports continue | Medium |
| `crates/canic/src/lib.rs` | broad facade root | the main facade remains intentionally wide, with nested `api`, `access`, `dto`, `ids`, and macro-support seams all rooted in one file; that is a design choice, but it is now the broadest remaining intentional surface | medium if convenience exports continue to accumulate | Medium |
| `crates/canic-testkit/src/pic/mod.rs` | public seam hub | the public PocketIC contract still centralizes the `Pic` entry surface, but most implementation ownership has moved to sibling modules | low-to-medium; future helper growth could still reconcentrate here | Low |
| `crates/canic-testing-internal/src/pic/mod.rs` | internal seam hub | now mainly an internal barrel rather than a logic center; pressure is naming/coordination only | low | Low |

### 6A. Hub Import Pressure

| Hub Module | Top Imported Sibling Subsystems (by Symbol Count) | Unique Sibling Subsystems Imported | Cross-Layer Dependency Count | Delta vs Previous Report | HIP | Pressure Band | Risk |
| --- | --- | ---: | ---: | --- | ---: | --- | --- |
| `crates/canic-core/src/lib.rs` | `canic_memory` re-exports (`4`), `storage` (`2`), `canic_cdk` (`1`), crate error aliases (`1`) | 4 | 1 | improved: `domain` no longer published; `access`, `bootstrap`, `dispatch`, and `error` are now hidden | 0.25 | low | Medium |
| `crates/canic-testkit/src/pic/mod.rs` | `canic::dto` (`5`), `candid` (`4`), `canic::ids/protocol/cdk` (`4`), `pocket_ic` (`2`) | 4 | 1 | improved again: diagnostics, snapshots, calls, and lifecycle helpers also moved into sibling modules; root file shrank `1324 -> 349` lines | 0.25 | low | Low |
| `crates/canic-testing-internal/src/pic/mod.rs` | internal fixture roots plus `canic_testkit::pic` re-export | 6 | 1 | improved: remains a thin barrel (`25` lines) rather than a logic hub | 0.17 | low | Low |

Interpretation:

- no current seam is in high HIP territory
- the main risk is breadth and coordination concentration, not hidden cross-layer import count

## 7. Drift Sensitivity Summary

| Growth Vector | Affected Subsystems | Why Multiple Layers Would Change | Drift Risk |
| --- | --- | --- | --- |
| new public `canic-testkit` helper | `canic-testkit`, `canic-testing-internal`, `canic-tests` | public helper promotion still tends to touch generic testkit, internal harness adaptation, and downstream test consumers together | Medium |
| new `canic-core` convenience export | `canic-core`, `canic`, support crates | each new root export can bypass the cleaner `canic` facade story and widen lower-level API commitments | High |
| new memory bootstrap primitive | `canic-memory`, `canic-core`, `canic` | hidden support roots are now cleaner, but new root re-exports would immediately regress facade containment | Medium |
| new demo/test/audit canister role | `canisters/**`, `test-canisters/**`, `audit-canisters/**`, harness code | ownership is currently clear, but any new role still needs explicit placement or the split will erode | Medium |

## 8. Structural Risk Index

| Category | Risk Index (1-10, lower is better) | Basis |
| --- | ---: | --- |
| Public Surface Discipline | 3 | `canic` and `canic-testkit` are disciplined, and the earlier `canic-core`/`canic-memory` overexposure was narrowed further; remaining risk is mostly broad lower-level API reach, not internal leaks |
| Layer Directionality | 2 | crate direction and runtime layering remain clean in this run |
| Circularity Safety | 1 | no real crate or subsystem cycle was confirmed |
| Visibility Hygiene | 3 | the obvious root leaks were fixed; remaining pressure is broad-but-intentional public seams, especially `canic-core` root breadth and the main `canic` facade surface | Low |
| Facade Containment | 3 | facade crates are more contained than baseline because backend/root-support leaks were hidden or removed | Low |

### Overall Structural Risk Index

**3 / 10**

Interpretation:

- low structural pressure
- no confirmed high/critical direction or cycle violation
- main cleanup opportunity is continued restraint on lower-level public surface growth, not fixing active boundary failures

## Known Intentional Exceptions

| Exception | Why Intentional | Scope Guardrail | Still Valid This Run? |
| --- | --- | --- | --- |
| `canic::__internal` and `canic::__build` | macro/build expansion requires reachable support namespace | kept hidden from docs and explicitly documented as non-public contract | yes |
| hidden `canic-core` support roots (`access`, `bootstrap`, `dispatch`, `error`, `__control_plane_core`) | facade/build/macro/endpoint support still requires root-reachable paths | keep hidden and do not widen them back into ordinary published modules | yes |
| `canic-testkit` public PocketIC helpers | crate is intentionally a public generic test infrastructure surface | must stay generic and must not absorb Canic-only root harness concepts | yes |
| `canic-testing-internal` broad `pic` barrel | crate is `publish = false` and exists specifically to hold Canic-only test harnesses | acceptable only while it remains internal and one-way dependent on `canic-testkit` | yes |
| public DTO/ID families under `canic` | stable transfer/value contracts are meant to be root-reachable | DTO and ID exposure must not pull storage or workflow internals with them | yes |

## Delta Since Baseline

| Delta Type | Item / Subsystem | Previous | Current | Impact |
| --- | --- | --- | --- | --- |
| narrowed root visibility | `crates/canic-core/src/lib.rs` -> `domain` | `pub mod domain` | `pub(crate) mod domain` | lower-level policy/model ownership is no longer published from the crate root |
| narrowed root visibility | `crates/canic-core/src/lib.rs` -> `bootstrap`, `dispatch` | ordinary `pub mod` | hidden `pub mod` | build/endpoint support stays reachable without presenting as normal runtime API |
| narrowed root visibility | `crates/canic-core/src/lib.rs` -> `access`, `error` | ordinary `pub mod` | hidden `pub mod` | facade/support paths remain reachable without presenting them as normal downstream root modules |
| removed root re-exports | `crates/canic-memory/src/lib.rs` -> `MEMORY_MANAGER`, `init_eager_tls` | root `pub use` | removed | backend/bootstrap state no longer leaks from the support-crate root |
| reduced hub pressure | `crates/canic-testkit/src/pic/mod.rs` | `776` lines after the first split | `349` lines with `diagnostics.rs`, `snapshot.rs`, `calls.rs`, and `lifecycle.rs` extracted | public seam is still intentional, but root implementation ownership is now much cleaner |
| reduced internal hub pressure | `crates/canic-testing-internal/src/pic/mod.rs` | flagged as a growing coordination center | currently `25` lines and acting as a barrel only | remaining pressure is low and mostly organizational |

## 9. Verification Readout

| Check | Status | Notes |
| --- | --- | --- |
| public crate root inspection (`canic`, `canic-core`, `canic-testkit`, `canic-cdk`, `canic-memory`) | PASS | root module and re-export surfaces inspected directly |
| crate dependency direction via `Cargo.toml` inspection | PASS | no public/internal reverse dependency breach found |
| test leakage scan over runtime/support crates | PASS | no non-test leakage beyond intentional `#[cfg(test)]` roots found |
| demo/test/audit seam scan | PASS | audit probes are routed through `canic-testing-internal` and `instruction_audit`, not demo canisters |
| structural judgment | PASS | no high or critical structural violation confirmed; baseline pressure is lower than the earlier same-day run |
