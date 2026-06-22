# Module Surface Hardening: canic-host icp_config

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
| `in_scope_roots` | `crates/canic-host/src/icp_config/` |
| `excluded_roots` | install/deploy mutation, backup/recovery workflows, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused module tests |
| `audit_tier` | `Tier 1` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `2 / 10`.
- Tier: `Tier 1`, because this module exposes public host config-inspection
  surface used by CLI status/replica/fleet and query-transport code, but it is
  read-only over `icp.yaml`, fleet configs, environment variables, and
  workspace discovery.
- Cleanup result: no high-confidence delete, narrow, inline, or move action was
  found. The retained surface has current CLI/operator ownership and focused
  tests.

The module remains the host-owned ICP project configuration inspection
boundary. It resolves the current ICP project root, reads local gateway port
configuration, and reports whether `icp.yaml` contains the canister,
environment, and local-network entries implied by Canic fleet configs. It does
not mutate `icp.yaml`, apply deployment truth, or own install/backup/recovery
behavior.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| cleanup runner fallback review | `sed -n '1,280p' docs/audits/modular/module-cleanup-runner.md` | PASS: cleanup runner checked as the implementation fallback; no patch was needed | prior terminal output |
| MSH definition review | `sed -n '1,320p' docs/audits/modular/module-surface-hardening.md`; `sed -n '321,620p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules checked for this run | prior terminal output |
| target inventory | `find crates/canic-host/src/icp_config -maxdepth 2 -type f | sort`; `wc -l crates/canic-host/src/icp_config/mod.rs crates/canic-host/src/icp_config/tests.rs` | PASS: module totals `627` LOC across implementation and focused tests | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/icp_config -g '*.rs'` | PASS: public surface is config error/report types, default port, root resolution, gateway-port read, and config inspection; no stale or dead-code markers found | terminal output |
| consumer check | `rg -n "IcpConfigError|IcpProjectConfigReport|DEFAULT_LOCAL_GATEWAY_PORT|configured_local_gateway_port|configured_local_gateway_port_from_root|inspect_canic_icp_yaml|inspect_canic_icp_yaml_from_root|resolve_current_canic_icp_root|local_gateway_port_from_yaml|top_level_named_items|local_network_block|discover_project_spec" crates/canic-host crates/canic-cli crates/canic-backup -g '*.rs'` | PASS: public surface is consumed by CLI status/install/replica/fleets/metrics/cycles/snapshot/deploy/token/backup flows and host replica-query transport | terminal output |
| authority boundary scan | `rg -n "fs::|read_to_string|write|create_dir|remove_dir|canonicalize|current_dir|env::var|CANIC_ICP_ROOT|icp.yaml|fleets|canic.toml|configured_" crates/canic-host/src/icp_config -g '*.rs'` | PASS with expected read/discovery signals; production module uses `read_to_string`, `canonicalize`, current-dir/env lookup, and fleet config projection, with writes only in tests | terminal output |
| focused tests | `cargo test --locked -p canic-host icp_config:: -- --nocapture` | PASS: 10 focused tests passed | terminal output |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| None | `REJECT CLEANUP` | No stale compatibility, orphaned helper, overexposed external surface, or one-caller abstraction without an invariant was found. | Focused `icp_config::` tests passed. |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `IcpConfigError` | `canic-host::icp_config` | Error boundary for ICP root discovery, config projection, and filesystem reads. | Revisit if host config errors move into a shared operator diagnostics type. |
| `IcpProjectConfigReport` | `canic-host::icp_config` | Passive report consumed by CLI replica/fleet/status output to explain missing `icp.yaml` canisters, environments, and local-network entries. | Revisit if `icp.yaml` readiness becomes a structured deployment-truth report. |
| `DEFAULT_LOCAL_GATEWAY_PORT` | `canic-host::icp_config` | Shared fallback for local replica URL construction when `icp.yaml` does not provide a local gateway port. | Revisit only if ICP's default local gateway changes. |
| `configured_local_gateway_port_from_root` and crate-local `configured_local_gateway_port` | `canic-host::icp_config` | Read configured local replica port from the selected ICP project root while preserving fallback behavior in callers. | Revisit if gateway endpoint resolution moves wholly into `replica_query`. |
| `inspect_canic_icp_yaml` and `inspect_canic_icp_yaml_from_root` | `canic-host::icp_config` | Read-only inspection of `icp.yaml` against current Canic fleet configs; tests prove the source file is not mutated. | Revisit if `icp.yaml` generation/repair becomes a separate owner. |
| `resolve_current_canic_icp_root` | `canic-host::icp_config` | Canonical root-selection order for `CANIC_ICP_ROOT`, current Canic project roots, and fallback ICP workspace discovery. | Revisit with any project-root discovery redesign. |
| Private YAML scanners and fleet-spec discovery helpers | `canic-host::icp_config` | Local implementation detail for extracting top-level canister/environment/local-network readiness without taking mutation authority. | Revisit only if a structured ICP config parser becomes the project standard. |

## Runtime Authority Drift Check

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| ICP project root resolution | `CANIC_ICP_ROOT`, current Canic project root, then ICP root discovery/release-set fallback. | No stronger owner found in inspected scope. | `resolve_current_canic_icp_root`, `current_project_search_root`, nested-root tests. | Yes | Root resolution is centralized and tested. | Wrong root selection can query or render status for the wrong workspace. |
| `icp.yaml` readiness | Fleet config projection owns expected roles/environments; `icp_config` only reports missing YAML entries. | Higher-level CLI owns display/action. | `inspect_canic_icp_yaml_from_root`, read-only test. | Yes | No mutation authority drift found. | Treating report as repair authority would be a future Tier 2 change. |
| Local gateway port | `icp.yaml` local network block if present, else default port. | `replica_query` consumes the result but does not parse YAML directly. | gateway-port tests. | Yes | Parser is narrow and bounded to top-level `networks:`. | Incorrect parser could point local replica queries at wrong port. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Structured YAML parser replacement | The current scanner is narrow and covered for the supported readiness fields. Replacing it would widen dependency/parse behavior without a cleanup win. | Owner decision plus status/replica/fleet config tests proving identical readiness and port behavior. |

## Verification

- `cargo fmt --all -- --check`: not run; no source edits.
- `cargo test --locked -p canic-host icp_config:: -- --nocapture`: PASS, 10 focused tests passed.
- `cargo check --locked -p canic-host`: not run; focused tests compiled `canic-host`.
- `cargo clippy --locked -p canic-host --all-targets --all-features -- -D warnings`: not run; no source edits.
- wasm/raw-size check: not applicable; host config-inspection audit with no runtime wasm payload change.
