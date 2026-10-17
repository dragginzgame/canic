# canic-backup

`canic-backup` contains the backup and restore behavior that runs on an
operator's computer. Application canisters do not receive access to backup
files, local credentials, or restore commands.

The crate owns the host-side contracts behind the `canic` backup CLI:
manifests, topology hashing, download journals, durable artifact integrity,
backup layout validation, restore planning, restore apply journals, and native
runner summaries.

Published `ic-backup` owns file/tree checksums, descriptor no-follow traversal and
create-new private artifact staging. Canic projects its checksum records into the
maintained manifest shape and preserves native I/O failures. Other traversal
failures use `ArtifactChecksumError::Artifact(ic_backup::ops::artifacts::ArtifactError)`.
Tree names must be UTF-8 so checksum paths retain exact identity.

Host Artifacts owns raw SHA-256 calculation and formatting for topology hashes,
layout sidecar names and restore-authority digests. Canic retains their exact
input framing and persisted representation.

IC Backup owns journal, command and layout guards, durable restore references and
guarded terminal release. Canic retains original Fleet authority, publication
barriers and same-operation recovery. Execution journals store exact original
command identities before dispatch; retries never recreate missing custody.
Ordinary JSON creation and replacement use Host Tooling's durable publication
engine after serialization succeeds. The existing `PersistenceError::Io` kind
is retained; its source carries typed publication visibility, native filesystem
failure and any independent staging-cleanup failure. A failure after publication
requires reconciliation against the retained document before retrying.
Its publication walk synchronizes admitted descriptors and computes the existing
directory framing before the no-replace rename; a path-based checksum cannot
replace that walk without losing its descriptor and barrier guarantees.

Backup creation/execution, restore preparation/run and prune share a parent-side
layout lock. Restore journal publication requires a `BackupLayoutGuard` and
durably retains the source layout before publishing recovery authority. Paused
or failed restores retain that reference; terminal runner replay releases it
after verifying command quiescence. Custom external journals use the same contract.

Fresh CLI backup execution currently rejects because Component Registry topology
preflight is unimplemented. The runner and same-release recovery contracts remain
maintained; the accepted 0.112 completion batch includes the missing capture
workflow and its actual same-release recovery proof. See the
[availability boundary](../../docs/features/backup-and-restore/README.md#current-availability).

`DeploymentBackupManifest::validate()` enforces the hard manifest contract.
Restore-readiness checks stay focused on executable v1 restore requirements:
artifact integrity, safe verification, uploaded snapshot receipts, and journaled
execution state. Code/module hash metadata remains useful provenance, but it is
not a prerequisite for snapshot load because snapshot load restores code and
state together.

Restore upload receipts and load selection compare validated digest identities
through IC Backup. Equivalent hexadecimal case does not change artifact identity;
malformed hashes, different bytes and unsupported algorithms still reject.
Validation preserves the original checksum text in operations and receipts.

## Continue From Here

- [Use backup and restore](../../docs/features/backup-and-restore/README.md)
- [Operate a Fleet](../../docs/operations/README.md)
- [Browse all documentation](../../docs/README.md)
- [Back to the main README](../../README.md)

Backup manifests, plans, mappings and journals have a 64 MiB encoded JSON limit.
Snapshot artifact streams have their own separate limits. Readers acquire a
regular file through Host's descriptor admission, bound the complete byte stream,
then decode and validate the record. Writers refuse oversized documents before
publication, retaining the previous record. Explicit file aliases remain usable;
layout ownership, manifest path checks and fresh terminal receipt validation
remain required.
