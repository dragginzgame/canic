# Module Surface Hardening: canic-host installed_deployment

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
| `in_scope_roots` | `crates/canic-host/src/installed_deployment/` |
| `excluded_roots` | lower-level install-state persistence internals, registry parser internals, ICP CLI adapter internals, CLI rendering code, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused module tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `3 / 10`.
- Tier: `Tier 2`, because this module is a public host facade over persisted
  install state plus live root registry observation, and it is consumed by CLI
  list/status/info-env/metrics/cycles/token/backup/snapshot flows and
  deployment-truth observation.
- Cleanup result: no safe delete, narrow, inline, or move candidate was found
  in this read-only pass.

The module is small and intentionally adapter-shaped. It does not own install
state persistence or registry parsing, but it does own the current
installed-deployment resolution contract: read the named install state, query
the installed root registry through the configured ICP boundary, normalize
source/error shape, and provide topology projections for command code.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,220p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules checked for this run | terminal output |
| target inventory | `wc -l crates/canic-host/src/installed_deployment/mod.rs crates/canic-host/src/installed_deployment/tests.rs` | PASS: `346` total LOC across module and focused tests | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/installed_deployment -g '*.rs'` | PASS: public request/response/error/topology facade identified; no stale markers or suppressions found | terminal output |
| consumer check | `rg -n "resolve_installed_deployment\\(|resolve_installed_deployment_from_root\\(|read_installed_deployment_state\\(|read_installed_deployment_state_from_root\\(|InstalledDeploymentResolution|ResolvedDeploymentTopology|InstalledDeploymentRegistry|InstalledDeploymentSource|detect_lost_local_root|LostLocalDeployment" crates/canic-host crates/canic-cli crates/canic-backup -g '*.rs'` | PASS: public surface is live across host deployment-truth observation plus CLI list/status/info-env/metrics/cycles/token/backup/snapshot/endpoints/medic flows | terminal output |
| authority boundary scan | `rg -n "fs::|std::fs|read_to_string|write|create_dir_all|Command::new|IcpCli|query_subnet_registry_json|parse_registry_entries|read_named_deployment_install_state|existing_local_canister_candid_path|canister.*not found|not found|fallback|legacy|compat|TODO|FIXME" crates/canic-host/src/installed_deployment -g '*.rs'` | PASS: module reads install state through `install_root`, queries registry through `subnet_registry`, parses through `registry`, and performs no direct state writes | terminal output |
| focused tests | `cargo test --locked -p canic-host installed_deployment -- --nocapture` | PASS: 2 focused tests passed | terminal output |
| lint | `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | PASS | terminal output |

## Reachable Surface Inventory

| Item | Kind | Visibility | Consumer Evidence | Consumer Should Exist? | Authority Reason | Surface Class | Owner | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `InstalledDeploymentRequest` | request DTO | `pub` | CLI commands construct it before resolving installed deployments. | Yes | Carries deployment/network/ICP selection and lost-local-root detection policy at the host facade boundary. | `live-authority` | `installed_deployment` | `RETAIN WITH OWNER` | Medium; wrong inputs target wrong deployment. |
| `InstalledDeploymentResolution` | response DTO | `pub` | CLI commands and host deployment-truth observation consume state, registry, source, and topology. | Yes | Bundles persisted state with observed root registry data for downstream operator commands. | `live-authority` | `installed_deployment` | `RETAIN WITH OWNER` | Medium; downstream commands select canisters from it. |
| `InstalledDeploymentSource` | observation enum | `pub` | Used by resolution and callers that distinguish local replica vs ICP CLI source. | Yes | Preserves provenance of registry observation. | `live-diagnostics` | `installed_deployment` | `RETAIN WITH OWNER` | Low to medium. |
| `InstalledDeploymentRegistry` | registry projection | `pub` | Resolution and topology construction expose parsed registry entries. | Yes | Host-owned projection over root registry entries, not the parser owner. | `live-authority` | `installed_deployment` | `RETAIN WITH OWNER` | Medium. |
| `ResolvedDeploymentTopology` | topology projection | `pub` | CLI list/info-env/cycles/metrics/token/backup flows consume canister parent/role lookups. | Yes | Avoids each command reparsing registry rows and keeps ordering deterministic. | `live-authority` | `installed_deployment` | `RETAIN WITH OWNER` | Medium; can affect backup/cycles targets. |
| `InstalledDeploymentError` | facade error | `pub` | CLI commands map variants into command-specific errors. | Yes | Keeps state, replica, ICP, lost-root, registry, and IO failures explicit. | `live-diagnostics` | `installed_deployment` | `RETAIN WITH OWNER` | Medium. |
| `resolve_installed_deployment` | default-root resolver | `pub` | Available through host facade; from-root variant is the dominant CLI path. | Yes | Provides current-root resolution when caller does not already have an ICP project root. | `live-authority` | `installed_deployment` | `RETAIN WITH OWNER` | Medium. |
| `resolve_installed_deployment_from_root` | explicit-root resolver | `pub` | Main CLI and host deployment-truth observation path. | Yes | Resolves persisted deployment state plus live root registry under explicit ICP root/candid context. | `live-authority` | `installed_deployment` | `RETAIN WITH OWNER` | Medium to high for backup/snapshot/cycles targeting. |
| `read_installed_deployment_state` and `read_installed_deployment_state_from_root` | state reader facade | `pub` | CLI status/list/medic use read-only state checks. | Yes | Converts `install_root` optional state reads into installed-deployment error shape. | `live-authority` | `installed_deployment` wrapping `install_root::state` | `RETAIN WITH OWNER` | Medium; persisted state read boundary. |

## Dead / Stale Surface Signals

| Candidate | File | Signal | Current Consumers | Consumer Should Exist? | Authority Reason | Surface Class | Deletion Confidence | Disposition | Risk If Removed |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `resolve_installed_deployment` default-root variant | `mod.rs` | Less commonly consumed than `_from_root` in current direct search. | Public host facade; no direct CLI hit in this scan. | Yes, unless the host facade policy removes default-root helpers consistently. | Mirrors the installed-state reader pair and supports callers that rely on current-root discovery. | `live-authority` | Low | `RETAIN WITH OWNER` | Removing it is a public facade break without enough consumer proof. |
| `detect_lost_local_root` and `LostLocalDeployment` | `mod.rs` | Diagnostic branch around local missing-root text. | Status/list/cycles/token/backup callers opt in; tests cover detection. | Yes | Provides operator-specific recovery guidance when local state points at a root missing from the local replica. | `live-diagnostics` | Low | `RETAIN WITH OWNER` | Removing it would collapse a useful recovery diagnostic into a generic replica query error. |

## Removed / Narrowed / Inlined / Moved

No changes were made in this read-only run.

## Runtime Authority Drift Check

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Persisted install state | `install_root::state` owns schema/path/read behavior; this module adapts it to installed-deployment errors. | No duplicate writer found. | `read_named_deployment_install_state*` calls only. | Yes | Read-only wrapper is correctly not schema authority. | Wrong error mapping can hide missing state. |
| Root registry observation | `subnet_registry` owns local/ICP query behavior; `registry` owns parse shape. | No duplicate parser found. | `query_subnet_registry_json`, `parse_registry_entries`. | Yes | Module composes observation and topology projection without mutating registry. | Wrong source/error projection can mislead operator flows. |
| Topology projection | `installed_deployment` owns command-facing parent/role lookup shape. | No duplicate projection owner found in scope. | `ResolvedDeploymentTopology::from_registry` and tests. | Yes | Deterministic sorted child lists are local and covered. | Target selection errors affect list/cycles/backup. |
| Lost local root detection | `installed_deployment` owns installed-state-vs-local-replica diagnostic. | No alternate owner found. | `detect_lost_local_root`, `LostLocalDeployment`, focused test. | Yes | Diagnostic is intentionally opt-in per caller. | Text matching is inherently brittle but bounded to diagnostics. |

## Complexity And Runtime Shape

| Module | Complexity Signal | Retention Justification | Dead-Surface Link | Public/Hidden Items | Current Consumers | Shrink Action | Disposition | Expected Blast Radius | Risk |
| ---- | ---- | ---- | ---- | ----: | ---- | ---- | ---- | ---- | ---- |
| `mod.rs` | Small facade with persisted-state read, live registry query, error conversion, and topology projection. | Public host commands need one installed-deployment resolution contract instead of each command combining state and registry separately. | Default-root variant is retained as public facade. | Public request/response/error/read/resolve functions; private adapter helpers. | CLI list/status/info-env/metrics/cycles/token/backup/snapshot/endpoints/medic and host deployment-truth observation. | None. | `RETAIN WITH OWNER` | Medium to broad across operator commands. | Medium. |
| `tests.rs` | Two focused invariant tests. | Covers deterministic topology ordering and lost-local-root text recognition. | None. | Test-only. | Host unit tests. | None. | `RETAIN WITH OWNER` | Low. | Low. |

## Facade / Generated Boundary Review

| Surface | Boundary Type | Generated Consumer Evidence | Could Narrow? | Required Replacement | Deletion Confidence | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `pub mod installed_deployment` from `canic-host` | Host facade | No generated consumer found. | Not safely in this slice; CLI command modules and host deployment-truth observation consume it directly. | Dedicated narrower state-only, registry-only, and topology APIs with CLI migration proof. | Low | `RETAIN WITH OWNER` | Public host operator contract. |
| `ResolvedDeploymentTopology` public fields | Host facade DTO | No generated consumer found. | Not safely; consumers read parent/role maps directly. | Accessor migration or private projection helper in every consumer. | Low | `RETAIN WITH OWNER` | Command target selection. |

## Feature / Diagnostics / Test Surface Review

| Surface | Feature/Cfg | Production Consumer? | Test/Diagnostics Consumer? | Visibility Could Narrow? | Action | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `#[cfg(test)] mod tests` | test only | No | Yes. | Already test-only. | None. | `RETAIN WITH OWNER` | Low. |
| Local missing-canister text detection | normal production diagnostics | Yes, for opt-in lost local root guidance. | Yes. | No safe narrowing found. | None. | `RETAIN WITH OWNER` | Medium diagnostics risk. |

## Removal Safety Plan

| Candidate | Action | Disposition | Owner Boundary | Hotness | Required Proof | Focused Validation | Wasm Raw Bytes Relevant? | Follow-Up Trigger |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Default-root resolver/read helpers | Keep. | `RETAIN WITH OWNER` | `installed_deployment` host facade | `cold` operator command path | Public facade policy deciding default-root helpers are obsolete plus consumer migration proof. | Host and CLI command tests. | No | Maintainer chooses explicit-root-only API. |
| Lost-local-root diagnostic branch | Keep. | `RETAIN WITH OWNER` | `installed_deployment` diagnostics | `cold` operator failure path | Better structured local replica error code from lower query layer. | Focused installed-deployment tests and CLI lost-root tests. | No | `subnet_registry` exposes structured canister-not-found classification. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Public facade splitting | The module is already small, and splitting state, registry observation, and topology projection would touch many CLI consumers without deleting authority. | CLI migration plan plus command test coverage proving identical user-facing errors and target selection. |
| Replacing missing-canister text detection | Current local replica error source arrives as text at this boundary. | Structured lower-layer error classification from the replica/query adapter. |

## Verification

- `cargo fmt --all`: not run; no code edits were made for this module.
- `cargo test --locked -p canic-host installed_deployment -- --nocapture`: PASS, 2 focused tests passed.
- `cargo check --locked -p canic-host`: not run; focused tests compiled `canic-host`.
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; host read-only/operator facade audit with no runtime wasm payload change.
