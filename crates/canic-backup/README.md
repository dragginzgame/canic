# canic-backup

Host-side manifest and orchestration primitives for Canic deployment backup and
restore workflows.

The crate owns the host-side contracts behind the `canic` backup CLI:
manifests, topology hashing, download journals, durable artifact integrity,
backup layout validation, restore planning, restore apply journals, and native
runner summaries.

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
