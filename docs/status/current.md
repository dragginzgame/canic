# Current handoff — 2026-10-08

The maintainer reports 0.110.53 pushed. HEAD remains
`4c51a87c6a32397196bb3f65d064641194df10a5`; package versions remain 0.110.53.
The accepted sibling audit's first cleanup outcome is implemented in the
uncommitted worktree. Auth remains WIP. The next undated 0.110.54 changelog draft
records this outcome without authorizing a version or publication transaction.

The [frozen 15-sibling audit](../audits/reports/2026-10/2026-10-08/sibling-offload/01/report.md)
retains its original inputs and source-only verdict. The implementation evidence
below is newer. GitHub issues own acceptance and follow-up; this descriptive
handoff is not release authority.

## Implemented ownership convergence

[Host #458](https://github.com/dragginzgame/canic/issues/458): shared closed-writer
publication now owns single-executable replacement. Existing installer errors
retain admission, native I/O, publication-phase and failed-cleanup causes.
Immutable executable/library bundles retain their distinct Canic transaction.
Shared install reports own code-body/function/global limits; metrics keep complete
section payloads. Gzip uses the numeric API; Host's flate2 dependency is test-only.

[Backup #490](https://github.com/dragginzgame/canic/issues/490): checked upstream
relative-checksum framing owns path ordering and exact directory identities.
Canic keeps descriptor synchronization, manifest projection, no-replace publication
and interruption recovery. Typed path failures use existing public error variants.

[Shared Tooling #453](https://github.com/dragginzgame/canic/issues/453): 81 exact
selected exports adopt committed 0.1.23
`0ba0ad00ed94848e54ecc82629b6b7873b7284c0`. Canonical cache/disk mechanics replace
local copies; a thin Canic adapter retains classified compiler fallback.
`ic-tool-pins.awk` is included as an executable installer dependency.

[Progress #492](https://github.com/dragginzgame/canic/issues/492) and
[diagnostic custody #474](https://github.com/dragginzgame/canic/issues/474): ordinary
nested runner progress streams before EOF; worker prefix/line share one write.
Failure and interruption stop owned processes while retaining full raw scratch,
printing its path and preserving original status. Successful scratch clears.
Public qualification helpers borrow the caller-owned server through bounded
Testkit connect, eliminating three hidden starts without public API changes.

[Testkit #29](https://github.com/dragginzgame/ic-testkit/issues/29) now has its
output-file custody API in selected published registry 0.23.0, source
`59b1c1de924116752282eac48c6531dce159ccc9`. Exact-source native CI
[37772507706](https://github.com/dragginzgame/ic-testkit/actions/runs/37772507706)
was queued at review. Full Canic process-owner adoption and retained-output
qualification remain under #474/#484; API availability is not consumer adoption.

## Local readiness HTTP follow-up

[Host #458](https://github.com/dragginzgame/canic/issues/458): local status and
readiness now share reqwest for URL parsing, HTTP framing and body decoding.
Anonymous CBOR/Candid and ICP environment selection remain Canic-owned. Requests
use the selected origin without redirects/proxies or URL credentials, with a
30-second deadline. Chunked replies decode correctly; transport failures retain
the HTTP cause and native I/O kind through existing error variants.

The initial thirteen focused replica-query cases and warning-denied Host/CLI
Clippy passed on the independently selected Query 0.49.1 / Host 0.5.2 graph,
lock `fb0dc97742edb721c314e994a74afe6befe162bdd3722e4c076b7c1f595eb0ca`.
Its read-only peer verifier passed without regeneration. The maintainer then
completed selections of direct Host 0.6.0, Testkit 0.23.0, Memory 0.31.5,
Metrics 0.2.12, Timers 0.14.16 and Backup 0.7.0. Query 0.49.1 retains Host
Artifacts/FS 0.5.2 transitively; [Query #24](https://github.com/dragginzgame/ic-query/issues/24)
owns upstream convergence. No dependency or package version change was made by
this slice. The final manifest SHA-256 is
`0262a4034e380c1506096e9d85a143a407fcac5dfe6f94cd6260e86811c54d9a`;
lock SHA-256 is
`607405cc74f0f5ca687201bbb4535ccc9032f4b10c1bb729c7184c510dfef79b`.
The old Observatory error-construction fixture required the new `group_error`
field; its test-only initializer is corrected. All 13 HTTP and two Observatory
cases pass on that current graph, as does warning-denied Host/CLI Clippy.
Explicit final-graph refresh regenerated Wasm/provenance together and read-only
verification passes. Current peer SHA-256:
`f4d0087deefefbf181eed360971b9f70c1fa117a92c870a9af221b26976ad4a7`;
provenance SHA-256:
`21f2d47079cc9fb9d92fe3ab6bbf3746c98351d56ac3aa37ce93fb2ec4e0990b`.
The exact production initializer qualification passes in 174.02s (244s governed
invocation), retaining the same exact-byte/refusal/interruption/replay scope
described below. Complete case output is
`target/test-runs/20261008T120314Z-2342210.drPHiO/2.log`. Owned server cleanup
completed and successful `.tmp/test-runtime.ypy4Fr` cleared. Earlier journeys and
results remain on their recorded inputs.
The preceding attempt on a moving graph failed before initializer execution:
generated artifact metadata could not obtain offline Memory 0.31.5. Its full log
and `.tmp/test-runtime.1u56qy` remain retained; the owned server stopped.
Explicit locked cache preparation preceded final-graph qualification.
Focused evidence and exact source inputs are retained under
`target/review-validation/replica-http-cleanup-20261008/inputs.json`.

Fleet `write_current` already uses shared durable byte publication after fallible
serialization. That small projection stays: replacing it with streaming would
change pre-serialization refusal and typed errors without retiring a local file
engine. The owning issue records this inspected boundary rather than another
wrapper or publication framework.

## First cleanup inputs and Linux evidence

Independent root manifest/lock edits advanced during implementation and were
preserved. The first cleanup selected Host 0.5.2, Backup 0.7.0, Testkit 0.22.2, Memory 0.31.4,
Metrics 0.2.11, Timers 0.14.15 and Query 0.49.0. Query shares Host 0.5 identities;
the earlier root Host 0.4 generation is absent.

Manifest SHA-256:
`391a75ec3ee2e60b44ac225e8a91b49bc6fe739a14a6cebcb22d499ef645c2e8`.
Lock SHA-256:
`1d8847fbf1f8bd83a0ce74356546093619540dbf9fc98c77471af27dcf820719`.

On those first-cleanup inputs, 8 executable-publication, 7 Binaryen, 6 ic-wasm,
23 artifact-admission, 14 Backup artifact and 76 persistence cases pass.
Warning-denied Clippy passes for Canic, Host, Backup and CLI, all targets/features.
Focused shell fixtures qualify nested before-EOF progress, startup/test failure,
cancellation, original status, complete out-of-tail server bytes and owned cleanup.
Release/tool fixtures use stubs for Git effects. Snapshot/governance closure,
prepared tool checks, shell lint and dependency pin/inheritance checks pass.

Explicit embedded-peer refresh regenerated Wasm/provenance together on that
first-cleanup root lock. Wasm SHA-256:
`9c3de9b31340d4c52f4e5df886b4a1e01c365fced8a95460cbc69ab63e5567b1`;
provenance SHA-256:
`9ed6378c2dae40d96700febd9b66cf388d2c8cc2911b0f5fead2ca12b2abd029`.
The exact production Root initializer case passes in 101.31s (175s governed
invocation). It covers exact 16 KiB target bytes, wrong-target/caller refusal,
interruption, a discarded binding ingress reply and effect-free replay; it does
not discard a management install callback.
The independent read-only peer verifier also passes. Earlier input-bound tests
remain historical evidence, not current receipts.

## Blob adapter delivery boundary

[#444](https://github.com/dragginzgame/canic/issues/444): the adapter 0.1.0 package
candidate has registry Canic 0.110.53 requirements, license, README and bounded
contents. `cargo package --locked --offline --allow-dirty` verifies its normalized
registry manifest. Archive SHA-256:
`457942bb620ae7eaeffb2d467b0405e81749e582e580ea6abb0b8c9068b91abd`.
It is prepared, not registry-published.

Blob 0.18 was tagged but unpublished at verification. The adapter selects
published 0.17.2; its independent consumer locks select Memory 0.31.3, Timers
0.14.15 and Metrics 0.2.11. This separate selected deployment graph is not the
later root graph above. [Blob #27](https://github.com/dragginzgame/ic-blob-storage/issues/27#issuecomment-6057786245)
records the publication feedback.

Disposable Canic-owned consumers use the extracted verified candidate plus
registry Canic 0.110.53, without overrides or local framework paths. Both complete
managed builds pass. Eight artifact-manifest-bound Wasm graphs have exact hashes
and one selected facade/Core/Memory/Timers/Metrics/CDK identity each. Generated
infrastructure uses registry-cache paths verified against exact published archive
bytes/checksums. It does not supply an application checkout acceptance claim.
Dedicated/embedded PocketIC journeys pass with the final native helper
(Host 0.5.2/Testkit 0.22.2/Memory 0.31.4) and refreshed peer. Complete 32-method Candid parity, actual-service initialization,
caller/tenant refusal, discarded mutation reply reconciliation, metrics, same-release
restoration and repeated current-instance recovery are the owning proof scope.

## Review and publication boundary

Evidence, exact graphs and retained attempts live under
`target/review-validation/sibling-adoption-20261008/`. Failed invocation scratch
remains intact; successful owned processes stop and successful scratch clears.
No broad workspace gate, commit, staging, push, package publication or live
provider effect ran by the agent; incoming staged work was preserved. No sibling checkout was changed.

The bounded in-repository cleanup outcomes are ready for review on Linux. The
whole planned release batch is not publication-complete: native Canic
macOS acceptance remains separate/unrun, and #444 retains adapter publication and
actual application-owned Toko acceptance. Paid ICP custody, further Host reporting,
Backup transport/runners and testing-package consolidation are separate outcomes,
not implicitly completed by this batch. Auth is deliberately deferred.
The human-owned exact 0.110 closeout and #459 minor boundary remain in force;
passing focused tests does not accept that audit or start the next minor.

Preserve `.canic/local-work/`, `.canic/incident-repairs/canic188/` and retained
failure evidence. Earlier handoffs remain in Git history; prior historical paths
are not asserted to survive maintainer cleanup. Do not revive predecessor-state
compatibility or discard paid-operation custody to simplify the tree.
