# Dependency Hygiene Audit - 2026-05-10

## Report Preamble

- Scope: workspace root `Cargo.toml`; published/support crate manifests under
  `crates/canic`, `crates/canic-cdk`, `crates/canic-control-plane`,
  `crates/canic-core`, `crates/canic-macros`, `crates/canic-memory`,
  `crates/canic-testkit`, `crates/canic-wasm-store`, `crates/canic-backup`,
  `crates/canic-host`, and `crates/canic-cli`; internal manifests under
  `crates/canic-testing-internal`, `crates/canic-tests`, `canisters/test/**`,
  `canisters/audit/**`, `canisters/sandbox/**`, and `fleets/**`.
- Compared baseline report path:
  `docs/audits/reports/2026-04/2026-04-06/dependency-hygiene.md`
- Code snapshot identifier: `d6ea5e3b` with dirty worktree refactor/audit
  changes in progress.
- Method tag/version: `dependency-hygiene-current`
- Comparability status: non-comparable: the 0.33 ICP CLI hard cut added the
  published operator crates `canic-cli`, `canic-host`, and `canic-backup`,
  renamed the proc-macro package surface to `canic-macros`, removed the old
  installer package shape from scope, and moved active fleet canisters under
  `fleets/**`.
- Exclusions applied: lockfile-only noise, generated target outputs, `.icp`
  runtime cache, `.tmp` runtime fixtures, and test/fleet/audit canister package
  posture when judging published runtime package surfaces except explicit seam
  checks.
- Notable methodology changes vs baseline: operator crates are now audited as
  published package surfaces; `fleets/**` are included as active local canister
  manifests; dependency direction claims are grounded in direct manifest
  inspection plus `cargo metadata` and focused `cargo tree` checks.

## Baseline Capture

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Published crates with internal runtime edges | 0 | 0 | 0 |
| Published crates with test-only leakage concerns | 0 | 0 | 0 |
| Optional facade features reviewed | 4 | 4 | 0 |
| Publish-surface dependency mismatches | 0 | 0 | 0 |
| Published operator support crates in scope | 0 | 3 | +3 |
| Duplicate or overlapping support seams | 2 | 2 | 0 |
| Published crates with path-only or workspace-fragile package assumptions | 0 | 0 | 0 |
| Public crates with default-feature widening concerns | 1 | 1 | 0 |

Notes:

- The new operator support crates are `canic-cli`, `canic-host`, and
  `canic-backup`.
- The current overlapping support seams are `canic` vs `canic-core` and
  `canic` vs `canic-memory`.
- No published crate depends on `canic-testing-internal` or `canic-tests`.

## Crate Dependency Direction

| Crate | Publish Intent | Runtime Depends On | Optional / Build / Dev Edges | Internal Runtime Edge Found? | Direction Assessment | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | published facade | `canic-cdk`, `canic-control-plane`, `canic-core`, `canic-macros`, `canic-memory`, `candid`, non-wasm `flate2`/`sha2` | build `canic-core`; dev `ic-cdk`, `serde_json`, `toml`; default features `metrics`, `control-plane`, `sharding`, `auth-crypto` | no | Broad but expected pre-1.0 facade. Defaults now cover managed root/auth/sharding canisters so fleet manifests stay simple. | Medium |
| `canic-core` | published lower-level runtime | `canic-cdk`, `canic-memory`, `async-trait`, `candid`, `ctor`, `k256`, `remain`, `serde`, `serde_bytes`, `sha2`, `thiserror`, non-wasm `proc-macro2`/`quote`/`toml` | build `candid`; dev `criterion`, `futures`; features `sharding`, `auth-crypto` | no | Direction remains downward; no internal harness edge in the published crate. | Low |
| `canic-control-plane` | published support/runtime crate | `canic-cdk`, `canic-core`, `canic-memory`, `async-trait`, `candid`, `serde`, `sha2`, `thiserror` | none | no | Correctly below the facade and above core support crates. | Low |
| `canic-cdk` | published support crate | IC SDK/support crates, `candid`, `serde`, `serde_bytes`, `sha2` | build `candid` | no | Narrow support substrate; no workspace-internal edge. | Low |
| `canic-memory` | published support crate | `canic-cdk`, `ctor`, `serde`, `serde_cbor`, `thiserror` | none | no | Standalone memory support remains independent of the `canic` facade. | Low |
| `canic-macros` | published proc-macro crate | `proc-macro2`, `quote`, `syn` | `syn` with `extra-traits`, `full`, `visit` | no | Narrow proc-macro edge set. | Low |
| `canic-testkit` | published generic test infrastructure | `canic`, `candid`, `pocket-ic`, `serde` | none | no | Generic testkit still does not depend on internal testing harnesses. | Low |
| `canic-wasm-store` | published canonical canister crate | `canic`, `candid`, `ic-cdk` | build `canic` | no | Published canister role is broad but intentional; depends on published facade only. | Medium |
| `canic-backup` | published backup/restore library | `candid`, `serde`, `serde_json`, `sha2`, `thiserror` | none | no | Domain library stays independent of facade/runtime crates. | Low |
| `canic-host` | published host/operator library | `canic-core`, `candid`, `flate2`, `serde`, `serde_bytes`, `serde_cbor`, `serde_json`, `sha2`, `toml` | none | no | New 0.33 host seam is role-aligned and no longer links the canister facade; it should not absorb CLI UX or backup domain logic. | Low |
| `canic-cli` | published binary/library package | `canic-core`, `canic-backup`, `canic-host`, `candid`, `clap`, `serde`, `serde_json`, `thiserror` | binary `canic` | no | Correct top-level operator dependency direction: CLI depends on core/host/backup, not the canister facade. | Low |
| `canic-testing-internal` | internal (`publish = false`) | `canic`, `canic-control-plane`, `canic-core`, `canic-testkit`, `candid`, `serde` | none | internal-only | Correct internal harness sink. | Low |
| `canic-tests` | internal (`publish = false`) | dev-only `canic`, `canic-control-plane`, `canic-core`, `canic-testing-internal`, `canic-testkit`, `pocket-ic`, serialization crates | none | internal-only | Integration tests remain unpublished and one-way. | Low |
| `canisters/**`, `fleets/**` | internal canister packages | `canic`, `candid`, `ic-cdk`, selected feature flags | build `canic`; selected dev/build helpers | internal-only | Local canister manifests are `publish = false`; no test/audit canister edge leaks into published crates. | Low |

## Public/Internal Seam Checks

| Seam | Status | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |
| Published crates must not depend on `canic-testing-internal` | clean | `rg` found `canic-testing-internal` only in `crates/canic-testing-internal/Cargo.toml`, `crates/canic-tests/Cargo.toml`, and workspace dependency aliases; no published crate manifest uses it. | none | Low |
| Published crates must not depend on `canic-tests` | clean | `crates/canic-tests/Cargo.toml` is `publish = false`; no published manifest references it. | none | Low |
| `canic-testkit` must stay generic | clean | `crates/canic-testkit/Cargo.toml` depends on `canic`, `candid`, `pocket-ic`, and `serde`, not the internal harness. | none | Low |
| Operator library direction | clean with pressure | `canic-cli` depends on `canic-core`, `canic-host`, and `canic-backup`; neither support crate depends on `canic-cli` or the canister facade. | ownership pressure only | Low |
| Backup domain independence | clean | `canic-backup` has no dependency on `canic`, `canic-host`, or `canic-cli`. | none | Low |
| Active fleets/test/audit canisters | clean | representative `fleets/**`, `canisters/test/**`, `canisters/audit/**`, and `canisters/sandbox/**` manifests are `publish = false`. | none | Low |

## Feature Hygiene

| Crate | Feature | Enables | Default? | Responsibility Fit | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | `metrics` | default metrics endpoint/profile support | yes | facade-owned and intentional while pre-1.0 | default-feature widening pressure | Medium |
| `canic` | `control-plane` | default `canic-control-plane` dependency | yes | root/store orchestration is standard for managed Canic fleets before 1.0 | default-feature widening pressure | Medium |
| `canic` | `sharding` | default `canic-core/sharding` | yes | facade alias over lower-level placement capability; config/build cfg decides role behavior | default-feature widening pressure | Medium |
| `canic` | `auth-crypto` | default `canic-core/auth-crypto` | yes | facade alias for crypto-heavy auth flows; config/build cfg decides role behavior | default-feature widening pressure | Medium |
| `canic-core` | `sharding`, `auth-crypto` | lower-level runtime behavior | no | narrow and role-aligned | none | Low |
| `canic-macros` | `syn` feature set | `extra-traits`, `full`, `visit` | n/a | proc-macro parsing support | none | Low |
| `canic-host`, `canic-backup` | `serde/derive` | local serialization derives | n/a | ordinary package implementation detail | none | Low |

Feature summary:

- The public feature graph remains small.
- No feature exists only for workspace-local testing or audit behavior.
- `canic` now defaults to the standard pre-1.0 managed-fleet surface
  (`metrics`, `control-plane`, `sharding`, and `auth-crypto`), so the facade
  default is intentionally not the smallest possible compile/API surface.

## Package / Publish Surface

| Crate | Publish Intent | Package Dependency Concern | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- |
| `canic` | published | broad facade graph | `cargo tree -p canic --depth 1` shows facade pulling core, cdk, memory, macros, default control-plane, and non-wasm build-support deps. | pressure | Medium |
| `canic-host` | published | host/operator graph | `cargo tree -p canic-host --depth 1` shows host depending on `canic-core`, compression, TOML, and serialization crates, with no direct `canic` facade edge. | bounded | Low |
| `canic-cli` | published binary package | operator graph | `cargo tree -p canic-cli --depth 1` shows CLI depending on `canic-core`, `canic-host`, and `canic-backup`, with no direct `canic` facade edge. | bounded | Low |
| `canic-backup` | published | no Canic runtime dependency | manifest depends only on backup/manifest serialization and hashing crates. | none | Low |
| `canic-wasm-store` | published canister crate | broad canister/support graph | manifest depends on plain `canic` and build-time `canic`; facade defaults provide root/store support. | pressure | Medium |
| `canic-control-plane` | published support crate | no internal edge | manifest depends on published support crates only. | none | Low |
| `canic-core`, `canic-cdk`, `canic-memory`, `canic-macros`, `canic-testkit` | published support crates | no internal edge or path-only harness dependency | direct manifests inspected; `cargo metadata` confirms publish posture and package targets. | none | Low |

## Redundant / Overlapping Support Seams

| Area | Overlap Signal | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |
| `canic` vs `canic-core` | two public runtime entry surfaces | `canic` depends on `canic-core`, while `canic-core` is also published and consumed by support/operator crates. | pressure | Medium |
| `canic` vs `canic-memory` | facade re-exports plus standalone memory support | `canic` depends on `canic-memory`; `canic-memory` remains independently published. | pressure, bounded | Low |

## Dependency Risk Index

| Category | Risk Index | Basis |
| --- | ---: | --- |
| Runtime Dependency Direction | 1 / 10 | no published crate runtime edge into `publish = false` crates was found |
| Public/Internal Seam Discipline | 1 / 10 | internal harnesses remain unpublished and one-way |
| Feature Hygiene | 3 / 10 | feature graph is small, but `canic` keeps default-on `metrics` |
| Package / Publish Surface | 2 / 10 | 0.33 operator crates are now published surfaces, but `canic-host` and `canic-cli` no longer link the canister facade |
| Support-Crate Ownership Clarity | 2 / 10 | facade/core/memory overlap remains intentional, while CLI/host/backup direction is clean and facade-free |

Overall dependency hygiene risk index: **2 / 10**.

Interpretation:

- no High or Critical dependency hygiene violation was confirmed
- risk is below the earlier 0.33 run after narrowing both host and CLI package
  graphs, though the published package surface now includes operator crates
- the main pressure is broad but intentional published support packages, not
  public/internal dependency leakage

## Delta Since Baseline

| Delta Type | Crate / Edge / Feature | Previous | Current | Impact |
| --- | --- | --- | --- | --- |
| scope expansion | operator crates | not materially in scope | `canic-cli`, `canic-host`, `canic-backup` are published and audited | package-boundary pressure increased |
| package replacement | old installer/proc-macro names | `canic-installer`, `canic-dsl-macros` in baseline wording | active packages are `canic-cli`, `canic-host`, `canic-backup`, `canic-macros` | non-comparable package map |
| active canister layout | demo/test canisters | older `canisters/**` layout | active fleets under `fleets/**`; test/audit/sandbox remain `canisters/**` | local manifests remain `publish = false` |
| narrowed host graph | `canic-host -> canic` | not in baseline package map | removed; host now depends on `canic-core` plus host data/formatting crates | lower package-boundary pressure |
| narrowed CLI graph | `canic-cli -> canic` | not in baseline package map | removed; CLI now depends on `canic-core`, `canic-host`, and `canic-backup` | lower package-boundary pressure |
| retained cleanliness | internal harness edges | no published internal runtime edges | still no published internal runtime edges | no regression |
| expanded default feature posture | `canic` default features | `default = ["metrics"]` in April rerun | `default = ["metrics", "control-plane", "sharding", "auth-crypto"]` | intentional pre-1.0 UX tradeoff |

## Verification Readout

| Check | Status | Notes |
| --- | --- | --- |
| recurring definition review | PASS | `docs/audits/recurring/system/dependency-hygiene.md` reviewed before report generation. |
| baseline review | PASS | `docs/audits/reports/2026-04/2026-04-06/dependency-hygiene.md` reviewed. |
| workspace metadata scan | PASS | `cargo metadata --no-deps --format-version 1` captured package roles, publish posture, features, and targets. |
| direct manifest inspection | PASS | root manifest, published crate manifests, internal crate manifests, and representative local canister manifests inspected. |
| internal seam grep | PASS | `rg "canic-testing-internal|canic-tests|publish = false" crates canisters fleets -g Cargo.toml` found no published-crate internal dependency. |
| feature scan | PASS | `rg "default-features|features\\s*=|optional\\s*=|\\[features\\]" . -g Cargo.toml` reviewed current feature/optional-edge surface. |
| focused graph checks | PASS | `cargo tree -p canic-cli --depth 1`, `cargo tree -p canic-host --depth 1`, and `cargo tree -p canic --depth 1` reviewed. |
| operator package build check | PASS | `cargo check -p canic-cli -p canic-host -p canic-backup`, `cargo clippy -p canic-cli --all-targets -- -D warnings`, and `cargo clippy -p canic-host --all-targets -- -D warnings` passed. |

## Follow-up Actions

1. Operator maintainers: keep `canic-cli` UX ownership separate from
   `canic-host` filesystem/build/install mechanics and `canic-backup`
   manifest/restore primitives.
2. Host maintainers: keep host features on `canic-core`/data dependencies
   unless a future facade dependency is deliberately justified.
3. Facade maintainers: keep default-on `metrics` documented as intentional
   default-feature widening while Canic is pre-1.0.
4. Package maintainers: keep `canic-testing-internal`, `canic-tests`, fleets,
   and test/audit/sandbox canisters explicitly unpublished.
