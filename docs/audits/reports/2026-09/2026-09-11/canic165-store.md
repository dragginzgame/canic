# CANIC-165 retained Store boundary

Date: 2026-09-11. Canic-only implementation; release position remains unassigned.

The Store now retains opaque fixture descriptors, verified chunks, ingestion
progress and exact target grants in its own allocation at ID 68. Executable
template storage remains separate. This is the source/delivery primitive inside
FP2, not completed automatic fixture provisioning.

## Maintained contract

Content identity hashes the schema-1 descriptor's format hash, encoded length,
ordered chunk lengths/digests and expected completion summary. It excludes the
release and target to avoid a circular build identity. Commands reuse the
existing bounded command envelope and uploads reuse the existing 1 MiB chunk
transport. These are transport bounds, not instruction-budget qualifications.

Only the Store's bound Root can prepare/upload fixtures or set grants. A grant
retains the Registry's existing managed-canister identity, including either a
Component or a child with its owning Component, parent, role and target.
Store admission verifies Root, subnet, Fleet/epoch, release and nonzero
installation identity. Root's installed-target verification and selection of
reviewed content remain required in the subsequent orchestration.

Grant changes compare the expected revision. Exact repeated intent returns the
current result; delayed grant/revoke intent cannot overwrite a newer revision.
Disabled revisions remain retained so revocation does not reopen revision zero.
Reads require both the exact target caller and the complete current enabled
grant. Completion of source upload is distinct from application data readiness.

Each upload verifies its indexed length/digest before committing chunk bytes,
the cursor and byte accounting synchronously. Earlier exact chunks replay
without writes; skipped/conflicting chunks fail. Shared capacity charges
retained serialized keys, descriptors, payloads and grants alongside executable
templates. This is logical retained-byte accounting, not allocated physical
stable-memory pages.

Ordinary Store GC cannot retire a Store with retained fixture state. This
conservative retention rule protects future consumers; reviewed release of
references and terminal Store retirement remain unfinished. It is not a
complete source-retention lifecycle.

## Evidence and limits

The focused native checks cover source replay/reconstruction, invalid content,
grant revision/revocation/replacement, child-only authority, capacity in both
fixture and executable-template admission, and the reserved memory ownership
and state declarations.

The focused PocketIC journey builds the canonical Store and uses an actual
application canister as the read caller. It checks a 1 MiB chunk, restart during
upload, replayed completion, wrong caller, conflicting chunk, retained grant
after restart, exact revoke replay and fencing of replaced grants. Store
restart uses the same Wasm and release identity. The application relay is a
test endpoint; it does not install a production importer or readiness gate.

The test directly submits commands with the bound Root sender. It qualifies
Store-side authentication and persistence, not Root's autonomous issuance,
physical target verification, placement sequencing or funding. The earlier
application commit and held-response proofs remain separate. No combined
parent/child data-readiness result is claimed.

Final checks passed: 44 focused core tests, 10 Store/state tests, scoped
warning-denied Clippy for the runtime/probe and integration target, and the
Root-only feature compile. All four composed lifecycle PocketIC tests passed
in 50.82 seconds (94-second runner). The real Store runtime Cargo/link step
took 7.76 seconds in that run. These are local qualification timings, not
deployment or production-load measurements.

All 1,605 recorded Rust, Cargo/config and Candid files stayed unchanged across
the final PocketIC run. See the [qualification record](canic165-store-qualification.json)
for commands, source digests and retained logs.

## Remaining FP2 work

- Bind fixture descriptors to reviewed release/build inputs and publish them
  through the existing resumable host/Root owners.
- Derive grants from verified installation/allocation intent, including later
  Shards after the operator exits.
- Connect the registered application importer, exact completion receipt,
  deployment completion and application dispatch/placement gate.
- Qualify combined Prepared-Root/parent/child ordering, source outages/backoff,
  funding review, retained references and terminal retirement.
- Finish facade/config/runbook propagation and measure production-sized payloads.

The CANIC-165 batch and combined worktree are not push-ready. No package
version, dependency, Git publication, sibling repository or live Fleet changed
in this implementation. The existing .15 operator corrections remain
separately qualified; their presence does not complete this added capability.

