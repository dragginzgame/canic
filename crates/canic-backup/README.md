# canic-backup

`canic-backup` contains the backup and restore behavior that runs on an
operator's computer. Application canisters do not receive access to backup
files, local credentials, or restore commands.

The crate owns the host-side contracts behind the `canic` backup CLI:
manifests, topology hashing, download journals, durable artifact integrity,
backup layout validation, restore planning, restore apply journals, and native
runner summaries.

Published `ic-backup` owns checksum identities. IC Host supplies streaming hashes,
descriptor no-follow traversal and deterministic directory checksum framing.
Canic maps those results into Backup checksum records, retains its staging/fsync
policy and manifest checks, and preserves native I/O failures. Other traversal
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
Restore artifact staging copies admitted source descriptors into create-new
private destinations and synchronizes staged files and directories. Host computes
the digest and directory framing; Canic admits the result against retained evidence.

Backup creation/execution, restore preparation/run and prune share a parent-side
layout lock. Restore journal publication requires a `BackupLayoutGuard` and
durably retains the source layout before publishing recovery authority. Paused
or failed restores retain that reference; terminal runner replay releases it
after verifying command quiescence. Custom external journals use the same contract.

Live CLI backup creation returns `LiveCreateUnavailable` before local or remote
effects. Dry-run creation initializes passive plans and journals; capture preflight
application, execution transitions and download-state mutation APIs are removed.
Retained evidence validation and status projections remain available.

Restore parent ordering and operation-identity validation use IC Backup effect
graphs. Canic maps Fleet parent edges and serial restore phases into those graphs;
identifiers need not be contiguous. Backup graph bounds apply before execution.
The existing Canic runner validates its own serial journal completion ordering;
those states are not Backup Applied receipts. Backup 0.42 removes the former
caller-declared readiness API; completion without completed prerequisites still
returns a typed refusal before dispatch.

Canic still owns the active restore runner, apply journal and ICP subprocess
adapter. Adopting Backup’s execution coordinator and original plan/stage/attempt
authority remains separate work in [#513](https://github.com/dragginzgame/canic/issues/513),
requiring qualified recovery and subprocess custody. Retained failed states must
not become fresh spending authority. Application checkpoints and live capture
remain tracked by [#550](https://github.com/dragginzgame/canic/issues/550) and
[#394](https://github.com/dragginzgame/canic/issues/394). See the
[availability boundary](../../docs/features/backup-and-restore/README.md#current-availability).

`checkpoint::FrozenCheckpoint` binds one complete Root preparation unit to IC
Backup 0.48's original workflow and finite stage/attempt authority. Publish its
create-only intent under the original `BackupLayoutGuard`, then pass a
`CheckpointDriver` to Backup's new/retained workflow coordinators. Retain that
intent, every original journal and exact authenticated request/reply bytes across
interruption. Pending stages require qualified reconciliation under their original
observation allowance before retained continuation; rebuilding intent grants no
fresh attempt.

The required `CheckpointProvider` qualifies actual network/caller/release, Root
installation, current complete membership and controller/command custody. It
submits exactly one retained ingress and independently qualifies the original
reply. Additional observations need their own prior original accounting. This
adapter accounts only for host BEGIN/ADVANCE ingress. It does not grant or settle
Root-paid snapshot calls, restore/start effects or terminal release. There is no
default live provider. The installed fixture's exclusive simulator custody and
direct snapshot/load authority are qualification inputs, not production authority.

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
