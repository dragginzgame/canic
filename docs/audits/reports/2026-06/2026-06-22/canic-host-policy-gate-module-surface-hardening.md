# Module Surface Hardening: canic-host policy_gate

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
| `in_scope_roots` | `crates/canic-host/src/policy_gate/` |
| `excluded_roots` | CLI evidence command rendering/output, lower-level evidence envelope schema helpers, build-provenance payload construction, generated output, target artifacts |
| `generated_code_inclusion` | excluded |
| `test_surface_inclusion` | focused module tests |
| `audit_tier` | `Tier 2` |
| `patch_mode` | `implementation-requested` |

## Verdict

- Status: `PASS`.
- Risk score: `3 / 10`.
- Tier: `Tier 2`, because this module owns public V1 CI policy, project
  evidence manifest, policy gate report, and project evidence gate report
  schemas that the CLI serializes as JSON and wraps into stable evidence
  envelopes.
- Cleanup result: the test-only private build-provenance rule import was
  removed from the production facade root and moved into the test module that
  consumes it. Public V1 schema/report contracts remain unchanged.

The module is passive: it parses policy/manifest inputs, fingerprints files,
evaluates existing evidence envelopes, and returns report DTOs. It does not run
builds, deploy, query live deployment state, or mutate inputs. The residual risk
comes from schema and CI decision authority rather than runtime mutation; after
the test-import narrowing, no production facade widening remains in the module
root.

## Evidence Log

| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| MSH definition review | `sed -n '1,220p' docs/audits/modular/module-surface-hardening.md` | PASS: `MSH-2.0` rules checked for this run | terminal output |
| target inventory | `find crates/canic-host/src/policy_gate -type f -name '*.rs' | sort`; `wc -l crates/canic-host/src/policy_gate/*.rs crates/canic-host/src/policy_gate/*/*.rs` | PASS: `2217` total LOC across policy facade, evaluation, manifest gate, model, validation, and focused tests | terminal output |
| public surface inventory | `rg -n "pub\\(|pub\\(crate\\)|pub\\(super\\)|pub\\(in |pub |allow\\(dead_code\\)|expect\\(dead_code\\)|expect\\(unused_imports\\)|doc\\(hidden\\)|legacy|compat|compatibility|fallback|shim|deprecated|temporary|TODO|FIXME" crates/canic-host/src/policy_gate -g '*.rs'` | PASS: public V1 policy/report facade identified; no stale markers or lint suppressions found | terminal output |
| consumer check | `rg -n "PolicyGateReportV1|ProjectEvidenceGateReportV1|ProjectEvidenceManifestV1|CiPolicyV1|parse_ci_policy_v1|parse_project_evidence_manifest_v1|PolicyEvaluationStatusV1|PolicyFindingSeverityV1|PolicyRequirementV1|ProjectEvidenceManifestTargetV1|ProjectEvidenceManifestEntryV1" crates/canic-host crates/canic-cli crates/canic-backup -g '*.rs'` | PASS: public report/evaluation surface is live in `canic evidence gate` JSON, text rendering, envelope wrapping, and tests | terminal output |
| authority boundary scan | `rg -n "fs::|read_to_string|write|create_dir_all|file_input_fingerprint|serde_json::from|toml::from|evaluate_policy|combine_exit_classes|payload_schema|required_input|build_provenance|TODO|FIXME" crates/canic-host/src/policy_gate -g '*.rs'` | PASS: module parses/fingerprints/evaluates evidence and performs manifest file reads; no deployment mutation or live-state authority found | terminal output |
| cleanup patch | direct source inspection and diff review | PASS: moved test-only `PolicyBuildProvenanceRuleV1` import from `policy_gate::mod.rs` to `policy_gate::tests::mod.rs` | source diff |
| focused tests | `cargo test --locked -p canic-host policy_gate -- --nocapture` | PASS: 21 focused tests passed | terminal output |
| lint | `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | PASS | terminal output |

## Reachable Surface Inventory

| Item | Kind | Visibility | Consumer Evidence | Consumer Should Exist? | Authority Reason | Surface Class | Owner | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `parse_ci_policy_v1` and `CiPolicyV1` rule DTOs | policy parser/schema | `pub` | CLI evidence gate and tests parse CI policy TOML. | Yes | Defines the V1 passive policy contract over evidence envelopes, exit classes, summaries, required inputs, and build provenance. | `live-authority` | `policy_gate` | `RETAIN WITH OWNER` | Medium; schema mistakes alter CI gate decisions. |
| `parse_project_evidence_manifest_v1` and manifest DTOs | manifest parser/schema | `pub` | CLI manifest gate path and tests parse project evidence manifests. | Yes | Defines the V1 project evidence manifest contract and target matching input. | `live-authority` | `policy_gate` | `RETAIN WITH OWNER` | Medium; wrong manifest validation can skip or mis-target evidence. |
| `evaluate_policy_gate` and `PolicyGateRequest` | single-envelope evaluator | `pub` | `canic evidence gate --envelope` calls this facade. | Yes | Evaluates one existing evidence envelope against policy without mutating state. | `live-authority` | `policy_gate::evaluation` through facade | `RETAIN WITH OWNER` | Medium; CI/blocking decision authority. |
| `evaluate_project_evidence_manifest_gate` and request DTO | manifest evaluator | `pub` | `canic evidence gate --manifest` calls this facade. | Yes | Evaluates required/optional project evidence entries, payload schema, target matching, and nested policy reports. | `live-authority` | `policy_gate::manifest_gate` | `RETAIN WITH OWNER` | Medium. |
| `PolicyGateReportV1`, `ProjectEvidenceGateReportV1`, entry reports, requirements, findings, severity/status enums | serialized reports | `pub` | CLI writes JSON, renders text, and wraps reports into evidence envelopes. | Yes | Stable operator/CI output schema and envelope payload. | `live-authority` | `policy_gate::model` | `RETAIN WITH OWNER` | Medium to high for evidence compatibility. |
| `PolicyBuildProvenanceRulesV1` with private rule enum | public opaque policy sub-shape | `pub` struct with private rules | Parser/evaluator/tests use it; callers cannot construct arbitrary internal rules directly. | Yes | Keeps TOML-facing build-provenance booleans public while retaining internal rule representation. | `live-authority` | `policy_gate::model` | `RETAIN WITH OWNER` | Low; good narrowing already present. |

## Dead / Stale Surface Signals

| Candidate | File | Signal | Current Consumers | Consumer Should Exist? | Authority Reason | Surface Class | Deletion Confidence | Disposition | Risk If Removed |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Root `#[cfg(test)] use model::PolicyBuildProvenanceRuleV1` | `mod.rs` | Test-only import from private model enum. | Focused child tests. | Yes, but only in tests. | Tests inspect parsed build-provenance rule activation without widening production visibility. | `overexposed-internal` | Medium | `NARROWED` | Fixed by importing directly from `tests/mod.rs`. |
| Public model fields | `model/mod.rs` | Broad public DTO field visibility. | CLI JSON/text/envelope paths and tests. | Yes. | These are schema/report contracts and serde output surfaces. | `live-authority` | Low | `RETAIN WITH OWNER` | Narrowing fields would break CLI rendering/envelope wrapping and JSON schema expectations. |

## Removed / Narrowed / Inlined / Moved

| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |
| Test-only `PolicyBuildProvenanceRuleV1` import in `policy_gate::mod.rs` | `NARROW NOW` | The private enum is only used by parser tests. Moving the import to `tests/mod.rs` keeps test access local and removes a production-root test convenience import without changing policy parsing, evaluation, or report schemas. | `cargo test --locked -p canic-host policy_gate -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` |

## Runtime Authority Drift Check

| Area | Runtime Authority | Alternate Authority Found? | Evidence | Allowed Role? | Finding | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| CI policy schema | `policy_gate::model` owns V1 TOML and report DTOs. | No duplicate policy schema owner found. | `CiPolicyV1`, `ProjectEvidenceManifestV1`, report structs. | Yes | Schema ownership is explicit and tested with parser/schema tests. | Breaking changes affect CI/operator automation. |
| Policy evaluation | `policy_gate::evaluation` owns passive decision logic over one envelope. | CLI only orchestrates file IO/output. | `evaluate_policy_gate`, CLI `evaluate_gate_files`. | Yes | Evaluation is separated from CLI rendering and envelope wrapping. | Wrong exit-class combination can misclassify gates. |
| Manifest gate | `policy_gate::manifest_gate` owns manifest entry resolution, missing evidence handling, schema/target checks, and nested policy evaluation. | No duplicate manifest evaluator found. | `evaluate_project_evidence_manifest_gate`. | Yes | File reads are bounded to manifest evidence inputs and no mutation is performed. | Path/required semantics can affect CI pass/fail. |
| Evidence envelope schemas | `evidence_envelope` owns schema identifiers and fingerprint helpers. | No duplicate schema constants here. | `evidence_envelope_schema`, `policy_gate_report_schema`, `project_evidence_gate_report_schema`. | Yes | Policy gate correctly references envelope-owned schema helpers. | Schema mismatch would break envelope consumers. |

## Complexity And Runtime Shape

| Module | Complexity Signal | Retention Justification | Dead-Surface Link | Public/Hidden Items | Current Consumers | Shrink Action | Disposition | Expected Blast Radius | Risk |
| ---- | ---- | ---- | ---- | ----: | ---- | ---- | ---- | ---- | ---- |
| `mod.rs` | Thin facade with parser/evaluator exports. | Keeps CLI-facing policy-gate API centralized while child modules own evaluation, manifest, model, and validation. | Test-only enum import narrowed in this slice. | Public parse/evaluate functions and DTO re-exports. | CLI evidence gate and tests. | Completed for this slice. | `RETAIN WITH OWNER` | Medium. | Public schema facade. |
| `model/mod.rs` | Broad public V1 DTO set with serde contracts. | V1 policy/report/manifest shapes need stable fields for JSON, TOML, text rendering, and evidence envelope payloads. | Public field narrowing blocked by consumers. | Public DTOs; private build-provenance rule enum and helper methods. | CLI rendering/envelope wrapping/tests. | None. | `RETAIN WITH OWNER` | Broad for evidence tooling. | Schema compatibility. |
| `evaluation/mod.rs` | Moderate pure decision logic. | Owns policy requirements, findings, build-provenance policy, and exit-class combination for one envelope. | None. | Internal evaluator. | Facade and manifest gate. | None. | `RETAIN WITH OWNER` | Medium. | CI decision semantics. |
| `manifest_gate/mod.rs` | Manifest file reads plus per-entry evaluation. | Owns required/optional evidence behavior and manifest schema/target checks. | None. | Public evaluator plus private path/report helpers. | CLI manifest gate. | None. | `RETAIN WITH OWNER` | Medium. | CI decision semantics. |
| `validation/mod.rs` | Input validation helpers. | Rejects unsupported schema versions, empty allow lists, empty required inputs, duplicate normalized evidence paths, and selectorless targets. | None. | Internal validation. | Parser facade. | None. | `RETAIN WITH OWNER` | Low to medium. | Input safety. |

## Facade / Generated Boundary Review

| Surface | Boundary Type | Generated Consumer Evidence | Could Narrow? | Required Replacement | Deletion Confidence | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `pub mod policy_gate` from `canic-host` | Host facade | No generated consumer found. | Not safely in this slice; CLI evidence gate consumes the facade directly. | CLI migration to narrower parse/evaluate/report modules with JSON/envelope compatibility proof. | Low | `RETAIN WITH OWNER` | Public evidence tooling contract. |
| Report DTO public fields | Serialized JSON/envelope payload | CLI output and envelope wrapping consume fields directly. | No. | Accessor migration plus JSON snapshot/schema proof. | Low | `RETAIN WITH OWNER` | Stable report schema. |

## Feature / Diagnostics / Test Surface Review

| Surface | Feature/Cfg | Production Consumer? | Test/Diagnostics Consumer? | Visibility Could Narrow? | Action | Disposition | Risk |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| `#[cfg(test)] mod tests` and private rule test import | test only | No | Yes. | Import now lives in `tests/mod.rs`, not the production facade root. | Narrowed in this slice. | `RETAIN WITH OWNER` | Low. |
| Finding codes/messages | normal production diagnostics | Yes, CLI text/JSON/envelope summaries. | Yes. | No safe narrowing; they are operator/CI diagnostics. | None. | `RETAIN WITH OWNER` | Medium. |

## Removal Safety Plan

| Candidate | Action | Disposition | Owner Boundary | Hotness | Required Proof | Focused Validation | Wasm Raw Bytes Relevant? | Follow-Up Trigger |
| ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- |
| Public report and manifest DTO fields | Keep. | `RETAIN WITH OWNER` | `policy_gate::model` | `cold` evidence/CI path | Versioned schema migration plus CLI JSON/envelope compatibility proof. | Policy gate tests and CLI evidence tests. | No | Owner introduces V2 policy/report schema. |
| Test-only private rule import | Move into `tests/mod.rs`. | `NARROW NOW` | `policy_gate` tests | `test-only` | Focused tests and host clippy. | `cargo test --locked -p canic-host policy_gate -- --nocapture`; `cargo clippy --locked -p canic-host --all-targets -- -D warnings` | No | Complete. |
| Manifest path normalization | Keep. | `RETAIN WITH OWNER` | `policy_gate::validation` | `cold` evidence/CI path | Cross-platform duplicate-path tests and policy that absolute paths are disallowed or normalized differently. | Manifest parser tests. | No | Manifest schema V2 changes path semantics. |

## Blocked / Measure First

| Item | Reason | Required proof |
| ---- | ---- | ---- |
| Splitting or hiding report DTO fields | They are serialized output and CLI rendering/envelope inputs. | Versioned V2 schema and downstream CLI/golden-output compatibility proof. |
| Deleting default parser functions in favor of evaluator-only API | CLI and tests use parser behavior directly, and parsing owns validation. | Consumer migration plus parser/validation test replacement. |
| Reworking build-provenance rule storage | Current shape keeps the public policy sub-struct opaque while supporting TOML booleans. | Parser compatibility and policy evaluation equivalence proof. |

## Verification

- `cargo fmt --all`: PASS.
- `cargo test --locked -p canic-host policy_gate -- --nocapture`: PASS, 21 focused tests passed.
- `cargo check --locked -p canic-host`: not run; focused tests compiled `canic-host`.
- `cargo clippy --locked -p canic-host --all-targets -- -D warnings`: PASS.
- wasm/raw-size check: not applicable; host evidence-policy audit with no runtime wasm payload change.
