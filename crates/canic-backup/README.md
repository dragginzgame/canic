# canic-backup

Host-side manifest and orchestration primitives for Canic fleet backup and
restore workflows.

The crate owns the host-side contracts behind the `canic` backup CLI:
manifests, topology hashing, download journals, durable artifact integrity,
backup layout validation, restore planning, restore apply journals, and native
runner summaries.

`FleetBackupManifest::validate()` enforces the hard manifest contract.
Restore-readiness checks stay focused on executable v1 restore requirements:
artifact integrity, safe verification, uploaded snapshot receipts, and journaled
execution state. Code/module hash metadata remains useful provenance, but it is
not a prerequisite for snapshot load because snapshot load restores code and
state together.
