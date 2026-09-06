# Idea: Optional Encrypted Canister Snapshot Archives

Reviewed: 2026-09-06

## Status

- Deferred and unnumbered. No investigation, protocol, provider or release
  work is authorized by this note.
- Retained need: recover a completed same-release local backup after loss of
  the operator's primary disk.
- Owner: `canic-backup` for any future packaging and retrieval boundary;
  external tools or adapters own remote storage.
- Priority: conditional on a real recovery need that existing encrypted backup
  tooling cannot meet.
- Repository scope: Canic only; external provider implementation remains
  separately owned.

## Current Boundary

[Maintained backup and restore](../../../features/backup-and-restore/README.md)
already own capture, manifests, checksums, journals and restore execution.
They support same-release operational recovery. Backup selection follows the
last converged Fleet Ensure inventory and rejects an in-progress apply.

Remote archival is not implemented. The earlier proposal's permanent
cross-release readers and compatibility exception are withdrawn. Pre-1.0
release transitions remain reinstall-only, with no active exception.
Recovering an old archive must not import application state or authority into
a different Canic release.

## Smallest Useful Direction

First evaluate an operator procedure using an existing encrypted backup tool
around a completed, verified local backup. Canic should add a subsystem only
if a concrete source-integrity, retry or reconstruction gap requires one.

If that gap is demonstrated, an optional archive workflow would:

1. validate and freeze one complete local backup under exact same-release
   source authority;
2. package and encrypt it on the operator host using a reviewed authenticated
   encryption implementation;
3. publish only ciphertext through a separately configured storage boundary;
4. retain enough independently recoverable key, release and archive identity
   to find and authenticate the backup after primary-disk loss;
5. retrieve into private staging, authenticate before publication, verify all
   required artifact bytes and reconstruct the canonical local backup; and
6. delegate restore validation and execution to the existing owner.

Remote failure must not rewrite a successful local capture as incomplete.
Any stronger combined-job policy must report capture and archival separately.

## Required Boundaries

- No raw snapshot bytes, plaintext secrets, encryption keys or repository
  credentials reach a canister or public blob service.
- Bind format, source release, backup identity, object digest and destination
  to one operation. Reject unsupported source releases before provider
  mutation or restore; do not guess formats or add fallback decoders.
- Retain one current v1 contract if Canic needs a new protocol. There is no
  promise that a later release reads predecessor archives.
- Key and credential recovery must work without the lost disk. Encryption
  without a tested recovery bootstrap does not establish recoverability.
- Reuse existing durable I/O, source validation and restore logic. Reject
  incomplete, mutable or checksum-invalid sources and unsafe archive paths.
- Bound bytes, cost and retention. Journal intent before paid effects and
  reconcile uncertain upload, release and deletion responses exactly.
- Separate logical release, physical deletion and billing termination.
  Define provider retention guarantees and an independent maximum debit.
- Treat a local adapter's privileges honestly; configuration and signatures
  alone do not sandbox an executable.

## Evidence Before Scheduling

- A documented limitation of existing encrypted backup tooling and one
  concrete operator recovery case.
- A bounded threat model, selected crypto/key integration and exact source
  lease, format and provider contracts.
- Successful recovery on a blank second host using the same Canic release,
  independently retained release/tool artifacts, keys and credentials.
- Corrupt ciphertext, wrong release/key/source, interrupted publication,
  lost-response and exact-retry cases without duplicate paid effects.
- Byte-identical reconstruction through maintained backup verification and
  same-release PocketIC restore.
- An accepted owner, release position and complete batch plan.

## Disposition

Retain only the same-release encrypted off-site recovery need. The former
multi-release archive product, permanent reader policy and pre-approved M0
plan are removed. Detailed old crypto/provider sketches remain in Git history,
not as a second active contract.

This idea does not depend on
[standalone blob extraction](../standalone-blob-service-extraction/design.md).
Any provider must qualify independently; Canic gains no blob-specific
production dependency from archival.
