# Dependency Hygiene Audit - 2026-06-19

## Report Preamble

- Scope: workspace root `Cargo.toml`; published crate manifests under
  `crates/canic`, `crates/canic-backup`, `crates/canic-cli`,
  `crates/canic-control-plane`, `crates/canic-core`, `crates/canic-host`,
  `crates/canic-macros`, and `crates/canic-wasm-store`; internal manifests
  under `crates/canic-testing-internal`, `crates/canic-tests`,
  `canisters/test/**`, `canisters/audit/**`, `canisters/sandbox/**`, and
  `fleets/**`.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-29/dependency-hygiene.md`
- Code snapshot identifier: `16894709` with dirty worktree.
- Method tag/version: `dependency-hygiene-current`.
- Comparability status: `non-comparable`: the live audit definition now
  requires standard recurring-audit structural hotspot, hub-pressure,
  fan-in-pressure, early-warning, and `## Risk Score` sections. Core manifest
  and package-boundary checks remain comparable as mechanical context.
- Exclusions applied: lockfile-only noise, generated target output, `.icp`
  runtime cache, local `.tmp` test-runtime manifests, and code-level Rust
  implementation changes outside manifest/package boundaries.
- Notable methodology changes vs baseline: report-section contract expanded;
  current focus questions now explicitly cover the post-0.68 delegated-auth
  feature split and root-proof provisioning fixture posture.
- Auditor: `codex`.
- Run timestamp: `2026-06-19`.
- Worktree: `dirty`; unrelated Rust source changes were left untouched.

## Executive Summary

Verdict: **PASS**.

No dependency hygiene violation was found. Published crates still avoid
runtime dependencies on unpublished workspace members. `canic-cli`,
`canic-host`, and `canic-backup` keep the intended one-way operator direction,
with no `canic-cli -> canic` facade edge. `canic` default features remain narrow
at `default = ["metrics"]`; control-plane, sharding, and delegated-auth proof
creation/verification surfaces remain explicit feature choices.

Overall dependency hygiene risk index: **2 / 10**.

## Baseline Capture

| Metric | Previous | Current | Delta |
| ------ | -------: | ------: | ----: |
| Published crates with internal runtime edges | 0 | 0 | N/A |
| Published crates with test-only leakage concerns | 0 | 0 | N/A |
| Optional features reviewed | 6 | 8 | N/A |
| Publish-surface mismatches | 0 | 0 | N/A |
| Duplicate or overlapping support seams | 0 | 0 | N/A |
| Published crates with path-only or workspace-fragile package assumptions | 0 | 0 | N/A |
| Public crates with default-feature widening concerns | 0 | 0 | N/A |

Deltas are marked `N/A` because the report structure changed materially. The
mechanical seam checks still show no regression against the prior package
boundary posture.

## Structural Hotspots

| Hotspot | Manifest / Field Evidence | Why It Matters | Risk |
| ------- | ------------------------- | -------------- | ---- |
| Public facade default surface | `crates/canic/Cargo.toml`: `default = ["metrics"]`; explicit `control-plane`, `sharding`, and auth feature aliases | This is the public downstream entry point. Default widening would immediately affect canister users. | Low |
| Auth proof dependency gates | `crates/canic-core/Cargo.toml`: optional `ic-canister-sig-creation`, `ic-certification`, and `ic-signature-verification` behind auth features | Canister-signature creation/verification dependencies must stay tied to the owning auth behavior. | Low |
| Operator package direction | `crates/canic-cli/Cargo.toml`: depends on `canic-backup`, `canic-core`, and `canic-host`; not `canic` | Prevents operator tooling from becoming another canister facade. | Low |
| Internal harness sink | `crates/canic-testing-internal/Cargo.toml` and `crates/canic-tests/Cargo.toml`: `publish = false`, depend on `ic-testkit` | Keeps test infrastructure from leaking into published runtime surfaces. | Low |
| Canister artifact package | `crates/canic-wasm-store/Cargo.toml`: `publish = true`, `crate-type = ["cdylib"]`, depends on `canic` with `control-plane` | This is intentionally publishable as an artifact crate, but it must not emit an `rlib`. | Low |

## Hub Module Pressure

| Crate / Package Hub | Fan-In Signal | Fan-Out / Feature Signal | Pressure Score | Basis |
| ------------------- | ------------- | ------------------------ | -------------: | ----- |
| `canic` | `cargo tree -i canic --locked` shows runtime/build fan-in from canister fixtures, fleets, `canic-wasm-store`, and internal tests | Public facade has 8 feature entries, only `metrics` default-on | 3 / 10 | Expected public facade hub; pressure is bounded by narrow defaults and unpublished fixtures. |
| `canic-core` | `cargo tree -i canic-core --locked` shows fan-in from facade, CLI, host, control-plane, tests, and fixtures | Auth optional deps are feature-gated; default features are empty | 3 / 10 | Core runtime hub by design; no internal reverse dependency violation. |
| `canic-cli` | No reverse package fan-in in inspected public support crates | Depends on host/backup/core and external CLI/data crates | 2 / 10 | Operator leaf package with broad command responsibility but clean direction. |
| `canic-host` | `cargo tree -i canic-host --locked` shows `canic-cli` runtime fan-in and `canic-tests` dev fan-in | Host-side deployment support depends on core, not facade | 2 / 10 | Correct operator support layer; no CLI reverse edge. |
| `canic-testing-internal` | `cargo tree -i canic-testing-internal --locked` shows only `canic-tests` | Internal-only test harness depends on facade/core/control-plane/testkit | 1 / 10 | Proper unpublished sink. |

## Dependency Fan-In Pressure

| Crate / Feature | Incoming Edge Signal | Evidence | Pressure or Violation | Risk |
| --------------- | -------------------- | -------- | --------------------- | ---- |
| `canic` | Broad canister/fleet/build fan-in | `cargo tree -i canic --locked` | Pressure: public facade breadth is expected, but default feature widening would amplify downstream builds. | Low |
| `canic-core` | Runtime fan-in from facade, CLI, host, control-plane, internal tests, and fixtures | `cargo tree -i canic-core --locked` | Pressure: central runtime crate; no internal edge or reverse-cycle violation found. | Low |
| `canic-control-plane` | Fan-in from facade feature, wasm-store, internal harnesses, and root fixtures | `cargo tree -i canic-control-plane --locked` | Pressure: explicit control-plane feature and unpublished root fixtures keep this bounded. | Low |
| `canic-testing-internal` | Internal test package only | `cargo tree -i canic-testing-internal --locked` | none: unpublished sink only. | Low |
| `ic-testkit` | Internal harness and `runtime_probe` dev-dependency | `cargo tree -i ic-testkit --locked` | none: no published runtime edge. | Low |

## Early Warning Signals

| Signal | Evidence | Status | Trigger to Revisit |
| ------ | -------- | ------ | ------------------ |
| Default-feature widening | `crates/canic/Cargo.toml`: `default = ["metrics"]`; `cargo tree -p canic --depth 1 --locked` excludes `canic-control-plane` | Clean | Any addition to `canic` defaults beyond narrow facade-owned behavior. |
| Public feature aliases exposing internal layout | `canic` aliases `canic-core` sharding/auth features | Bounded pressure | Revisit if aliases stop mapping one-to-one to core-owned behavior. |
| Optional auth dependencies becoming always-on | `cargo tree -p canic-core --depth 1 --locked` excludes `ic-canister-sig-creation`, `ic-certification`, and `ic-signature-verification`; feature tree includes them only with auth features | Clean | Revisit if any auth proof dependency becomes non-optional or default-on. |
| Path-only package assumptions | Published crate manifests use workspace dependencies; path aliases live in root workspace definitions. Internal audit/sandbox canisters use path-only `canic` edges with `publish = false`. | Clean for published crates | Revisit if a publishable crate adds a direct path-only dependency. |
| Build-script/proc-macro workspace coupling | `canic-macros` is proc-macro; canister artifact crates use `canic` as build-dependency; `workspace_manifest` guard passed | Bounded pressure | Revisit if build-dependencies point at unpublished crates from published packages. |
| Internal harness leakage | `canic-testing-internal`, `canic-tests`, and test/audit/sandbox canisters are `publish = false`; `workspace_manifest` guard passed | Clean | Revisit if a published crate depends on `canic-testing-internal`, `canic-tests`, or `ic-testkit`. |
| Support crates becoming alternate facades | `canic-host` and `canic-cli` depend on `canic-core`, not `canic`; `canic-backup` has no runtime/operator dependency | Clean | Revisit if host or CLI begins re-exporting facade behavior or depends on `canic`. |

## Crate Dependency Direction

| Crate | Publish Intent | Runtime Depends On | Optional Depends On | Build Depends On | Dev Depends On | Internal Runtime Edge Found? | Reverse/Upward Pressure Found? | Risk |
| ----- | -------------- | ------------------ | ------------------- | ---------------- | -------------- | ---------------------------- | ------------------------------ | ---- |
| `canic` | published facade | `candid`, `canic-core`, `canic-macros`, non-wasm `flate2`/`toml` | `canic-control-plane` through `control-plane` | package build script; no internal crate edge | `candid_parser`, `ic-cdk`, `serde_json` | no | bounded facade pressure | Low |
| `canic-core` | published runtime core | IC SDK/support crates, `ic-memory`, ledger types, serialization/hash/error crates | `ic-canister-sig-creation`, `ic-certification`, `ic-signature-verification` | `candid` | `criterion`, `futures` | no | central runtime fan-in pressure | Low |
| `canic-control-plane` | published support/runtime crate | `canic-core`, `ic-memory`, `async-trait`, `candid`, `serde`, `sha2`, `thiserror` | none | none | none | no | below facade, above core support; bounded | Low |
| `canic-macros` | published proc-macro crate | `proc-macro2`, `quote`, `syn` | none | none | none | no | public macro support only | Low |
| `canic-wasm-store` | published canister artifact | `candid`, `canic` with `control-plane`, `ic-cdk` | none | `canic` | none | no | artifact package pressure; guarded as `cdylib` only | Low |
| `canic-backup` | published backup/restore library | `candid`, `serde`, `serde_json`, `sha2`, `thiserror` | none | none | none | no | independent of facade/operator/runtime crates | Low |
| `canic-host` | published host/operator library | `candid`, `candid_parser`, `canic-core`, compression, serialization/hash/error/TOML crates | none | none | none | no | facade-free operator support | Low |
| `canic-cli` | published binary/library package | `candid`, `canic-backup`, `canic-core`, `canic-host`, `clap`, serialization/error crates | none | binary target only | none | no | correct operator leaf direction | Low |
| `canic-testing-internal` | internal (`publish = false`) | `canic`, `canic-control-plane`, `canic-core`, `ic-testkit`, `candid` | none | none | none | internal-only | unpublished sink | Low |
| `canic-tests` | internal (`publish = false`) | `canic`, `canic-testing-internal`, `ic-testkit` | none | none | `candid`, `canic-control-plane`, `canic-core`, `canic-host`, `ic-memory`, serialization crates | internal-only | unpublished sink | Low |
| `canisters/**`, `fleets/**` | internal fixtures/artifacts (`publish = false`) | `canic`, `candid`, `ic-cdk`, selected features | selected facade features only | `canic` and fixture-specific build helpers | `runtime_probe` dev-depends on `ic-testkit` | internal-only | unpublished fixture pressure | Low |

## Public/Internal Seam Checks

| Seam | Status | Evidence | Pressure or Violation | Risk |
| ---- | ------ | -------- | --------------------- | ---- |
| Published crates must not depend on `canic-testing-internal` | clean | `workspace_manifest` passed; `rg` shows `canic-testing-internal` only in workspace aliases and internal crates | none | Low |
| Published crates must not depend on `canic-tests` | clean | `crates/canic-tests/Cargo.toml` is `publish = false`; no published manifest references it | none | Low |
| Generic test infrastructure must stay outside published runtime crates | clean | `cargo tree -i ic-testkit --locked` shows internal harnesses and `runtime_probe` dev-dependency only | none | Low |
| Operator library direction | clean | `cargo tree -p canic-cli --depth 1 --locked`; `cargo tree -p canic-host --depth 1 --locked` | bounded pressure | Low |
| Backup domain independence | clean | `cargo tree -p canic-backup --depth 1 --locked` has no `canic`, `canic-core`, `canic-host`, or `canic-cli` dependency | none | Low |
| Canister artifact boundary | clean | `workspace_manifest` `cdylib_members_do_not_emit_rlib_artifacts` passed; `canic-wasm-store` has `crate-type = ["cdylib"]` | none | Low |

## Feature Hygiene

| Crate | Feature | Enables | Default? | Public/User-Facing? | Responsibility Fit | Pressure or Violation | Risk |
| ----- | ------- | ------- | -------- | ------------------- | ------------------ | --------------------- | ---- |
| `canic` | `metrics` | facade-owned metrics support | yes | yes | narrow default behavior | none | Low |
| `canic` | `control-plane` | optional `canic-control-plane` dependency | no | yes | explicit root/store orchestration surface | none | Low |
| `canic` | `icp-refill` | facade feature marker | no | yes | narrow named feature | none | Low |
| `canic` | `sharding` | `canic-core/sharding` | no | yes | explicit placement behavior | none | Low |
| `canic` | `auth-root-canister-sig-create` | `canic-core/auth-root-canister-sig-create` | no | yes | explicit root proof creation support | none | Low |
| `canic` | `auth-root-canister-sig-verify` | `canic-core/auth-root-canister-sig-verify` | no | yes | explicit root proof verification support | none | Low |
| `canic` | `auth-issuer-canister-sig-create` | `canic-core/auth-issuer-canister-sig-create` | no | yes | explicit issuer proof creation support | none | Low |
| `canic` | `auth-issuer-canister-sig-verify` | `canic-core/auth-issuer-canister-sig-verify` | no | yes | explicit issuer proof verification support | none | Low |
| `canic` | `auth-delegated-token-verify` | `canic-core/auth-delegated-token-verify` | no | yes | explicit protected endpoint verification support | none | Low |
| `canic-core` | `sharding` | lower-level placement behavior | no | through facade/core users | role-aligned | none | Low |
| `canic-core` | `auth-root-canister-sig-create` | `ic-canister-sig-creation`, `ic-certification` | no | through facade/core users | role-aligned optional proof creation deps | none | Low |
| `canic-core` | `auth-issuer-canister-sig-create` | `ic-canister-sig-creation`, `ic-certification` | no | through facade/core users | role-aligned optional proof creation deps | none | Low |
| `canic-core` | `auth-root-canister-sig-verify` | `ic-signature-verification` | no | through facade/core users | role-aligned optional verification deps | none | Low |
| `canic-core` | `auth-issuer-canister-sig-verify` | `ic-signature-verification` | no | through facade/core users | role-aligned optional verification deps | none | Low |
| `canic-core` | `auth-delegated-token-verify` | root and issuer verification features | no | through facade/core users | composed verification feature | none | Low |

## Package / Publish Surface

| Crate | Publish Intent | Package Surface Concern | Evidence | Pressure or Violation | Risk |
| ----- | -------------- | ----------------------- | -------- | --------------------- | ---- |
| `canic` | published | public facade graph | `Cargo.toml` and `cargo tree -p canic --depth 1 --locked`; default excludes control-plane/auth/sharding | bounded pressure | Low |
| `canic-core` | published | central runtime dependency hub | `Cargo.toml` and `cargo tree -p canic-core --depth 1 --locked`; optional auth deps absent by default | bounded pressure | Low |
| `canic-host` | published | host/operator graph | `cargo tree -p canic-host --depth 1 --locked`; depends on `canic-core`, not `canic` | bounded pressure | Low |
| `canic-cli` | published | operator graph | `cargo tree -p canic-cli --depth 1 --locked`; depends on host/backup/core, not facade | bounded pressure | Low |
| `canic-backup` | published | backup package independence | `cargo tree -p canic-backup --depth 1 --locked`; only serialization/hash/error deps | none | Low |
| `canic-wasm-store` | published artifact | canister artifact package, not reusable Rust library | `crate-type = ["cdylib"]`; `workspace_manifest` guard passed | bounded pressure | Low |
| `canic-testing-internal`, `canic-tests` | internal | must remain unpublished sinks | `publish = false`; reverse tree shows only internal fan-in | none | Low |
| `canisters/**`, `fleets/**` | internal fixtures/artifacts | feature-enabled canisters must not become reusable package surfaces | `rg` manifest scan shows `publish = false` for auth/control-plane/sharding fixture packages | none | Low |

## Redundant / Overlapping Support Seams

| Area | Overlap Signal | Evidence | Pressure or Violation | Risk |
| ---- | -------------- | -------- | --------------------- | ---- |
| Public facade vs lower-level crates | `canic` exposes facade features that mirror `canic-core` feature ownership | `crates/canic/Cargo.toml` feature aliases | Pressure: intentional facade mapping, not a violation while aliases remain one-to-one and default-off. | Low |
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
| root/delegated-auth fixture canisters | feature-enabled unpublished artifacts | Several test/fleet canisters enable auth/control-plane features, but all are `publish = false`. | Medium: any publish posture change would need review. | Low |
| workspace path aliases | package inheritance pressure | Workspace dependencies use local paths with matching versions. Published crates inherit them through workspace declarations. | Low: normal workspace packaging pattern; package checks should keep guarding this. | Low |

## Risk Score

| Category | Risk Index (1-10, lower is better) | Basis |
| -------- | ---------------------------------: | ----- |
| Runtime Dependency Direction | 1 | no published crate runtime edge into `publish = false` crates was found |
| Public/Internal Seam Discipline | 1 | internal harnesses remain unpublished and one-way |
| Feature Hygiene | 2 | auth/control-plane/sharding features are explicit and default-off; facade alias breadth is bounded pressure |
| Package / Publish Surface | 2 | operator and artifact crates are publishable surfaces but have clean direction and guards |
| Support-Crate Ownership Clarity | 2 | facade/core/control-plane/host/backup split remains intentional and bounded |

### Overall Dependency Hygiene Risk Index (1-10, lower is better)

**2 / 10**.

No High or Critical dependency/package violation was found. The main residual
risk is ordinary facade/core hub pressure: future default-feature widening,
auth optional-dependency drift, or publishing a feature-enabled fixture canister
would have broad package impact.

## Delta Since Baseline

| Delta Type | Crate / Edge / Feature | Previous | Current | Impact |
| ---------- | ---------------------- | -------- | ------- | ------ |
| Methodology | report structure | 2026-05-29 report did not require standard recurring structural/hub sections | current report includes required standard sections | non-comparable report shape |
| Feature naming | auth features | prior report referenced stale `auth-crypto` examples | current manifests use root/issuer canister-signature create/verify and delegated-token verify features | documentation/reporting drift corrected in current run |
| Runtime direction | published crates to internal crates | 0 | 0 | no regression |
| Default features | `canic` | `default = ["metrics"]` | `default = ["metrics"]` | no regression |

## Verification Readout

Status: **PASS**.

Commands passed:

- `cargo metadata --no-deps --format-version 1`
- `rg -n "^publish\\s*=\\s*false|^publish\\s*=\\s*true|canic\\s*=\\s*\\{[^\\n]*(features|path|workspace)|ic-testkit|canic-testing-internal|canic-tests|crate-type|proc-macro|optional\\s*=\\s*true|default-features|\\[features\\]" Cargo.toml crates canisters fleets -g Cargo.toml`
- `rg -n "path\\s*=" crates/canic*/Cargo.toml Cargo.toml -g Cargo.toml`
- `cargo tree -p canic --depth 1 --locked`
- `cargo tree -p canic-core --depth 1 --locked`
- `cargo tree -p canic-host --depth 1 --locked`
- `cargo tree -p canic-cli --depth 1 --locked`
- `cargo tree -p canic-backup --depth 1 --locked`
- `cargo tree -p canic-wasm-store --depth 1 --locked`
- `cargo tree -i canic --locked`
- `cargo tree -i canic-core --locked`
- `cargo tree -i canic-host --locked`
- `cargo tree -i canic-backup --locked`
- `cargo tree -i canic-testing-internal --locked`
- `cargo tree -i ic-testkit --locked`
- `cargo tree -i canic-control-plane --locked`
- `cargo tree -p canic --all-features --depth 1 --locked`
- `cargo tree -p canic-core --features auth-root-canister-sig-create,auth-root-canister-sig-verify,auth-issuer-canister-sig-create,auth-issuer-canister-sig-verify,auth-delegated-token-verify --depth 1 --locked`
- `cargo test -p canic --test workspace_manifest --locked -- --nocapture`

The run is not blocked. Manifest and package evidence was sufficient for the
dependency-hygiene judgment.

## Follow-up Actions

1. Keep `canic` defaults narrow; control-plane, sharding, and auth proof
   surfaces should remain explicit features.
2. Keep root/issuer canister-signature creation and verification dependencies
   optional in `canic-core`.
3. Keep auth/control-plane-enabled fixture and fleet canisters `publish = false`.
4. Keep `canic-cli`, `canic-host`, and `canic-backup` dependency direction
   one-way and facade-free.
