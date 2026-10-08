# Current handoff — 2026-10-08

Current work refreshes the embedded allocation peer after the selected runtime
dependencies changed. Earlier work qualified the Blob/Memory integration under
[#444](https://github.com/dragginzgame/canic/issues/444) and
[#459](https://github.com/dragginzgame/canic/issues/459). Earlier Host/Backup
cleanup and fixture work remain intact. The maintainer committed the pinning repair
at `95038e1a3` and advanced the root lock at `2140b0ba0`. The subsequent embedded
peer refresh and its handoff/changelog updates remain uncommitted.
Workspace packages remain `0.110.52`;
the existing `0.110.53` changelog draft is extended rather than allocating another
patch. Earlier Backup adoption, fixture refresh and incoming dependency changes
are preserved. This handoff describes evidence; GitHub issues own follow-up work.

## Current fixture refresh

The root lock now selects Memory 0.31.2, Timers 0.14.13 and Testkit 0.21.3;
its SHA-256 is `a2526c2e27312c7572670b58e410c7fbeb77cb6d79c3b990f46ec4779786038a`.
The reported test preflight correctly rejected the earlier embedded allocation
peer after these inputs changed. Explicit locked/offline refresh regenerated the
Wasm and provenance together. The read-only verifier passes, as does the exact
managed Component Group child lifecycle case (155.23s; governed invocation 286s,
including inventory and runner completion). This covers initial/on-demand child
allocation, typed admission refusals, timer restoration, authority delivery and
same-release fencing. The governed runner completed its owned scratch cleanup.

The new peer SHA-256 is
`03c973cc5314bb98910b3d0502411a7cc3137105859c5aeba99e8fd88624dd5c`;
provenance SHA-256 is
`4e1804cb62a5d759baa12c5d5816945d1894fe0e6bacefc0050fb64a21ad7e4d`.
Evidence is retained under `target/review-validation/embedded-peer-refresh-20261008/`
and `target/test-runs/20261008T074322Z-71047.tTyPL9/`. Manifests and lockfiles
remain unchanged by the repair. No broad validation ran; the qualification below
is bound to the preceding selected graph and must not be relabelled as current.

## Earlier integration inputs and qualification

The root catalog requires Host Tooling 0.4.6 for all four packages and Backup
0.5.3 or later. The October 7 lock selected Host 0.4.6, Backup 0.5.4, Query 0.48.1,
Testkit 0.21.2, Memory 0.31.1, Metrics 0.2.9 and Timers 0.14.12. Backup, Query and
Testkit advanced independently after the preceding qualification; those incoming
selections were not reverted or attributed to this cleanup.

- Manifest SHA-256:
  `7305e716d7880a0cc5d2979cfa3290ed4b85f40a4addfe947863ea74f03902b7`.
- Lock SHA-256:
  `5bc37eeca0f7adc56ce9076a24e9e4ea345deb001724516b50225668864975f9`.
- Focused Linux evidence and exact input bindings:
  `target/review-validation/host-fingerprint-cleanup-20261007/`.

On this graph, the executable fingerprint case, four build-input snapshot cases,
seven tool installation cases and the complete build-cache repeat/tampered-output
case pass. Warning-denied Clippy passes for Host, Backup and CLI with all targets
and features. Six representation, 25 current-protocol and two lock-seed cases
also pass; the existing manual query measurement remains ignored.
Locked cache preparation succeeds. All 11 Backup artifact,
76 persistence and 33 restore-runner tests pass against selected Backup 0.5.4.
Independent embedded-peer verification passes without changing its executable
bytes or provenance.
The exact CLI live-create refusal passes before filesystem or ICP effects.
These checks do not establish native Canic macOS or a broad validation receipt.

## October 7 Blob and ledger qualification

The adapter pins published Blob 0.17.1; all three independent locks now select
Memory 0.31.1, Timers 0.14.12 and Metrics 0.2.9, matching the October 7 root graph. Only
those four selected packages changed in each lock. Both complete managed Fast
builds pass. Artifact-manifest-derived graphs verify exact Wasm hashes and one
runtime identity across all eight application/infrastructure graphs. Dedicated
and embedded PocketIC proofs pass complete service Candid parity, installation,
caller/tenant refusals, lost mutation-response reconciliation, aggregate metrics,
same-release restoration and repeated current-instance recovery.

The reported dependency-pins failure came from the exception still naming Blob
0.15.2 after the adapter adopted 0.17.1. The matching exact-selector exception and
its current lifecycle/protocol rationale are repaired; `make dependency-pins-gate`
passes with Cargo inheritance checks. Manifests, selected versions and all
lockfiles remain unchanged. This focused gate is not a new broad validation receipt.

The exact production Root initializer case passes in 192.98s (328s governed
invocation). It covers held allocation, exact 16,384-byte target binding,
wrong-target/caller refusal, interruption, a discarded binding ingress reply,
effect-free replay and application restoration. It does not discard a management
install callback. Fifteen owning ledger tests, four memory ABI guards and the
exact public memory Candid-shape case pass on Memory 0.31.1. Core native and Wasm
and adapter Wasm strict Clippy pass. Both consumer shells pass native strict
Clippy; four initializer, one install-intent and two Host encoder cases pass.
Logs, trees, exact input
bindings and the temporary consumer proof remain under
`target/review-validation/blob-memory-qualification-20261007/`.

The adapter/dedicated/embedded lock SHA-256 values are respectively
`f373ab1664b8f63996c9f29764a7f08ece8aad7c4ac36583ff16897a88848a2a`,
`2f263b4b205160ab4954c6570d1b202b5761d20548f55480558b3a14e7913724` and
`6749a268db95f0fd2c128b2071070846d771ced3c2f8a08c4a153f52f088a013`.
The published Blob archive binds `7c41e3a90996157312aa40985a9861f4c03ca35e` and
matches its registry checksum; exact upstream CI `37640042157` passes Linux and
both macOS architectures. This batch qualifies Canic on Linux, not native Canic
macOS or a live provider.

## Host consolidation

[#458](https://github.com/dragginzgame/canic/issues/458) owns the accepted shared
Host adoption. Release representation checks now use exact streaming gzip
comparison, avoiding a second complete decoded Wasm. Store publication uses the
shared ordered chunk/whole-upload identities without an intermediate copied
chunk collection. Generated lock-seed JSON uses typed durable publication within
its existing 4 KiB budget. Digest identities, request bounds/order, pretty JSON,
permissions, typed causes and lock-before-seed recovery remain Canic-owned.

Build inputs and local executable fingerprints now use the same shared descriptor
reader. Two private hashing loops and the installer's duplicate artifact-error
conversion are removed. Regular-file/no-follow admission, the executable's
512 MiB bound, exact SHA-256 values and native I/O causes remain. Regressions
include directories, symlinks, missing files, oversized executables, input drift,
cache repeat, output tampering and rejected tool staging.

The earlier adoption also removed copied durable-file, process-capture and codec
mechanics. Direct durable consumers import `ic-host-fs`; response-error consumers
match `IcpJsonResponseError::Envelope`. Ordinary Fleet journal publication retains
its pre-serialization/recovery contract. Paid ICP process custody and installer
archive/bundle admission remain local while their shared prerequisites are
unqualified; [Host #5](https://github.com/dragginzgame/ic-host-tooling/issues/5)
owns the process gap. [Host #20](https://github.com/dragginzgame/ic-host-tooling/issues/20)
owns staged executable admission after closing the writer and before publication;
the retained-writer Linux `ETXTBSY` probe passes. Do not mechanically replace
these stronger contracts.

The preceding Host 0.4.6 graph passed 47 selected Host tests, 11 Backup artifact,
76 persistence and 33 restore-runner tests, strict package-scoped Clippy, the CLI
live-create guard and read-only peer verification. Its exact lock was
`f94cd1280398bbfef86a9692f3164a5f9487b7a62ef07fa6b61b6cd5d2db42cd`; evidence is
`target/review-validation/host-046-adoption-20261007/`, not proof of newly selected
dependencies. Published Host archives bind commit
`0fb05f9e18f032425188d68e1d69317a0f0127d5`; exact upstream CI `37648086908` passes
Linux, Intel macOS, Apple Silicon macOS and MSRV. Canic macOS remains unqualified.

## Backup adoption and remaining boundary

The artifact foundation is implemented under
[#488](https://github.com/dragginzgame/canic/issues/488): IC Backup owns checksums,
relative no-follow traversal and private create-new staging. Canic's 367-line
filesystem engine and stream-copy helper are deleted. Digest bytes and manifest
shape remain; shared traversal errors replace local entry/platform variants
through `ArtifactChecksumError::Artifact`, preserving native I/O causes. This
public error hard cut joins the existing minor-boundary requirement.

Canic retains descriptor publication barriers, custody and same-operation
interruption recovery. The remaining checked directory-framing adoption belongs
to [#490](https://github.com/dragginzgame/canic/issues/490) and
[IC Backup #26](https://github.com/dragginzgame/ic-backup/issues/26); it requires a
published, qualified API. A dirty sibling implementation is not an adopted
dependency. Live capture remains unavailable under
[#394](https://github.com/dragginzgame/canic/issues/394); artifact adoption does not
provide complete live backup/restore runners or transport.

## Release and fixture repairs

[#486](https://github.com/dragginzgame/canic/issues/486) owns release cache
preparation: locked fetch precedes the offline gate without changing caller
offline policy. [#450](https://github.com/dragginzgame/canic/issues/450) owns the
reported fixture/compiler repairs: private Make fixtures include shared records
and clear inherited release assignments; Testkit workspace discovery explicitly
requires its fallible result; ordinary validation refuses stale embedded bytes.
These repairs are already in the maintainer-owned base, except the retained peer
Wasm/provenance refresh. Original failure fixtures and logs remain intact.

The October 7 peer SHA-256 was
`fb77457871de89db5c62c92608dfe54bdf1b02110f76f56437ef3184c80eeeef`.
The exact managed Component lifecycle case passed in 150.59s after explicit
refresh, with real installation/restoration and authorization refusals. Its
evidence is bound to the older input graph under
`target/review-validation/embedded-peer-refresh-20261007/` and
`target/test-runs/20261007T151207Z-63666.kocik3/`. Later read-only verification
must not be described as a new PocketIC journey or rewrite the fixture implicitly.

## Publication boundary and accepted scope

The Canic-owned managed graph qualification boundary under #444 was cleared on
the October 7 selected graph; its proofs do not qualify the subsequently advanced root lock.
the issue retains its application-owned Toko acceptance. Read-only source
inspection finds no required-init flag or production binding caller, and its
older lock is not this batch's managed evidence. No sibling edit or build ran.
Downstream application acceptance is not an additional Canic release gate.

The whole open release batch is **not push/publication ready**. Public/stable hard cuts still
require the human-owned minor decision recorded in
[#459](https://github.com/dragginzgame/canic/issues/459). Passing focused checks
does not resolve these boundaries. Package versions and release receipts remain
unchanged; no commit, release, publication or live IC effect ran for this cleanup.
The ledger compiler repair is complete. The old closeout report qualifies its
frozen .42 checkpoint; current FR1 still exposes observation/assessment without
the accepted release-to-capacity CLI/execution outcome. The exact current
0.110 closeout and accepted scope disposition remain human-owned.

The [0.110 design](../design/0.110-fleet-runtime-contraction/0.110-design.md) and
[status](../design/0.110-fleet-runtime-contraction/status.md) retain accepted
sequencing; historical checkpoints are evidence, not permission to revive
predecessor state or cross a minor boundary. Further deployment simplification
has its own owner in [#452](https://github.com/dragginzgame/canic/issues/452).
External checkouts remain read-only; do not sweep retained build or paid-operation
evidence to make a release look clean.

## Earlier integration evidence

The imported Toko timing diagnostics were qualified against isolated published
.50 source `718531020dd0311b682d2539c3178627549b2486` in
`.canic/local-work/toko-feedback-20261002/`: 88 Host import, seven timing and
15 CLI receipt tests, scoped Clippy and the exact signed-handoff PocketIC case
(306.33s). Root/Coordinator queries and reconciliation share bounded invocation
receipts; child attribution and incomplete cancellation/panic observations remain.
Those logs qualify the isolated source, not today's combined graph.

Preserve `.canic/local-work/`, `.canic/incident-repairs/canic188/` and retained
validation logs. The withdrawn incident repair's
[historical decision](../audits/release-lines/supporting/0.110-fleet-runtime-contraction/canic188-issued-import-recovery.md)
is evidence only. Earlier handoffs remain in Git history and the
[existing archive](archive/2026-10-01-prior-fleet-handoffs.md); the full pre-cleanup
handoff is also retained with this cleanup's local review artifacts. GitHub
issues and owning design/audit documents retain decisions without a second local
issue queue.
