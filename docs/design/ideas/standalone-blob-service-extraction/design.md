# Idea: Standalone Blob Service Extraction

Reviewed: 2026-09-06

## Status

- Deferred and unnumbered. No investigation, implementation or release is
  authorized by this note.
- Retained need: separate application blob-storage semantics from Canic's
  infrastructure lifecycle responsibilities.
- Canic owners: runtime/facade, host/CLI and testing owners for the eventual
  removal and generic Component integration.
- External owner: unassigned. A standalone service needs its own maintainer,
  repository, package names and release plan before extraction is scheduled.
- Repository scope: Canic only. This note authorizes no external repository
  creation or mutation.

## Current Boundary

Canic still has the `blob-storage` and `blob-storage-billing` features,
`canic blob-storage` commands, provider adapters and stable allocations.
Extraction has not shipped. Current allocations are recorded in
[the allocation owner](../../../../crates/canic-core/src/role_contract/allocation.rs).

The [Caffeine inventory](../../../contracts/BLOB_STORAGE_INVENTORY.md) and
[Cashier inventory](../../../contracts/BLOB_STORAGE_CASHIER_INVENTORY.md)
retain earlier protocol evidence. A future extraction must refresh that
evidence against exact provider source or deployed interfaces before selecting
a backend or promising its behavior.

## Retained Direction

A standalone service should own content identities, tenant references,
quotas, provider access, billing, retention and deletion. Canic would manage it
as an ordinary application Component with exact artifact and lifecycle
authority. Blob-specific types and policy would leave Canic's production
dependency graph; no fourth infrastructure role would be introduced.

The proposed external boundaries are:

| Boundary | Responsibility |
| --- | --- |
| Protocol library | Passive identities, requests, responses, errors and canonical Candid |
| Client library | Checkpointable hashing, upload/download and byte verification |
| Service library | Tenant authority, storage invariants, provider policy and workflows |
| Canister entrypoint | Thin endpoint/lifecycle adapters delegating to the service |

Package names and optional shared provider code are decisions for the external
owner. Linking a library must not silently export endpoints. A composed
Canic application must delegate to the same service handlers and satisfy
Canic's maintained managed-role contract. An opaque standalone Wasm is not
automatically a managed Component.

Caffeine remains the first backend candidate. Its suitability must be proved;
a generic backend plugin framework is outside this idea.

## Contracts Worth Preserving

- Distinguish a portable raw-content digest from a provider-specific object
  identity. Persist service, tenant, reference and immutable content identity
  together; keep gateway URLs and credentials out of durable references.
- Authorize reference release by exact tenant and actor authority. Possession
  of a digest or Canic controller status grants no tenant authority.
- Bound objects, references, bytes, sessions, receipts and global capacity.
  Define counter ownership and atomic reference/accounting transitions.
- Persist upload and deletion intent before paid effects. Reconcile uncertain
  completion through exact provider evidence; HTTP success or caller assertion
  alone must not prove durable storage or deletion.
- Bind upload recovery to its originating actor and epoch. Define finite
  receipt horizons and durable sequence bounds so expired receipts cannot
  authorize duplicate effects or reference-identity reuse.
- Separate logical reference release, provider deletion and billing stop.
  Consumers need a durable release outbox when deleting application records.
- Qualify MIME handling, active-content isolation and public-serving policy.
  Public-by-reference storage does not provide confidentiality.
- Keep provider request types and callback authority in one external owner;
  client and service adapters must not independently recreate that contract.

## Evidence Before Scheduling

1. Establish a concrete consumer and an external owner willing to maintain
   the service independently of Canic.
2. Refresh provider hashing, completion, deletion, gateway and Cashier evidence,
   including bounds, namespace exclusivity and uncertain-response behavior.
3. Qualify one standalone non-Canic consumer and one ordinary Canic Component
   using the same service implementation, with exact Candid and artifact
   evidence and no duplicate endpoint exports.
4. Prove tenant isolation, quota enforcement, resumable upload/read/release,
   corrupt-byte rejection and same-release restart/restore in PocketIC.
5. Inventory the complete Canic feature, runtime, allocation, macro, CLI,
   Medic, fixture, package and documentation removal.
6. Assign a complete Canic batch containing that hard cut, its direct evidence,
   propagation and changelog work. External service publication must be
   available before the integration relies on it.

The eventual Canic cut removes old surfaces completely, including their
stable allocation ownership. It provides no aliases, old-state readers,
cross-release migration or mixed-version recovery. Downstream application
adoption remains separately owned.

## Disposition

Retain as a separation candidate, not an approved external-product build.
The former numbered implementation plan and detailed speculative service
schema are retired; Git history retains the earlier research. If no external
owner or consumer emerges, reconsider the need before designing the service.

[Optional encrypted backup archival](../optional-encrypted-canister-snapshot-archives/design.md)
is independent and must not become a prerequisite for extraction.
