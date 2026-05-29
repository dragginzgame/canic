# Dependency Hygiene Audit - 2026-05-29

## Report Preamble

- Scope: workspace root `Cargo.toml`; published crate manifests under
  `crates/canic`, `crates/canic-backup`, `crates/canic-cli`,
  `crates/canic-control-plane`, `crates/canic-core`, `crates/canic-host`,
  `crates/canic-macros`, and `crates/canic-wasm-store`; internal manifests
  under `crates/canic-testing-internal`, `crates/canic-tests`,
  `canisters/test/**`, `canisters/audit/**`, `canisters/sandbox/**`, and
  `fleets/**`.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-10/dependency-hygiene.md`
- Code snapshot identifier: `89cccc85`
- Method tag/version: `dependency-hygiene-current`
- Comparability status: `comparable with crate-map drift`
- Exclusions applied: lockfile-only noise, generated target outputs, `.icp`
  runtime cache, local runtime fixture directories, and test/fleet/audit
  canister package posture when judging published runtime package surfaces
  except explicit seam checks.
- Notable methodology changes vs baseline: removed crates from the prior
  report (`canic-cdk`, `canic-memory`, `canic-testkit`) are no longer active
  workspace members; the audit now treats sibling `ic-testkit` as generic
  external test infrastructure and keeps it out of published runtime surfaces.
- Auditor: `codex`
- Run timestamp: `2026-05-29`
- Worktree: `dirty`

## Executive Summary

Verdict: **PASS**.

No dependency hygiene violation was found. Published crates do not depend on
unpublished workspace members, operator package direction remains
`canic-cli -> canic-host/canic-backup/canic-core`, `canic-host` and
`canic-cli` remain facade-free, `canic-backup` stays independent of runtime and
operator crates, and `canic-wasm-store` remains a canister artifact crate with
only a `cdylib` target.

The main improvement since the 2026-05-10 report is feature hygiene:
`crates/canic/Cargo.toml` now has `default = ["metrics"]` only. The heavier
`control-plane`, `sharding`, and `auth-crypto` surfaces are explicit feature
choices instead of default-on graph width.

Overall dependency hygiene risk index: **2 / 10**.

## Baseline Capture

| Metric | 2026-05-10 | Current | Delta |
| --- | ---: | ---: | ---: |
| Published crates with internal runtime edges | 0 | 0 | 0 |
| Published crates with test-only leakage concerns | 0 | 0 | 0 |
| Publish-surface dependency mismatches | 0 | 0 | 0 |
| Published operator support crates in scope | 3 | 3 | 0 |
| Publishable canister crates emitting `rlib` | not guarded in this report | 0 | improved |
| Public facade default feature pressure | medium | low | improved |
| Removed/renamed active support crates vs report map | 0 | 3 | crate-map drift |

## Crate Dependency Direction

| Crate | Publish Intent | Runtime Depends On | Optional / Build / Dev Edges | Internal Runtime Edge Found? | Direction Assessment | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | published facade | `candid`, `canic-core`, `canic-macros`, non-wasm `flate2`/`toml` | optional `canic-control-plane`; features `metrics`, `control-plane`, `sharding`, `auth-crypto`; dev `candid_parser`, `ic-cdk`, `serde_json` | no | Facade graph is narrower than the 2026-05-10 report because only `metrics` is default-on. | Low |
| `canic-core` | published lower-level runtime | IC SDK/support crates, `ic-memory`, `icrc-ledger-types`, `k256`, serialization/hash/error crates | build `candid`; dev `criterion`, `futures`; non-wasm `proc-macro2`/`quote`/`toml`; features `sharding`, `auth-crypto` | no | Direction remains downward; no internal harness edge. | Low |
| `canic-control-plane` | published support/runtime crate | `canic-core`, `ic-memory`, `async-trait`, `candid`, `serde`, `sha2`, `thiserror` | none | no | Correctly below facade and above core support crates. | Low |
| `canic-macros` | published proc-macro crate | `proc-macro2`, `quote`, `syn` | `syn` features `extra-traits`, `full`, `visit` | no | Narrow proc-macro edge set. | Low |
| `canic-wasm-store` | published canister artifact crate | `candid`, `canic` with `control-plane`, `ic-cdk` | build `canic`; `crate-type = ["cdylib"]` | no | Runtime artifact boundary is explicit; no `rlib` target. | Low |
| `canic-backup` | published backup/restore library | `candid`, `serde`, `serde_json`, `sha2`, `thiserror` | none | no | Domain library stays independent of facade/runtime/operator crates. | Low |
| `canic-host` | published host/operator library | `candid`, `canic-core`, `flate2`, serialization/hash/error/TOML crates | none | no | Host remains facade-free and does not depend on CLI or backup. | Low |
| `canic-cli` | published binary/library package | `candid`, `candid_parser`, `canic-backup`, `canic-core`, `canic-host`, `clap`, serialization/error crates | binary `canic` | no | Correct top-level operator dependency direction. | Low |
| `canic-testing-internal` | internal (`publish = false`) | `canic`, `canic-control-plane`, `canic-core`, `ic-testkit`, `candid`, `serde` | none | internal-only | Correct internal harness sink. | Low |
| `canic-tests` | internal (`publish = false`) | `canic`, `canic-testing-internal`, `ic-testkit` | dev `candid`, `canic-control-plane`, `canic-core`, `ic-memory`, serialization crates | internal-only | Integration tests remain unpublished and one-way. | Low |
| `canisters/**`, `fleets/**` | internal canister packages | `canic`, `candid`, `ic-cdk`, selected explicit features | build `canic`; selected build helpers | internal-only | Local canister manifests are `publish = false`; no test/audit canister edge leaks into published crates. | Low |

## Public/Internal Seam Checks

| Seam | Status | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |
| Published crates must not depend on `canic-testing-internal` | clean | `workspace_manifest` test passed; manifest scan found it only in internal test crates and workspace dependency aliases. | none | Low |
| Published crates must not depend on `canic-tests` | clean | `crates/canic-tests/Cargo.toml` is `publish = false`; no published manifest references it. | none | Low |
| Generic test infrastructure must stay outside published runtime crates | clean | `ic-testkit` appears only in `canic-testing-internal`, `canic-tests`, and `runtime_probe`; no published runtime crate depends on it. | none | Low |
| Operator library direction | clean | `canic-cli` depends on `canic-core`, `canic-host`, and `canic-backup`; support crates do not depend on `canic-cli` or the canister facade. | bounded pressure | Low |
| Backup domain independence | clean | `canic-backup` has no dependency on `canic`, `canic-host`, or `canic-cli`. | none | Low |
| Canister artifact boundary | clean | `workspace_manifest` proves `cdylib` workspace members do not emit `rlib`; `canic-wasm-store` manifest has `crate-type = ["cdylib"]`. | none | Low |

## Feature Hygiene

| Crate | Feature | Enables | Default? | Responsibility Fit | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | `metrics` | default metrics endpoint/profile support | yes | facade-owned and intentional | low default-feature pressure | Low |
| `canic` | `control-plane` | optional `canic-control-plane` dependency | no | explicit root/store orchestration surface | none | Low |
| `canic` | `sharding` | `canic-core/sharding` | no | explicit placement/sharding behavior | none | Low |
| `canic` | `auth-crypto` | `canic-core/auth-crypto` | no | explicit crypto-heavy auth flows | none | Low |
| `canic-core` | `sharding`, `auth-crypto` | lower-level runtime behavior | no | narrow and role-aligned | none | Low |
| `canic-macros` | `syn` feature set | `extra-traits`, `full`, `visit` | n/a | proc-macro parsing support | none | Low |

## Package / Publish Surface

| Crate | Publish Intent | Package Dependency Concern | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- |
| `canic` | published | public facade graph | `cargo tree -p canic --depth 1 --locked` shows `candid`, `canic-core`, `canic-macros`, non-wasm `flate2`, and `toml`; optional control plane is not default-on. | bounded pressure | Low |
| `canic-host` | published | host/operator graph | `cargo tree -p canic-host --depth 1 --locked` shows `canic-core`, compression, TOML, and serialization crates, with no direct `canic` facade edge. | bounded | Low |
| `canic-cli` | published binary package | operator graph | `cargo tree -p canic-cli --depth 1 --locked` shows CLI depending on `canic-core`, `canic-host`, and `canic-backup`, with no direct `canic` facade edge. | bounded | Low |
| `canic-backup` | published | no Canic runtime dependency | Manifest depends only on backup/manifest serialization and hashing crates. | none | Low |
| `canic-wasm-store` | published canister artifact | canister runtime artifact graph | `cargo tree -p canic-wasm-store --depth 1 --locked` shows `candid`, `canic`, and `ic-cdk`; manifest exposes only `cdylib`. | bounded | Low |

## Dependency Risk Index

| Category | Risk Index | Basis |
| --- | ---: | --- |
| Runtime Dependency Direction | 1 / 10 | no published crate runtime edge into `publish = false` crates was found |
| Public/Internal Seam Discipline | 1 / 10 | internal harnesses remain unpublished and one-way |
| Feature Hygiene | 1 / 10 | `canic` defaults are narrow again; heavier surfaces are explicit features |
| Package / Publish Surface | 2 / 10 | operator crates and canister artifact crates are published surfaces but have clean direction and guards |
| Support-Crate Ownership Clarity | 2 / 10 | facade/core/control-plane/host split remains intentional and bounded |

Overall dependency hygiene risk index: **2 / 10**.

## Verification Readout

Commands passed:

- `cargo +1.96.0 metadata --no-deps --format-version 1`
- `rg "canic-testing-internal|canic-tests|ic-testkit|pocket-ic|publish = false|\\[features\\]|default-features|features\\s*=|optional\\s*=" crates canisters fleets Cargo.toml -g Cargo.toml -n`
- `cargo +1.96.0 tree -p canic --depth 1 --locked`
- `cargo +1.96.0 tree -p canic-host --depth 1 --locked`
- `cargo +1.96.0 tree -p canic-cli --depth 1 --locked`
- `cargo +1.96.0 tree -p canic-wasm-store --depth 1 --locked`
- `cargo +1.96.0 test -p canic --test workspace_manifest --locked`
- `cargo +1.96.0 check -p canic-cli -p canic-host -p canic-backup --locked`
- `cargo +1.96.0 check -p canic-wasm-store --locked`

## Follow-up Actions

1. Keep `canic-cli`, `canic-host`, and `canic-backup` dependency direction
   one-way and facade-free as operator package surfaces grow.
2. Keep `ic-testkit` restricted to internal test harnesses and test/audit
   canisters; do not add it to published runtime crates.
3. Keep `canic-wasm-store` and fleet/test canisters as runtime artifacts, not
   reusable Rust libraries.
4. Keep `canic` default features narrow; require explicit feature selection for
   control-plane, sharding, and auth-crypto behavior.
