# Module Surface Hardening: canic-host release_set

## Preamble

| Field | Value |
| ---- | ---- |
| `method_version` | `MSH-2.0` |
| `surface_taxonomy` | `ST-1` |
| `authority_taxonomy` | `AT-1` |
| `deletion_confidence_model` | `DC-1` |
| `compatibility_policy` | `pre-1.0-hard-cut` |
| `wasm_signal_rule` | `raw-wasm-primary` |
| `hot_path_risk_model` | `HP-1` |
| `proof_policy` | `read-only-first` |
| `baseline_report` | `N/A` |
| `comparability_status` | `non-comparable`: first targeted MSH run for this module |
| `code_snapshot` | `4bcad983` |
| `in_scope_roots` | `crates/canic-host/src/release_set/` |
| `excluded_roots` | lower-level deployment-truth diff/report internals, ICP CLI adapter internals, canic-core config model internals, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused module and direct release-set-consumer tests selected by test filter |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `3 / 10`.
- Tier: `Tier 2`, because this module owns release-set config projection and
  mutation, artifact manifest emission, artifact validation, workspace/artifact
  path resolution, root release-set staging, and root bootstrap resume helpers.
- Cleanup result: test-only source-helper imports were moved out of the
  production facade root and into `tests/mod.rs`; the internal `stage` module
  was made private behind the existing public/crate re-exports; and the private
  host-clock helper no longer carries an unused `root_canister` parameter.

`release_set` is a live host authority boundary. Its config readers feed build,
install, status, list, fleets, scaffold, deployment-truth, and canister-build
flows. Its mutation helpers edit `canic.toml` and role package metadata. Its
manifest/staging helpers validate built `.wasm.gz` artifacts, emit
`root.release-set.json`, stage manifests/chunks into root, and resume root
bootstrap. The module remains high consequence at runtime, but its residual
surface-hardening risk is now bounded: the facade exposes live owner APIs, the
stage implementation module is private, and test-only helper imports no longer
inflate the production module root.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,220p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules checked for this run | terminal output |
| target inventory | `find crates/canic-host/src/release_set -type f -name '*.rs' | sort`; `wc -l crates/canic-host/src/release_set/*.rs crates/canic-host/src/release_set/*/*.rs crates/canic-host/src/release_set/*/*/*.rs` | PASS: `4186` total LOC across config, paths, manifest, stage, and tests | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/release_set -g '*.rs'` | PASS: public config/path/manifest/stage surface identified; no stale markers or lint suppressions found | terminal output |
| consumer check | `rg -n 'configured_release_roles\\(|configured_deployable_roles\\(|configured_bootstrap_roles\\(|configured_install_targets\\(|configured_fleet_name\\(|configured_controllers\\(|configured_pool_expectations\\(|configured_role_lifecycle\\(|declare_fleet_role\\(|attach_fleet_role\\(|rename_fleet_role\\(|matching_fleet_config_paths\\(|configured_role_kinds\\(|configured_role_capabilities\\(|configured_role_auto_create\\(|configured_role_topups\\(|configured_role_metrics_profiles\\(|configured_role_details\\(|emit_root_release_set_manifest|stage_root_release_set|resume_root_bootstrap|workspace_root\\(|icp_root\\(|resolve_artifact_root\\(' crates/canic-host crates/canic-cli crates/canic-backup -g '*.rs'` | PASS: public surface is live across CLI scaffold/build/fleets/status/list/deploy/install plus canister-build, install-root, deployment-truth, and ICP config flows | terminal output |
| authority boundary scan | direct inspection of `config`, `manifest`, `paths`, and `stage` modules | PASS: module reads and mutates fleet config, emits release-set manifests, validates gzip wasm artifacts, and stages release chunks through ICP calls | source inspection |
| cleanup patch | direct source inspection and diff review | PASS: moved test-only source-helper imports to `tests/mod.rs`, made `stage` private behind re-exports, and removed the unused private `root_time_secs` parameter | source diff |
| focused tests | `cargo test --locked -p canic-host release_set -- --nocapture` | PASS: 43 release-set-filtered tests passed | terminal output |
| lint | `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | PASS | terminal output |

## Reachable Surface Inventory

| Item | Kind | Visibility | Consumer Evidence | Consumer Should Exist? | Authority Reason | Surface Class | Owner | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Config projection functions | config readers | `pub` | Build, install, status, list, fleets, deployment-truth, and ICP config flows consume role/fleet/controller/pool/topology projections. | Yes | Host-owned projection from `canic.toml` into operator/build/install decisions. | `live-authority` | `release_set::config` | `RETAIN WITH OWNER` | High; wrong projection targets wrong roles or controllers. |
| Config mutation functions | config writers | `pub` | CLI scaffold/fleets declare, attach, and rename roles. | Yes | Controlled operator mutation of fleet config and role package metadata. | `live-authority` | `release_set::config::mutation` | `RETAIN WITH OWNER` | High; edits persistent config. |
| Config DTOs | result DTOs | `pub` | CLI fleet/list/status rendering and mutation output consume structured rows. | Yes | Boundary data for operator output after validated config projection/mutation. | `live-authority` | `release_set::config::model` | `RETAIN WITH OWNER` | Medium. |
| Workspace/path helpers | path API | `pub` | Build, install, list, metrics, cycles, token, snapshot, deployment catalog, and status resolve roots and manifests. | Yes | Centralizes downstream workspace/ICP/artifact path rules and environment overrides. | `live-authority` | `release_set::paths` | `RETAIN WITH OWNER` | Medium to high; wrong root affects operator targets. |
| Release-set manifest DTO and IO | serialized manifest | `pub` | Canister build, install-root plan artifacts, deployment-truth observation, and staging consume manifests. | Yes | Stable root release-set manifest contract for built artifacts. | `live-authority` | `release_set::manifest` | `RETAIN WITH OWNER` | High; artifact digest/source authority. |
| Release staging and bootstrap resume | runtime side effects | `pub` | Install-root staging and activation operations call staging/resume helpers. | Yes | Publishes approved release manifests/chunks into root and resumes bootstrap. | `live-authority` | `release_set::stage` | `RETAIN WITH OWNER` | High; root install mutation. |
| `icp_query_on_network` re-export | internal crate query helper | `pub(crate)` | Release-set readiness/install diagnostics use query-only ICP calls. | Yes | Shared host adapter for query-only `icp canister call`. | `live-diagnostics` | `release_set::stage::call` | `RETAIN WITH OWNER` | Medium. |

## Dead / Stale Surface Signals

| Candidate | File | Signal | Current Consumers | Consumer Should Exist? | Authority Reason | Surface Class | Deletion Confidence | Disposition | Risk If Removed |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Test-only source-projection imports in `release_set::mod.rs` | `mod.rs` | `#[cfg(test)] use ...` imports for child tests. | Release-set child tests. | Yes, but only in tests. | Tests assert config parsing/mutation/projection invariants without widening production visibility. | `overexposed-internal` | Medium | `NARROWED` | Fixed by importing directly from `tests/mod.rs`. |
| `root_time_secs(root_canister)` unused parameter | `mod.rs` | Parameter was ignored. | Stage flow used host clock only. | No. | The staging timestamp helper does not currently query root time. | `orphaned-helper` | High | `NARROWED` | Fixed by removing the unused private parameter. |
| Artifact-root local fallback | `paths/artifacts.rs` | Falls back from `.icp/<network>/canisters` to `.icp/local/canisters`. | Build/install artifact discovery. | Yes. | Supports local artifact layout when network-specific artifact root is absent. | `live-authority` | Low | `RETAIN WITH OWNER` | Removing it may break local build/install flows. |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| Test-only source-helper imports in `release_set::mod.rs` | `NARROW NOW` | The imports existed only for release-set child tests. Tests can import directly from `config` and `stage`, so production facade root does not need to carry them. | `cargo test --locked -p canic-host release_set -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` |
| `pub(crate) mod stage` | `NARROW NOW` | External callers already use the explicit `stage_root_release_set`, `resume_root_bootstrap`, and crate-visible `icp_query_on_network` re-exports. No direct `release_set::stage` consumer exists. | `cargo test --locked -p canic-host release_set -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` |
| `root_time_secs(root_canister)` | `NARROW NOW` | The private helper ignored the parameter and reads host time only. Removing the unused argument changes no staging behavior. | `cargo test --locked -p canic-host release_set -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` |

## Runtime Authority Drift Check

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Fleet config projection | `release_set::config::projection` reads `canic.toml` through canic-core config model and returns host projections. | Deployment-truth consumes projections but does not own parsing. | Consumer scan and config tests. | Yes | Projection owner is centralized. | Wrong role sets affect build/install/deploy checks. |
| Fleet config mutation | `release_set::config::mutation` edits role declarations, topology attachments, and package metadata. | CLI orchestrates operator commands only. | Mutation functions and tests. | Yes | Mutation path validates fleet, role names, duplicates, root exclusions, and reparses updated config. | Persistent config corruption risk. |
| Artifact manifest emission | `release_set::manifest` builds `root.release-set.json` from gzip wasm artifacts. | Install-root can emit plan-derived manifests for overrides, but normal built-artifact manifest owner remains here. | Manifest functions and install-root plan artifact consumer. | Yes | Manifest ownership is explicit. | Digest/path drift can stage wrong artifacts. |
| Release staging | `release_set::stage` stages manifests/chunks and resumes bootstrap through ICP calls. | Install-root owns orchestration and receipts. | `stage_root_release_set`, `resume_root_bootstrap`, install-root operations. | Yes | Side effects are isolated in stage child modules. | Root mutation risk. |
| Workspace/path resolution | `release_set::paths` owns root/config/artifact path conventions. | `workspace_discovery` owns lower-level discovery primitives. | Path helpers and tests. | Yes | Path rules are centralized and covered. | Wrong roots can affect many commands. |

## Complexity And Runtime Shape

| Module | Complexity Signal | Retention Justification | Dead-Surface Link | Public/Hidden Items | Current Consumers | Shrink Action | Disposition | Expected Blast Radius | Risk |
| ---- | ---- | ---- | ---- | ----: | ---- | ---- | ---- | ---- | ---- |
| `mod.rs` | Broad but cleaner facade over config, manifest, paths, and stage re-exports. | Existing callers rely on `release_set` as host operator/build/install facade. | Test-only imports and private clock parameter narrowed in this slice. | Public re-exports plus internal constants and clock helper. | Broad CLI/host consumers. | Completed for this slice. | `RETAIN WITH OWNER` | Broad. | Lower facade pressure; runtime authority remains high. |
| `config/*` | Many projection and mutation functions. | Owns host interpretation and mutation of fleet config. | None beyond test import routing. | Public readers/writers/DTOs; internal source helpers. | CLI build/fleets/list/status/scaffold and deployment-truth. | None. | `RETAIN WITH OWNER` | Broad. | Config authority. |
| `manifest.rs` | Serialized release-set manifest IO. | Root install/build need a canonical artifact manifest with hashes and chunks. | None. | Public manifest DTO/functions. | Canister build, install-root, deployment-truth. | None. | `RETAIN WITH OWNER` | Medium to broad. | Artifact authority. |
| `paths/*` | Environment-sensitive root/path helpers. | Centralizes workspace, ICP, config, artifact, and manifest paths. | Local fallback retained. | Public path helpers. | Broad CLI/host consumers. | None. | `RETAIN WITH OWNER` | Broad. | Target selection. |
| `stage/*` | ICP side effects, chunk staging, progress rendering. | Root release-set staging and bootstrap resume are explicit install authority operations. | Stage module is now private behind explicit re-exports. | Public staging functions; private implementation module and call/chunk helpers. | Install-root activation/staging. | Module visibility narrowed in this slice. | `RETAIN WITH OWNER` | Medium. | Root mutation. |

## Facade / Generated Boundary Review

| Surface | Boundary Type | Generated Consumer Evidence | Could Narrow? | Required Replacement | Deletion Confidence | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `pub mod release_set` from `canic-host` | Host facade | No generated consumer found. | Not safely in this slice; CLI and host modules consume broad facade exports directly. | Split public config/path/manifest/stage facades with broad CLI/host migration proof. | Low | `RETAIN WITH OWNER` | Public host operator/build/install contract. |
| `RootReleaseSetManifest` and `ReleaseSetEntry` public fields | Serialized manifest DTO | No generated consumer found. | No; serde manifest and deployment-truth/install consumers read fields. | Versioned manifest schema and migration proof. | Low | `RETAIN WITH OWNER` | Artifact compatibility. |

## Feature / Diagnostics / Test Surface Review

| Surface | Feature/Cfg | Production Consumer? | Test/Diagnostics Consumer? | Visibility Could Narrow? | Action | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Source-helper imports in `tests/mod.rs` | test only | No | Yes. | Already test-local after this cleanup. | Narrowed in this slice. | `RETAIN WITH OWNER` | Low. |
| Stage progress rendering | production operator output | Yes | Indirectly. | No safe narrowing found. | None. | `RETAIN WITH OWNER` | Low diagnostics risk. |
| Artifact validation | production install/build safety | Yes | Yes. | No. | None. | `RETAIN WITH OWNER` | High artifact safety. |

## Removal Safety Plan

| Candidate | Action | Disposition | Owner Boundary | Hotness | Required Proof | Focused Validation | Wasm Raw Bytes Relevant? | Follow-Up Trigger |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Test-only facade imports | Move imports to `tests/mod.rs` with direct owner-module paths. | `NARROW NOW` | `release_set` tests | `test-only` | Focused release-set tests and host clippy. | `cargo test --locked -p canic-host release_set -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | No | Complete. |
| Public facade split | Keep. | `DEFER WITH TRIGGER` | `canic-host::release_set` | `cold/warm` operator paths plus install mutation | Broad CLI/host migration proof. | CLI build/fleets/status/list/install/deploy and host release-set tests. | No | Facade churn blocks future work. |
| Root time host-clock helper | Keep with no unused canister parameter. | `NARROW NOW` | `release_set::stage` | `install-authority` | Focused release-set tests and host clippy. | `cargo test --locked -p canic-host release_set -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | No | Complete. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Removing local artifact-root fallback | Local build/install layout compatibility is active. | Proof all local artifact consumers use network-specific artifact roots only. |
| Consolidating config projection readers | They share a file-read/error-wrap pattern, but each public function names a distinct operator projection. | Refactor proof that error messages and consumer behavior remain identical; otherwise this is style-only churn. |
| Changing staging Candid string construction | The current code builds exact IDL strings for ICP CLI calls. | Integration proof against `icp canister call` and root staging endpoints. |

## Verification

- `cargo fmt --all`: PASS.
- `cargo test --locked -p canic-host release_set -- --nocapture`: PASS, 43 release-set-filtered tests passed.
- `cargo check --locked -p canic-host`: not run; focused tests compiled `canic-host`.
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`: PASS.
- `git diff --check`: PASS.
- trailing whitespace scan over touched release-set and report files: PASS.
- wasm/raw-size check: not applicable; host release-set audit with no runtime wasm payload change.
