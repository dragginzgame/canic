# 0.94 Backup and Restore Operational Readiness - Status

Last updated: 2026-07-20

## Current State

The maintainer released the executable protocol baseline as `v0.94.2`. All
three findings from the capability and first recovery traces are fixed in
released code.

The current worktree proves process death on both durable-write sides of the
preflight-applied plan (`B02`), preflight acceptance (`B03`), and all six
post-preflight pending claims (`B04`). The committed-stop case (`B05`) also
passes: restart reconciles `Stopped` into the normal receipt without a second
stop. Two reproduced liveness defects are fixed. Pending checksum verification
and manifest finalization resume without a second claim; snapshot-create,
start, and download remain fail-closed without effect-specific evidence. The
aggregate verification journey passes; the backup crash journey remains
pending until every assigned case passes. Persisted journals, CLI/JSON output,
Candid, and package versions are unchanged. The public injected backup executor
trait receives one required typed status observation as a pre-1.0 hard cut.

Known non-blocking structural residue deferred from 0.93: none. The baseline
risks below are bounded operational proof gaps intentionally assigned to 0.94,
not unfinished structural cleanup.

## Immutable Baseline

- Release anchor: `v0.93.36`.
- Source commit: `f9c28c48bdc72055d873e8291d201aac1c871f5e`.
- Source tree: `590abeec5d23d5163dc72663ca63359453bfb057`.
- Product-tree hash:
  `46445b89c955e741211206a15402ef8b8557b28f9e5a6b1ae594e19d950ea5cf`.
- Cargo.lock SHA-256:
  `0835d36e4f5acbe7ae80b7985f32dc419fa11ebf0c126b9e0ff21ba636a7de80`.
- Rust toolchain: `rustc 1.97.0 (2d8144b78 2026-07-07)`.
- Workspace package version at anchor: `0.93.36`.

## Required Journeys

| Journey | State | Evidence | Findings |
| --- | --- | --- | --- |
| `CANIC-094-J01` complete backup/verify/restore | pending | none | none |
| `CANIC-094-J02` backup crash matrix | pending | [protocol baseline](../../audits/reports/2026-07/2026-07-19/0.94-executable-recovery-protocol-baseline.md); [preflight publication](../../audits/reports/2026-07/2026-07-20/0.94-preflight-publication-crash-cases.md); [pending claims](../../audits/reports/2026-07/2026-07-20/0.94-backup-pending-claim-crash-cases.md); [stop reconciliation](../../audits/reports/2026-07/2026-07-20/0.94-stop-effect-reconciliation.md); `B01`-`B05` | `CANIC-094-BACKUP-001` and `-002` fixed |
| `CANIC-094-J03` verification interruption | pass | [protocol baseline](../../audits/reports/2026-07/2026-07-19/0.94-executable-recovery-protocol-baseline.md); `V01`-`V03`; resumed | none |
| `CANIC-094-J04` restore crash matrix | pending | none | none |
| `CANIC-094-J05` completed-operation replay | pending | none | none |
| `CANIC-094-J06` corruption/rejection matrix | pending | none | none |
| `CANIC-094-J07` realistic multi-canister journey | pending | none | none |

`pending` is planning state only, not a journey result. Once execution starts,
results use `pass`, `fail`, or `blocked_by_environment`; recovery dispositions
are tracked separately.

## Frozen Protocol Inventory

- Backup variants: four bundle-completed preflight variants and six
  post-preflight execution variants.
- Restore variants: six apply variants.
- Mutating external variants: four backup and four restore.
- Expected interruption cases: 52 backup, three verification, and 41 restore.
- Expected rejection cases: ten.
- Frozen minimum: 106 protocol cases plus the seven aggregate journeys.
- Executable manifest: frozen with exact area counts, point coverage, unique
  case identities, and operation-variant multipliers.

Adding a variant, durable transition, external effect, or crash-case generator
requires a design amendment and updated count before execution continues.

## Early Capability Gate

State: `completed_with_findings`.

Evidence:
[0.94 early platform capability gate](../../audits/reports/2026-07/2026-07-19/0.94-capability-gate.md).

- Disposable local ICP owns the maintained real snapshot CLI boundary.
- Snapshot create and upload identities were returned and recovered from
  inventory.
- Exact repeated snapshot restore succeeded while the target remained stopped,
  with restored module state observable through management status.
- Snapshot inventory alone does not bind an uploaded ID to the source checksum;
  reconciliation still requires exclusive activity and exact owned evidence.
- A child survived owner death, confirming that the current host command path
  cannot establish restart quiescence.

## Baseline Risks To Prove

- Both runners persist a pending claim before an external effect and persist a
  terminal receipt after it. Abrupt death between those boundaries can leave
  a committed effect with no durable terminal proof.
- An external command may survive runner death. Restart must establish command
  quiescence rather than infer it from an unchanged target or dead lock owner.
- Snapshot create, upload, and load are not admitted as blindly repeatable.
- At the release anchor, actual platform observation and replay capabilities
  had not been probed. The early gate now records the available evidence and
  its checksum-identity limitation.
- Backup artifact progress and backup execution progress use two coordinated
  durable journals that must remain exact across interruption.
- Restore upload staging is private and non-authoritative but may survive an
  abrupt process death.
- At the release anchor, the sidecar journal lock depends on `Drop` for
  removal. `CANIC-094-LOCK-001` now proves and fixes orphan recovery without
  weakening live exclusion or leaking ownership into spawned commands.
- Backup recovery coordinates execution, artifact-journal, and
  manifest/published-artifact authority; weaker evidence cannot manufacture a
  stronger state.
- Restore containment changes after the first committed load and protects the
  maintained entrypoint, not direct-principal child access.
- Existing coverage is strong at model and injected-executor level but has not
  yet closed the required real multi-canister process-restart journey.

These began as validation targets rather than pre-judged implementation fixes.
Confirmed items are tracked in the finding index; any further product change
still requires a reproducible required-journey finding.

## Finding Index

| Finding | Severity | Status | Owner | Evidence |
| --- | --- | --- | --- | --- |
| `CANIC-094-LOCK-001` | P1 | fixed in `v0.94.0` | backup persistence lock | `C07`; live exclusion, close-on-exec, unsafe entry, and `SIGKILL` tests |
| `CANIC-094-PROC-001` | P1 | fixed in `v0.94.1` | host external command execution | [command-quiescence report](../../audits/reports/2026-07/2026-07-19/0.94-command-quiescence-and-pending-recovery.md); `B18`, `R14` |
| `CANIC-094-RESTORE-001` | P1 | fixed in `v0.94.1` | restore pending recovery | [pending-recovery report](../../audits/reports/2026-07/2026-07-19/0.94-command-quiescence-and-pending-recovery.md) |
| `CANIC-094-BACKUP-001` | P1 | fixed in current 0.94.3 draft | backup pending local recovery | [pending-claim report](../../audits/reports/2026-07/2026-07-20/0.94-backup-pending-claim-crash-cases.md); `B04` |
| `CANIC-094-BACKUP-002` | P1 | fixed in current 0.94.3 draft | backup stop recovery | [stop-reconciliation report](../../audits/reports/2026-07/2026-07-20/0.94-stop-effect-reconciliation.md); `B05` |

## Validation State

- Exact operation/durable-transition inventory: confirmed at the anchor.
- Disposable local ICP capability probe: complete; fixture deleted and network
  stopped.
- Journal-lock focused tests: four passed, including abrupt owner death.
- Backup live-lock runner regression: passed.
- Restore CLI live-lock regression: passed.
- A completed-backup layout remains integrity-valid with the stable sidecar:
  passed.
- Command-lifetime owner-death and descendant proof: passed.
- Intended-child-only host descriptor inheritance proof: passed.
- Backup and restore active-command rejection plus quiescent unknown-outcome
  rejection: passed.
- Pending read-only restore verification replay: passed.
- Restore CLI option and run suites after the pending-reset hard cut: passed.
- Executable 106-case manifest count, point, uniqueness, and variant guards:
  passed.
- `B01` before-rename and after-directory-sync process-death cases: passed.
- `B02` preflight-applied-plan publication before rename and after directory
  sync: passed; restart performs only read-only target status checks.
- `B03` preflight-acceptance publication before rename and after directory
  sync: passed; restart either repeats read-only preflight or adopts the exact
  accepted journal without commands.
- All 12 `B04` pending-claim cases: passed. Before-claim loss executes the
  selected operation once; a pending stop observes status before action;
  durable snapshot-create, start, and download claims halt without a command;
  pending checksum and finalization operations resume once.
- `B05` committed-stop/receipt-loss: passed. Restart observes the exact target
  as `Stopped`, appends one normal receipt, and issues no second stop. Unsettled,
  failed, wrong-identity, and unknown status observations reject.
- `V01` before-validation, `V02` during-checksum, and `V03` after-result
  process-death cases: passed; the backup layout path/type/byte inventory is
  unchanged.
- Strict targeted Clippy for `canic-backup`, `canic-host`, and `canic-cli`:
  passed.
- Changelog governance: passed.
- Design/status Markdown and link review: passed.
- Whitespace/diff hygiene: passed.
- Crash-point execution: 22 cases passed; 84 remain pending.
- Realistic environment journey: not started.

## Next Action

Execute `B06` at the snapshot-create effect/receipt boundary. Recovery must
bind one observed snapshot identity to the exact target without creating a
second snapshot; operator action remains acceptable only if the maintained
flow subsequently completes.
