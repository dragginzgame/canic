# Publish Surface Audit - 2026-05-11

## Report Preamble

- Scope: workspace root package policy; publishable crate manifests under
  `crates/canic`, `crates/canic-backup`, `crates/canic-cdk`,
  `crates/canic-cli`, `crates/canic-control-plane`, `crates/canic-core`,
  `crates/canic-host`, `crates/canic-macros`, `crates/canic-memory`,
  `crates/canic-testkit`, and `crates/canic-wasm-store`; package-local
  README posture; public binary/example/bench surface where present; package
  verification output from `cargo package`.
- Compared baseline report path:
  `docs/audits/reports/2026-04/2026-04-06/publish-surface.md`
- Code snapshot identifier: `bfa521d4` plus intentionally dirty 0.34 backup CLI
  worktree edits.
- Method tag/version: `publish-surface-current`
- Comparability status: partially comparable. The publish-surface checklist is
  unchanged, but the published crate set changed after the 0.33 hard cut:
  `canic-installer`/`canic-dsl-macros` are no longer present, and
  `canic-cli`, `canic-host`, `canic-backup`, and `canic-macros` are now the
  current package names/surfaces.
- Exclusions applied: unpublished crates with `publish = false`, fleet/test/
  audit/sandbox canister crates, generated package unpack directories, and
  non-package-local scripts except where package READMEs present them as part
  of the public install surface.
- Notable methodology changes vs baseline: current run includes `cargo package`
  verification for every publishable crate and treats the active 0.34 backup
  CLI edits as package-surface evidence, not a clean committed release snapshot.

## 0. Baseline Capture

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Published crates reviewed | 9 | 11 | +2 |
| Published crates with thin docs posture | 2 | 2 | 0 |
| Published crates with `readme = false` pressure | 0 | 0 | 0 |
| Publish-surface mismatches | 0 | 1 | +1 |
| Published crates with binary/example posture pressure | 1 | 1 | 0 |
| Alternate-facade ambiguity seams | 2 | 3 | +1 |
| Published crates with default-feature contract pressure | 1 | 1 | 0 |
| Publishable-but-underspecified crates | 0 | 0 | 0 |

Notes:

- The current publishable set is `11` crates: `canic`, `canic-backup`,
  `canic-cdk`, `canic-cli`, `canic-control-plane`, `canic-core`, `canic-host`,
  `canic-macros`, `canic-memory`, `canic-testkit`, and `canic-wasm-store`.
- Package verification passed for all `11` publishable crates with
  `cargo package --locked --allow-dirty`.
- The one current mismatch is docs posture, not package build failure:
  `crates/canic/Cargo.toml` defaults `metrics`, `control-plane`, `sharding`,
  and `auth-crypto`, while `crates/canic/README.md` only documents `metrics`
  under “Default surface”.

## 1. Manifest Publish Posture

| Crate | Publish Intent | Manifest Posture | README / Docs Metadata | Binary / Example Surface | Package Contract Clarity | Risk |
| --- | --- | --- | --- | --- | --- | --- |
| `canic` | main public facade | `publish = true`; default features are `metrics`, `control-plane`, `sharding`, `auth-crypto` in `crates/canic/Cargo.toml` | `readme = "README.md"`, `documentation = "https://docs.rs/canic"` | `examples/minimal_root.rs` | role is clear, but default-feature README text is incomplete | Medium |
| `canic-backup` | backup/restore domain primitives | `publish = true` in `crates/canic-backup/Cargo.toml` | README explains manifests, topology hashing, journals, restore planning, runner summaries | no public binary | clear enough for current 0.34 model/domain role | Low |
| `canic-cdk` | standalone curated IC CDK facade | `publish = true` | README states stable import surface and tells most users to prefer `canic::cdk` | no public binary | clear support role | Low |
| `canic-cli` | installed operator CLI | `publish = true`; `[[bin]] name = "canic"` | README states installed binary is the operator interface and Rust library surface is intentionally narrow | one public `canic` binary | clear binary/tool contract | Low |
| `canic-control-plane` | lower-level root/store support crate | `publish = true` | README tells most users to prefer `canic` unless working on root/bootstrap/store behavior | no public binary | clear lower-level role | Low |
| `canic-core` | lower-level runtime/support crate | `publish = true`; default features empty | README says most canister projects should depend on `canic` | one `serialize` bench | clear lower-level role | Low |
| `canic-host` | host-side build/install/fleet/release-set library | `publish = true` | README scopes direct use to CI/local automation and tells normal operators to prefer `canic` commands | no public binary | clear host-library role | Low |
| `canic-macros` | proc-macro support crate | `publish = true`; `proc-macro = true` | README says most users should access macros through `canic` | proc-macro library only | clear support role | Low |
| `canic-memory` | standalone stable-memory helpers | `publish = true` | README has standalone install and quick-start guidance | no public binary | clear standalone support role | Low |
| `canic-testkit` | standalone generic PocketIC/test infrastructure | `publish = true` | README separates public generic helpers from unpublished internal harnesses | no public binary | clear standalone test role | Low |
| `canic-wasm-store` | canonical published `wasm_store` canister crate | `publish = true`; `cdylib` + `rlib` | README explains canonical role crate and DID ownership | no public binary | clear role-specific canister crate | Low |

## 2. Findings

### Medium - `canic` README under-documents the default facade surface

Evidence:

- `crates/canic/Cargo.toml` defines
  `default = ["metrics", "control-plane", "sharding", "auth-crypto"]`.
- `crates/canic/README.md` under “Default surface” lists only:
  `metrics — exports canic_metrics in ordinary builds unless you opt out`.
- The same README lists `control-plane`, `sharding`, and `auth-crypto` under
  “Optional features”, which is now ambiguous because they are optional in the
  feature model but enabled by default in the package contract.

Impact:

- Downstream users reading the package-local README can reasonably infer that
  only metrics is enabled by default.
- This is a package-contract mismatch because the facade's default dependency
  and endpoint/capability surface is broader than the README says.

Recommended cleanup:

- Update `crates/canic/README.md` so “Default surface” lists all default
  features, and clarify that disabling default features opts out of the
  pre-1.0 standard runtime bundle.

### Low - Recurring publish-surface audit template has stale crate names

Evidence:

- `docs/audits/recurring/system/publish-surface.md` still names
  `crates/canic-installer` and `crates/canic-dsl-macros` in its canonical
  published crate map.
- The current workspace publishes `canic-cli`, `canic-host`, `canic-backup`,
  and `canic-macros` instead.

Impact:

- The stale template does not change package output, but it can skew future
  audit scope unless each rerun corrects the map manually in the report.

Recommended cleanup:

- Refresh the canonical published crate map in the recurring audit definition.

### Low - Package verification emits an expected packaged-build warning

Evidence:

- `cargo package` verification for `canic` completed successfully but printed:
  `CANIC_CONFIG_PATH not set and default config not found ... skipping config validation (likely a packaged build)`.

Impact:

- This is not a package failure and appears intentional, but it is visible to
  downstream package verification. Keep the warning wording stable and
  intentional.

Recommended cleanup:

- No immediate code change required. Revisit only if package consumers report
  the warning as confusing.

## 3. Example / Binary Surface

| Crate | Surface Item | Surface Type | What It Implies | Supported / Intended? | Risk |
| --- | --- | --- | --- | --- | --- |
| `canic` | `examples/minimal_root.rs` | example | users should start from the facade crate | yes | Low |
| `canic-cli` | `[[bin]] name = "canic"` in `crates/canic-cli/Cargo.toml` plus README install commands | installed binary | `canic-cli` owns the `canic` operator command | yes | Low |
| `canic-core` | `benches/serialize.rs` | bench | lower-level serialization performance surface exists for maintainers | yes | Low |
| `canic-macros` | proc-macro lib | macro support surface | direct use is possible but README says most users should prefer `canic` | yes | Low |

## 4. Package Verification

`cargo package -p canic -p canic-backup -p canic-cdk -p canic-cli -p canic-control-plane -p canic-core -p canic-host -p canic-macros -p canic-memory -p canic-testkit -p canic-wasm-store --locked --allow-dirty`
passed.

| Crate | `.crate` size |
| --- | ---: |
| `canic` | 34,962 bytes |
| `canic-backup` | 81,142 bytes |
| `canic-cdk` | 21,009 bytes |
| `canic-cli` | 81,034 bytes |
| `canic-control-plane` | 60,867 bytes |
| `canic-core` | 297,773 bytes |
| `canic-host` | 55,040 bytes |
| `canic-macros` | 13,607 bytes |
| `canic-memory` | 21,246 bytes |
| `canic-testkit` | 41,668 bytes |
| `canic-wasm-store` | 12,641 bytes |

## Overall Publish Surface Risk Index

**3 / 10**

Interpretation:

- Package buildability is healthy: all publishable crates package and verify.
- The current package roles are broadly clear after the 0.33/0.34 surface
  changes.
- Risk rose from `2/10` to `3/10` because the audit found facade README default
  feature drift and a stale canonical crate map in the audit template. Both
  were cleaned up in the follow-up below.

## Follow-up Cleanup

- Updated `crates/canic/README.md` so the default surface lists `metrics`,
  `control-plane`, `sharding`, and `auth-crypto`, matching
  `crates/canic/Cargo.toml`.
- Updated `docs/audits/recurring/system/publish-surface.md` so the canonical
  published crate map uses the current `canic-cli`, `canic-host`,
  `canic-backup`, and `canic-macros` package names.

## Verification Readout

| Check | Status | Notes |
| --- | --- | --- |
| `find docs/audits/reports ...` | PASS | Identified `publish-surface` and `instruction-footprint` as the oldest latest-run recurring audits, both last retained on 2026-04-06. |
| manifest inspection | PASS | Reviewed package metadata, `publish`, README/docs.rs fields, default features, binaries, examples, and benches for publishable crates. |
| package-local README inspection | PASS | Reviewed all publishable package READMEs under `crates/**/README.md`. |
| `cargo package ... --locked --allow-dirty` | PASS | Packaged and verified all 11 publishable crates from the intentionally dirty worktree. |
| `find target/package -maxdepth 1 -type f -name '*.crate'` | PASS | Captured current packaged artifact sizes. |
| `cargo metadata --no-deps --format-version 1` | PASS | Confirmed current workspace package metadata and target surfaces. |
