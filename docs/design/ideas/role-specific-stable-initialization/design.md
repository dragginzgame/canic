# Idea: Role-specific stable-store initialization

Date: 2026-09-13

## Status

Deferred at the maintainer's request. This is an unnumbered idea, not an active
release requirement or implementation authority. Scope is Canic only.

## Opportunity

The scaling registry (ID 50) and placement-index registry (ID 51) initialize
eagerly although their ownership is role-specific. With the current 1 MiB
bucket default, avoiding an unused allocation could save 1 MiB per store per
canister. The authority restore fence (ID 59), owned by Root/Coordinator roles,
is another candidate for non-authority roles after tracing its callers.

The initial estimate is 2–3 MiB per eligible canister and roughly 1–2
developer-days for a bounded implementation and focused qualification. These
are source-derived planning estimates, not measured savings or a delivery
commitment. A Fleet of 100 canisters that need neither registry could save
approximately 200 MiB collectively. Existing allocations cannot shrink;
measure fresh installations at the reinstall-only release boundary.

Sharding, blob storage and Coordinator state already have feature gates.
ICP refill state already initializes lazily to avoid opening Root-owned state
on other roles. Do not count these existing exclusions as new savings. The
Root's observed bucket slack is not wholly removable: many small stores are
required by its current responsibilities.

## Direction and owners

Use the canonical role/capability catalog to identify owners, retaining memory
declaration validation before storage access. Prefer a small change to the
specific stores and their owning lifecycle paths over a replacement runtime
initializer. Keep stable identifiers, single-manager bootstrap and same-release
recovery authority intact. An observation path must not accidentally allocate
an otherwise unused store. First-use allocation alone does not establish that
the store is unnecessary throughout that role's lifetime.

Canic core owns the storage/lifecycle boundary; host role manifests and the
generated-canister fixtures must agree with the maintained role contract.
Bucket geometry, database capacity, record limits and application schemas are
outside this idea. This does not propose merging independent authority stores.

## Evidence before promotion

- Trace startup, diagnostics, timers, snapshots and first-use callers for each
  candidate; distinguish never-used state from temporarily empty state.
- Measure allocated IDs and physical bytes for representative generated roles
  before and after the change with identical bucket geometry.
- Prove owning-role initialization, stable reopen and same-release recovery.
- Prove non-owning roles leave the allocations unopened through observation
  and ordinary maintenance, and preserve role/manifest ownership checks.
- Record scoped native/PocketIC evidence, the complete bounded release batch
  and actual savings before scheduling a larger initialization redesign.

See the maintained [layout and measurements](../../../features/runtime/stable-memory-layout.md)
and [delivery policy](../../../governance/delivery-cadence.md).
