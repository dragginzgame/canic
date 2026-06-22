# Module Surface Hardening: canic-host subnet_registry

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
| `code_snapshot` | `5bc5a458` |
| `in_scope_roots` | `crates/canic-host/src/subnet_registry/` |
| `excluded_roots` | registry JSON parser internals, replica-query wire internals, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused host and CLI consumer tests selected by test filter |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `3 / 10`.
- Tier: `Tier 2`, because this module is a public host facade for live subnet
  registry query evidence used by installed-deployment, backup, and snapshot
  flows.
- Cleanup result: query-source provenance was narrowed to `canic-host` by
  making `SubnetRegistryQuery::source` and `SubnetRegistryQuerySource`
  crate-visible; `registry_json` remains public for CLI consumers.

`subnet_registry` is intentionally small: it selects direct local replica
queries for local/http networks, falls back to ICP CLI JSON queries for other
targets, and preserves whether the result came from local replica or ICP CLI
for same-crate installed-deployment source reporting. The residual risk is
bounded because the public cross-crate contract is now only the query function,
the public registry JSON payload, and the structured error type needed by CLI
callers.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| target inventory | `find crates/canic-host/src/subnet_registry -type f -name '*.rs'`; `wc -l crates/canic-host/src/subnet_registry/mod.rs` | PASS: single 109-line module | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/subnet_registry -g '*.rs'` | PASS: public query DTO, source enum, error enum, and query function identified; no stale markers or lint suppressions found | terminal output |
| consumer check | `rg -n "subnet_registry|SubnetRegistry|query_subnet_registry|registered_subnet|subnet registry|SubnetRegistryQueryError" crates/canic-host crates/canic-cli crates/canic-backup -g '*.rs'` | PASS: facade consumed by installed-deployment, backup create, and snapshot download flows | terminal output |
| cleanup patch | source inspection and diff review | PASS: cross-crate callers only read `registry_json`; source provenance is only consumed in `canic-host` | source diff |
| focused host tests | `cargo test --locked -p canic-host subnet_registry -- --nocapture`; `cargo test --locked -p canic-host installed_deployment -- --nocapture` | PASS: query-adjacent host filters passed | terminal output |
| CLI consumer tests | `cargo test --locked -p canic-cli backup -- --nocapture`; `cargo test --locked -p canic-cli snapshot -- --nocapture` | PASS: 64 backup-filtered tests and 9 snapshot-filtered tests passed | terminal output |
| lint | `cargo clippy --locked -p canic-host --all-targets -- -D warnings`; `cargo clippy --locked -p canic-cli --all-targets -- -D warnings` | PASS | terminal output |

## Reachable Surface Inventory

| Item | Kind | Visibility | Consumer Evidence | Consumer Should Exist? | Authority Reason | Surface Class | Owner | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `query_subnet_registry_json` | query facade | `pub` | Installed-deployment, backup create, and snapshot download call it. | Yes | Centralizes local-replica vs ICP CLI subnet-registry lookup. | `live-authority` | `subnet_registry` | `RETAIN WITH OWNER` | Medium to high; wrong lookup affects deployment membership evidence. |
| `SubnetRegistryQuery.registry_json` | query payload | `pub` | Host and CLI callers parse the returned registry JSON. | Yes | Boundary data for downstream registry parser and deployment membership checks. | `live-authority` | `subnet_registry` | `RETAIN WITH OWNER` | Medium. |
| `SubnetRegistryQuery.source` | query provenance | `pub(crate)` | Installed-deployment maps local replica vs ICP CLI source to output provenance. | Yes, but only in `canic-host`. | Preserves local-vs-CLI evidence without exposing it as cross-crate API. | `live-diagnostics` | `installed_deployment` via `subnet_registry` | `NARROWED` | Low. |
| `SubnetRegistryQuerySource` | source enum | `pub(crate)` | Same-crate installed-deployment source mapping. | Yes, but only in `canic-host`. | Typed provenance for installed-deployment source reporting. | `live-diagnostics` | `subnet_registry` | `NARROWED` | Low. |
| `SubnetRegistryQueryError` | error enum | `pub` | Backup, snapshot, and installed-deployment map replica vs ICP errors differently. | Yes | Preserves error-source distinction for operator diagnostics and local lost-root handling. | `live-diagnostics` | `subnet_registry` | `RETAIN WITH OWNER` | Medium. |

## Dead / Stale Surface Signals

| Candidate | File | Signal | Current Consumers | Consumer Should Exist? | Authority Reason | Surface Class | Deletion Confidence | Disposition | Risk If Removed |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Cross-crate `source` field visibility | `mod.rs` | `source` was public, but only same-crate installed-deployment reads it. | `installed_deployment`. | Yes, but not outside `canic-host`. | Source provenance is host reporting evidence. | `overexposed-internal` | High | `NARROWED` | None after host/CLI tests and clippy. |
| Cross-crate `SubnetRegistryQuerySource` visibility | `mod.rs` | Public enum had no cross-crate consumers. | `installed_deployment`. | Yes, but not outside `canic-host`. | Typed source provenance for installed-deployment output. | `overexposed-internal` | High | `NARROWED` | None after host/CLI tests and clippy. |
| Local-replica query path | `mod.rs` | Alternate path before ICP CLI query. | Local/http network flows. | Yes. | Avoids CLI process parsing and uses configured local replica endpoint for local targets. | `live-authority` | Low | `RETAIN WITH OWNER` | Removing it would regress local discovery behavior. |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| `SubnetRegistryQuery::source` | `NARROW NOW`: `pub` to `pub(crate)` | Cross-crate callers only consume `registry_json`; same-crate installed-deployment can still read source provenance. | `cargo test --locked -p canic-host installed_deployment -- --nocapture`; CLI backup/snapshot tests; host/CLI clippy |
| `SubnetRegistryQuerySource` | `NARROW NOW`: `pub` to `pub(crate)` | The enum is only named by same-crate installed-deployment code after the source field is narrowed. | `cargo test --locked -p canic-host installed_deployment -- --nocapture`; CLI backup/snapshot tests; host/CLI clippy |

## Runtime Authority Drift Check

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Local vs CLI registry lookup | `subnet_registry::query_subnet_registry_json` | Direct local query implementation lives in `replica_query`, but this module owns the selection. | Source inspection and consumer scan. | Yes | Selection is centralized and source provenance retained internally. | Wrong source can alter operator evidence. |
| Registry JSON parsing | `registry` module, not `subnet_registry` | This module only returns raw JSON. | Consumer scan. | Yes | Query and parse ownership remain separated. | Parser compatibility is not owned here. |
| Error-source mapping | `SubnetRegistryQueryError` | Callers map errors to command-specific diagnostics. | Backup/snapshot/installed-deployment call sites. | Yes | Public error enum remains justified. | Bad mapping can hide local lost-root detection. |

## Retained With Owner

| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |
| `query_subnet_registry_json` | `subnet_registry` | Shared query facade for installed-deployment, backup, and snapshot membership evidence. | If callers move to typed direct replica responses and no longer need raw registry JSON. |
| `SubnetRegistryQuery.registry_json` | `subnet_registry` | Cross-crate query payload consumed by CLI backup/snapshot and host installed-deployment parsing. | If query and parse ownership are merged. |
| `SubnetRegistryQueryError` | `subnet_registry` | Public callers distinguish replica errors from ICP CLI errors for operator diagnostics. | If command error mapping becomes host-owned. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Removing raw JSON return shape | Backup/snapshot/installed-deployment currently parse the shared registry JSON shape downstream. | Parser ownership migration and focused backup/snapshot/installed-deployment validation. |
| Hiding `SubnetRegistryQuery` entirely | CLI callers still need a returned object that exposes registry JSON. | New public function returning only JSON or parser migration across all callers. |

## Verification

- `cargo fmt --all`: PASS.
- `cargo test --locked -p canic-host subnet_registry -- --nocapture`: PASS, query-adjacent host filter passed.
- `cargo test --locked -p canic-host installed_deployment -- --nocapture`: PASS, 2 installed-deployment-filtered tests passed.
- `cargo test --locked -p canic-cli backup -- --nocapture`: PASS, 64 backup-filtered tests passed.
- `cargo test --locked -p canic-cli snapshot -- --nocapture`: PASS, 9 snapshot-filtered tests passed.
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`: PASS.
- `cargo clippy --locked -p canic-cli --all-targets -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; host/CLI query facade visibility cleanup with no runtime wasm payload change.
