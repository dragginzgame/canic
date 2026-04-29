# Dependency Hygiene Audit - 2026-04-06

## Report Preamble

- Scope: workspace root `Cargo.toml`; published/support crate manifests under `crates/canic`, `crates/canic-cdk`, `crates/canic-control-plane`, `crates/canic-core`, `crates/canic-dsl-macros`, `crates/canic-installer`, `crates/canic-memory`, `crates/canic-testkit`, and `crates/canic-wasm-store`; internal/support manifests under `crates/canic-testing-internal`, `crates/canic-tests`, `canisters/reference-support`, `canisters/**`, `crates/canic-core/test-canisters/**`, and `crates/canic-core/audit-canisters/**`
- Compared baseline report path: same-day earlier retained run at this path before the revised `dependency-hygiene` audit wording, the `canic` `metrics` default-feature change, and the later `MemoryApi` naming cleanup
- Code snapshot identifier: `9b3aade1ef65dbb856a9c1a966f8dd63a5b3a6cb`
- Method tag/version: `dependency-hygiene-current`
- Comparability status: `comparable`
- Exclusions applied: lockfile-only noise, generated outputs, packaged artifacts, and test-only fixture crates when judging runtime package surface except explicit leakage checks
- Notable methodology changes vs baseline: revised `dependency-hygiene` audit wording adopted for this rerun; conclusions remain based on the same direct manifest inspection plus `cargo metadata` method, and this rerun also reflects the `canic-core` PocketIC test move into `canic-tests`, the `MemoryApi` public rename cleanup, and the new default `metrics` feature on `canic`

## 0. Baseline Capture

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Published crates with internal runtime edges | 0 | 0 | 0 |
| Published crates with test-only leakage concerns | 0 | 0 | 0 |
| Optional features reviewed | 3 | 4 | +1 |
| Publish-surface mismatches | 0 | 0 | 0 |
| Duplicate or overlapping support seams | 2 | 2 | 0 |
| Published crates with path-only or workspace-fragile package assumptions | 0 | 0 | 0 |
| Public crates with default-feature widening concerns | 0 | 1 | +1 |

Notes:
- The two overlapping support seams are `canic` vs `canic-core` as dual public entry surfaces, and `canic` vs `canic-memory` for memory-related support.
- The earlier `canic-core` path-only PocketIC self-test seam is gone: the PocketIC integration tests now live in the internal `canic-tests` crate.
- The new same-day pressure is `crates/canic/Cargo.toml` enabling the public `metrics` feature by default; this is intentional, but it is now the one explicit default-feature widening point in the public graph.

## 1. Crate Dependency Direction

| Crate | Publish Intent | Runtime Depends On | Optional Depends On | Build Depends On | Dev Depends On | Internal Runtime Edge Found? | Reverse/Upward Pressure Found? | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `canic` | published facade | `canic-cdk`, `canic-core`, `canic-dsl-macros`, `canic-memory`, `candid`, `ic-cdk`, non-wasm `flate2`/`sha2` | `canic-control-plane` via `control-plane`; compile-surface feature toggles `metrics`, `sharding`, `auth-crypto` | `canic-core` | `toml` | no | no runtime reverse edge; facade breadth is intentional, but default metrics now widens the ordinary endpoint surface | Medium |
| `canic-cdk` | published support crate | IC SDK/support crates only (`candid`, `ic-cdk`, `ic-cdk-management-canister`, `ic-cdk-timers`, `ic-stable-structures`, `serde`, `sha2`, helpers) | none | `candid` | none | no | no | Low |
| `canic-core` | published lower-level runtime | `canic-cdk`, `canic-memory`, `async-trait`, `candid`, `ctor`, `derive_more`, `k256`, `remain`, `serde`, `serde_bytes`, `sha2`, `thiserror`, non-wasm `proc-macro2`/`quote`/`toml` | none | `candid` | `criterion`, `futures` | no | no internal or workspace-local test-harness edge remains in the published crate | Low |
| `canic-control-plane` | published support/runtime crate | `canic-cdk`, `canic-core`, `canic-memory`, `async-trait`, `candid`, `serde`, `sha2`, `thiserror` | none | none | none | no | no | Low |
| `canic-memory` | published support crate | `canic-cdk`, `candid`, `ctor`, `serde`, `serde_cbor`, `thiserror` | none | none | none | no | no | Low |
| `canic-testkit` | published generic test infrastructure | `canic`, `candid`, `derive_more`, `pocket-ic`, `serde` | none | none | none | no | no reverse seam into internal harness crates | Low |
| `canic-dsl-macros` | published proc-macro crate | `proc-macro2`, `quote`, `syn` | none | none | none | no | no | Low |
| `canic-installer` | published tooling crate | `canic`, `canic-core`, `flate2`, `serde`, `serde_json`, `sha2`, `toml` | none | none | none | no | no internal edge; broad support ownership is tooling pressure only | Low |
| `canic-wasm-store` | published canonical canister crate | `canic` with `control-plane`, `candid`, `ic-cdk` | none | `canic` | none | no | no internal seam, but crate role depends on the broad `canic` facade rather than a narrower support subset | Medium |
| `canic-testing-internal` | internal (`publish = false`) | `canic`, `canic-control-plane`, `canic-core`, `canic-internal`, `canic-testkit`, `candid`, `pocket-ic`, `serde`, `sha2` | none | none | none | acceptable internal-only edge set | no | Low |
| `canic-tests` | internal (`publish = false`) | none | none | none | `canic`, `canic-control-plane`, `canic-internal`, `canic-testing-internal`, `canic-testkit`, `candid`, `serde`, `serde_json` | acceptable internal-only edge set | no | Low |
| `canic-internal` (`canisters/reference-support`) | internal (`publish = false`) | `canic-core` | none | none | none | acceptable internal-only support edge | no | Low |
| demo canisters (`canisters/**`) | internal demo/reference | `canic`, `canic-internal`, `candid`, `ic-cdk`, `serde` as needed | feature use on `canic` (`control-plane`, `auth-crypto`, `sharding`) | `canic`, sometimes `sha2` | `pocket-ic` in some demo canisters | no | no dependency on test or audit canisters found | Low |
| test canisters (`crates/canic-core/test-canisters/**`) | internal correctness fixtures | `canic`, `canic-core`, `canic-internal`, `candid`, `ic-cdk`, crypto helpers as needed | none | `canic` in several crates | none | acceptable internal-only edge set | no | Low |
| audit canisters (`crates/canic-core/audit-canisters/**`) | internal audit probes | `canic`, `canic-control-plane`, `candid`, `ic-cdk` as needed | none | `canic` in probe crates | none | acceptable internal-only edge set | no | Low |

## 2. Public/Internal Seam Checks

| Seam | Status | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |
| `canic-testkit` must not depend on `canic-testing-internal` | clean | `crates/canic-testkit/Cargo.toml` has runtime deps on `canic`, `candid`, `derive_more`, `pocket-ic`, and `serde`; `canic-testing-internal` appears only in the internal crate manifest, not here | none | Low |
| published crates must not depend on `canic-tests` | clean | no published crate manifest inspected in this run includes `canic-tests`; only `crates/canic-tests/Cargo.toml` consumes published/internal support crates through `dev-dependencies` | none | Low |
| demo canisters must not depend on test or audit canisters | clean | `canisters/root/Cargo.toml` and sibling demo manifests depend on `canic` and `canic-internal`; no manifest under `canisters/**` references `crates/canic-core/test-canisters/**` or `crates/canic-core/audit-canisters/**` | none | Low |
| public support crates must not rely on internal crates through runtime or build edges | clean | `crates/canic/Cargo.toml`, `crates/canic-cdk/Cargo.toml`, `crates/canic-control-plane/Cargo.toml`, `crates/canic-memory/Cargo.toml`, `crates/canic-testkit/Cargo.toml`, `crates/canic-installer/Cargo.toml`, and `crates/canic-wasm-store/Cargo.toml` contain no runtime or build edge to `publish = false` crates | none | Low |
| published crates must not inherit internal seams through workspace aliases | clean | workspace root defines `canic-testing-internal` and `canic-internal` in `[workspace.dependencies]`, but no published crate uses either at runtime, build time, or dev time after the PocketIC tests moved to `crates/canic-tests` | none | Low |
| public crate self-test posture should not shape runtime package posture | clean | `crates/canic-core/Cargo.toml` now keeps only external dev deps (`criterion`, `futures`); PocketIC integration coverage lives in internal `crates/canic-tests` instead | none | Low |

## 3. Feature Hygiene

| Crate | Feature | Enables | Default? | Public/User-Facing? | Responsibility Fit | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `canic` | `control-plane` | optional runtime edge `dep:canic-control-plane` | no | yes | strong fit; expands facade into root/store orchestration only when requested | none | Low |
| `canic` | `metrics` | default-enabled metrics endpoint bundle | yes | yes | acceptable fit for the facade, but it now widens the default endpoint/compile surface compared with the earlier slimmer baseline | pressure | Medium |
| `canic` | `sharding` | `canic-core/sharding` | no | yes | acceptable facade alias over lower-level capability; feature maps to one clear subsystem concern | low pressure because it tunnels a lower-level feature outward, but still matches facade role | Low |
| `canic` | `auth-crypto` | `canic-core/auth-crypto` | no | yes | acceptable facade alias for crypto-heavy auth surfaces used by root/user-hub/user-shard flows | low pressure because it couples facade and core feature names, but not a leak | Low |
| `canic-core` | `sharding` | internal lower-level runtime behavior | no | lower-level / not primary downstream facade | narrow and role-aligned | none | Low |
| `canic-core` | `auth-crypto` | internal lower-level runtime behavior | no | lower-level / not primary downstream facade | narrow and role-aligned | none | Low |
| public support crates without features (`canic-cdk`, `canic-memory`, `canic-testkit`, `canic-control-plane`, `canic-installer`, `canic-wasm-store`) | none | no optional feature graph | n/a | n/a | avoids feature sprawl in support/tooling crates | none | Low |

Feature summary:
- `crates/canic/Cargo.toml` now enables `metrics` by default, so the facade no longer has an empty default feature set.
- That default is explicit and role-aligned, but it is still the one current default-feature widening point in the public graph.
- No public feature exists only for workspace-local testing or audit behavior.

## 4. Package / Publish Surface

| Crate | Publish Intent | Package Surface Concern | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- |
| `canic` | published | no material package mismatch found | `publish = true`, docs/readme/repository metadata are present; runtime/build edges stay on published support crates | none | Low |
| `canic-core` | published | no material package mismatch found | runtime and build edges are publish-safe, and the earlier path-only internal dev seam was removed by moving PocketIC integration tests into `crates/canic-tests` | none | Low |
| `canic-cdk` | published | no material package mismatch found | published support crate with explicit docs/readme metadata and no internal edges | none | Low |
| `canic-control-plane` | published | minimal docs posture only | `readme = false`, but runtime/build graph stays publish-safe and crate role is still clearly declared in package metadata | low pressure | Low |
| `canic-memory` | published | no material package mismatch found | `README.md` explicitly documents standalone use and the crate has no dependency on `canic`; public package posture matches actual role | none | Low |
| `canic-testkit` | published | no material package mismatch found | public generic PocketIC crate with readme/docs metadata and no internal harness dependency | none | Low |
| `canic-dsl-macros` | published | proc-macro package surface is narrow and explicit | `proc-macro = true`; deps are confined to `proc-macro2`, `quote`, and `syn` | none | Low |
| `canic-installer` | published | broad downstream-tooling surface, but publish-safe | `README.md` documents installed binaries and downstream use; runtime deps remain on published crates only | low pressure | Low |
| `canic-wasm-store` | published | public crate role is broad by design | `crates/canic-wasm-store/Cargo.toml` depends on `canic` with `features = ["control-plane"]` and also uses `canic` as a build dependency; package still resolves via published crates, but responsibility breadth is larger than a thin leaf canister | pressure | Medium |
| `canic-testing-internal`, `canic-tests`, `canic-internal` | internal | no accidental publishability found | each manifest explicitly sets `publish = false` | none | Low |

## 5. Redundant / Overlapping Support Seams

| Area | Overlap Signal | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |
| `canic` vs `canic-core` | two public entry surfaces over related runtime ownership | `crates/canic/Cargo.toml` exposes the main facade, while `crates/canic-core/Cargo.toml` remains a published lower-level runtime crate consumed by `canic`, `canic-control-plane`, and some support/test crates | pressure | Medium |
| `canic` vs `canic-memory` | both surface memory-related support | `canic` depends on `canic-memory` and re-exports memory helpers, while `canic-memory` now also carries its own standalone `README.md` and public `api` module for runtime-selected registration/inspection/query flows | pressure, but substantially reduced because standalone ownership is now explicit | Low |
| `canic-testkit` vs `canic-testing-internal` | former overlap concern remains resolved | `crates/canic-testkit/Cargo.toml` stays generic and public; `crates/canic-testing-internal/Cargo.toml` remains `publish = false` and depends one-way on `canic-testkit` | none | Low |
| `canic-installer` vs `canic` facade/build helpers | tooling overlap exists but is intentional | `canic-installer` uses `canic` and `canic-core` to stage release/build flows, but README positioning is explicit about installed-binary ownership | low pressure | Low |

## 6. Dead / Convenience Edge Review

| Crate | Edge / Re-export | Why It Exists | Narrower Alternative? | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- |
| `canic-core` | remaining external `dev-dependencies` (`criterion`, `futures`) | benchmark scaffold plus internal async/unit-test support | narrower alternative is not necessary right now; these are standard external dev edges rather than workspace-only convenience seams | none | Low |
| `canic-wasm-store` | runtime/build dependency on `canic` | canonical `wasm_store` canister uses the main facade plus `control-plane` hooks and build support | possible lower-level split exists in theory, but would trade one broad facade edge for several direct lower-level edges | low pressure | Low |
| `canic-installer` | runtime dependency on both `canic` and `canic-core` | installer owns downstream build/install/release-set tooling and needs both facade and lower-level types | narrower alternative is not obvious without moving installer responsibilities or duplicating protocol/types | low pressure | Low |
| `canic` | runtime dependency on `canic-memory` | facade intentionally re-exports stable-memory helpers for downstream convenience | downstreams can already choose `canic-memory` directly; the edge still matches the facade role | none | Low |

## 7. Feature / Package Pressure Indicators

| Crate / Area | Pressure Type | Why This Is Pressure (Not Yet Violation) | Drift Sensitivity | Risk |
| --- | --- | --- | --- | --- |
| workspace root `Cargo.toml` | workspace-inherited dependency policy | `[workspace.dependencies]` and `[patch.crates-io]` centralize version and local override policy, so package review still requires reasoning about the combined workspace and leaf manifests even though the obvious leaf-level path-only seam is gone | medium if public crates start depending on internal aliases or workspace-only assumptions at runtime | Medium |
| `canic` | broad published facade | `crates/canic/Cargo.toml` depends on four sibling support/runtime crates plus one optional runtime crate, and it now enables `metrics` by default; this is expected for a facade, but it remains the broadest public ownership seam | medium if new default-enabled bundles or convenience re-exports accumulate | Medium |
| `canic-wasm-store` | broad published canister role | canonical published canister crate depends on `canic` with `control-plane` and also on `canic` in build time, which is broader than a thin leaf package even though it stays within published support crates | medium if more facade-owned responsibilities are added here | Medium |
| `canic-tests` | internal integration ownership breadth | `crates/canic-tests/Cargo.toml` now owns `canic-core`, `canic-testing-internal`, `canic-testkit`, and `pocket-ic` together, which is correct but keeps the full PocketIC integration seam concentrated in one internal crate | low; internal-only and consistent with crate role | Low |

## 8. Dependency Risk Index

| Category | Risk Index (1-10, lower is better) | Basis |
| --- | ---: | --- |
| Runtime Dependency Direction | 1 | no published crate runtime edge into `publish = false` crates was found, and the earlier public-crate PocketIC test seam is now gone |
| Public/Internal Seam Discipline | 1 | `canic-testkit` stays cleanly separated from `canic-testing-internal`, no published crate depends on `canic-tests` or demo/test/audit canisters, and `canic-core` no longer depends on internal harness crates even in dev posture |
| Feature Hygiene | 3 | public feature surface is still small, but `canic` now has one intentional default feature (`metrics`), so the graph no longer has a fully minimal default facade posture |
| Package / Publish Surface | 3 | the main remaining pressure is workspace-root dependency policy and a few broad intentional support crates, not public crates depending on internal harnesses |
| Support-Crate Ownership Clarity | 3 | `canic` vs `canic-core` and `canic` vs `canic-memory` are still overlapping support seams, but they are documented and currently bounded rather than confused |

## Overall Dependency Hygiene Risk Index

**3 / 10**

Interpretation:
- low dependency/package pressure
- no confirmed High/Critical public/internal dependency breach
- main pressure is broad intentional support overlap, workspace-root policy, and one explicit default-enabled facade feature, not public/internal seam failure

## Delta Since Baseline

| Delta Type | Crate / Edge / Feature | Previous | Current | Impact |
| --- | --- | --- | --- | --- |
| feature surface widened | `crates/canic/Cargo.toml` default features | `default = []` | `default = ["metrics"]` | one explicit default-enabled facade feature now widens the ordinary compile/endpoint surface |
| optional features reviewed | public support feature count | 3 | 4 | the new `metrics` feature is now part of the public review set |
| public-crate test seam | `crates/canic-core/Cargo.toml` dev posture | path-only PocketIC harness deps present in earlier same-day baseline | no internal or path-only PocketIC harness deps remain | earlier package-pressure cleanup held through this rerun |

## Verification Readout

| Check | Status | Notes |
| --- | --- | --- |
| workspace root manifest inspection | PASS | `Cargo.toml` root members, workspace dependencies, and `[patch.crates-io]` policy inspected directly |
| public/support crate manifest inspection | PASS | `crates/canic`, `canic-cdk`, `canic-control-plane`, `canic-core`, `canic-dsl-macros`, `canic-installer`, `canic-memory`, `canic-testkit`, and `canic-wasm-store` inspected directly |
| internal seam manifest inspection | PASS | `crates/canic-testing-internal`, `crates/canic-tests`, `canisters/reference-support`, and representative demo canister manifests inspected directly |
| graph cross-check | PASS | `cargo metadata --no-deps --format-version 1` confirmed workspace package roles, feature declarations, and publish posture |
| build/test verification | PASS | earlier same-day `cargo clippy -p canic-core -p canic-tests --all-targets -- -D warnings`, `cargo test -p canic-core --lib --tests trap_guard -- --nocapture`, and `cargo test -p canic-tests --test pic_intent_race -- --nocapture` already covered the `canic-core` test move; the new metrics-feature pass was checked separately via `cargo clippy -p canic --lib --all-features -- -D warnings`, `cargo clippy -p canister_minimal --all-targets -- -D warnings`, and `cargo test -p canic --lib` |
| dependency hygiene judgment | PASS | no High/Critical dependency or publish-surface violation confirmed; only Low/Medium pressure remains |
