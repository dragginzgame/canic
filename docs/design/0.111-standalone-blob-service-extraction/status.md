# Blob Extraction Implementation Status

Date: 2026-10-07

The current Canic-owned adapter selects published Blob Storage 0.17.1. Its
independent workspace and both consumer lockfiles select Memory 0.31.1,
Timers 0.14.12 and arithmetic-only Metrics 0.2.9, matching Canic's runtime stack.
Timers directly re-exports Metrics' summary type. There are no cross-version
conversion adapters, application overrides or copied storage implementations.
[Canic#444](https://github.com/dragginzgame/canic/issues/444) owns the downstream
acceptance record; the [design](0.111-design.md) and
[current handoff](../../status/current.md) describe implementation and evidence.

Canic remains the sole lifecycle and default memory/timer runtime owner.
`mount!` contributes endpoints and application-selected memory requests;
`lifecycle::install` and `restore` run synchronously after Canic restoration.
The embedded application composes its durable counter and service rows into its
sole application sampler. `canister!` assembles those same parts for the dedicated
shell. DTO imports remain passive. The independent service owns its storage,
authority, provider, accounting and recovery contracts. Neither the adapter nor
the temporary service proof enters Canic's default release/test dependency graph.
The generic production initialization probe remains framework-owned coverage.

Root now supports production application initialization. A Component Spec with
`application_init_required = true` holds an allocated Component before installing
it. The controller binds 1–16,384 opaque bytes to the exact allocated Principal
and retained member operation. Root atomically retains and charges that binding;
its hash participates in the install intent. Ordinary and Component Group
installation deliver those exact inner bytes inside Canic's protected envelope.
Changed targets or bytes are refused; identical replay is effect-free, including
after completion. Host owns the bounded command encoder without knowing the
application schema. The application remains responsible for creating its input
after observing the real target. A fixture callback is test support only.

Complete release-bound dedicated and embedded Fast builds pass. Single-role
compilation does not embed the required release-build identity and therefore does
not qualify managed installation. Four native initializer boundary/recovery cases,
one install-intent reservation/replay case and two Host encoding cases pass.
Current Core native all-target/all-feature
and Wasm library Clippy, plus adapter Wasm library Clippy, pass with warnings
denied. Both consumer shells pass native scoped lint; authoritative Wasm qualification
belongs to the managed builds. Direct consumer Wasm Clippy correctly refuses
without validated build authority; no admission flag or guard was bypassed.
Both current-stack consumer forms pass PocketIC installation, typed caller and
tenant refusals, discarded mutation-response reconciliation, aggregate metrics,
same-release restoration and repeated current-instance recovery. Their complete
blob endpoint schemas match structurally. Eight complete normal Wasm graphs
(two application roles and their six infrastructure roles) each contain exactly
one Memory, Timers and Metrics identity. Evidence is retained in
`target/review-validation/blob-memory-qualification-20261007/consumer-pocketic.log`,
its per-package tree files and `runtime-identities.json`. That inventory derives
its graphs from the built application/infrastructure manifests and verifies
their exact artifact hashes. All three independent locks are refreshed; the
primary manifest/lock and earlier dirty work are unchanged.

The governed production Root case passes on this current graph in 192.98s.
It verifies held
allocation, wrong-target and caller refusals, controller allocation inspection,
Root interruption before binding, a discarded binding-command ingress reply,
exact full-bound 16,384-byte delivery, effect-free binding replay and synchronous
same-release application restoration. Its retained log is
`target/review-validation/blob-memory-qualification-20261007/production-pocketic.log`;
the exact governed runner finishes in 328s with normal server/scratch cleanup.
Root upgrade during installation quiesces pending IC management callbacks; this
case does not drop an install callback. Native tests separately qualify retained
install-intent bytes/hash and renewal after interruption. An actual uncertain
management install-response interruption is therefore not claimed as managed
execution evidence. These are local PocketIC results, not live deployment proof.

Read-only Toko source inspection still finds a consumer-owned shell using the
reusable adapter. Its retained lock includes older Blob Storage 0.15.2 and
Memory 0.28.4 alongside Memory 0.31.1; no current complete Toko managed graph is
qualified by the Canic proofs above. Its blob
Component Spec still lacks the required-init flag, and no application-owned
initializer producer or production binding caller was found in the inspected
source. Downstream acceptance needs those application changes and its own managed
execution. No Toko files were changed, and no Toko build or installation ran.
Independent preparer updates do not update the managed adapter or supply Root
arguments. Live uploads, reads and provider readiness remain unqualified.

The extraction removes the former embedded service, command group, billing Medic
option, stable allocations and blob-specific default test/CI lane. Published
historical evidence retains its recorded source/graph scope in earlier handoffs;
it does not qualify this graph. The temporary consumer execution proof does not
restore an engine suite or a permanent blob runner. Decoder-budget feedback stays
in [ic-blob-storage#7](https://github.com/dragginzgame/ic-blob-storage/issues/7).

These configuration, wire and stable-record changes are a pre-1.0 hard cut,
requiring clean reinstall across releases while retaining same-release recovery.
The maintainer-selected 0.110.53 draft remains unchanged; its existing human-owned
minor/release boundaries remain in
[#459](https://github.com/dragginzgame/canic/issues/459). No commit, version bump,
publication, deployment, broad gate or sibling mutation ran. Source removal does
not delete provider data, end billing or settle existing cycle/storage liabilities.
