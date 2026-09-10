# ic-memory handoff for CANIC-162

Historical handoff: the upstream allocation API shipped in 0.13.0 and the
substrate re-export shipped in 0.13.1. Canic now adopts 0.13.1; see the
[current assessment](canic162-memory.md) for qualification and remaining work.
The original prompt is retained below.

```text
Please implement the ic-memory portion of CANIC-162: bounded, read-only physical
memory allocation attribution, then assess a supported bucket-size policy.
Work only in ic-memory; inspect Canic and Toko Miner read-only. Preserve existing
dirty changes. Read this repository's AGENTS.md and current handoff first.

Context:
- Toko Miner reports 232 MiB of allocated stable memory on a small Game Hub
  with Canic sharding and no application IcyDB store. This is an observation,
  not proof of a leak or attribution to 29 stores.
- Canic currently locks ic-memory 0.12.3 / ic-stable-structures 0.7.2.
  MemoryRuntime::new uses MemoryManager::init (128-page / 8 MiB buckets).
  Twenty-nine buckets plus a 64 KiB manager header equal 232.0625 MiB.
- Existing diagnostic_export reports virtual extents, not payload occupancy;
  it decodes ledger history. It does not expose the actual persisted bucket
  size or complete physical allocation attribution.
- Canic's pending MemoryQuery::allocations() integration uses committed
  declarations plus the existing manager's virtual handle size reads. Its
  protected MemoryAllocations observation is bounded to current usable IDs,
  omits ic-memory-owned/retired allocations, and leaves bucket attribution
  and residual fields explicitly unknown. The Root relay retains controller
  authorization. Do not add an endpoint in ic-memory or a second manager.

Implement:
1. An owned bounded diagnostic API on MemoryRuntime and the default runtime.
   Report actual persisted bucket size, physical extent, virtual pages/bytes,
   per-ID bucket allocation, known stable-key/owner bindings, manager metadata,
   the ic-memory ledger allocation, and explicit unknown/unmanaged residuals.
   Distinguish measured values from estimates and unavailable values. Never
   call virtual extent payload use or compute overhead as physical minus
   virtual without explaining the accounting. Make conservation checkable.
2. Bound collection before reading/decoding/copying historical state. Do not
   merely truncate an already-materialized unbounded export. No store init,
   grow/write, bootstrap commit, generation advance, or alternate manager.
   If upstream lacks a getter, isolate any supported, validated read-only
   layout adapter inside the substrate owner; reject unsupported/corrupt
   metadata with typed errors instead of assuming the requested setting.
3. Measure disposable fresh-state small-Hub-like and growing-store workloads.
   Include many initialized-but-small IDs, unopened IDs, bucket crossings,
   ledger overhead, reopen/read-only replay, and incomplete attribution.
   Compare the current default with justified smaller candidates. Report
   capacity consequences of the finite bucket table, growth and access cost,
   and physical/virtual bytes separately. No arbitrary performance threshold.
4. If measurements justify configurable bucket sizing, expose it through the
   existing owned bootstrap/construction API, validate it before effects, bind
   the policy to runtime authority, and report the actual persisted setting.
   Do not silently change the default just because a smaller bucket exists.
   Reopening current-release memory must honor or explicitly reject a setting
   mismatch; it must not pretend existing memory shrank. No cross-release
   migration, compatibility lane, or v2 product generation.
5. Add focused tests for conservation, zero-size IDs, bucket boundaries,
   unauthorized/reserved access separation, corrupt/foreign metadata,
   no-write/no-growth diagnostics, and same-release recovery/replay. Update
   current docs/changelog. Do not bump versions, publish, deploy, or edit
   sibling repositories.

Return the exact API and integration example Canic should adopt, measurements
and limitations, targeted validation results, and whether the evidence supports
retaining the default or selecting a smaller policy. Keep actual Toko live
attribution and downstream adoption explicitly separate from fixture evidence.
```
