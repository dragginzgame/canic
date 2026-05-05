# canic-backup

Host-side manifest and orchestration primitives for Canic fleet backup and
restore workflows.

The crate owns the host-side contracts behind the `canic` backup CLI:
manifests, topology hashing, download journals, durable artifact integrity,
backup layout inspection, provenance reports, restore planning, restore apply
journals, and native runner summaries.

`FleetBackupManifest::validate()` enforces the hard manifest contract.
`FleetBackupManifest::design_conformance_report()` exposes the softer v1 design
readiness checks used by operator preflight flows.
