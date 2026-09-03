# Backup And Restore

Canic's backup and restore capability is host-side operational recovery for
canister snapshots. The CLI selects topology, freezes the source view, records
manifests and checksums, and journals both download and restore execution.

## What It Provides

- topology-aware full-Fleet and subtree backup selection
- snapshot download journals with durable artifact paths and hashes
- manifest validation and byte-integrity verification
- restore-readiness checks and explicit principal mapping
- parent-before-child restore planning
- resumable, bounded restore execution with operator-attention states
- local pruning kept separate from live snapshot deletion

A compact operator path is:

```bash
canic backup create <fleet>
canic backup verify <backup>
canic restore prepare <backup> --require-verified --require-restore-ready
```

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

Optional encrypted remote snapshot archival is design work, not part of the
current local backup contract. Product blob storage is a separate feature.

## Start Here

- [CLI backup and restore guide](../../../crates/canic-cli/README.md)
- [Backup domain crate](../../../crates/canic-backup/README.md)
- [Recovery and retry runbooks](../../operations/recovery-retry-runbooks.md)
- [Optional archive idea](../../design/ideas/optional-encrypted-canister-snapshot-archives/design.md)
