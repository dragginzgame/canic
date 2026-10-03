# canic-backup

`canic-backup` contains the backup and restore behavior that runs on an
operator's computer. Application canisters do not receive access to backup
files, local credentials, or restore commands.

The crate owns the host-side contracts behind the `canic` backup CLI:
manifests, topology hashing, download journals, durable artifact integrity,
backup layout validation, restore planning, restore apply journals, and native
runner summaries.

Backup creation/execution, restore preparation/run and prune share a parent-side
layout lock. Restore journal publication requires a `BackupLayoutGuard` and
durably retains the source layout before publishing recovery authority. Paused
or failed restores retain that reference; terminal runner replay releases it
after verifying command quiescence. Custom external journals use the same contract.

Fresh CLI backup execution currently rejects because Component Registry topology
preflight is unimplemented. The runner and same-release recovery contracts remain
maintained; this gap has no accepted implementation slice. See the
[availability boundary](../../docs/features/backup-and-restore/README.md#current-availability).

`DeploymentBackupManifest::validate()` enforces the hard manifest contract.
Restore-readiness checks stay focused on executable v1 restore requirements:
artifact integrity, safe verification, uploaded snapshot receipts, and journaled
execution state. Code/module hash metadata remains useful provenance, but it is
not a prerequisite for snapshot load because snapshot load restores code and
state together.
