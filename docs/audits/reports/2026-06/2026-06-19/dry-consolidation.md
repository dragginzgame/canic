# DRY Consolidation Audit - 2026-06-19

## Report Preamble

- Definition path: `docs/audits/recurring/system/dry-consolidation.md`
- Scope: maintained Canic source under `crates/**`, `canisters/**`,
  `fleets/**`, `scripts/**`, plus current root proof provisioning and
  delegated-auth lifecycle ownership.
- Exclusions: `target/**`, `.git/**`, generated package archives, generated
  proof roots, and historical audit reports except as baselines.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-02/dry-consolidation.md`
- Code snapshot identifier: `ef55e53c`
- Method tag/version: `DRY Consolidation V6 / root-proof provisioning split`
- Comparability status: `partially-comparable`; the core CLI, host, backup,
  evidence, and release-script scans remain comparable, while this run adds an
  explicit root proof provisioning ownership scan and observes the post-split
  deploy module layout.
- Auditor: `codex`
- Run timestamp: `2026-06-19`
- Worktree state: `dirty before report write`; existing CLI, auth delegation,
  audit-definition, and audit-report edits were present and were not reverted.

## Audit Definition Maintenance

The audit definition remains worth keeping as a recurring system audit. It
catches repeated ownership decisions that module-surface-hardening audits do
not see, especially across `canic-cli`, `canic-host`, `canic-backup`, evidence
reports, release-proof scripts, and root proof provisioning.

This run refreshed the definition before execution so current root proof
provisioning prepare/get/install, active proof status, pending proof metadata,
install outcome, and verifier configuration ownership are scanned explicitly.

No source cleanup was applied from this audit. The scan found watchpoints, but
not a behavior-bearing duplicate owner that should be centralized in this pass.

## Executive Summary

Current consolidation risk is **3 / 10**, down from **4 / 10** on June 2.

No High or Medium DRY issue was found. The previous `deploy/mod.rs` pressure
has materially improved: the old multi-thousand-line deploy owner is now a
small facade over focused private submodules, and deploy output-format parsing
has a local owner. Evidence envelope assembly, CLI command-family glue,
backup/snapshot registry traversal, test fixture setup, and release-proof
scripts remain watchpoints, but they have clear owners or domain-specific
reasons to remain local.

Root proof provisioning is a positive result in this audit. The current code
has distinct owners for endpoint guards, workflow broadcast, ops metadata/proof
operations, stable records, DTO boundary shapes, replay policy, and verifier
configuration. No duplicate root-proof lifecycle owner was found.

## Delta Since Baseline

- `crates/canic-cli/src/deploy/mod.rs` is now a focused facade rather than the
  dominant CLI file.
- Deploy subfamilies are split into focused modules including catalog, compare,
  authority, install, register, resume report, root, external, promote, and
  deployment-check support.
- Root proof provisioning has been added to the DRY audit scan set.
- Evidence gate envelope construction has moved into
  `crates/canic-cli/src/evidence/gate/envelope.rs`, reducing dispatcher
  pressure while keeping command-specific evidence semantics local.
- Maintained script surface has grown since June 2, mostly through local dev,
  packaged-proof, app, and inventory-gate helpers; the retained scripts answer
  distinct release or developer questions.

## Inventory

| Area | Files / Lines | Readout |
| --- | ---: | --- |
| Full maintained source under `crates`, `canisters`, `fleets`, `scripts` | 1,561 files / 228,377 lines | Broad source inventory, excluding generated outputs. |
| Operator slice: `canic-cli`, `canic-host`, `canic-backup` | 117,860 Rust lines | Main cross-crate consolidation pressure area. |
| Rust files >= 600 LOC across `crates` | 30 largest files sampled | Large-file pressure is dominated by tests, CLI surfaces, control-plane storage, and runtime/auth hotspots. |
| Scripts | 4,747 total lines | One large audit script still dominates; local dev and package-verification scripts are the next largest retained helpers. |

Largest current operator files above the threshold:

| Lines | File |
| ---: | --- |
| 1,278 | `crates/canic-cli/src/fleets/tests.rs` |
| 1,249 | `crates/canic-backup/src/restore/tests/apply_journal.rs` |
| 899 | `crates/canic-cli/src/restore/tests/run.rs` |
| 878 | `crates/canic-cli/src/cycles/wallet.rs` |
| 862 | `crates/canic-cli/src/fleets/adoption_report.rs` |
| 850 | `crates/canic-cli/src/scaffold/mod.rs` |
| 819 | `crates/canic-cli/src/tests.rs` |
| 640 | `crates/canic-cli/src/build.rs` |
| 626 | `crates/canic-host/src/deployment_truth/tests/lifecycle/verification/mod.rs` |
| 614 | `crates/canic-backup/src/plan/types.rs` |
| 613 | `crates/canic-backup/src/restore/runner/types.rs` |
| 610 | `crates/canic-cli/src/replica/mod.rs` |

Largest current scripts:

| Lines | File |
| ---: | --- |
| 1,066 | `scripts/ci/wasm-audit-report.sh` |
| 330 | `scripts/dev/install_dev.sh` |
| 247 | `scripts/ci/verify-packaged-downstream-cli.sh` |
| 222 | `scripts/ci/verify-packaged-downstream-wasm-store.sh` |
| 183 | `scripts/app/README.md` |
| 179 | `scripts/ci/instruction-audit-report.sh` |
| 176 | `scripts/ci/check-runner-disk-space.sh` |
| 175 | `scripts/dev/gh-ci.sh` |
| 160 | `scripts/ci/check-blob-storage-cashier-inventory-gate.sh` |
| 159 | `scripts/ci/check-blob-storage-inventory-gate.sh` |

Current deploy production module sizes:

| Lines | File |
| ---: | --- |
| 486 | `crates/canic-cli/src/deploy/promote/command.rs` |
| 475 | `crates/canic-cli/src/deploy/external/command.rs` |
| 473 | `crates/canic-cli/src/deploy/check.rs` |
| 329 | `crates/canic-cli/src/deploy/authority.rs` |
| 322 | `crates/canic-cli/src/deploy/external/mod.rs` |
| 311 | `crates/canic-cli/src/deploy/promote/mod.rs` |
| 292 | `crates/canic-cli/src/deploy/catalog.rs` |
| 285 | `crates/canic-cli/src/deploy/root.rs` |
| 185 | `crates/canic-cli/src/deploy/mod.rs` |
| 65 | `crates/canic-cli/src/deploy/output_format.rs` |

## Positive Consolidation Readout

- `canic-host::evidence_envelope` owns stable envelope DTOs, schema refs,
  exit-class precedence, evidence summary mapping, payload hashing, and file
  input fingerprinting.
- `canic-host::build_provenance` owns the stable
  `canic.build_provenance.v1` payload and build-provenance envelope creation.
- `canic-host::policy_gate` owns policy parsing, project evidence manifests,
  gate evaluation, and policy report DTOs.
- `canic-host::deployment_catalog` owns local-state-only catalog report
  construction and catalog warning messages.
- `canic-host::registry::parse_registry_entries` remains the registry parser
  owner.
- `canic-host::response_parse` remains the shared low-level parser owner for
  JSON field lookup, `response_candid`, numeric parsing, and cycle balances.
- `canic-cli::output` owns common text and pretty-JSON output-file helpers.
- `canic-cli::deploy::output_format` owns deploy-local JSON/text output-format
  parsing.
- Root proof provisioning has distinct owners for API guards, workflow
  broadcast, ops metadata/proof operations, storage records, replay policy, and
  DTO boundary shapes.

## Findings

### Resolved - `deploy/mod.rs` is no longer the dominant DRY hotspot

Evidence:

- The June 2 report identified `crates/canic-cli/src/deploy/mod.rs` as the
  clearest patch-radius risk at 5,950 lines after partial cleanup.
- The current file is 185 lines and primarily declares submodules, shared
  deploy options, and small local helpers.
- Current production deploy submodules are all under 500 lines in the sampled
  inventory. The largest are `promote/command.rs`, `external/command.rs`, and
  `check.rs`.
- Deploy-local output-format parsing now lives in
  `crates/canic-cli/src/deploy/output_format.rs`.

Impact:

- The deploy command family still has many behaviors, but ownership is now
  visible by subdomain instead of hidden in one file.
- The remaining repeated command-family glue is easier to inspect and does not
  currently justify a broad generic command framework.

Recommended consolidation:

- Keep future deploy cleanup domain-first. Split or tighten a deploy submodule
  only when that specific command family changes.
- Do not create a generic nested-command framework unless two or more command
  families converge on the same behavior-bearing parsing or fallback rule.

### Watchpoint - CLI command-family glue remains intentionally local

Evidence:

- Command families still call shared helpers such as `print_help_or_version`,
  `parse_subcommand`, `disable_help_flag(true)`, and command catalog helpers.
- The repeated pattern appears across deploy, evidence, backup, restore,
  cycles, metrics, fleet, and top-level CLI command surfaces.
- Shared parsing/help helpers already exist under `crates/canic-cli/src/cli`.

Impact:

- This is a patch-radius risk if many command families change together.
- It is not a current correctness issue because local command parsing encodes
  domain-specific options, authority checks, and safety text.

Recommended consolidation:

- Leave the glue local for now.
- Extract only behavior-neutral helpers when a repeated fallback, output, or
  help rule changes in multiple command families in the same patch.

### Watchpoint - Evidence envelope assembly remains command-specific

Evidence:

- Stable envelope DTOs and hashing helpers are host-owned in
  `canic-host::evidence_envelope`.
- Deployment check, fleet adoption, and evidence gate each assemble their own
  command-specific `EvidenceEnvelopeV1`.
- Evidence gate envelope construction is now isolated under
  `crates/canic-cli/src/evidence/gate/envelope.rs`.

Impact:

- The shared schema and DTO rules are centralized, so drift risk is contained.
- Local assembly preserves distinct target, payload, source-config,
  fingerprint, and summary rules.

Recommended consolidation:

- Keep assembly local while the source finding types and target semantics
  differ.
- Revisit if a fourth envelope emitter is added or if two emitters gain the
  same optional-input or output-mode behavior.

### Low - Registry traversal duplication remains command-specific

Evidence:

- `canic-host::registry::parse_registry_entries` owns registry parsing.
- `canic-host::subnet_registry::query_subnet_registry_json` owns ICP registry
  query transport.
- Backup preflight and snapshot download still perform command-specific
  traversal and fallback decisions around those shared helpers.

Impact:

- Parser drift risk is low because the JSON parser and query helper have one
  host owner.
- Remaining duplication is tied to command-specific membership, diagnostic,
  and fallback behavior.

Recommended consolidation:

- Keep command-specific traversal local until backup and snapshot need the
  same new behavior.
- Do not reintroduce command-local registry JSON parsing.

### Low - Test fixture duplication is contained

Evidence:

- Backup CLI fixtures are now split under
  `crates/canic-cli/src/backup/tests/fixtures`.
- Restore CLI, backup persistence, backup apply-journal, and host deployment
  truth tests still retain local fixtures.
- The largest operator files remain mostly tests or command-specific operator
  modules.

Impact:

- The duplication increases test maintenance cost, but does not create runtime
  ownership drift.
- Keeping fixtures close to tests preserves scenario readability.

Recommended consolidation:

- Consolidate only fixture constructors that become shared by multiple test
  modules and encode the same persisted schema.
- Avoid moving one-off scenario setup into broad test helper crates.

### Watchpoint - Release and development scripts remain separate by purpose

Evidence:

- `scripts/ci/wasm-audit-report.sh` remains the largest script at 1,066 lines.
- Packaged downstream CLI and wasm-store verification scripts are separate.
- `scripts/dev/install_dev.sh` has grown since the June 2 report.
- Blob-storage inventory gates remain separate script checks.

Impact:

- Script duplication can become hard to audit if shared package-root,
  temporary-directory, or installed-binary isolation rules drift.
- The retained scripts currently answer distinct release, proof, inventory, or
  developer-local questions.

Recommended consolidation:

- Keep scripts separate while their release questions differ.
- If package-root or installed-binary isolation logic changes again, consider a
  small sourced shell helper rather than a mega-script.

## Root Proof Provisioning Ownership Check

The root proof provisioning scan found no duplicate lifecycle owner.

| Concern | Current owner |
| --- | --- |
| Endpoint macros and protocol constants | `crates/canic/src/macros/endpoints`, `crates/canic-core/src/protocol.rs` |
| API guard surface | `crates/canic-core/src/api/auth/mod.rs` |
| Domain policy | `crates/canic-core/src/domain/policy/auth/root_provisioning.rs` |
| DTO boundary shapes | `crates/canic-core/src/dto/auth.rs` |
| Workflow broadcast/install orchestration | `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs` |
| Ops metadata/proof helpers | `crates/canic-core/src/ops/auth/delegation/{active,batch,pending,root_issuer_policy}.rs` |
| Stable record mapping | `crates/canic-core/src/ops/storage/auth/mapper.rs`, `crates/canic-core/src/storage/stable/auth` |
| Replay policy coverage | `crates/canic-core/src/ops/replay/policy.rs`, `crates/canic-core/src/replay_policy` |

The split matches the 0.68 MVP direction: root prepares and certifies proof
material, direct root query retrieval is the proof assembly path, install
orchestration is workflow-owned, and signer-local active proof state is
separate from normal delegated-token prepare/get behavior.

## Verification Readout

Required scans were run for:

- installed-fleet and registry ownership
- response parsing ownership
- command-family glue
- test fixture duplication
- output conventions
- evidence envelope and stable report ownership
- evidence input and fingerprint ownership
- release proof script shape
- root proof provisioning and delegated-auth lifecycle ownership
- deploy module sizing

The scan set found no High or Medium duplicate owner. The main output is the
watchpoint list above.

Focused validation also passed:

- `cargo fmt --all -- --check`
- `cargo test --locked -p canic-cli deploy -- --nocapture`
- `cargo test --locked -p canic-cli evidence -- --nocapture`
- `cargo test --locked -p canic-core ops::auth::delegation --lib -- --nocapture`
- `git diff --check`

## Risk Matrix

| Category | Risk | Notes |
| --- | ---: | --- |
| Ownership boundaries | 3 / 10 | Shared parser, registry, evidence DTO, deployment catalog, and root-proof owners are clear. |
| Runtime code duplication | 3 / 10 | Root proof provisioning is split by layer; no duplicate runtime proof lifecycle owner found. |
| CLI command duplication | 4 / 10 | Command-family glue is repeated but domain-local; deploy pressure is much lower after the split. |
| Backup/restore fixture duplication | 4 / 10 | Fixture setup remains sizable but test-contained and mostly scenario-specific. |
| Evidence/report duplication | 4 / 10 | Envelope assembly remains command-specific while stable DTO/schema behavior is centralized. |
| Script duplication | 4 / 10 | Large scripts remain, but retained scripts still answer distinct proof, inventory, or developer questions. |
| Overall | 3 / 10 | Low residual DRY risk; no source cleanup target justified in this pass. |

## Risk Score

Risk Score: **3 / 10**

This is low risk. The score moved down because the previous deploy hotspot now
has clear local owners and shared deploy output-format parsing, while root
proof provisioning has a visible layer split instead of duplicate lifecycle
logic.

## Follow-Up

- Re-run this audit after the next broad CLI/deploy/evidence change.
- Revisit evidence envelope helper extraction only if another emitter appears
  or two existing emitters converge on the same output/fingerprint behavior.
- Watch `scripts/dev/install_dev.sh` and packaged-proof scripts for shared
  package-root or installed-binary isolation drift.
- Keep root proof provisioning ownership explicit as 0.68 MVP feedback lands.
