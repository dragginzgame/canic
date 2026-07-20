# 0.94 Backup and Restore Operational Readiness - Status

Last updated: 2026-07-20

## Current State

The maintainer released durable artifact and manifest reconciliation as
`v0.94.10`.
The capability, command-lifetime, pending-claim, lifecycle, snapshot-create,
download-effect, downloaded-transition, checksum, and artifact-publication
findings and proof gaps are fixed or completed in released code.

The current 0.94.11 draft completes all 12 terminal execution receipt/state
publication cases (`B16`) and final-successful-response loss (`B17`). Every
post-preflight operation publishes terminal state and its receipt in one
durable execution-journal document. Restart from the pre-write side reconciles
through the operation's canonical authority; restart from the post-write side
skips the completed operation with exactly one receipt and no repeated
mutation. Losing the final response after complete persistence replays the
same completed backup with zero commands, receipts, or layout changes. No
product defect or public recovery surface was required. CLI syntax, Candid,
durable document shapes, package versions, and genuine version `1` fields are
unchanged. The aggregate verification journey passes; the backup crash journey
remains pending until every assigned case passes.

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
| `CANIC-094-J02` backup crash matrix | pending | [protocol baseline](../../audits/reports/2026-07/2026-07-19/0.94-executable-recovery-protocol-baseline.md); [preflight publication](../../audits/reports/2026-07/2026-07-20/0.94-preflight-publication-crash-cases.md); [pending claims](../../audits/reports/2026-07/2026-07-20/0.94-backup-pending-claim-crash-cases.md); [stop reconciliation](../../audits/reports/2026-07/2026-07-20/0.94-stop-effect-reconciliation.md); [snapshot-create reconciliation](../../audits/reports/2026-07/2026-07-20/0.94-snapshot-create-reconciliation.md); [start reconciliation](../../audits/reports/2026-07/2026-07-20/0.94-start-effect-reconciliation.md); [download staging](../../audits/reports/2026-07/2026-07-20/0.94-download-staging-reconciliation.md); [downloaded transition](../../audits/reports/2026-07/2026-07-20/0.94-downloaded-transition-reconciliation.md); [checksum calculation](../../audits/reports/2026-07/2026-07-20/0.94-checksum-effect-recomputation.md); [checksum transition](../../audits/reports/2026-07/2026-07-20/0.94-checksum-transition-reconciliation.md); [artifact publication](../../audits/reports/2026-07/2026-07-20/0.94-artifact-publication-reconciliation.md); [durable transition](../../audits/reports/2026-07/2026-07-20/0.94-durable-artifact-transition-reconciliation.md); [manifest publication](../../audits/reports/2026-07/2026-07-20/0.94-manifest-publication-reconciliation.md); [terminal receipt](../../audits/reports/2026-07/2026-07-20/0.94-terminal-receipt-publication.md); [final response](../../audits/reports/2026-07/2026-07-20/0.94-final-response-loss.md); `B01`-`B17` | `CANIC-094-BACKUP-001` through `-010` fixed; B11, B16, and B17 no finding |
| `CANIC-094-J03` verification interruption | pass | [protocol baseline](../../audits/reports/2026-07/2026-07-19/0.94-executable-recovery-protocol-baseline.md); `V01`-`V03`; resumed | none |
| `CANIC-094-J04` restore crash matrix | pending | none | none |
| `CANIC-094-J05` completed-operation replay | pending | [backup replay](../../audits/reports/2026-07/2026-07-20/0.94-final-response-loss.md); restore replay pending | none |
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
| `CANIC-094-BACKUP-001` | P1 | fixed in `v0.94.3` | backup pending local recovery | [pending-claim report](../../audits/reports/2026-07/2026-07-20/0.94-backup-pending-claim-crash-cases.md); `B04` |
| `CANIC-094-BACKUP-002` | P1 | fixed in `v0.94.3` | backup stop recovery | [stop-reconciliation report](../../audits/reports/2026-07/2026-07-20/0.94-stop-effect-reconciliation.md); `B05` |
| `CANIC-094-BACKUP-003` | P1 | fixed in `v0.94.4` | backup snapshot-create recovery | [snapshot-create report](../../audits/reports/2026-07/2026-07-20/0.94-snapshot-create-reconciliation.md); `B06`-`B07` |
| `CANIC-094-BACKUP-004` | P1 | fixed in `v0.94.5` | backup start recovery | [start-reconciliation report](../../audits/reports/2026-07/2026-07-20/0.94-start-effect-reconciliation.md); `B08` |
| `CANIC-094-BACKUP-005` | P1 | fixed in `v0.94.6` | backup download-staging recovery | [download-staging report](../../audits/reports/2026-07/2026-07-20/0.94-download-staging-reconciliation.md); `B09` |
| `CANIC-094-BACKUP-006` | P1 | fixed in `v0.94.7` | downloaded-artifact transition recovery | [downloaded-transition report](../../audits/reports/2026-07/2026-07-20/0.94-downloaded-transition-reconciliation.md); `B10` |
| `CANIC-094-BACKUP-007` | P1 | fixed in `v0.94.9` | checksum-verified transition recovery | [checksum-transition report](../../audits/reports/2026-07/2026-07-20/0.94-checksum-transition-reconciliation.md); `B12` |
| `CANIC-094-BACKUP-008` | P1 | fixed in `v0.94.9` | checksum-bound artifact publication | [artifact-publication report](../../audits/reports/2026-07/2026-07-20/0.94-artifact-publication-reconciliation.md); `B13` |
| `CANIC-094-BACKUP-009` | P1 | fixed in 0.94.10 draft | durable artifact reconciliation | [durable-transition report](../../audits/reports/2026-07/2026-07-20/0.94-durable-artifact-transition-reconciliation.md); `B14` |
| `CANIC-094-BACKUP-010` | P1 | fixed in 0.94.10 draft | immutable manifest publication | [manifest-publication report](../../audits/reports/2026-07/2026-07-20/0.94-manifest-publication-reconciliation.md); `B15` |

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
  selected operation once; pending stop and start observe status before
  action; an empty exact snapshot delta justifies one create; an exact
  `Created` download resumes into reset private staging; pending checksum and
  finalization operations resume once.
- `B05` committed-stop/receipt-loss: passed. Restart observes the exact target
  as `Stopped`, appends one normal receipt, and issues no second stop. Unsettled,
  failed, wrong-identity, and unknown status observations reject.
- `B06` committed-snapshot/receipt-loss: passed under `SIGKILL`. Restart adopts
  one exact new inventory identity, preserves its timestamp and size, and
  issues no second create. Missing baseline, duplicate, and multiple-new
  identities reject without journal mutation.
- Both `B07` created-artifact publication sides: passed. Missing evidence uses
  the exact `B06` reconciliation; complete evidence reconstructs the normal
  receipt without inventory observation or external mutation.
- `B08` committed-start/receipt-loss: passed under `SIGKILL`. Restart observes
  the exact target as `Running`, appends one normal receipt, and issues no
  second start. Exact `Stopped` status justifies one start; returned command
  failure and explicit retry reconcile before another mutation.
- `B09` interrupted private download staging: passed under `SIGKILL`. Restart
  replaces partial synchronized bytes, downloads once, records one receipt,
  and leaves the artifact unpublished in exact `Downloaded` state. Unsafe
  staging roots reject without being followed, while `Downloaded` or later
  journal evidence is preserved for its canonical transition recovery.
- Both `B10` downloaded-artifact transition sides: passed under `SIGKILL`.
  Before rename, restart retains `Created`, performs one redownload, and
  proceeds to checksum. After rename and directory sync, restart adopts exact
  `Downloaded` evidence, reconstructs one receipt, and proceeds to checksum
  with zero download commands. Missing or mismatched evidence rejects without
  execution-journal mutation.
- `B11` checksum calculation: passed under `SIGKILL`. A checksum completed only
  in child memory is lost, while restart recomputes the same value from
  unchanged staged bytes, records one receipt, and executes no external
  command. Missing and unsafe input reject without claiming checksum-verified
  artifact state.
- Both `B12` checksum-verified transition sides: passed under `SIGKILL`.
  Before rename, restart retains `Downloaded` and computes the normal checksum.
  After rename and directory sync, restart rehashes exact staging, adopts the
  recorded checksum, and reconstructs one receipt without rewriting the row.
  Changed staging returns the typed checksum mismatch without weakening either
  durable journal or publishing a canonical artifact.
- Both `B13` canonical artifact-publication sides: passed under `SIGKILL`.
  Before rename, restart retains and publishes only exact checksum-bound
  staging. After rename and directory sync, restart rehashes and adopts the
  canonical directory without another rename. Changed staging and conflicting
  destinations reject without advancing the artifact journal or manifest.
- Both `B14` durable artifact-journal transition sides: passed under `SIGKILL`.
  Restart verifies the exact canonical directory against the durable checksum
  and preserves its inode without download or publication. Missing and changed
  canonical bytes return typed causes and cannot create a manifest.
- Both `B15` final-manifest publication sides: passed under `SIGKILL`. Restart
  publishes an absent manifest or adopts an exact durable manifest without
  replacement. Conflicting content is preserved and rejected; premature
  content blocks the runner before another operation can execute.
- All 12 `B16` terminal execution receipt/state publication sides: passed
  under `SIGKILL`. Before the durable rename, restart reconciles each of the
  six operations from its canonical authority and writes one terminal pair.
  After rename and directory sync, restart skips the completed operation with
  exactly one matching receipt. No mutating command or receipt is duplicated.
- `B17` final-successful-response loss: passed under `SIGKILL`. Replay returns
  the same completed backup identity and terminal summary with zero executor
  commands or executed operations. The journal, receipts, and complete layout
  tree remain byte-for-byte unchanged.
- `V01` before-validation, `V02` during-checksum, and `V03` after-result
  process-death cases: passed; the backup layout path/type/byte inventory is
  unchanged.
- Strict targeted Clippy for `canic-backup`, `canic-host`, and `canic-cli`:
  passed.
- Changelog governance: passed.
- Design/status Markdown and link review: passed.
- Whitespace/diff hygiene: passed.
- Crash-point execution: 51 cases passed; 55 remain pending.
- Realistic environment journey: not started.

## Next Action

Execute all four `B18` owner-dead/command-in-flight cases for stop,
snapshot-create, start, and download. Restart must reject while the original
command tree is alive, then reconcile or resume only after quiescence.
