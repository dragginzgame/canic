# Current handoff — 2026-10-27

## Accepted scope and candidate

Continue the accepted 0.112 Blob/Wire hard cut using the
[closeout plan](../audits/working/0.112-push-readiness/plan.md) and its
[graph-specific evidence](../audits/working/0.112-push-readiness/owned-candid-cut.json).
Current inspected base: main at `c4f90ec5f`, with committed seven-target test repairs.
Own package versions remain 0.111.0; the open changelog target is 0.112.0.
The maintainer explicitly authorized staging, committing as `0.112.0`, running
`make release-minor`, and repairing/retrying failures until release completes.
The first attempt passed Clippy, invariant gates, ordinary workspace tests and
native feature checks, then stopped preparing the Fleet PocketIC artifact.
During validation the root lock changed from Backup 0.45.0 to 0.45.1; the latter
was not cached. Offline metadata reproduced the child failure (exit 101,
124 stderr bytes). Preserve that selection; locked fetching now supplies it and
offline metadata passes. No version preparation or release tag occurred.
The prior passes describe the earlier graph; the retry must qualify the new lock.

Required external Candid is accepted. Owned endpoints, initialization and
hash contracts use Wire; redundant external DTO migration remains intended.
A zero-Candid checkout is not an acceptance requirement. Production portable
Auth, live Backup and unrelated unstarted simplifications remain deferred.
Preserve completed simplifications and issue-batch evidence linked from the plan.

## Seven-target regression repair

The complete retained failure source is
target/test-runs/20261027T125830Z-396709.bw3IUE/2.log: eleven cases across seven targets.
Maximum Root/target admission requests exceeded native allocation accounting;
1 MiB now admits both while preserving the 16 KiB frame, node and depth caps.
Allocation-only diagnostic expansion passed; node-only and depth-only did not.
Four read-only Root authority/registry methods now have replay-policy entries.
The native authority hash golden includes an independent domain/payload check.
Startup conversion, schema-producing canister rlibs, and native timer custody
guards follow their current owners. The timer macro scanner distinguishes method
and qualified provider actions from ordinary resume arguments. Host fixtures use
raw hex responses and an argument-specific budget. Leaf serialization tests call
pure response construction; endpoint startup admission remains intact.

Focused passes: Control Plane library 351, Canic integration targets 11+3,
timer guard 15, replay policy 40, Host protocol 3 and Store 4, admission contract 8,
leaf 2. Core's initial library run passed 1331 cases with only the subsequently
fixed replay entry failing. The initial wider Host run was stopped after unrelated
temporary Cargo-consumer fixture failures while a nested Cargo metadata command
stalled in the sandbox; those failures were not fully diagnosed and the run is
not a full Host pass. Final affected Host tests use the governed scratch wrapper.
Strict all-feature library/test Clippy passes for Canic, Contracts, Core, Host and
leaf. Direct Cargo Wasm checking correctly refused the missing role-contract
admission; the governed offline canic build of standalone/app using leaf_probe's
config passes, including final Wasm/schema qualification. Embedded allocation-peer
verification passes without refresh. Formatting and diff checks pass.
Logs: /tmp/canic-seven-*.log. No commit, version change or full release gate.

## Current gzip repair

The maintainer requested one shared gzip method. IC Tooling already owns it in
ic-host-artifacts 0.69.0: artifact::encode_gzip/decode_gzip. Canic production and
Rust fixtures now delegate to that owner with one level-9 compression policy.
Remove direct flate2 from the Host dev catalog and root lock edge; retain upstream
transitive selections. The packaged CLI size fixture reuses qualified gzip bytes.
No upstream checkout or compression implementation was changed.

Issue #698’s remaining baseline and audit Root level-6 helpers are removed.
They now use the existing production in-memory and file adapters exposed through
canister_build; the file path retains durable publication. No new test-only
production API or encoder is introduced. The fixture byte/size/hash regression
and cache-input test pass (one existing opt-in case ignored), all 90 release-set
tests pass, and strict Host/Testing Internal library/test Clippy passes with
governed-pocketic-tests enabled. Logs: /tmp/canic-698-{fixtures-enabled,release-set,clippy}.log.
The initial default-feature filter selected zero tests and is not qualification.
No installed test or full release gate ran. Shared Tooling 0.26.23 is now adopted
from committed revision 48ec9794892da6f8e2812653ec16c5e8ea23df47 through its exporter,
completing #698’s snapshot requirement locally. The snapshot verifies 139 files.
Canic now selects its existing three formatter roots, derive sorting and manifest
order declaratively; the copied runner is removed. Hook scratch views explicitly
disable reuse. Routing, Shared driver, real fmt-check and hook tests pass, as do
scoped ShellCheck and diff checks. Hook installer fixtures now pass --consumer.
Current formatter reuse is conservatively disabled by frontend node_modules
symlinks; per-pass timing works and the complete check took about 11 seconds.
Logs: /tmp/canic-shared-02623-*.log. No issue closure, commit or release performed.

The two reported application digest failures reproduced with the default
backend. Enabling zlib-rs changed those digests again and exposed the same fragile
golden in infrastructure tests. Three digest goldens now use fixed gzip inputs;
compilation still qualifies each against its current raw Wasm. Other tests use
the shared live encoder. This preserves exact-byte identity without treating
upstream compression byte choices as a release-manifest serialization contract.
Six small gzip fixtures are under release_set/{application,infrastructure}/fixtures.

All 90 release-set tests pass with both tested backends. Another 75 owning tests
pass across artifact IO, generation, current protocol and build reuse; two existing
opt-in tests remain ignored. Strict Host library/test Clippy and scoped ShellCheck
pass. Logs: /tmp/canic-gzip-{default-final,zlib-final,callers,clippy-final}.log.
No full workspace or packaged-downstream execution was rerun. Production gzip
settings, stored retry bytes and custody/recovery rules remain unchanged.

## Selected dependencies and earlier qualification

Root and both independent Blob consumers select Platform/macros 0.7.0,
Blob/Contracts 0.51.0, Auth/protocol types 0.18.0, Memory/Storage 0.51.10,
Timers 0.31.10, Wire 0.7.18 and Metrics 0.16.3 where applicable.
Root Host/Tooling/Testkit is 0.69.0, Query 0.75.0 and Backup 0.45.0.
Shared Tooling: 0.26.23 at 48ec9794892da6f8e2812653ec16c5e8ea23df47.
Browser Platform: 0.1.1; SDK Core: 6.1.1. Earlier publication blockers are resolved.
No compatibility adapters or local dependency overrides were introduced.

Workspace all-target/all-feature warning-denied Clippy passed before these latest
repairs. Allocation-peer refresh and verification subsequently passed, preserving
all three locks at that point. Those results retain their recorded input scope.
Combined tests later exposed schema feature unification: registration now requires
the consumer's declared wire-schema feature as well as Canic support. All nine
failing consumers compile in the mixed-feature case; the explicit schema registry
test passes. Logs: /tmp/canic-schema-{affected,registration-test}.log.

The validation adapter's stale ordering was repaired and its regression passes.
Risk diagnostics now print advisory/package/patch details. The maintainer accepts
RUSTSEC-2026-0387 for the exact crypto-bigint 0.5.5 checksum in the risk inventory;
it remains a warning, not a release blocker. Identity-change rejection tests and
captured audit replay pass. Other vulnerabilities remain blocking. See the scoped
acceptance in docs/governance/ci-deployment.md; no crypto dependency was changed.

## Remaining scope and safety

External DTO consumer migration and its acceptance remain separate from adoption.
Earlier Root provisioning stopped on an input-stability failure; retirement
qualification and complete final-graph installed qualification remain unclaimed.
Earlier dedicated/embedded Blob passes, issue fixes and Wasm measurements apply
only to their recorded graphs; retain the structured evidence rather than relabel it.

Preserve endpoint authentication, explicit identities/controllers, bounded spending,
original retry bytes, uncertain-effect reconciliation, same-release recovery and
cycle conservation. The unqualified confirm_credit wrapper remains removed;
uncertain payments and replay obligations remain. Blob owns its service engine;
Canic owns mounting, caller context, lifecycle/memory composition and installation.
Storage retains serde_cbor 0.11.2 with nested u128 encoded as 16 big-endian bytes.
Wire carries native numbers; no Ciborium or fallback codec is adopted.
