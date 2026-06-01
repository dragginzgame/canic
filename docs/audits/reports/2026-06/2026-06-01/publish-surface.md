# Publish Surface Audit - 2026-06-01

## Report Preamble

- Scope: workspace root package policy; the eight current published crates
  under `crates/canic`, `crates/canic-backup`, `crates/canic-cli`,
  `crates/canic-control-plane`, `crates/canic-core`, `crates/canic-host`,
  `crates/canic-macros`, and `crates/canic-wasm-store`; package-local README
  posture; public binary/example/bench surface where present; retained
  installed/packaged proof scripts; package verification output from
  `cargo package`.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-11/publish-surface.md`
- Code snapshot identifier: `27a00430`
- Method tag/version: `publish-surface-current-v1`
- Comparability status: partially comparable. The baseline still counted
  historical crates removed by later hard cuts (`canic-cdk`, `canic-memory`,
  `canic-testkit`), while the current package contract is the post-0.56
  eight-crate surface.
- Exclusions applied: unpublished crates with `publish = false`, fleet/test/
  audit/sandbox canister crates, generated package unpack directories, and
  scripts not presented as part of the installed/packaged release contract.
- Notable methodology changes vs baseline: this run includes the post-0.56
  installed/packaged proof posture and checks the `canic-wasm-store` package as
  a `cdylib`-only canister artifact source.

## Executive Summary

Verdict: **PASS**

Overall publish surface risk: **2 / 10**.

The package contract is healthy. The current published set is eight crates, all
of them have package-local README posture, docs.rs metadata, repository/homepage
metadata, and a clear role in the current package map. The main user-facing
entry points remain:

- `canic` for canister projects;
- `canic-cli` for the installed `canic` operator binary;
- `canic-wasm-store` only for the special canonical bootstrap/runtime
  `wasm_store` canister artifact source.

No High/Critical publish-surface violations were found. The remaining pressure
is intentional thinness in lower-level crates and the special nature of the
`wasm_store` package.

## Audit Definition Refresh

The recurring definition was tightened for the current hard-cut posture:

- current published crate count is eight;
- removed historical crates must stay out of current counts;
- published MSRV is the workspace `rust-version` (`1.91.0`);
- `canic` default features must stay documented when they change;
- `canic-wasm-store` must remain a `cdylib` canister artifact source rather
  than an ordinary `rlib` dependency surface;
- install URLs and release-script default versions are release-preparation
  material and should not be bumped by ordinary audit slices.

## Baseline Capture

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Published crates reviewed | 11 | 8 | -3 |
| Published crates with thin docs posture | 2 | 2 | 0 |
| Published crates with `readme = false` pressure | 0 | 0 | 0 |
| Publish-surface mismatches | 1 | 0 | -1 |
| Published crates with binary/example posture pressure | 1 | 1 | 0 |
| Alternate-facade ambiguity seams | 3 | 2 | -1 |
| Published crates with default-feature contract pressure | 1 | 0 | -1 |
| Publishable-but-underspecified crates | 0 | 0 | 0 |

Comparability note: deltas are partially comparable because the current crate
set has been hard-cut to eight published crates. The removed historical crates
are no longer part of the current package contract.

## Manifest Publish Posture

| Crate | Publish Intent | Manifest Posture | README / Docs Metadata | Binary / Example Surface | Package Contract Clarity | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | main public facade | `publish = true`; default feature is `metrics`; `rust-version = 1.91.0` | package README, docs.rs, repository, homepage | library facade, tests | clear primary entry surface | Low |
| `canic-backup` | backup/restore domain primitives | `publish = true`; `rust-version = 1.91.0` | package README explains manifest/journal/restore contracts | library only | clear role-specific support crate | Low |
| `canic-cli` | installed operator CLI | `publish = true`; `[[bin]] name = "canic"`; `rust-version = 1.91.0` | README documents installed binary and compact v1 command set | public `canic` binary | clear binary/tool contract | Low |
| `canic-control-plane` | lower-level root/store support crate | `publish = true`; `rust-version = 1.91.0` | README tells most users to use `canic` | library only | clear lower-level role | Low |
| `canic-core` | lower-level runtime/support crate | `publish = true`; default features empty; `rust-version = 1.91.0` | README points normal users to `canic` and explains module map | library plus `serialize` bench | clear lower-level role | Low |
| `canic-host` | host-side build/deployment/evidence/fleet support | `publish = true`; `rust-version = 1.91.0` | README scopes direct use to backend/automation and points operators to CLI | library/examples | clear host-library role | Low |
| `canic-macros` | proc-macro support crate | `publish = true`; `proc-macro`; `rust-version = 1.91.0` | README says most users should access macros through `canic` | proc-macro library | clear support role | Low |
| `canic-wasm-store` | canonical special `wasm_store` canister artifact source | `publish = true`; `crate-type = ["cdylib"]`; `rust-version = 1.91.0` | README explicitly says not a reusable Rust dependency and no `rlib` | canister artifact source only | clear special role | Low |

## README / Docs Alignment

| Crate | README Posture | Standalone-Ready? | Redirect/Thin-Wrapper Signal | Downstream Contract Impact | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | facade and package metadata examples | yes | points to workspace guide for full setup | users know to use `build!`/`start!` and `fleet`/`role` metadata | none | Low |
| `canic-cli` | installed binary and compact v1 command set | yes | library surface intentionally narrow | users know CLI is the operator interface | none | Low |
| `canic-host` | host backend/library role | mostly | tells normal operators to prefer CLI | direct use is scoped to automation/backend work | pressure only | Low |
| `canic-core` | lower-level runtime crate | mostly | tells normal projects to use `canic` | lower-level role is explicit | pressure only | Low |
| `canic-control-plane` | root/store support | yes | tells normal projects to use `canic` | role-specific surface is explicit | none | Low |
| `canic-macros` | proc-macro support | yes | tells normal users to prefer `canic` re-exports | direct macro use is understandable but not primary | none | Low |
| `canic-backup` | backup/restore contracts | yes | none needed | role-specific host contract is explicit | none | Low |
| `canic-wasm-store` | special canister artifact source | yes | says ordinary canisters should use `canic` | avoids rlib/dependency misuse | none | Low |

## Example / Binary Surface

| Crate | Surface Item | Surface Type | What It Implies To Users | Supported / Intended? | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic-cli` | `[[bin]] name = "canic"` | binary | installed operator command is public | yes | none | Low |
| `canic-cli` | compact v1 commands in README | CLI examples | setup/build/evidence/catalog are explicit surfaces | yes | none | Low |
| `canic-core` | `benches/serialize.rs` | bench | lower-level serialization maintenance surface | yes | pressure only | Low |
| `canic-macros` | macro examples | README snippets | direct macro use is possible but facade preferred | yes | none | Low |
| `canic-wasm-store` | `wasm_store.did` | package artifact | crate owns canonical DID | yes | none | Low |

## Feature / Package Contract Alignment

| Crate | Feature / Package Lever | Default? | What It Widens | Docs / README Alignment | Pressure or Violation | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | `metrics` | yes | exposes `canic_metrics` in ordinary builds | documented as the default surface | none | Low |
| `canic` | `control-plane`, `sharding`, `auth-crypto` | no | optional runtime/control-plane features | documented as optional when defaults are off | none | Low |
| `canic-core` | `auth-crypto`, `sharding` | no | lower-level runtime surfaces | lower-level README states normal users should use facade | pressure only | Low |
| `canic-wasm-store` | `crate-type = ["cdylib"]` | yes | canister artifact output only | README says no `rlib` / not reusable dependency | none | Low |
| Workspace | `rust-version = "1.91.0"` | yes | published MSRV package contract | root README explains internal Rust may be newer | none | Low |

## Installed / Packaged Proof Surface

| Proof | Package Contract Claim | Repository Shortcut Guard | Current v1 Surface? | Risk |
| --- | --- | --- | --- | --- |
| `scripts/ci/verify-installed-canic-cli.sh` | installed `canic` binary can run maintained v1 smoke | rejects `target/debug/canic`; installs into temp root | yes | Low |
| `scripts/ci/verify-packaged-downstream-cli.sh` | packaged CLI/support crates work from unpacked package root | rejects repository crate paths and `target/debug/canic`; creates archives first | yes | Low |
| `scripts/ci/verify-packaged-downstream-wasm-store.sh` | special generated `wasm_store` wrapper uses packaged Canic sources | rejects repository crate paths and `target/debug/canic`; verifies packaged path patches | special bootstrap/runtime proof | Low |

The retained probe inventory still contains stale command shapes only as
negative guardrail examples. They are not presented as supported commands.

## Package Verification

Command:

```bash
cargo package -p canic -p canic-backup -p canic-cli -p canic-control-plane -p canic-core -p canic-host -p canic-macros -p canic-wasm-store --locked --allow-dirty
```

Result: PASS.

Packaged crate sizes:

| Crate | `.crate` size |
| --- | ---: |
| `canic` | 46,493 bytes |
| `canic-backup` | 96,769 bytes |
| `canic-cli` | 207,391 bytes |
| `canic-control-plane` | 61,637 bytes |
| `canic-core` | 338,657 bytes |
| `canic-host` | 291,478 bytes |
| `canic-macros` | 17,234 bytes |
| `canic-wasm-store` | 13,372 bytes |

`canic-wasm-store` packaged manifest verification:

```toml
[lib]
name = "canister_wasm_store"
crate-type = ["cdylib"]
path = "src/lib.rs"
```

The package archive includes `README.md`, `canic.toml`, `src/lib.rs`, and
`wasm_store.did`.

## Delta Since Baseline

| Delta Type | Crate / Surface | Previous | Current | Impact |
| --- | --- | --- | --- | --- |
| Crate count | published crate set | 11 | 8 | removed historical crates are no longer current package contract |
| Default features | `canic` | default README mismatch | default is `metrics`, README matches | default-feature pressure resolved |
| Canister package shape | `canic-wasm-store` | previous package still carried broader concern | `cdylib` only and README says no `rlib` | downstream dependency misuse risk reduced |
| Release proof posture | installed/packaged scripts | not part of May 11 method | included as package-contract evidence | current packaged story is verifiable |

## Risk Index

| Category | Risk Index | Basis |
| --- | ---: | --- |
| Manifest Publish Discipline | 1 | all eight publishable crates have explicit publish posture and package metadata |
| README / Docs Contract Clarity | 2 | lower-level crates are intentionally thin but clearly redirect users |
| Example / Binary Surface Discipline | 2 | CLI binary is clear; lower-level examples/bench surfaces are bounded |
| Feature / Default Surface Discipline | 1 | `canic` default feature docs match manifest |
| Facade / Ownership Clarity | 2 | `canic` facade is clear; support crates are role-specific or lower-level |

Overall publish surface risk: **2 / 10**.

## Verification Readout

| Check | Status | Notes |
| --- | --- | --- |
| Manifest inspection | PASS | reviewed publish posture, metadata, crate types, default features, binary targets |
| Package-local README inspection | PASS | reviewed all eight published crate READMEs |
| Stale-current-surface search | PASS | stale command shapes only appeared as negative guardrails |
| Retained proof script inspection | PASS | installed/packaged scripts guard against repo shortcuts |
| `cargo metadata --no-deps --format-version 1 --locked` | PASS | confirmed workspace package metadata and published MSRV |
| `cargo package ... --locked --allow-dirty` | PASS | packaged and verified all eight published crates |
| `cargo fmt --all --check` | PASS | formatting unchanged |
| `cargo test -p canic --test changelog_governance --locked` | PASS | changelog governance still satisfied |
| `cargo test -p canic --test workspace_manifest --locked` | PASS | publishable and package-metadata checks passed |
| `git diff --check` | PASS | no whitespace errors |

## Final Verdict

PASS.

No publish-surface violations were found. Keep watching `canic-host` and
`canic-core` for lower-level docs thinness, but their current READMEs make the
facade/operator boundaries clear enough for downstream users.
