# MSH Compact Audit: canic-cli endpoints

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
| `code_snapshot` | `ef49146a` |
| `in_scope_roots` | `crates/canic-cli/src/endpoints/`, `crates/canic-cli/src/lib.rs` |
| `excluded_roots` | deploy modules currently under separate edit, historical audit reports, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused endpoints tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `read-only` |

## Verdict

- Status: `PASS`.
- Risk score: `4 / 10`.
- Tier: `Tier 2` because the module resolves installed deployment truth, reads
  live canister metadata, falls back to local Candid artifacts, and parses
  structured Candid types.
- Patch mode: read-only.
- Cleanup result: no safe `DELETE NOW`, `NARROW NOW`, or `INLINE NOW` action
  was identified in this run.

The module is current and read-only. It parses `canic endpoints`, resolves a
fleet-scoped target by principal or role, reads live `candid:service` metadata
when available, falls back to local role `.did` artifacts, parses the service
methods into structured endpoint data, and renders text or JSON. The audited
path does not install code, update canisters, mutate deployment state, register
deployments, or call DFX/ICP mutation commands. The score reflects live
metadata lookup plus Candid parser complexity, not mutation authority.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,260p' docs/audits/modular/module-surface-hardening.md` | PASS: confirmed read-only-first mode and Tier 2 escalation for deployment-truth, query/encode, and runtime-evidence surfaces | terminal output |
| target inventory | `wc -l crates/canic-cli/src/endpoints/*.rs` | endpoints module totals `1068` LOC across model/parse/render/transport/tests | terminal output |
| command inspection | `sed -n '1,220p' crates/canic-cli/src/endpoints/mod.rs` | PASS: command parses fleet, canister-or-role, network, ICP path, and output mode before delegating to transport/render | terminal output |
| model inspection | `sed -n '1,220p' crates/canic-cli/src/endpoints/model.rs` | PASS: model is passive CLI output data plus target resolution data; no validation or mutation authority found | terminal output |
| parser inspection | `sed -n '1,260p' crates/canic-cli/src/endpoints/parse.rs` | PASS: parser uses `candid_parser` and maps service method types into structured endpoint data with named-type recursion guarding | terminal output |
| render inspection | `sed -n '1,220p' crates/canic-cli/src/endpoints/render.rs` | PASS: renderer maps structured endpoint entries to stable text output and Candid-safe method names | terminal output |
| transport inspection | `sed -n '1,220p' crates/canic-cli/src/endpoints/transport.rs` | PASS: transport resolves installed deployment registry, reads live `candid:service` metadata, or reads local role `.did` artifacts; no write/update primitive found | terminal output |
| focused test inspection | `sed -n '1,320p' crates/canic-cli/src/endpoints/tests.rs` | PASS: tests cover Candid parsing, multiline and multiple arguments, service-named fields, table rendering, structured JSON, option parsing, and removed `--did`/`--role` overrides | terminal output |
| surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-cli/src/endpoints` | PASS: no dead-code allowances or stale compatibility markers; exported items are private-module or crate CLI entry surfaces | terminal output |
| authority boundary scan | `rg -n "update_settings|install_code|create_canister|delete_canister|stop_canister|uninstall_code|provisional_create_canister|canister_metadata|canister_query|canister_update|query|update|icp|dfx|fs::|read_|write_|metadata|resolve_current_canic_icp_root|resolve_installed_deployment|include_str!|source_between" crates/canic-cli/src/endpoints` | PASS with expected read signals: `canister_metadata_output`, `resolve_installed_deployment_from_root`, `resolve_current_canic_icp_root`, and `fs::read_to_string`; no mutation primitive found | terminal output |
| consumer check | `rg -n "endpoints::run|mod endpoints|EndpointsOptions|endpoint_report|parse_candid_service_endpoints|render_plain_endpoints|CANDID_SERVICE_METADATA" crates/canic-cli/src -g '*.rs'` | PASS: production entry is `endpoints::run`; helper surfaces are module-internal and covered by focused tests | terminal output |
| focused tests | `cargo test -p canic-cli endpoints -- --nocapture` | PASS: 9 endpoints tests passed | terminal output |
| owning package check | `cargo check -p canic-cli` | PASS | terminal output |

## Findings

| Item | Class | Confidence | Disposition | Authority / Reason |
| ---- | ---- | ---- | ---- | ---- |
| `endpoints::run` CLI entry | `live-authority` | high | `RETAIN WITH OWNER` | Owner: `canic-cli::endpoints`; this is the top-level CLI entry for fleet-scoped endpoint inspection. |
| `EndpointsOptions` parsing | `live-authority` | high | `RETAIN WITH OWNER` | Owner: CLI boundary. It keeps target, network, ICP path, and output mode explicit while rejecting direct `--did` and `--role` override lanes. |
| `endpoint_report` transport orchestration | `live-authority` | high | `RETAIN WITH OWNER` | Owner: endpoints transport. It resolves target identity, attempts live metadata, and falls back to role artifacts without mutating state. |
| `read_live_candid` metadata lookup | `live-authority` | high | `RETAIN WITH OWNER` | Owner: endpoints transport. It reads `candid:service` metadata through `IcpCli::canister_metadata_output`, not update/install paths. |
| Local role `.did` fallback | `live-authority` | medium | `RETAIN WITH OWNER` | Owner: endpoints transport. It supports local/offline inspection from fleet-scoped role artifacts while keeping direct file override outside the public command. |
| Candid endpoint parser | `live-authority` | high | `RETAIN WITH OWNER` | Owner: endpoints parser. It turns current generated or live Candid service files into structured endpoint JSON/text data. |
| Text/JSON rendering | `live-authority` | medium | `RETAIN WITH OWNER` | Owner: CLI output boundary. Rendering maps structured endpoint data to stable operator output and Candid-safe method names. |

## Runtime Authority Drift

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Canister interface discovery | target canister owns live `candid:service` metadata; CLI reads and renders it | no | `read_live_candid` calls `canister_metadata_output` with `CANDID_SERVICE_METADATA` | yes | no interface mutation authority found | Low |
| Deployment registry selection | installed deployment resolution owns registry lookup | no | `resolve_installed_deployment_from_root` provides registry entries for role/principal mapping | yes | no parallel deployment truth mutation path found | Low |
| Local artifact fallback | fleet build artifacts own local role `.did` files | no | fallback searches `.icp/<network>/canisters/<role>/<role>.did` after target resolution | yes | direct arbitrary file selection remains removed from command options | Low |
| Canister/deployment mutation | install/deploy/register workflows own mutation | no | authority scan found no update/install/create/delete/register primitive | yes | no mutation authority drift found | Low |

## Hot / Wasm Risk

| Code Unit | Hotness | Proposed Cleanup | Optimization Risk | Required Proof | Disposition |
| ---- | ---- | ---- | ---- | ---- | ---- |
| Endpoints command/options/render | `cold` | none | CLI-only parsing/rendering; no canister runtime or wasm-sensitive path | focused endpoints tests and `cargo check -p canic-cli` | `RETAIN WITH OWNER` |
| Endpoint transport | `warm` | none | live metadata and local artifact fallback are operator-facing lookup paths; narrowing could break fleet-scoped endpoint inspection | focused option/transport-adjacent tests plus manual/live proof for behavior changes | `RETAIN WITH OWNER` |
| Candid parser | `warm` | none | parser simplification could lose structured nested type output or recursion safety | focused parser tests for Candid service shape coverage | `RETAIN WITH OWNER` |

## Disposition Ledger

| Disposition | Count |
| ---- | ----: |
| DELETE NOW | 0 |
| NARROW NOW | 0 |
| INLINE NOW | 0 |
| MOVE OWNER | 0 |
| MOVE TO TEST | 0 |
| RETAIN WITH OWNER | 7 |
| DEFER WITH TRIGGER | 0 |
| MEASURE FIRST | 0 |
| BLOCKED | 0 |

## Follow-up

No cleanup follow-up was found.

## Verification Readout

| Command | Status | Notes |
| ---- | ---- | ---- |
| `cargo test -p canic-cli endpoints -- --nocapture` | PASS | 9 endpoints tests passed |
| `cargo check -p canic-cli` | PASS | owning package compiles |
