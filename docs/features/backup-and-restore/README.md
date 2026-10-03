# Backup And Restore

A canister snapshot is a saved copy of a canister's state. Canic's backup tools
download and verify snapshots on the operator's computer. Its restore tools
check those files, map them to the intended canisters, and record progress so an
interrupted recovery can safely continue.

Backups are for recovery within the same Canic release. They are not a way to
carry application state across the clean reinstall required between pre-1.0
releases.

```text
Live canisters -> snapshots -> verified local backup
                                      |
                               restore preparation
                                      |
                              journaled restore run
                                      |
                              recovered canisters
```

## Current Availability

<img src="../../../assets/256x256/mechanic-attention.png" align="left" width="110" alt="The Canic mechanic raising a hand beside a warning symbol" />

**Creating a new backup is currently unavailable.** `canic backup create
<fleet>` stops before taking snapshots because the live topology safety check is
not yet implemented. `--dry-run` can prepare local planning files for a
supported inventory, but it does not prove the live layout or permissions and
does not create a backup. Current selection requires exactly one Fleet Subnet
Root.

<br clear="left" />

This gap dates from the 0.100.80 removal of the public Subnet Registry query.
It has no accepted implementation slice in the current 0.110 batch or scheduled
0.111 blob extraction. The proposed
[OC-5 inventory and backup work](../../design/ideas/openchat-scale-application-support/design.md)
is an unscheduled idea, not a delivery commitment. Completion needs an accepted
Host/CLI/Backup slice covering authoritative membership, controller/read authority,
quiescence, topology changes, interruption and same-release recovery.

Verification and restore of existing valid same-release backups retain their
own artifact, identity and journal checks. Preserve the backup runner and its
recovery machinery while the missing live preflight remains fail-closed.

Artifact SHA-256 verification accepts uppercase and lowercase hex as the same
digest. Malformed hashes and different artifact bytes still reject.

The local runner resolves the selected backup directory once before deriving
download and verification paths. Relative paths and directory links selected by
the operator are supported; links inside artifact trees remain rejected.
CLI layout creation and runner execution share the execution-journal lock.
They also share a layout lifetime lock with restore preparation, execution and
prune. That lock lives beside the backup directory so deletion cannot replace
its identity while another process still holds it.

If manifest publication finishes before its completion receipt, retry preserves
the published timestamp and tool metadata. It verifies the backup identity,
topology, snapshots and artifact checksums against the retained plan and journal
before adopting that file. Conflicting content is never overwritten.

## Maintained Primitives

- topology-aware full-Fleet and subtree backup selection
- snapshot download journals with durable artifact paths and hashes
- manifest validation and byte-integrity verification
- restore-readiness checks and explicit principal mapping
- parent-before-child restore planning
- resumable, bounded restore execution with operator-attention states
- local pruning kept separate from live snapshot deletion

For an existing backup, the operator path is:

```bash
canic backup verify <backup>
canic restore prepare <backup> --require-verified --require-restore-ready
```

## Local Retention

`canic backup prune --keep N` retains the newest N verified complete backups.
It verifies artifact checksums before counting a copy toward retention and keeps
those retained copies locked while deleting older eligible copies. `--dry-run`
uses the same eligibility checks. `--keep 0` explicitly selects no ordinary
retained copies.

Prepared restores retain a durable reference to their source backup, including
when `--journal-out` selects an external path. Paused, failed and interrupted
restores keep that reference. Removing or moving an unfinished journal does not
release its artifacts. Resume using its original journal location. Once the
restore completes and its external commands have exited, the runner releases its
reference. If interrupted at terminal publication, rerun the same journal to
finish that release without repeating completed effects.

Prune reports busy, restore-referenced and invalid layouts as skipped. A deletion
failure is reported alongside any directories already removed and makes the
command fail. Failed backup journals remain recovery evidence and are retained.

## Boundary

Canisters do not read or write backup files. Filesystem access, ICP CLI calls,
credentials, manifests, journals, and restore runners remain on the operator
host. Current releases use backup and restore for same-release recovery rather
than cross-release state migration. No active pre-1.0 design grants backup,
copied-state dry-run or restore-manifest authority to preserve application
state across releases.

Backup selection deliberately uses the last converged Fleet Ensure inventory.
An unapplied successor plan does not replace that snapshot. Once apply starts,
its nonterminal journal blocks backup until the successor state is fully
validated and published as converged. Other operator commands bind the exact
current plan and journal instead.

The ordering contract has these crash boundaries:

| Crash point | Journal visible to backup | Backup result |
| --- | --- | --- |
| before successor apply | prior `Converged` | prior converged topology; planning may refresh only backup-inert observation and prior journal-proven reinstall evidence |
| after apply publishes its journal, before or during effects | `InProgress` or `ReplanRequired` | refused as nonterminal |
| after validated terminal state is written, before terminal journal publication | nonterminal | refused as nonterminal |
| after terminal journal publication | successor `Converged` | successor validated topology |

This safety claim depends on Fleet Ensure retaining that write order: publish a
nonterminal journal before any effect-owned state and publish validated terminal
state before the matching `Converged` journal.

Remote snapshot archival is outside the current local backup contract.
Product blob storage is a separate feature.

## Start Here

- [CLI backup and restore guide](../../../crates/canic-cli/README.md)
- [Backup domain crate](../../../crates/canic-backup/README.md)
- [Recovery and retry runbooks](../../operations/recovery-retry-runbooks.md)
