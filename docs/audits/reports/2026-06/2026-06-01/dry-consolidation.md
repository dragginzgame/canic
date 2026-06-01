# DRY Consolidation Audit - 2026-06-01

## Report Preamble

- Definition path: `docs/audits/recurring/system/dry-consolidation.md`
- Scope: maintained Canic source under `crates/**`, `canisters/**`,
  `fleets/**`, `scripts/**`, plus current compact-v1 evidence/proof context.
- Exclusions: `target/**`, `.git/**`, generated package archives, generated
  proof roots, and historical audit reports except as baselines.
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-14/dry-consolidation.md`
- Code snapshot identifier: `5f525efc`
- Method tag/version: `DRY Consolidation V4`
- Comparability status: `partially comparable`; the core source and script
  scan is comparable with May 14, while V4 expands the method to include
  compact-v1 evidence envelopes, build provenance, policy gates, deployment
  catalog output, and packaged/installed release proofs.
- Auditor: `codex`
- Run timestamp: `2026-06-01`
- Worktree state: `dirty before report write`; pending `0.57.4`
  auth-abstraction audit files were already present and were not reverted.

## Executive Summary

Current consolidation risk is **4 / 10**, unchanged from May 14.

The major positive change since the baseline is that the compact-v1 evidence
work did not scatter the stable DTOs. `EvidenceEnvelopeV1`,
`ExitClassV1`, `InputFingerprintV1`, `EvidenceSummaryV1`, and related schema
helpers live in `canic-host::evidence_envelope`; build provenance lives in
`canic-host::build_provenance`; policy gates and manifests live in
`canic-host::policy_gate`; the passive deployment catalog lives in
`canic-host::deployment_catalog`.

No blocker or high-severity DRY issue was found.

The remaining pressure is practical, not architectural:

1. `canic-cli/src/deploy/mod.rs` is now the largest CLI owner and repeats many
   command-family, format, and report-writing patterns.
2. Evidence-envelope wrapper construction remains command-specific, but the
   duplicated path argument normalization/redaction helper was small enough to
   consolidate safely.
3. Packaged proof scripts repeat package-staging and temporary-root mechanics,
   but the 0.56 retained-probe inventory makes that intentional because each
   script answers a distinct release question.
4. `scripts/ci/wasm-audit-report.sh` remains a large isolated shell subsystem.

## Audit Definition Update

Before running the audit, the recurring definition was refreshed for the
post-0.56 state:

- added scans for `EvidenceEnvelopeV1`, `BuildProvenanceV1`,
  `PolicyGateReportV1`, `ProjectEvidenceManifestV1`, and deployment catalog
  DTO ownership;
- added scans for raw JSON / envelope JSON / `--output` handling;
- added scans for packaged/installed release proof script isolation;
- added explicit guardrails against consolidating separate proof scripts or
  command-specific envelope paths too early.

This is a method improvement only. It does not change product behavior.

## Inventory

| Area | Files / Lines | Readout |
| --- | ---: | --- |
| Full maintained source under `crates`, `canisters`, `fleets`, `scripts` | 944 files / 199,906 lines | Broad source inventory, excluding generated outputs. |
| Operator slice: `canic-cli`, `canic-host`, `canic-backup` | 108,923 Rust lines | Main consolidation pressure area. |
| Rust files >= 600 LOC across `crates` | 30 largest files sampled | Large-file pressure is now dominated by deployment truth, CLI deploy, host install, policy, and tests. |
| Scripts | 3,191 total lines | One large audit script dominates: `scripts/ci/wasm-audit-report.sh` at 1,066 lines. |

Largest current operator files above the threshold:

| Lines | File |
| ---: | --- |
| 13,427 | `crates/canic-host/src/deployment_truth/tests.rs` |
| 7,580 | `crates/canic-cli/src/deploy/mod.rs` |
| 5,279 | `crates/canic-host/src/deployment_truth/promotion.rs` |
| 4,017 | `crates/canic-host/src/deployment_truth/lifecycle.rs` |
| 3,907 | `crates/canic-host/src/install_root/tests.rs` |
| 3,009 | `crates/canic-host/src/install_root/mod.rs` |
| 2,613 | `crates/canic-host/src/deployment_truth/text.rs` |
| 2,416 | `crates/canic-host/src/deployment_truth/report.rs` |
| 2,379 | `crates/canic-cli/src/fleets/mod.rs` |
| 2,307 | `crates/canic-host/src/deployment_truth/model.rs` |
| 2,267 | `crates/canic-host/src/adoption.rs` |
| 2,157 | `crates/canic-host/src/policy_gate.rs` |
| 1,843 | `crates/canic-host/src/release_set/mod.rs` |
| 1,365 | `crates/canic-cli/src/backup/tests/mod.rs` |
| 1,311 | `crates/canic-cli/src/fleets/tests.rs` |
| 1,311 | `crates/canic-cli/src/evidence.rs` |
| 1,306 | `crates/canic-host/src/release_set/config.rs` |
| 1,249 | `crates/canic-backup/src/restore/tests/apply_journal.rs` |
| 1,094 | `crates/canic-host/src/icp.rs` |
| 957 | `crates/canic-host/src/deployment_truth/observe.rs` |
| 956 | `crates/canic-host/src/deployment_truth/root.rs` |
| 927 | `crates/canic-host/src/deployment_truth/receipt.rs` |
| 899 | `crates/canic-cli/src/restore/tests/run.rs` |
| 877 | `crates/canic-host/src/deployment_truth/authority.rs` |
| 870 | `crates/canic-cli/src/scaffold/mod.rs` |
| 856 | `crates/canic-backup/src/plan/tests.rs` |
| 820 | `crates/canic-cli/src/cycles/wallet.rs` |
| 788 | `crates/canic-host/src/build_provenance.rs` |
| 775 | `crates/canic-host/src/deployment_truth/multi.rs` |
| 762 | `crates/canic-cli/src/tests.rs` |

Largest current scripts:

| Lines | File |
| ---: | --- |
| 1,066 | `scripts/ci/wasm-audit-report.sh` |
| 247 | `scripts/ci/verify-packaged-downstream-cli.sh` |
| 222 | `scripts/ci/verify-packaged-downstream-wasm-store.sh` |
| 208 | `scripts/dev/install_dev.sh` |
| 182 | `scripts/app/README.md` |
| 132 | `scripts/ci/v1-readiness-smoke.sh` |
| 129 | `scripts/ci/publish-workspace.sh` |
| 108 | `scripts/ci/run-workspace-tests.sh` |
| 103 | `scripts/dev/cloc.sh` |
| 88 | `scripts/ci/v1-operator-proof.sh` |

## Positive Consolidation Readout

- `canic-host::evidence_envelope` owns the stable envelope DTOs, schema refs,
  exit-class precedence, evidence summary mapping, payload hashes, and file
  input fingerprinting.
- `canic-host::build_provenance` owns the stable
  `canic.build_provenance.v1` payload, source provenance, Cargo provenance,
  artifact provenance, and build-provenance envelope creation.
- `canic-host::policy_gate` owns policy parsing, project evidence manifests,
  gate evaluation, and policy report DTOs.
- `canic-host::deployment_catalog` owns local-state-only catalog report
  construction and catalog warning messages.
- `canic-cli::output` owns the common text/pretty-JSON output-file helpers,
  including parent-directory creation.
- `canic-host::registry::parse_registry_entries` remains the registry parser
  owner, reused by CLI tests and command paths.
- `canic-host::response_parse` remains the shared low-level parser owner for
  JSON field lookup, `response_candid`, numeric parsing, and cycle balances.
- `docs/operations/0.56-v1-release-probes.md` gives retained installed and
  packaged proof scripts distinct release questions, which makes their
  remaining local setup duplication intentional rather than accidental drift.

## Findings

### Medium - `deploy` remains a large multi-family CLI owner

Evidence:

- `crates/canic-cli/src/deploy/mod.rs` is 7,580 lines.
- The module owns deployment catalog, root verification, install, register,
  compare, promote, authority, external lifecycle, deployment truth, and
  deployment-check envelope handling.
- It defines multiple local output-format enums and parser/render paths:
  `CheckOutputFormat`, `CatalogOutputFormat`, `ExternalOutputFormat`,
  `PromotionOutputFormat`, `CompareOutputFormat`, `RootOutputFormat`, and
  `AuthorityOutputFormat`.

Impact:

- This is the clearest current patch-radius risk.
- It is not currently a safety bug because the local command families encode
  real domain boundaries and tests assert many command shapes.

Recommended consolidation:

- Do not add a generic command framework now.
- If another deploy subfamily changes, split by domain first:
  `deploy::catalog`, `deploy::check`, `deploy::authority`,
  `deploy::external`, or `deploy::promote`.
- Keep parsing and safety validation near each command family until a stable
  helper can be extracted without weakening command-specific rules.

### Medium-Low - Evidence envelope wrappers repeat small path and summary glue

Evidence:

- Adoption report envelope construction lives in
  `crates/canic-cli/src/fleets/mod.rs`.
- Deployment-check envelope construction lives in
  `crates/canic-cli/src/deploy/mod.rs`.
- Policy-gate envelope construction lives in `crates/canic-cli/src/evidence.rs`.
- Before the follow-up cleanup, `push_optional_path_arg` existed in both
  `fleets` and `deploy`.
- Command-specific summary mapping functions exist for adoption,
  deployment-check, and policy-gate output.

Impact:

- The stable DTOs are centralized, so the risk is limited to wrapper assembly.
- The local duplication currently preserves important domain distinctions:
  adoption has profile/evidence inputs, deployment check has safety-status
  mapping and build-provenance input handling, and policy gate has policy
  findings and manifest-vs-envelope payload modes.

Recommended consolidation:

- Consolidate behavior-neutral path argument normalization/redaction.
- Leave current wrapper construction local.
- If a fourth envelope emitter is added, consider extracting only tiny,
  behavior-neutral helpers:
  - optional input fingerprint insertion;
  - output selection for text/raw JSON/envelope JSON.
- Do not centralize summary mapping unless the source finding types converge.

Follow-up applied:

- Added `crates/canic-cli/src/evidence_support.rs` as the shared private owner
  for optional path argument normalization/redaction.
- Removed the duplicate `push_optional_path_arg` implementations from
  `crates/canic-cli/src/fleets/mod.rs` and
  `crates/canic-cli/src/deploy/mod.rs`.

### Medium-Low - Packaged proof scripts repeat package staging but remain deliberately separate

Evidence:

- `scripts/ci/verify-packaged-downstream-cli.sh` and
  `scripts/ci/verify-packaged-downstream-wasm-store.sh` both package Canic
  sibling crates, unpack package roots, reject repository crate paths, reject
  `target/debug/canic`, and isolate proof execution paths.
- `docs/operations/0.56-v1-release-probes.md` records separate release
  questions for installed CLI, packaged CLI, and packaged `wasm_store` proofs.

Impact:

- Package-staging duplication would be annoying if a third packaged proof were
  added.
- Today the separation is useful because the CLI proof and `wasm_store` proof
  intentionally validate different package graphs and different downstream
  expectations.

Recommended consolidation:

- Do not merge the scripts.
- If another packaged proof appears, extract shared staging helpers under
  `scripts/ci/package-proof/` while preserving separate top-level scripts and
  separate release-question docs.

### Low - Registry and installed-state paths are mostly shared, with command-specific exceptions

Evidence:

- `canic-host::installed_deployment` uses
  `registry::parse_registry_entries` and `replica_query`.
- `snapshot download` and `backup create` still call
  `parse_registry_entries` locally after command-specific registry queries or
  membership checks.

Impact:

- This is lower risk than the May baseline because registry parsing itself has
  one host owner.
- The remaining local usage appears tied to command-specific fallback and
  validation behavior.

Recommended consolidation:

- Keep local registry traversal only where a command needs a distinct fallback
  or authority/membership diagnostic.
- Avoid reintroducing local registry JSON parsing logic; keep using
  `canic-host::registry::parse_registry_entries`.

### Low - Backup/restore test fixtures remain duplicated but contained

Evidence:

- Large fixture-heavy files remain in:
  - `crates/canic-backup/src/restore/tests/apply_journal.rs`
  - `crates/canic-backup/src/plan/tests.rs`
  - `crates/canic-cli/src/backup/tests/mod.rs`
  - `crates/canic-cli/src/restore/tests/run.rs`
- Local fixture helpers build manifests, journals, fake ICP scripts, and
  artifact layouts.

Impact:

- Test duplication remains a maintenance cost.
- It is still acceptable because backup and restore behavior is explicit and
  fixtures are close to the assertions they protect.

Recommended consolidation:

- Defer broad fixture sharing.
- Extract only durable crate-local test builders if future backup/restore
  changes touch the same fixture setup repeatedly.

### Watchpoint - `wasm-audit-report.sh` is still a large shell subsystem

Evidence:

- `scripts/ci/wasm-audit-report.sh` is 1,066 lines.
- The next largest maintained proof scripts are much smaller:
  247 and 222 lines.

Impact:

- The script is isolated, but its size means future changes are more likely to
  create shell-level duplication or hidden state coupling.

Recommended consolidation:

- Leave the entrypoint stable.
- If the Wasm footprint audit changes again, split helper fragments under
  `scripts/ci/wasm-audit/` or move report assembly into a small Rust helper.

## Risk Matrix

| Category | Risk | Notes |
| --- | ---: | --- |
| Ownership boundaries | 3 / 10 | Core owners are clear: host owns stable evidence, policy, provenance, registry, parser, and catalog mechanics. |
| Runtime code duplication | 3 / 10 | Response parsing and registry parsing remain consolidated; command-specific page parsers are acceptable. |
| CLI command duplication | 5 / 10 | `deploy` and nested command-family glue are the largest patch-radius risks. |
| Backup/restore fixture duplication | 5 / 10 | Large tests remain, but fixtures still protect domain-specific behavior. |
| Evidence/report duplication | 4 / 10 | Stable DTOs are centralized; wrapper assembly repeats but is currently command-specific. |
| Script duplication | 4 / 10 | Packaged proofs repeat staging mechanics intentionally; one large Wasm audit script remains. |
| Overall | 4 / 10 | Low/moderate risk; no blocking DRY defect found. |

## Recommended Order

1. Keep evidence envelope wrapper construction local until the next emitter or
   envelope behavior change proves more shared helper code is worth it.
2. If `deploy` changes again, split one domain submodule before adding another
   large block to `deploy/mod.rs`.
3. Keep retained packaged proof scripts separate, but extract package-staging
   helpers if a third packaged proof appears.
4. Keep using host-owned registry/response parsers; avoid new command-local
   parser logic for shared ICP response shapes.
5. Revisit backup/restore test support only after another real backup/restore
   behavior slice.
6. Split `wasm-audit-report.sh` only if the Wasm footprint audit changes again.

## Commands Run

Inventory:

```bash
find crates canisters fleets scripts -type f \( -name '*.rs' -o -name '*.sh' -o -name '*.toml' -o -name '*.md' \) -not -path '*/target/*' | wc -l
find crates canisters fleets scripts -type f \( -name '*.rs' -o -name '*.sh' -o -name '*.toml' -o -name '*.md' \) -not -path '*/target/*' -print0 | xargs -0 wc -l | tail -1
find crates/canic-cli crates/canic-host crates/canic-backup -type f -name '*.rs' -print0 | xargs -0 wc -l | tail -1
find crates -type f -name '*.rs' -print0 | xargs -0 wc -l | awk '$1 >= 600 { print }' | sort -nr | head -30
find crates/canic-cli crates/canic-host crates/canic-backup -type f -name '*.rs' -print0 | xargs -0 wc -l | awk '$1 >= 500 { print }' | sort -nr
find scripts -type f -print0 | xargs -0 wc -l | sort -nr | head -20
```

Required scans:

```bash
rg -n "read_named_fleet_install_state|parse_registry_entries|query_subnet_registry_json|InstalledFleetResolution|installed_fleet" crates/canic-cli crates/canic-host crates/canic-backup -g '*.rs'
rg -n "parse_json|parse_.*candid|find_field|response_candid|canister_call_output|response_parse" crates/canic-cli crates/canic-host crates/canic-backup -g '*.rs'
rg -n "print_help_or_version|parse_subcommand|disable_help_flag|render_help|CommandSpec|command_catalog" crates/canic-cli/src -g '*.rs'
rg -n "TempDir|write_artifact|journal_with_checksum|backup-plan|backup-execution-journal|fixture|fake_" crates/canic-cli crates/canic-host crates/canic-backup -g '*.rs'
rg -n "render_table|ColumnAlign|write_text|write_pretty_json|println!" crates/canic-cli crates/canic-host crates/canic-backup -g '*.rs'
rg -n "EvidenceEnvelopeV1|ExitClassV1|InputFingerprintV1|PayloadSchemaRefV1|CommandProvenanceV1|EvidenceSummaryV1|EvidenceMessageV1" crates/canic-cli crates/canic-host crates/canic-core -g '*.rs'
rg -n "BuildProvenanceV1|ProjectEvidenceManifestV1|PolicyGateReportV1|DeploymentCatalogReportV1|DeploymentCatalogEntryV1" crates/canic-cli crates/canic-host crates/canic-core -g '*.rs'
rg -n "write_output|--output|OutputFormat|format json|envelope-json|write_pretty_json|write_text|serde_json::to_string_pretty" crates/canic-cli crates/canic-host -g '*.rs'
rg -n "file_input_fingerprint|InputFingerprintV1|payload_schema|build_provenance_schema|deployment_check_schema|policy_gate|evidence gate|manifest" crates/canic-cli crates/canic-host -g '*.rs'
rg -n "target/debug/canic|CARGO_HOME|CARGO_TARGET_DIR|TMPDIR|mktemp|cargo package|path dependency|patch.crates-io|package root" scripts/ci docs/operations -g '*.sh' -g '*.md'
```

Focused inspection:

```bash
sed -n '1,360p' crates/canic-host/src/evidence_envelope.rs
sed -n '1,300p' crates/canic-host/src/build_provenance.rs
sed -n '1,360p' crates/canic-host/src/deployment_catalog.rs
sed -n '1,260p' crates/canic-cli/src/output/mod.rs
sed -n '2460,2748p' crates/canic-cli/src/deploy/mod.rs
sed -n '1680,1968p' crates/canic-cli/src/fleets/mod.rs
sed -n '360,690p' crates/canic-cli/src/evidence.rs
sed -n '1,230p' scripts/ci/verify-packaged-downstream-cli.sh
sed -n '1,220p' docs/operations/0.56-v1-release-probes.md
```

## Verification Status

- Audit definition refreshed: PASS.
- DRY scan/report completed: PASS.
- Small source cleanup applied: PASS.
- No blocker or high-severity duplication issue was found.
