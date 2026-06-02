# DRY Consolidation Audit - 2026-06-02

## Report Preamble

- Definition path: `docs/audits/recurring/system/dry-consolidation.md`
- Scope: maintained Canic source under `crates/**`, `canisters/**`,
  `fleets/**`, `scripts/**`, plus current compact-v1 evidence/proof context.
- Exclusions: `target/**`, `.git/**`, generated package archives, generated
  proof roots, and historical audit reports except as baselines.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-01/dry-consolidation.md`
- Code snapshot identifier: `b6111896`
- Method tag/version: `DRY Consolidation V5`
- Comparability status: `comparable`; this reruns the June 1 recurring system
  audit after the upstream-watch CI removal.
- Auditor: `codex`
- Run timestamp: `2026-06-02`
- Worktree state: `dirty before report write`; pending upstream-watch removal,
  changelog/status edits, and a pre-existing `Cargo.lock` modification were
  present and were not reverted.

## Executive Summary

Current consolidation risk is **4 / 10**, unchanged from June 1.

No blocker or high-severity DRY issue was found. The upstream-watch workflow
and its dedicated helper script have been removed, which slightly reduces
script surface. The remaining DRY pressure is still concentrated in the same
areas:

1. `canic-cli/src/deploy/mod.rs` remains the largest CLI owner and repeats
   nested command-family and report-writing patterns; cleanup follow-up has
   moved output-format parsing, passive catalog handling, passive comparison
   handling, deployment-root handling, authority dry-run handling,
   resume-report handling, passive deployment-truth field rendering, explicit
   registration handling, and current-install handling into private deploy
   submodules.
2. Evidence envelope wrapper construction is centralized at the DTO/schema
   level but remains command-specific for deployment check, fleet adoption, and
   policy gate envelopes.
3. `backup create` and `snapshot download` retain command-specific registry
   transport branches, while still using the host-owned registry parser.
4. Backup/restore fixtures remain duplicated but are contained in test code.
5. `scripts/ci/wasm-audit-report.sh` remains a large isolated shell subsystem.

## Delta Since Baseline

- Removed `.github/workflows/upstream-watch.yml`.
- Removed `scripts/ci/check-icp-network-launcher-update.sh`.
- No new live references to the removed upstream watch helper remain under
  `.github` or `scripts`.
- Script inventory dropped from 3,191 maintained lines on June 1 to 3,153
  maintained lines.
- The core DRY findings did not materially change.

## Inventory

| Area | Files / Lines | Readout |
| --- | ---: | --- |
| Full maintained source under `crates`, `canisters`, `fleets`, `scripts` | 944 files / 199,869 lines | Broad source inventory, excluding generated outputs. |
| Operator slice: `canic-cli`, `canic-host`, `canic-backup` | 108,908 Rust lines | Main consolidation pressure area. |
| Rust files >= 600 LOC across `crates` | 30 largest files sampled | Large-file pressure remains dominated by deployment truth, CLI deploy, host install, policy, and tests. |
| Scripts | 3,153 total lines | One large audit script dominates: `scripts/ci/wasm-audit-report.sh` at 1,066 lines. |

Largest current operator files above the threshold:

| Lines | File |
| ---: | --- |
| 13,427 | `crates/canic-host/src/deployment_truth/tests.rs` |
| 5,950 | `crates/canic-cli/src/deploy/mod.rs` |
| 5,279 | `crates/canic-host/src/deployment_truth/promotion.rs` |
| 4,017 | `crates/canic-host/src/deployment_truth/lifecycle.rs` |
| 3,907 | `crates/canic-host/src/install_root/tests.rs` |
| 3,009 | `crates/canic-host/src/install_root/mod.rs` |
| 2,613 | `crates/canic-host/src/deployment_truth/text.rs` |
| 2,416 | `crates/canic-host/src/deployment_truth/report.rs` |
| 2,362 | `crates/canic-cli/src/fleets/mod.rs` |
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

- `canic-host::evidence_envelope` still owns stable envelope DTOs, schema
  refs, exit-class precedence, evidence summary mapping, payload hashing, and
  file input fingerprinting.
- `canic-host::build_provenance` owns the stable
  `canic.build_provenance.v1` payload and build-provenance envelope creation.
- `canic-host::policy_gate` owns policy parsing, project evidence manifests,
  gate evaluation, and policy report DTOs.
- `canic-host::deployment_catalog` owns local-state-only catalog report
  construction and catalog warning messages.
- `canic-cli::output` owns common text and pretty-JSON output-file helpers.
- `canic-host::registry::parse_registry_entries` remains the registry parser
  owner.
- `canic-host::response_parse` remains the shared low-level parser owner for
  JSON field lookup, `response_candid`, numeric parsing, and cycle balances.
- The retired upstream watch no longer contributes a single-purpose scheduled
  workflow or helper script to the maintained automation surface.

## Findings

### Medium - `deploy` remains a large multi-family CLI owner

Evidence:

- `crates/canic-cli/src/deploy/mod.rs` was 7,562 lines before the follow-up
  cleanup and is 5,950 lines after moving output-format parsing to
  `crates/canic-cli/src/deploy/output_format.rs` and passive catalog handling
  to `crates/canic-cli/src/deploy/catalog.rs`, and passive comparison handling
  to `crates/canic-cli/src/deploy/compare.rs`, and deployment-root handling to
  `crates/canic-cli/src/deploy/root.rs`, and explicit registration handling to
  `crates/canic-cli/src/deploy/register.rs`, and current-install handling to
  `crates/canic-cli/src/deploy/install.rs`, and authority dry-run handling to
  `crates/canic-cli/src/deploy/authority.rs`, and resume-report handling to
  `crates/canic-cli/src/deploy/resume_report.rs`, and passive
  deployment-truth field rendering to `crates/canic-cli/src/deploy/truth.rs`.
- The top-level deploy dispatcher fans into catalog, root verification,
  install, register, compare, promote, authority, external lifecycle,
  deployment truth, and deployment-check paths around
  `crates/canic-cli/src/deploy/mod.rs:860`.
- Nested command dispatch repeats the local pattern of help/version handling,
  `parse_subcommand`, fallback usage printing, and direct handler matching in
  catalog, root, promote, authority, external, and other families.
- Output-format parsing is now centralized in
  `crates/canic-cli/src/deploy/output_format.rs`.

Impact:

- This is the clearest current patch-radius risk.
- It is not currently a safety bug because the local command families encode
  real domain boundaries and tests assert many command shapes.

Recommended consolidation:

- Do not add a broad generic command framework.
- If another deploy subfamily changes, split by domain first:
  `deploy::catalog`, `deploy::check`, `deploy::authority`,
  `deploy::external`, or `deploy::promote`.
- After one or two subfamilies are split, extract only behavior-neutral helpers
  for output-format parsing and nested usage fallback.

Follow-up applied:

- Added `crates/canic-cli/src/deploy/output_format.rs`.
- Added `crates/canic-cli/src/deploy/catalog.rs`.
- Added `crates/canic-cli/src/deploy/compare.rs`.
- Added `crates/canic-cli/src/deploy/authority.rs`.
- Added `crates/canic-cli/src/deploy/install.rs`.
- Added `crates/canic-cli/src/deploy/register.rs`.
- Added `crates/canic-cli/src/deploy/resume_report.rs`.
- Added `crates/canic-cli/src/deploy/root.rs`.
- Added `crates/canic-cli/src/deploy/truth.rs`.
- Moved deploy output-format enums and parsers out of
  `crates/canic-cli/src/deploy/mod.rs`.
- Moved local-state-only `deploy catalog` parsing, help, request construction,
  report rendering, and dispatch out of `crates/canic-cli/src/deploy/mod.rs`.
- Moved artifact-only `deploy compare` parsing, help, report construction,
  rendering, and dispatch out of `crates/canic-cli/src/deploy/mod.rs`.
- Moved `deploy root` inspect/verify parsing, help, report construction,
  rendering, and dispatch out of `crates/canic-cli/src/deploy/mod.rs`.
- Moved `deploy authority` check/evidence/report/receipt parsing, help, dry-run
  artifact construction, rendering, and dispatch out of
  `crates/canic-cli/src/deploy/mod.rs`.
- Moved `deploy resume-report` parsing, help, latest-receipt lookup, receipt
  decoding, resume-safety report construction, and dispatch out of
  `crates/canic-cli/src/deploy/mod.rs`.
- Moved passive `deploy plan`, `deploy inventory`, `deploy diff`, and
  `deploy report` parsing, help, deployment-truth loading, field rendering, and
  dispatch out of `crates/canic-cli/src/deploy/mod.rs`.
- Moved explicit `deploy register` parsing, help, state-registration dispatch,
  and registration option conversion out of `crates/canic-cli/src/deploy/mod.rs`.
- Moved `deploy install` parsing, help, plan decoding, current-install option
  conversion, and dispatch out of `crates/canic-cli/src/deploy/mod.rs`.
- Consolidated repeated JSON/text output-format parser bodies inside the new
  module while preserving existing defaults and error text.
- Kept command semantics unchanged; the split is private CLI glue cleanup.

### Medium-Low - Evidence envelope wrapper assembly remains repeated

Evidence:

- Deployment-check envelope construction lives in
  `crates/canic-cli/src/deploy/mod.rs:2482`.
- Fleet adoption envelope construction lives in
  `crates/canic-cli/src/fleets/mod.rs:1697`.
- Policy-gate envelope construction lives in
  `crates/canic-cli/src/evidence.rs:403`.
- All three construct `EvidenceEnvelopeV1` directly and each maps payload,
  target, input fingerprints, payload schema, summary, and exit class locally.

Impact:

- The stable DTOs and schema helpers are centralized, so the risk is contained.
- Local assembly currently preserves important domain distinctions: deployment
  checks map safety status and observed deployment identity, adoption maps
  profile/evidence inputs, and policy gate maps policy findings plus
  manifest-vs-envelope payload modes.

Recommended consolidation:

- Keep summary mapping local while the source finding types differ.
- If a fourth envelope emitter is added, extract small behavior-neutral helpers
  for optional input fingerprint insertion and text/raw JSON/envelope JSON
  output selection.
- Avoid a one-size-fits-all envelope builder until target, source-config, and
  summary rules converge.

### Low - Registry transport remains locally repeated in backup/snapshot

Evidence:

- `canic-host::installed_deployment` owns the general installed deployment
  resolver, including local-replica vs ICP CLI registry query selection at
  `crates/canic-host/src/installed_deployment.rs:190`.
- `backup create` repeats the local-replica vs ICP CLI registry branch for
  backup preflight at `crates/canic-cli/src/backup/create.rs:522`.
- `snapshot download` repeats a similar branch for membership/driver registry
  traversal at `crates/canic-cli/src/snapshot/download/mod.rs:417`.
- Both command paths still use `canic-host::registry::parse_registry_entries`
  instead of local parser logic.

Impact:

- This is lower risk than the May baseline because registry parsing has one
  host owner.
- The remaining duplication is attached to command-specific authority,
  membership, and fallback behavior.

Recommended consolidation:

- Keep local registry traversal only where a command needs a distinct fallback
  or authority/membership diagnostic.
- Consider a host helper for "query registry from this explicit root under this
  ICP root" if backup and snapshot both change again.
- Do not reintroduce command-local registry JSON parsing.

### Low - Backup/restore test fixtures remain duplicated but contained

Evidence:

- Fixture-heavy files remain large:
  `crates/canic-cli/src/backup/tests/mod.rs`,
  `crates/canic-cli/src/restore/tests/run.rs`,
  `crates/canic-backup/src/restore/tests/apply_journal.rs`, and
  `crates/canic-backup/src/plan/tests.rs`.
- Repeated helpers still build manifests, journals, fake ICP scripts, artifact
  layouts, and checksummed backup artifacts.

Impact:

- Test duplication remains a maintenance cost.
- It is still acceptable because the fixtures keep domain behavior explicit and
  close to the assertions they protect.

Recommended consolidation:

- Defer broad fixture sharing.
- Extract only durable crate-local test builders if future backup/restore work
  touches the same setup repeatedly.

### Watchpoint - `wasm-audit-report.sh` is still a large shell subsystem

Evidence:

- `scripts/ci/wasm-audit-report.sh` is 1,066 lines.
- The next largest CI proof scripts are 247 and 222 lines.

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
| Ownership boundaries | 3 / 10 | Core owners remain clear across host evidence, policy, provenance, registry, parser, and catalog mechanics. |
| Runtime code duplication | 3 / 10 | Response parsing and registry parsing remain consolidated; command-specific page parsers are acceptable. |
| CLI command duplication | 5 / 10 | `deploy` and nested command-family glue are still the largest patch-radius risks. |
| Backup/restore fixture duplication | 5 / 10 | Large tests remain, but fixtures still protect domain-specific behavior. |
| Evidence/report duplication | 4 / 10 | Stable DTOs are centralized; wrapper assembly repeats but is currently command-specific. |
| Script duplication | 3 / 10 | Upstream watch was removed; packaged proof and Wasm audit scripts remain deliberate/local. |
| Overall | 4 / 10 | Low/moderate risk; no blocking DRY defect found. |

## Recommended Order

1. If `deploy` changes again, split one domain submodule before adding another
   large block to `deploy/mod.rs`.
2. Keep evidence envelope wrapper construction local until a fourth emitter or
   a shared behavior change proves extraction is worth it.
3. Keep using host-owned registry and response parsers; avoid new command-local
   parser logic for shared ICP response shapes.
4. Consider a narrow host registry-query helper only if both backup and
   snapshot registry traversal change again.
5. Revisit backup/restore test support only after another real backup/restore
   behavior slice.
6. Split `wasm-audit-report.sh` only if the Wasm footprint audit changes again.

## Commands Run

Inventory:

```bash
git rev-parse --short HEAD
git status --short
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
cargo fmt --all
cargo test -p canic-cli deploy
```

Focused inspection:

```bash
sed -n '1,260p' docs/audits/reports/2026-06/2026-06-01/dry-consolidation.md
sed -n '900,1225p' crates/canic-cli/src/deploy/mod.rs
sed -n '2380,2728p' crates/canic-cli/src/deploy/mod.rs
sed -n '4380,4480p' crates/canic-cli/src/deploy/mod.rs
sed -n '1680,1945p' crates/canic-cli/src/fleets/mod.rs
sed -n '360,675p' crates/canic-cli/src/evidence.rs
sed -n '320,550p' crates/canic-cli/src/backup/create.rs
sed -n '245,435p' crates/canic-cli/src/snapshot/download/mod.rs
sed -n '110,225p' crates/canic-host/src/installed_deployment.rs
```

## Verification Status

- DRY scan/report completed: PASS.
- Small source cleanup applied: PASS.
- Focused deploy CLI tests: PASS, 131 passed.
- No blocker or high-severity duplication issue was found.
