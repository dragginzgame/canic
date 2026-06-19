# Publish Surface Audit - 2026-06-19

## Report Preamble

- Scope: workspace root package policy; the eight current published crates
  under `crates/canic`, `crates/canic-backup`, `crates/canic-cli`,
  `crates/canic-control-plane`, `crates/canic-core`, `crates/canic-host`,
  `crates/canic-macros`, and `crates/canic-wasm-store`; package-local README
  posture; public binary/example/bench surface where present; retained
  installed/packaged proof scripts; current release package/install validation
  docs; package verification output from `cargo package`.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-01/publish-surface.md`
- Code snapshot identifier: `ef55e53c`
- Method tag/version: `publish-surface-current-v2`
- Comparability status: comparable.
- Exclusions applied: unpublished crates with `publish = false`, fleet/test/
  audit/sandbox canister crates, generated package unpack directories, and
  scripts not presented as part of the installed/packaged release contract.
- Notable methodology changes vs baseline: the audit definition now explicitly
  treats `docs/operations/release-package-install-validation.md`,
  `docs/operations/release-validation-matrix.md`, and
  `docs/operations/README.md` as the current package/install validation entry
  points, with the retained `0.56-*` probe docs as supporting inventories.

## Audit Selection

This was the next oldest recurring audit. The previous publish-surface report
was dated 2026-06-01.

## Audit Definition Maintenance

The audit definition was reviewed before execution. One small maintenance issue
was found: the retained installed/packaged proof checklist still named the
versioned `0.56-*` probe docs but did not name the newer non-versioned release
package/install validation docs that now act as the current entry point.

The definition now includes:

- `docs/operations/release-package-install-validation.md`;
- `docs/operations/release-validation-matrix.md`;
- `docs/operations/README.md`;
- the retained `0.56-*` probe docs as historical/supporting inventories.

No package-code cleanup was required.

## Executive Summary

Verdict: **PASS**.

Overall publish surface risk: **2 / 10**.

The package contract remains healthy. The current published set is still eight
crates, all eight have package-local README posture, docs.rs metadata,
repository/homepage metadata, and clear package roles. The intended public entry
points remain:

- `canic` for canister projects;
- `canic-cli` for the installed `canic` operator binary;
- `canic-wasm-store` for the special canonical `wasm_store` canister artifact
  source, not ordinary Rust `rlib` dependency use.

No High or Critical publish-surface violation was found. Residual pressure is
the same expected pressure as the previous run: lower-level crates are
intentionally thinner than the facade, and the packaged `wasm_store` path is a
special bootstrap/runtime proof rather than normal canister dependency
guidance.

## Baseline Capture

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Published crates reviewed | 8 | 8 | 0 |
| Published crates with thin docs posture | 2 | 2 | 0 |
| Published crates with `readme = false` pressure | 0 | 0 | 0 |
| Publish-surface mismatches | 0 | 0 | 0 |
| Published crates with binary/example posture pressure | 1 | 1 | 0 |
| Alternate-facade ambiguity seams | 2 | 2 | 0 |
| Published crates with default-feature contract pressure | 0 | 0 | 0 |
| Publishable-but-underspecified crates | 0 | 0 | 0 |

## Manifest Publish Posture

| Crate | Publish Intent | `publish` Posture | README / docs.rs Metadata | Binary / Example Surface | Package Contract Clarity | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | main public facade | `publish = true`; default feature `metrics`; `rust-version = 1.91.0` | `README.md`, docs.rs, repository, homepage | library facade and package tests | clear primary entry surface | Low |
| `canic-backup` | backup/restore domain primitives | `publish = true`; `rust-version = 1.91.0` | `README.md`, docs.rs, repository, homepage | library only | clear role-specific support crate | Low |
| `canic-cli` | installed operator CLI | `publish = true`; `[[bin]] name = "canic"`; `rust-version = 1.91.0` | `README.md`, docs.rs, repository, homepage | public `canic` binary | clear binary/tool contract | Low |
| `canic-control-plane` | lower-level root/store support crate | `publish = true`; `rust-version = 1.91.0` | `README.md`, docs.rs, repository, homepage | library only | clear lower-level control-plane role | Low |
| `canic-core` | lower-level runtime/support crate | `publish = true`; `default = []`; `rust-version = 1.91.0` | `README.md`, docs.rs, repository, homepage | library plus `serialize` bench | lower-level role is explicit | Low |
| `canic-host` | host-side build/deployment/evidence/fleet support | `publish = true`; `rust-version = 1.91.0` | `README.md`, docs.rs, repository, homepage | library plus `build_artifact` example | clear host-library role | Low |
| `canic-macros` | proc-macro support crate | `publish = true`; `proc-macro`; `rust-version = 1.91.0` | `README.md`, docs.rs, repository, homepage | proc-macro library | support role is explicit | Low |
| `canic-wasm-store` | canonical special `wasm_store` canister artifact source | `publish = true`; `crate-type = ["cdylib"]`; `rust-version = 1.91.0` | `README.md`, docs.rs, repository, homepage | canister artifact source only | special role is explicit | Low |

`canic-testing-internal` and `canic-tests` remain `publish = false`, and
workspace/fleet/test canister crates are excluded from the published crate
count.

## README / Docs Alignment

| Crate | README Posture | Standalone-Ready? | Redirect/Thin-Wrapper Signal | Downstream Contract Impact | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | facade crate, default surface, feature list, typical canister use | yes | points to workspace guide for full setup | users can identify `canic::build!`, `canic::start!`, and role metadata as the normal path | none | Low |
| `canic-cli` | installed operator binary and compact v1 command set | yes | says external callers should treat installed binary as operator interface | users see CLI as the supported command surface | none | Low |
| `canic-host` | lower-level host build/install/staging library | mostly | tells normal operators to prefer `canic-cli` and install docs | direct host-library use is scoped to automation/backend work | pressure only | Low |
| `canic-core` | lower-level runtime/support crate | mostly | tells normal projects to use `canic` | lower-level role is explicit enough for package users | pressure only | Low |
| `canic-control-plane` | lower-level root and `wasm_store` support | yes | tells normal projects to use `canic` | role-specific surface is explicit | none | Low |
| `canic-macros` | proc-macro support crate | yes | tells users to prefer `canic` re-exports | direct macro use is possible but not presented as primary | none | Low |
| `canic-backup` | backup/restore contracts | yes | CLI relationship is explicit | role-specific host contract is clear | none | Low |
| `canic-wasm-store` | special canister artifact source | yes | says ordinary canisters should use `canic` | avoids reusable `rlib` dependency confusion | none | Low |

## Example / Binary Surface

| Crate | Surface Item | Surface Type | What It Implies To Users | Supported / Intended? | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic-cli` | `[[bin]] name = "canic"` | binary | installed operator command is public | yes | none | Low |
| `canic-cli` | compact v1 command list in README | CLI examples | setup/build/evidence/catalog flows are explicit public commands | yes | none | Low |
| `canic-host` | `examples/build_artifact.rs` | example | lower-level host artifact builder can be run for role artifacts | yes; used by packaged `wasm_store` proof | none | Low |
| `canic-core` | `benches/serialize.rs` | bench | lower-level serialization/proof-shape maintenance surface | yes | pressure only | Low |
| `canic-macros` | README macro snippets | code examples | direct macro import is possible | yes, with facade-preferred note | none | Low |
| `canic-wasm-store` | `wasm_store.did` | package artifact | crate owns canonical special DID | yes | none | Low |

## Feature / Package Contract Alignment

| Crate | Feature / Package Lever | Default? | What It Widens | Docs / README Alignment | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | `metrics` | yes | exports `canic_metrics` in ordinary builds | README documents it as the small default surface | none | Low |
| `canic` | `control-plane`, `sharding`, auth canister-signature features | no | optional root/control-plane/sharding/auth proof support | README lists them explicitly | none | Low |
| `canic-core` | `auth-*` and `sharding` | no | lower-level runtime/proof surfaces | README says normal users should prefer facade | pressure only | Low |
| `canic-wasm-store` | `crate-type = ["cdylib"]` | yes | canister artifact output only | README says no reusable `rlib` dependency | none | Low |
| Workspace | `rust-version = "1.91.0"` | yes | published MSRV package contract | package manifests inherit it consistently | none | Low |

The `canic-host` README contains a tagged installer URL for the current package
line. That is release-surface material; this ordinary audit verified posture
but did not change install URLs or package versions.

## Installed / Packaged Proof Surface

| Proof | Package Contract Claim | Repository Shortcut Guard | Current v1 Surface? | Risk |
| --- | --- | --- | --- | --- |
| `scripts/ci/verify-installed-canic-cli.sh` | installed `canic` binary can run maintained v1 readiness smoke | rejects `target/debug/canic`; installs into a temp root | yes | Low |
| `scripts/ci/verify-packaged-downstream-cli.sh` | packaged CLI/support crates run current read-only downstream CLI commands | rejects repository crate paths and `target/debug/canic`; creates archives first | yes | Low |
| `scripts/ci/verify-packaged-downstream-wasm-store.sh` | special generated `wasm_store` wrapper uses packaged Canic sources | rejects repository crate paths and `target/debug/canic`; excludes packaged `canic-wasm-store` to exercise wrapper path | special bootstrap/runtime proof | Low |
| `docs/operations/release-package-install-validation.md` | current non-versioned package/install validation inventory | says package gates must avoid repo shortcuts and not commit artifacts | yes | Low |
| `docs/operations/release-validation-matrix.md` | release-readiness accounting for package/install gates | classifies package gates as RC/final-release checks | yes | Low |
| retained `0.56-*` probe docs | supporting proof inventories for installed/packaged v1 probes | document the same repository shortcut guards | retained support docs | Low |

The versioned `0.56-*` operation docs are explicitly retained probe
inventories. The current entry point is the non-versioned package/install
validation checklist.

## Package Verification

Command:

```bash
cargo package -p canic -p canic-backup -p canic-cli -p canic-control-plane -p canic-core -p canic-host -p canic-macros -p canic-wasm-store --locked --allow-dirty
```

Result: PASS.

Packaged crate sizes:

| Crate | `.crate` size |
| --- | ---: |
| `canic` | 52,024 bytes |
| `canic-backup` | 105,793 bytes |
| `canic-cli` | 232,456 bytes |
| `canic-control-plane` | 61,797 bytes |
| `canic-core` | 427,736 bytes |
| `canic-host` | 340,297 bytes |
| `canic-macros` | 14,796 bytes |
| `canic-wasm-store` | 11,860 bytes |

`canic-wasm-store` packaged manifest still exposes only a `cdylib` canister
artifact target:

```toml
[lib]
name = "canister_wasm_store"
crate-type = ["cdylib"]
path = "src/lib.rs"
```

## Delta Since Baseline

| Delta Type | Crate / Surface | Previous | Current | Impact |
| --- | --- | --- | --- | --- |
| Crate count | published crate set | 8 | 8 | no package-count drift |
| Workspace version | package metadata | `0.67`/early `0.68` line in prior snapshot | `0.68.25` | ordinary current-line movement; no role change |
| Package sizes | all eight crates | prior archive sizes | current archive sizes above | expected growth, especially `canic-core`/`canic-host`, but package roles remain clear |
| Release proof docs | installed/packaged proof surface | versioned probe docs named directly | non-versioned validation docs now named as current entry point | reduced stale-version audit pressure |
| Default features | `canic` | `metrics` default documented | `metrics` default documented | unchanged |
| Special canister package shape | `canic-wasm-store` | `cdylib` only | `cdylib` only | unchanged |

## Alternate Facade / Ownership Ambiguity

| Area | Ambiguity Signal | Evidence | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- |
| `canic` vs `canic-core` | lower-level runtime crate is published | `canic-core` README says normal projects should depend on `canic` and explains lower-level module map | pressure only | Low |
| `canic-cli` vs `canic-host` | host support crate is published and has a runnable example | `canic-host` README tells normal operators to prefer installed CLI and scopes direct use to backend/automation | pressure only | Low |
| `canic` vs `canic-macros` | proc-macro crate can be used directly | `canic-macros` README tells most users to use `canic` re-exports | none | Low |
| `canic` vs `canic-wasm-store` | special canister crate is published | `canic-wasm-store` README says it is a canister artifact source, not an `rlib` dependency | none | Low |

## Risk Index

| Category | Risk Index | Basis |
| --- | ---: | --- |
| Manifest Publish Discipline | 1 | eight published crates have explicit package posture, docs metadata, and inherited MSRV |
| README / Docs Contract Clarity | 2 | lower-level docs are intentionally thinner but clearly redirect users |
| Example / Binary Surface Discipline | 2 | CLI binary is clear; host example and core bench are bounded |
| Feature / Default Surface Discipline | 1 | `canic` default surface remains documented and narrow |
| Facade / Ownership Clarity | 2 | facade/operator/support roles are explicit |

Overall publish surface risk: **2 / 10**.

## Verification Readout

| Check | Status | Notes |
| --- | --- | --- |
| Audit definition review | PASS | definition updated to include non-versioned release package/install validation docs |
| Manifest inspection | PASS | reviewed publish posture, metadata, crate types, default features, binary/example targets |
| Package-local README inspection | PASS | reviewed all eight published crate READMEs |
| Example/bench/binary inspection | PASS | reviewed CLI binary target, host example, core bench, macro examples, and wasm-store DID posture |
| Installed/packaged proof script inspection | PASS | proof scripts package first and guard against repository shortcuts |
| Current release validation docs inspection | PASS | non-versioned package/install docs are the current entry point |
| `cargo metadata --no-deps --format-version 1 --locked` | PASS | confirmed package metadata and publish posture |
| `cargo test --locked -p canic --test workspace_manifest -- --nocapture` | PASS | 5 tests |
| `cargo test --locked -p canic --test changelog_governance -- --nocapture` | PASS | 1 test |
| `cargo package ... --locked --allow-dirty` | PASS | packaged and verified all eight published crates |
| `cargo fmt --all -- --check` | PASS | formatting unchanged |
| `git diff --check` | PASS | no whitespace errors |

## Final Verdict

PASS.

No publish-surface violations were found. Continue to treat `canic-core` and
`canic-host` as intentional lower-level pressure points, and keep
`canic-wasm-store` framed as a special canister artifact source rather than an
ordinary reusable Rust dependency.
