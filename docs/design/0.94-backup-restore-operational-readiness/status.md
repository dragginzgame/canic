# 0.94 Backup and Restore Operational Readiness - Status

Last updated: 2026-07-19

## Current State

The maintainer has approved closing 0.93 at `v0.93.36` and started 0.94 as a
focused backup/restore operational-readiness line. The exact operation and
durable-transition inventory is confirmed, and the early disposable-platform
capability gate is complete with two findings.

`CANIC-094-LOCK-001` is fixed in the current worktree by replacing
path-existence and `Drop` cleanup with one kernel-owned, no-follow,
close-on-exec lock descriptor. `CANIC-094-PROC-001` remains open: the current
host command boundary permits a child to survive runner death without a
restart-visible execution identity. No persisted schema, command syntax,
package version, Candid surface, or compatibility path has changed.

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
| `CANIC-094-J02` backup crash matrix | pending | none | none |
| `CANIC-094-J03` verification interruption | pending | none | none |
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
| `CANIC-094-LOCK-001` | P1 | fixed in worktree | backup persistence lock | `C07`; live exclusion, close-on-exec, unsafe entry, and `SIGKILL` tests |
| `CANIC-094-PROC-001` | P1 | open | host external command execution | `B18`, `R14`; child-lifetime probe |

## Validation State

- Exact operation/durable-transition inventory: confirmed at the anchor.
- Disposable local ICP capability probe: complete; fixture deleted and network
  stopped.
- Journal-lock focused tests: four passed, including abrupt owner death.
- Backup live-lock runner regression: passed.
- Restore CLI live-lock regression: passed.
- A completed-backup layout remains integrity-valid with the stable sidecar:
  passed.
- Strict all-target Clippy for `canic-backup` and `canic-cli`: passed.
- Changelog governance: passed.
- Design/status Markdown and link review: passed.
- Whitespace/diff hygiene: passed.
- Crash-point execution: not started.
- Realistic environment journey: not started.

## Next Action

Resolve `CANIC-094-PROC-001` at the existing host executor boundary without a
daemon or general supervisor. Freeze its deterministic direct-child and
descendant quiescence proof, then freeze the executable crash-point manifest
and begin the backup/verification baseline cases. The open lock finding is not
a reason to widen the slice or change persisted documents.
