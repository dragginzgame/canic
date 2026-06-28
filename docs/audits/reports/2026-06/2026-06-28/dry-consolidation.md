# DRY Consolidation Audit - 2026-06-28

## Report Preamble

- Definition path: `docs/audits/recurring/system/dry-consolidation.md`
- Scope: maintained Canic source under `crates/**`, `canisters/**`,
  `fleets/**`, and `scripts/**`, with focused scans across CLI/host/backup
  ownership, evidence reports, release-proof scripts, root proof provisioning,
  delegated-auth lifecycle ownership, and the current root-renewal and
  blob-storage splits.
- Exclusions: `target/**`, `.git/**`, generated package archives, generated
  proof roots, historical audit reports except as baselines, and source
  cleanup outside the audit report itself.
- Compared baseline report path:
  `docs/audits/reports/2026-06/2026-06-19/dry-consolidation.md`
- Code snapshot identifier: `b140a86c` with dirty worktree.
- Method tag/version:
  `DRY Consolidation V6 / root-renewal and blob-storage split refresh`
- Comparability status: `comparable`. The report section contract and scan
  set match the June 19 run; current notes are path-adjusted for the
  root-renewal directory-module split and the blob-storage API child-module
  split.
- Auditor: `codex`
- Run timestamp: `2026-06-28T14:34:18Z`
- Worktree state: `dirty before report write`; existing changelog,
  root-renewal, blob-storage, and audit-report edits were present and were not
  reverted.

Verification status: **PASS**.

No High or Medium DRY issue was found. No source cleanup was applied from this
audit. Current consolidation risk remains **3 / 10**, unchanged from June 19.

## Audit Definition Maintenance

The audit definition remains worth keeping as a recurring system audit. It
continues to catch repeated ownership decisions across CLI command families,
host parsing/report ownership, backup/restore fixtures, evidence envelopes,
release-proof scripts, and delegated-auth lifecycle paths.

No definition change was required for this run.

## Executive Summary

The current codebase shows low DRY consolidation risk. Registry parsing,
response parsing, evidence envelope/schema ownership, output-file helpers, and
root proof provisioning each still have a clear owner. The recent
root-renewal split and blob-storage API split are positive consolidation
results: parent modules now delegate to focused children without widening
public reachability.

Residual pressure is concentrated in operator code rather than duplicate
runtime ownership. `crates/canic-cli/src/auth/mod.rs` is now the largest
operator production file at 1,406 LOC, and `crates/canic-cli/src/blob_storage`
has a 710 LOC command module plus 918 LOC of tests. The script surface also
grew from 4,747 to 5,623 lines, mostly through audit/proof helpers. These are
watchpoints, not current violations, because the repeated pieces remain
domain-local and shared parsing/fingerprint/schema behavior is centralized.

## Delta Since Baseline

- Full maintained source grew from 1,561 files / 228,377 lines to 1,615 files
  / 249,108 lines.
- Operator Rust lines grew from 117,860 to 124,795.
- Script lines grew from 4,747 to 5,623. The largest script remains
  `scripts/ci/wasm-audit-report.sh`, now at 1,246 lines.
- `ops/auth/delegation/root_issuer_renewal` is now split into private
  `identity`, `install`, `retrieval`, `schedule`, `view`, and `tests`
  modules. Production children are below the large-file threshold.
- `api/blob_storage` is now split into private `hash`, `lifecycle`,
  `gateway`, `billing`, and `tests` modules. The parent is 41 lines.
- `deploy/mod.rs` remains small at 185 lines, and deploy production modules
  remain under 500 lines in the sampled inventory.
- CLI auth and blob-storage command modules are the main current operator
  growth watchpoints.

## Inventory

| Area | Previous | Current | Delta | Readout |
| --- | ---: | ---: | ---: | --- |
| Full maintained source under `crates`, `canisters`, `fleets`, `scripts` | 1,561 files / 228,377 lines | 1,615 files / 249,108 lines | +54 files / +20,731 lines | Broad source inventory, excluding generated outputs. |
| Operator slice: `canic-cli`, `canic-host`, `canic-backup` | 117,860 Rust lines | 124,795 Rust lines | +6,935 | Main cross-crate consolidation pressure area. |
| Scripts | 4,747 lines | 5,623 lines | +876 | Release/audit/proof helper surface grew. |
| Deploy production parent | 185 lines | 185 lines | 0 | Deploy remains split and no longer dominates the CLI surface. |

Largest current operator files above the threshold:

| Lines | File |
| ---: | --- |
| 1,406 | `crates/canic-cli/src/auth/mod.rs` |
| 1,269 | `crates/canic-cli/src/fleets/tests.rs` |
| 1,249 | `crates/canic-backup/src/restore/tests/apply_journal.rs` |
| 941 | `crates/canic-cli/src/tests.rs` |
| 918 | `crates/canic-cli/src/blob_storage/tests.rs` |
| 899 | `crates/canic-cli/src/restore/tests/run.rs` |
| 878 | `crates/canic-cli/src/cycles/wallet.rs` |
| 862 | `crates/canic-cli/src/fleets/adoption_report.rs` |
| 850 | `crates/canic-cli/src/scaffold/mod.rs` |
| 712 | `crates/canic-cli/src/auth/tests.rs` |
| 710 | `crates/canic-cli/src/blob_storage/mod.rs` |
| 650 | `crates/canic-cli/src/build.rs` |
| 626 | `crates/canic-host/src/deployment_truth/tests/lifecycle/verification/mod.rs` |
| 614 | `crates/canic-backup/src/plan/types.rs` |
| 613 | `crates/canic-cli/src/replica/mod.rs` |

Largest current scripts:

| Lines | File |
| ---: | --- |
| 1,246 | `scripts/ci/wasm-audit-report.sh` |
| 391 | `scripts/ci/blob-storage-cli-proof-lib.sh` |
| 318 | `scripts/dev/install_dev.sh` |
| 289 | `scripts/ci/check-blob-storage-inventory-gate.sh` |
| 268 | `scripts/ci/verify-packaged-downstream-cli.sh` |
| 222 | `scripts/ci/verify-packaged-downstream-wasm-store.sh` |
| 206 | `scripts/ci/check-blob-storage-cashier-inventory-gate.sh` |
| 185 | `scripts/ci/auth-renewal-cli-proof-lib.sh` |
| 183 | `scripts/app/README.md` |
| 179 | `scripts/ci/instruction-audit-report.sh` |

Current split-module sizes sampled for this audit:

| Lines | File |
| ---: | --- |
| 541 | `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/install.rs` |
| 394 | `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/schedule.rs` |
| 255 | `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/mod.rs` |
| 232 | `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/retrieval.rs` |
| 151 | `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/view.rs` |
| 102 | `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/identity.rs` |
| 460 | `crates/canic-core/src/api/blob_storage/billing.rs` |
| 122 | `crates/canic-core/src/api/blob_storage/lifecycle.rs` |
| 67 | `crates/canic-core/src/api/blob_storage/gateway.rs` |
| 41 | `crates/canic-core/src/api/blob_storage/mod.rs` |
| 22 | `crates/canic-core/src/api/blob_storage/hash.rs` |

## Positive Consolidation Readout

- `canic-host::evidence_envelope` owns stable envelope DTOs, schema refs,
  exit-class precedence, summary mapping, payload hashing, and file input
  fingerprinting.
- `canic-host::build_provenance` owns the stable build-provenance payload and
  build-provenance envelope creation.
- `canic-host::policy_gate` owns policy parsing, project evidence manifests,
  gate evaluation, and policy report DTOs.
- `canic-host::registry::parse_registry_entries` remains the registry parser
  owner, while `canic-host::subnet_registry::query_subnet_registry_json` owns
  registry query transport.
- `canic-host::response_parse` remains the shared low-level parser owner for
  JSON field lookup, `response_candid`, numeric parsing, and cycle balances.
- `canic-cli::output` owns common text and pretty-JSON output-file helpers.
- Root-renewal scheduling, retrieval, install outcome recording, identity
  mapping, and view conversion are split under private child modules.
- Blob-storage API behavior is split into private hash, lifecycle, gateway,
  and billing children; the parent stays a small facade.
- Root proof provisioning has distinct endpoint, API, workflow, ops, storage,
  replay-policy, verifier, and DTO owners.

## Findings

### Positive - root-renewal lifecycle ownership is now visible

Evidence:

- `root_issuer_renewal/mod.rs` declares private child modules for `identity`,
  `install`, `retrieval`, `schedule`, and `view`.
- Production child modules are below 600 LOC. The largest is
  `install.rs` at 541 LOC, followed by `schedule.rs` at 394 LOC.
- Focused auth delegation tests passed, including scheduled renewal prepare,
  retrieval, install preflight, success, retry, expiry, manual install, and
  status cases.

Impact:

- The split reduces parent-module pressure without creating a second lifecycle
  owner.
- Future renewal work has an obvious target module by operation phase.

Recommended consolidation:

- Keep the child modules private and route new lifecycle behavior to the
  matching owner.
- Avoid moving scheduler, retrieval, or install state mutation back into the
  parent facade.

### Positive - blob-storage API split reduced facade pressure

Evidence:

- `api/blob_storage/mod.rs` is now 41 LOC and delegates to private children.
- Production children are responsibility-specific: `hash`, `lifecycle`,
  `gateway`, and `billing`.
- The previous direct API-to-model conversion concern is closed by using
  ops-owned blob-storage conversion helpers.

Impact:

- The public API type remains the endpoint-facing facade, but behavior is no
  longer concentrated in one large file.
- No duplicate API/model conversion owner was found.

Recommended consolidation:

- Keep new blob-storage endpoint behavior in the matching private child.
- Keep canonical blob root hash conversion behind
  `ops::blob_storage::conversion`.

### Watchpoint - CLI auth command hub is the largest operator production file

Evidence:

- `crates/canic-cli/src/auth/mod.rs` is 1,406 LOC.
- `auth/codec.rs` is 501 LOC and now owns command-specific Candid/JSON parsing
  for renewal work by reusing `canic-host::response_parse` primitives.
- `auth/render.rs` is 224 LOC and keeps renewal output rendering out of the
  command dispatcher.

Impact:

- The size is a patch-radius risk if future auth CLI changes add more command
  families to the same parent.
- It is not a DRY violation yet because the shared low-level parser remains
  host-owned and the auth-specific codec/render pieces are local.

Recommended consolidation:

- Split another auth child module when a coherent command family changes.
- Do not create a broad CLI command framework unless two or more command
  families share behavior-bearing parsing, rendering, or fallback rules.

### Watchpoint - blob-storage CLI is large but domain-local

Evidence:

- `crates/canic-cli/src/blob_storage/mod.rs` is 710 LOC and
  `blob_storage/tests.rs` is 918 LOC.
- Blob-storage parsing and rendering have local children at 290 LOC and
  131 LOC.
- Required CLI tests and inventory/proof scripts exercise this surface, but no
  second blob-storage command owner was found.

Impact:

- Patch-radius risk exists if status, billing, gateway sync, and funding
  command behavior keep growing in the parent.
- The current split is still readable because parse/render behavior has local
  owners and runtime conversion lives below the API in ops.

Recommended consolidation:

- Extract only around a changed subcommand family, such as billing/funding or
  gateway sync, when that family changes next.
- Keep output rendering local to CLI and conversion logic out of CLI.

### Watchpoint - evidence envelope assembly remains command-specific

Evidence:

- Stable envelope and schema types are host-owned in
  `canic-host::evidence_envelope`.
- Build provenance, policy gate, fleet adoption, deployment check, and
  evidence gate each assemble command-specific payloads and fingerprints.
- `file_input_fingerprint`, schema refs, and exit-class precedence are shared
  host helpers.

Impact:

- The shared schema and fingerprint rules are centralized, so drift risk is
  bounded.
- Local assembly preserves distinct target, payload, source-config, input, and
  summary semantics.

Recommended consolidation:

- Keep assembly local while emitters differ by payload and target semantics.
- Revisit helper extraction if another emitter appears or if two emitters gain
  the same optional-input or output-mode behavior.

### Low - registry traversal and response parsing stay centralized enough

Evidence:

- Registry parsing is owned by `canic-host::registry::parse_registry_entries`.
- Registry query transport is owned by `canic-host::subnet_registry`.
- Response parsing helpers live under `canic-host::response_parse`.
- CLI snapshot, backup preflight, metadata, metrics, auth, and blob-storage
  callers reuse these helpers instead of carrying local generic parsers.

Impact:

- Parser drift risk is low.
- Remaining traversal and fallback behavior is command-specific and should
  stay local until a shared behavior change appears in multiple commands.

Recommended consolidation:

- Do not reintroduce command-local registry JSON parsing.
- Extract traversal only if backup and snapshot need the same new fallback or
  diagnostic behavior.

### Watchpoint - release and proof scripts keep growing

Evidence:

- Script surface grew to 5,623 lines.
- `scripts/ci/wasm-audit-report.sh` is 1,246 lines.
- `scripts/ci/blob-storage-cli-proof-lib.sh` and
  `scripts/ci/auth-renewal-cli-proof-lib.sh` are separate proof helpers.
- Packaged downstream proofs still isolate package roots and reject
  `target/debug/canic` usage in package proof roots.

Impact:

- Shell duplication can become hard to audit if package-root, temporary-root,
  Cargo cache, installed-binary, or patching rules drift independently.
- The retained scripts still answer distinct proof, inventory, audit, or
  developer-local questions.

Recommended consolidation:

- Keep scripts separate while their release questions differ.
- If package-root or installed-binary isolation changes again, prefer a small
  sourced shell helper over another broad script.

## Root Proof and Delegated-Auth Ownership Check

The root proof provisioning and delegated-auth lifecycle scan found no
duplicate lifecycle owner.

| Concern | Current owner |
| --- | --- |
| Endpoint macros and protocol constants | `crates/canic/src/macros/endpoints`, `crates/canic-core/src/protocol.rs` |
| API guard surface | `crates/canic-core/src/api/auth/mod.rs` |
| Domain policy | `crates/canic-core/src/domain/policy/auth/root_provisioning.rs` |
| DTO boundary shapes | `crates/canic-core/src/dto/auth.rs` |
| Workflow install orchestration | `crates/canic-core/src/workflow/runtime/auth/provisioning/mod.rs` |
| Generic root proof metadata/proof helpers | `crates/canic-core/src/ops/auth/delegation/{active,batch,pending,root_issuer_policy}.rs` |
| Root-renewal schedule/retrieval/install/view ownership | `crates/canic-core/src/ops/auth/delegation/root_issuer_renewal/*` |
| Stable record mapping | `crates/canic-core/src/ops/storage/auth/mapper.rs`, `crates/canic-core/src/storage/stable/auth` |
| Replay policy coverage | `crates/canic-core/src/ops/replay/policy.rs`, `crates/canic-core/src/replay_policy` |
| Active proof verification config | `crates/canic-core/src/ops/auth/types.rs` and delegated active-proof ops |

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
- root-renewal, blob-storage, deploy, operator, and script sizing

Focused validation passed:

- `cargo test --locked -p canic-cli deploy -- --nocapture`
- `cargo test --locked -p canic-cli evidence -- --nocapture`
- `cargo test --locked -p canic-core ops::auth::delegation --lib -- --nocapture`
- `git diff --check`

## Risk Matrix

| Category | Risk | Notes |
| --- | ---: | --- |
| Ownership boundaries | 3 / 10 | Shared parser, registry, evidence DTO, output, root-proof, root-renewal, and blob-storage owners are clear. |
| Runtime code duplication | 3 / 10 | Root proof provisioning and renewal lifecycle are split by layer/phase; no duplicate runtime lifecycle owner found. |
| CLI command duplication | 4 / 10 | Command-family glue is repeated and auth/blob-storage modules are large, but behavior is domain-local. |
| Backup/restore fixture duplication | 4 / 10 | Fixture setup remains sizable but test-contained and scenario-specific. |
| Evidence/report duplication | 4 / 10 | Envelope assembly remains command-specific while stable DTO/schema/fingerprint behavior is centralized. |
| Script duplication | 4 / 10 | Large scripts and proof libs remain separate by release/proof question; isolation-rule drift is the main watchpoint. |
| Overall | 3 / 10 | Low residual DRY risk; no source cleanup target justified in this pass. |

## Risk Score

Risk Score: **3 / 10**

This is low risk. The score stays flat because recent root-renewal and
blob-storage splits reduced runtime/API parent pressure, while CLI auth,
blob-storage CLI, and proof-script growth offset that improvement as
watchpoints.

## Follow-Up

- Keep root-renewal schedule/retrieval/install/view child modules private and
  route future lifecycle behavior to the matching owner.
- Keep the blob-storage API split intact and keep canonical blob root hash
  conversion behind `ops::blob_storage::conversion`.
- Watch `crates/canic-cli/src/auth/mod.rs` and
  `crates/canic-cli/src/blob_storage/mod.rs` before adding more command
  families to either parent.
- Revisit evidence helper extraction only if another envelope emitter appears
  or two emitters converge on the same output/fingerprint behavior.
- Watch proof scripts for shared package-root, temporary-root,
  installed-binary, and Cargo isolation drift.
