# Dependency Hygiene Audit - 2026-06-28

## Report Preamble

- Definition path: `docs/audits/recurring/system/dependency-hygiene.md`
- Scope: workspace root `Cargo.toml`; published crate manifests under
  `crates/canic`, `crates/canic-backup`, `crates/canic-cli`,
  `crates/canic-control-plane`, `crates/canic-core`, `crates/canic-host`,
  `crates/canic-macros`, and `crates/canic-wasm-store`; internal manifests
  under `crates/canic-testing-internal`, `crates/canic-tests`,
  `canisters/test/**`, `canisters/audit/**`, `canisters/sandbox/**`, and
  `fleets/**`.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/dependency-hygiene.md`
- Code snapshot identifier: `b140a86c` with dirty worktree.
- Method tag/version: `dependency-hygiene-current`.
- Comparability status: `comparable`. The report section contract and
  manifest-scan method match the June 19 run.
- Exclusions applied: lockfile-only noise, generated target output, `.icp`
  runtime cache, local `.tmp` test-runtime manifests, and code-level Rust
  implementation changes outside manifest/package boundaries.
- Notable methodology changes vs baseline: none.
- Auditor: `codex`.
- Run timestamp: `2026-06-28T14:27:18Z`.
- Worktree: `dirty`; prior 0.74.14 code/audit/changelog edits were preserved.

Verification status: **PASS**.

No dependency hygiene violation was found. Published crates still avoid
runtime dependencies on unpublished workspace members. `canic-cli`,
`canic-host`, and `canic-backup` keep one-way operator direction, and `canic`
defaults remain narrow at `default = ["metrics"]`. The main delta since June
19 is the addition of default-off blob-storage facade/core feature aliases.

## Baseline Capture

| Metric | Previous | Current | Delta |
| ------ | -------: | ------: | ----: |
| Published crates with internal runtime edges | 0 | 0 | 0 |
| Published crates with test-only leakage concerns | 0 | 0 | 0 |
| Optional public facade features reviewed | 8 | 10 | +2 |
| Publish-surface mismatches | 0 | 0 | 0 |
| Duplicate or overlapping support seams | 0 | 0 | 0 |
| Published crates with path-only or workspace-fragile package assumptions | 0 | 0 | 0 |
| Public crates with default-feature widening concerns | 0 | 0 | 0 |

The optional public facade feature count increased by two because
`crates/canic/Cargo.toml` now includes default-off `blob-storage` and
`blob-storage-billing` aliases. Both map to `canic-core` feature ownership and
do not widen the default dependency graph.

## Structural Hotspots

| Hotspot | Manifest / Field Evidence | Why It Matters | Risk |
| ------- | ------------------------- | -------------- | ---- |
| Public facade default surface | `crates/canic/Cargo.toml`: `default = ["metrics"]`; explicit `control-plane`, `blob-storage`, `blob-storage-billing`, `sharding`, and auth feature aliases | This is the public downstream entry point. Default widening would affect all facade users. | Low |
| Blob-storage facade aliases | `crates/canic/Cargo.toml`: `blob-storage = ["canic-core/blob-storage"]`, `blob-storage-billing = ["blob-storage", "canic-core/blob-storage-billing"]` | New public feature aliases must stay default-off and mapped to core-owned behavior. | Low |
| Auth proof dependency gates | `crates/canic-core/Cargo.toml`: optional `ic-canister-sig-creation`, `ic-certification`, and `ic-signature-verification` behind auth features | Canister-signature creation/verification dependencies must stay tied to the auth behavior that owns them. | Low |
| Operator package direction | `crates/canic-cli/Cargo.toml`: depends on `canic-backup`, `canic-core`, and `canic-host`; not `canic` | Prevents operator tooling from becoming another canister facade. | Low |
| Internal harness sink | `crates/canic-testing-internal/Cargo.toml` and `crates/canic-tests/Cargo.toml`: `publish = false`, depend on `ic-testkit` | Keeps Canic test infrastructure out of published runtime/package surfaces. | Low |
| Canister artifact package | `crates/canic-wasm-store/Cargo.toml`: `publish = true`, `crate-type = ["cdylib"]`, depends on `canic` with `control-plane` | This is intentionally publishable as an artifact crate, but must remain a canister artifact rather than a reusable Rust library. | Low |

## Hub Module Pressure

| Crate / Package Hub | Fan-In Signal | Fan-Out / Feature Signal | Pressure Score | Basis |
| ------------------- | ------------- | ------------------------ | -------------: | ----- |
| `canic` | `cargo tree -i canic --locked` shows runtime/build fan-in from canister fixtures, fleets, `canic-wasm-store`, and internal tests | Public facade has 12 feature entries including `default`; only `metrics` is default-on | 3 / 10 | Expected public facade hub; pressure is bounded by narrow defaults and unpublished fixtures. |
| `canic-core` | `cargo tree -i canic-core --locked` shows fan-in from facade, CLI, host, control-plane, tests, and fixtures | Auth optional deps remain gated; blob-storage features add no new optional dependency edges | 3 / 10 | Core runtime hub by design; no internal reverse dependency violation. |
| `canic-cli` | No reverse package fan-in from public support crates | Depends on host/backup/core and external CLI/data crates | 2 / 10 | Operator leaf package with broad command responsibility but clean direction. |
| `canic-host` | `cargo tree -i canic-host --locked` shows `canic-cli` runtime fan-in and `canic-tests` dev fan-in | Host-side deployment support depends on core, not facade | 2 / 10 | Correct operator support layer; no CLI reverse edge. |
| `canic-testing-internal` | `cargo tree -i canic-testing-internal --locked` shows only `canic-tests` | Internal-only test harness depends on facade/core/control-plane/testkit | 1 / 10 | Proper unpublished dependency sink. |

## Dependency Fan-In Pressure

| Crate / Feature | Incoming Edge Signal | Evidence | Pressure or Violation | Risk |
| --------------- | -------------------- | -------- | --------------------- | ---- |
| `canic` | Broad canister/fleet/build fan-in | `cargo tree -i canic --locked` | Pressure: public facade breadth is expected, but default-feature widening would amplify downstream builds. | Low |
| `canic-core` | Runtime fan-in from facade, CLI, host, control-plane, internal tests, and fixtures | `cargo tree -i canic-core --locked` | Pressure: central runtime crate; no internal edge or reverse-cycle violation found. | Low |
| `blob-storage-billing` facade/core features | New feature use from internal tests and fixture canisters | `crates/canic-tests/Cargo.toml`, `canisters/test/blob_storage_probe/Cargo.toml`, and `canisters/test/blob_storage_cashier_mock/Cargo.toml` enable `canic` `blob-storage-billing`; all are `publish = false` | Pressure: feature is public and should remain default-off; no published internal seam breach. | Low |
| `canic-control-plane` | Fan-in from explicit facade feature, wasm-store, internal harnesses, and root fixtures | `cargo tree -i canic-control-plane --locked` | Pressure: explicit control-plane feature and unpublished root fixtures keep this bounded. | Low |
| `canic-testing-internal` | Internal test package only | `cargo tree -i canic-testing-internal --locked` | none: unpublished sink only. | Low |
| `ic-testkit` | Internal harness and `runtime_probe` dev-dependency | `cargo tree -i ic-testkit --locked` | none: no published runtime edge. | Low |

## Early Warning Signals

| Signal | Evidence | Status | Trigger to Revisit |
| ------ | -------- | ------ | ------------------ |
| Default-feature widening | `crates/canic/Cargo.toml`: `default = ["metrics"]`; `cargo tree -p canic --depth 1 --locked` excludes `canic-control-plane`, auth proof deps, and blob-storage-only behavior | Clean | Any addition to `canic` defaults beyond narrow facade-owned behavior. |
| Public feature aliases exposing internal layout | `canic` aliases `canic-core` sharding/auth/blob-storage features | Bounded pressure | Revisit if aliases stop mapping one-to-one to core-owned behavior. |
| Optional auth dependencies becoming always-on | `cargo tree -p canic-core --depth 1 --locked` excludes `ic-canister-sig-creation`, `ic-certification`, and `ic-signature-verification`; feature tree includes them only with auth features | Clean | Revisit if any auth proof dependency becomes non-optional or default-on. |
| Blob-storage billing feature creep | `blob-storage-billing` is default-off and currently adds no new third-party direct dependency edge beyond the core graph | Clean with watchpoint | Revisit if billing adds external optional dependencies or becomes default-on. |
| Path-only package assumptions | Published crate manifests use workspace dependencies; root workspace definitions carry matching `version` plus `path`. Direct path-only canister edges are limited to `publish = false` audit/sandbox fixtures. | Clean for published crates | Revisit if a publishable crate adds a direct path-only dependency. |
| Build-script/proc-macro workspace coupling | `canic-macros` is proc-macro; canister artifact crates use `canic` as build-dependency; `workspace_manifest` guard passed | Bounded pressure | Revisit if published build-dependencies point at unpublished crates. |
| Internal harness leakage | `canic-testing-internal`, `canic-tests`, and test/audit/sandbox canisters are `publish = false`; reverse tree shows no published runtime fan-in | Clean | Revisit if a published crate depends on `canic-testing-internal`, `canic-tests`, or `ic-testkit`. |
| Support crates becoming alternate facades | `canic-host` and `canic-cli` depend on `canic-core`, not `canic`; `canic-backup` has no runtime/operator dependency | Clean | Revisit if host or CLI begins depending on `canic` or re-exporting facade behavior. |

## Crate Dependency Direction

| Crate | Publish Intent | Runtime Depends On | Optional Depends On | Build Depends On | Dev Depends On | Internal Runtime Edge Found? | Reverse/Upward Pressure Found? | Risk |
| ----- | -------------- | ------------------ | ------------------- | ---------------- | -------------- | ---------------------------- | ------------------------------ | ---- |
| `canic` | published facade | `candid`, `canic-core`, `canic-macros`, non-wasm `flate2`/`toml` | `canic-control-plane` through `control-plane` | package build script; no internal crate edge | `candid_parser`, `ic-cdk`, `serde_json` | no | bounded facade pressure | Low |
| `canic-core` | published runtime core | IC SDK/support crates, `ic-memory`, ledger types, serialization/hash/error crates | `ic-canister-sig-creation`, `ic-certification`, `ic-signature-verification` | `candid` | `criterion`, `futures` | no | central runtime fan-in pressure | Low |
| `canic-control-plane` | published support/runtime crate | `canic-core`, `ic-memory`, `async-trait`, `candid`, `serde`, `sha2`, `thiserror` | none | none | none | no | below facade, above core support; bounded | Low |
| `canic-macros` | published proc-macro crate | `proc-macro2`, `quote`, `syn` | none | none | none | no | public macro support only | Low |
| `canic-wasm-store` | published canister artifact | `candid`, `canic` with `control-plane`, `ic-cdk` | none | `canic` | none | no | artifact package pressure; guarded as `cdylib` only | Low |
| `canic-backup` | published backup/restore library | `candid`, `serde`, `serde_json`, `sha2`, `thiserror` | none | none | none | no | independent of facade/operator/runtime crates | Low |
| `canic-host` | published host/operator library | `candid`, `candid_parser`, `canic-core`, `flate2`, `ic-query`, serialization/hash/error/TOML crates | none | none | none | no | facade-free operator support | Low |
| `canic-cli` | published binary/library package | `candid`, `canic-backup`, `canic-core`, `canic-host`, `clap`, serialization/error crates | none | binary target only | none | no | correct operator leaf direction | Low |
| `canic-testing-internal` | internal (`publish = false`) | `canic`, `canic-control-plane`, `canic-core`, `ic-testkit`, `candid` | none | none | none | internal-only | unpublished sink | Low |
| `canic-tests` | internal (`publish = false`) | `canic` with `blob-storage-billing`, `canic-testing-internal`, `ic-testkit` | none | none | `candid`, `canic-control-plane`, `canic-core`, `canic-host`, `ic-memory`, serialization crates | internal-only | unpublished sink | Low |
| `canisters/**`, `fleets/**` | internal fixtures/artifacts (`publish = false`) | `canic`, `candid`, `ic-cdk`, selected features | selected facade features only | `canic` and fixture-specific build helpers | `runtime_probe` dev-depends on `ic-testkit` | internal-only | unpublished fixture pressure | Low |

## Public/Internal Seam Checks

| Seam | Status | Evidence | Pressure or Violation | Risk |
| ---- | ------ | -------- | --------------------- | ---- |
| Published crates must not depend on `canic-testing-internal` | clean | manifest scan found `canic-testing-internal` only in root workspace aliases and internal crates; `workspace_manifest` passed | none | Low |
| Published crates must not depend on `canic-tests` | clean | `crates/canic-tests/Cargo.toml` is `publish = false`; no published manifest references it | none | Low |
| Generic test infrastructure must stay outside published runtime crates | clean | `cargo tree -i ic-testkit --locked` shows internal harnesses and `runtime_probe` dev-dependency only | none | Low |
| Operator library direction | clean | `cargo tree -p canic-cli --depth 1 --locked`; `cargo tree -p canic-host --depth 1 --locked` | bounded pressure | Low |
| Backup domain independence | clean | `cargo tree -p canic-backup --depth 1 --locked` has no `canic`, `canic-core`, `canic-host`, or `canic-cli` dependency | none | Low |
| Canister artifact boundary | clean | `workspace_manifest` `cdylib_members_do_not_emit_rlib_artifacts` passed; `canic-wasm-store` has `crate-type = ["cdylib"]` | none | Low |
| Blob-storage feature fixtures | clean | `canic-tests`, `blob_storage_probe`, and `blob_storage_cashier_mock` enable `blob-storage-billing` and are `publish = false` | pressure only: internal fixtures exercise public features without becoming package surface | Low |

## Feature Hygiene

| Crate | Feature | Enables | Default? | Public/User-Facing? | Responsibility Fit | Pressure or Violation | Risk |
| ----- | ------- | ------- | -------- | ------------------- | ------------------ | --------------------- | ---- |
| `canic` | `metrics` | facade-owned metrics support | yes | yes | narrow default behavior | none | Low |
| `canic` | `control-plane` | optional `canic-control-plane` dependency | no | yes | explicit root/store orchestration surface | none | Low |
| `canic` | `icp-refill` | facade feature marker | no | yes | narrow named feature | none | Low |
| `canic` | `blob-storage` | `canic-core/blob-storage` | no | yes | explicit blob-storage endpoint support | none | Low |
| `canic` | `blob-storage-billing` | `blob-storage`, `canic-core/blob-storage-billing` | no | yes | explicit billing/status/funding endpoint support | pressure: new public alias, default-off and mapped to core | Low |
| `canic` | `sharding` | `canic-core/sharding` | no | yes | explicit placement behavior | none | Low |
| `canic` | `auth-root-canister-sig-create` | `canic-core/auth-root-canister-sig-create` | no | yes | explicit root proof creation support | none | Low |
| `canic` | `auth-root-canister-sig-verify` | `canic-core/auth-root-canister-sig-verify` | no | yes | explicit root proof verification support | none | Low |
| `canic` | `auth-issuer-canister-sig-create` | `canic-core/auth-issuer-canister-sig-create` | no | yes | explicit issuer proof creation support | none | Low |
| `canic` | `auth-issuer-canister-sig-verify` | `canic-core/auth-issuer-canister-sig-verify` | no | yes | explicit issuer proof verification support | none | Low |
| `canic` | `auth-delegated-token-verify` | `canic-core/auth-delegated-token-verify` | no | yes | explicit protected endpoint verification support | none | Low |
| `canic-core` | `sharding` | lower-level placement behavior | no | through facade/core users | role-aligned | none | Low |
| `canic-core` | `blob-storage` | lower-level blob-storage behavior marker | no | through facade/core users | role-aligned | none | Low |
| `canic-core` | `blob-storage-billing` | `blob-storage` | no | through facade/core users | role-aligned billing behavior marker | pressure only | Low |
| `canic-core` | `auth-root-canister-sig-create` | `ic-canister-sig-creation`, `ic-certification` | no | through facade/core users | role-aligned optional proof creation deps | none | Low |
| `canic-core` | `auth-issuer-canister-sig-create` | `ic-canister-sig-creation`, `ic-certification` | no | through facade/core users | role-aligned optional proof creation deps | none | Low |
| `canic-core` | `auth-root-canister-sig-verify` | `ic-signature-verification` | no | through facade/core users | role-aligned optional verification deps | none | Low |
| `canic-core` | `auth-issuer-canister-sig-verify` | `ic-signature-verification` | no | through facade/core users | role-aligned optional verification deps | none | Low |
| `canic-core` | `auth-delegated-token-verify` | root and issuer verification features | no | through facade/core users | composed verification feature | none | Low |

## Package / Publish Surface

| Crate | Publish Intent | Package Surface Concern | Evidence | Pressure or Violation | Risk |
| ----- | -------------- | ----------------------- | -------- | --------------------- | ---- |
| `canic` | published | public facade graph | `Cargo.toml` and `cargo tree -p canic --depth 1 --locked`; default excludes control-plane/auth/sharding/blob-storage-only behavior | bounded pressure | Low |
| `canic-core` | published | central runtime dependency hub | `Cargo.toml` and `cargo tree -p canic-core --depth 1 --locked`; optional auth deps absent by default | bounded pressure | Low |
| `canic-host` | published | host/operator graph | `cargo tree -p canic-host --depth 1 --locked`; depends on `canic-core`, not `canic` | bounded pressure | Low |
| `canic-cli` | published | operator graph | `cargo tree -p canic-cli --depth 1 --locked`; depends on host/backup/core, not facade | bounded pressure | Low |
| `canic-backup` | published | backup package independence | `cargo tree -p canic-backup --depth 1 --locked`; only serialization/hash/error deps | none | Low |
| `canic-wasm-store` | published artifact | canister artifact package, not reusable Rust library | `crate-type = ["cdylib"]`; `workspace_manifest` guard passed | bounded pressure | Low |
| `canic-testing-internal`, `canic-tests` | internal | must remain unpublished sinks | `publish = false`; reverse tree shows only internal fan-in | none | Low |
| `canisters/**`, `fleets/**` | internal fixtures/artifacts | feature-enabled canisters must not become reusable package surfaces | manifest scan shows `publish = false` for every retained fleet/test/audit/sandbox canister package | none | Low |

## Redundant / Overlapping Support Seams

| Area | Overlap Signal | Evidence | Pressure or Violation | Risk |
| ---- | -------------- | -------- | --------------------- | ---- |
| Public facade vs lower-level crates | `canic` exposes facade features that mirror `canic-core` feature ownership | `crates/canic/Cargo.toml` feature aliases | Pressure: intentional facade mapping, not a violation while aliases remain one-to-one and default-off. | Low |
| Blob-storage facade/core feature pair | `canic/blob-storage-billing` composes `blob-storage` and `canic-core/blob-storage-billing` | `crates/canic/Cargo.toml`; `workspace_manifest` test `blob_storage_billing_feature_is_opt_in_and_implies_blob_storage` passed | Pressure only; public feature maps to one owning core feature and stays default-off. | Low |
| Operator support crates | `canic-cli`, `canic-host`, and `canic-backup` split operator concerns | Direct manifests and `cargo tree -p canic-cli --depth 1 --locked` | none: CLI composes host/backup/core; host and backup stay independent of CLI. | Low |
| Test harnesses | `ic-testkit`, `canic-testing-internal`, and `canic-tests` all participate in tests | Reverse tree and manifests | Pressure: layered test support exists, but all Canic-specific harness packages are unpublished. | Low |

## Dead / Convenience Edge Review

| Crate | Edge / Re-export | Why It Exists | Narrower Alternative? | Pressure or Violation | Risk |
| ----- | ---------------- | ------------- | --------------------- | --------------------- | ---- |
| `canic-cli` | `canic-core` runtime edge | CLI uses core runtime DTOs/logic alongside host and backup support | none proven by this audit | Pressure only; no facade dependency or internal edge. | Low |
| `canic-host` | `canic-core` runtime edge | Host build/install/deployment support shares runtime contract types | none proven by this audit | Pressure only; host remains facade-free. | Low |
| `canic-wasm-store` | `canic` runtime/build edge | Artifact canister uses public facade with `control-plane` feature | no narrower artifact path identified | Pressure only; guarded by `cdylib` package shape. | Low |
| `canic-core` | non-wasm `proc-macro2`, `quote`, `toml` target deps | Build/support code for non-wasm contexts | no stale edge proven by manifest audit | Pressure only; target-gated away from wasm. | Low |

## Feature / Package Pressure Indicators

| Crate / Area | Pressure Type | Why This Is Pressure (Not Yet Violation) | Drift Sensitivity | Risk |
| ------------ | ------------- | ---------------------------------------- | ----------------- | ---- |
| `canic` facade | public feature alias breadth | Multiple explicit facade features mirror core behavior for downstream users. This is normal facade work, not an internal seam breach. | Medium: default-on changes would be high impact. | Low |
| `canic-core` | central package fan-in | Many public/support crates and fixtures depend on core runtime contracts. | Medium: new optional deps or default features can spread quickly. | Low |
| blob-storage billing fixtures | feature-enabled unpublished artifacts | Blob-storage billing tests and probe/mock canisters enable the public feature but remain `publish = false`. | Medium: any publish posture change would need review. | Low |
| root/delegated-auth fixture canisters | feature-enabled unpublished artifacts | Several test/fleet canisters enable auth/control-plane features, but all are `publish = false`. | Medium: any publish posture change would need review. | Low |
| workspace path aliases | package inheritance pressure | Workspace dependencies use local paths with matching versions; published crates inherit them through workspace declarations. | Low: normal workspace packaging pattern; package checks should keep guarding this. | Low |

## Risk Score

| Category | Risk Index (1-10, lower is better) | Basis |
| -------- | ---------------------------------: | ----- |
| Runtime Dependency Direction | 1 | no published crate runtime edge into `publish = false` crates was found |
| Public/Internal Seam Discipline | 1 | internal harnesses remain unpublished and one-way |
| Feature Hygiene | 2 | auth/control-plane/sharding/blob-storage features are explicit and default-off; facade alias breadth is bounded pressure |
| Package / Publish Surface | 2 | operator and artifact crates are publishable surfaces but have clean direction and guards |
| Support-Crate Ownership Clarity | 2 | facade/core/control-plane/host/backup/testkit split remains intentional and bounded |

### Overall Dependency Hygiene Risk Index (1-10, lower is better)

**2 / 10**.

No High or Critical dependency/package violation was found. The main residual
risk is ordinary facade/core hub pressure: future default-feature widening,
auth/blob-storage optional-dependency drift, or publishing a feature-enabled
fixture canister would have broad package impact.

## Delta Since Baseline

| Delta Type | Crate / Edge / Feature | Previous | Current | Impact |
| ---------- | ---------------------- | -------- | ------- | ------ |
| Feature surface | `canic` `blob-storage` and `blob-storage-billing` | absent in the 2026-06-19 dependency-hygiene report | present, default-off, mapped to `canic-core` | bounded public facade feature growth |
| Feature surface | `canic-core` `blob-storage` and `blob-storage-billing` | absent in the 2026-06-19 dependency-hygiene report | present, default-off; billing implies blob-storage | bounded core feature growth with no new optional dependency edge |
| Internal fixture feature use | blob-storage billing probes/tests | not present in baseline feature scan | `canic-tests`, `blob_storage_probe`, and `blob_storage_cashier_mock` enable `blob-storage-billing` | acceptable because all are `publish = false` |
| Runtime direction | published crates to internal crates | 0 | 0 | no regression |
| Default features | `canic` | `default = ["metrics"]` | `default = ["metrics"]` | no regression |
| Path-only package assumptions | published crates | none | none | no regression |

## Verification Readout

Status: **PASS**.

| Check / Command | Status | Notes |
| --- | --- | --- |
| definition review | PASS | reviewed `docs/audits/recurring/system/dependency-hygiene.md`; no definition changes were required. |
| baseline review | PASS | compared against `docs/audits/reports/2026-06/2026-06-19/dependency-hygiene.md`. |
| `git rev-parse --short HEAD` | PASS | code snapshot identifier `b140a86c`. |
| `date -u +%Y-%m-%dT%H:%M:%SZ` | PASS | timestamp `2026-06-28T14:27:18Z`. |
| `cargo metadata --locked --no-deps --format-version 1` | PASS | workspace metadata resolved. |
| manifest scan for publish fields, features, internal crates, optional deps, and crate types | PASS | all retained fleet/test/audit/sandbox canisters are `publish = false`; published crates have no internal runtime edges. |
| path dependency scan | PASS | direct path-only canister edges are limited to `publish = false` audit/sandbox fixtures; published crates inherit workspace path+version aliases. |
| `cargo tree -p canic --depth 1 --locked` | PASS | default graph excludes control-plane, auth proof deps, and blob-storage-only behavior. |
| `cargo tree -p canic-core --depth 1 --locked` | PASS | optional auth proof dependencies are absent by default. |
| `cargo tree -p canic-cli --depth 1 --locked` | PASS | CLI depends on backup/core/host, not the facade. |
| `cargo tree -p canic-host --depth 1 --locked` | PASS | host depends on core, not facade or CLI. |
| `cargo tree -p canic-backup --depth 1 --locked` | PASS | backup remains independent from facade/operator/runtime crates. |
| `cargo tree -p canic-wasm-store --depth 1 --locked` | PASS | artifact crate depends on facade with control-plane and is `cdylib`-guarded. |
| reverse tree checks for `canic`, `canic-core`, `canic-host`, `canic-backup`, `canic-control-plane`, `canic-testing-internal`, `canic-tests`, and `ic-testkit` | PASS | internal harnesses remain sinks; no published crate depends on `canic-testing-internal` or `canic-tests`. |
| `cargo tree -p canic --all-features --depth 1 --locked` | PASS | all-feature facade graph adds explicit `canic-control-plane` only as expected. |
| `cargo tree -p canic-core --features auth-root-canister-sig-create,auth-root-canister-sig-verify,auth-issuer-canister-sig-create,auth-issuer-canister-sig-verify,auth-delegated-token-verify,blob-storage-billing --depth 1 --locked` | PASS | auth proof dependencies appear only under explicit auth features; blob-storage billing adds no new direct third-party dependency edge. |
| `cargo tree -p canic --no-default-features --features blob-storage-billing --depth 1 --locked` | PASS | blob-storage billing does not widen direct package dependencies. |
| `cargo tree -p canic-core --features blob-storage-billing --depth 1 --locked` | PASS | blob-storage billing does not enable auth proof optional dependencies. |
| `cargo tree -e features` checks for facade/core blob-storage billing | PASS | feature-specific graph stayed on the same direct dependency set. |
| `cargo test --locked -p canic --test workspace_manifest -- --nocapture` | PASS | 6 tests passed, including blob-storage feature implication and publish/internal dependency guards. |

## Follow-up Actions

1. Keep `canic` defaults narrow; control-plane, sharding, auth proof, and
   blob-storage billing surfaces should remain explicit features.
2. Keep root/issuer canister-signature creation and verification dependencies
   optional in `canic-core`.
3. Keep blob-storage billing probes, test canisters, and integration harnesses
   `publish = false`.
4. Keep `canic-cli`, `canic-host`, and `canic-backup` dependency direction
   one-way and facade-free.
