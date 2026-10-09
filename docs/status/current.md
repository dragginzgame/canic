# Current handoff — 2026-10-09

HEAD remains tagged v0.110.54 at
`c4c046f947b2b28f4342cbf6efe9221ba1ed5f70`; package versions remain
0.110.54. Changes are uncommitted and extend the undated 0.110.55 notes.
This handoff describes source and evidence; GitHub issues own acceptance.

## Current tooling hard cut

Shared Tooling 0.2.2 is adopted through its canonical exporter at reviewed
revision `ee48bb37c98c771e77b92fd891f0757d8c1c8b99`. Selected bytes/modes,
required companions and the task catalog are recorded in the snapshot.
No maintenance schedule is activated. Dependency exceptions now require exactly
one JSON document; the owning regression passes under
[#503](https://github.com/dragginzgame/canic/issues/503).

Direct Host packages select 0.9.2, Query 0.52.1 and Testkit 0.27.0.
Host repository release 0.9.2 publishes new package versions with unchanged
Rust source from 0.9.1. Canic adopts the same Shared Tooling 0.2.2 corrections.
The selected root lock is
`c0e30304b41c72c58db7a64e36c4cebc833445b33ed5490034509f77fc399ea2`.
It also selects Auth/protocol-types 0.2.1, Backup 0.11.1, Memory 0.33.0,
Metrics 0.3.1 and Timers 0.16.0. The latest owner releases remove the older
Host/Metrics generations. No local override is added. Query's 0.52.1 runtime
source is unchanged from 0.51.1; its Host helper edges moved to 0.9.
A final Query 0.52.1 patch arrived during the managed inventory compilation;
its Rust runtime source is also unchanged, and the actual case uses that selection.

[#498](https://github.com/dragginzgame/canic/issues/498) transfers PocketIC
provisioning, exact binary admission, protocol admission and command lifetime to
the published Testkit CLI. Canic's launcher, port polling, PID/resource observer,
Linux process-scanning cleanup and private binary/version pins are deleted.
The runner retains case ordering, compile/fixture barriers, complete server
output files, bounded failure tails and invocation scratch. Every worker attempt
owns a fresh server; cancellation waits for the canonical command owner before
releasing scratch. A real published-CLI fixture exposed and qualified the
cancellation correction. The cache observer now recognizes Canic's actual wrapper.

Explicit setup: `make install-tools install-testkit-server`. Offline admission:
`make tools-check testkit-server-check`. Shared IC setup owns five tools;
Testkit owns PocketIC. Old bundles and receipts are retained. Ordinary checks
never install. The thin selector refuses package/version/profile overrides.
The isolated-index hook selects original-checkout prepared tools; Canic's
historical symlink overlay remains in place under
[#454](https://github.com/dragginzgame/canic/issues/454).

Public managed fixtures and both internal test adapters consume only canonical
`IC_TESTKIT_POCKET_IC_URL`, without an old-variable fallback or hidden spawn.
The private startup error is replaced by `ic_testkit::pic::PocketIcStartupError`
([#501](https://github.com/dragginzgame/canic/issues/501)).

## Focused qualification

Current Blob/Memory and [#33](https://github.com/dragginzgame/canic/issues/33)
evidence: `target/review-validation/blob-memory-33-20261009/`.
Published Blob runtime/contracts 0.21.0 are un-yanked, their archives match registry
checksums and both identify `cea2d0f2740e0874a107a6a5e1c75a533b49e94b`.
All three independent locks share Memory 0.33.0, Timers 0.16.0 and Metrics 0.3.1.
Both complete managed Apps build with Rust 1.91; all eight manifest-bound normal
Wasm graphs have one runtime identity per family. Both 32-method Blob projections
match canonical 0.21 Candid strictly. Dedicated and embedded PocketIC cases pass,
including tenant-only separate capacity dimensions, unenrolled refusal, two
one-byte reservations, retained permissions, fenced restoration/exact resume and
application counter 42. Test harness/fixture production uses Rust 1.99 separately
from the minimum-version App artifacts. Provider upload, GC and funding are unqualified.

The root's 63 focused native Memory/admission/inventory/lifecycle/runtime-identity
and initializer cases pass, as do all seven owning lifecycle-boundary PocketIC
cases and the exact production target-bound initialization/recovery case
(discarded binding ingress reply, retained bytes and exact replay). Root's code
section is 8.93 MiB. This is not proof of a discarded management install callback.
[#443](https://github.com/dragginzgame/canic/issues/443) is closed: committed
constructor delegation now has its remaining actual Canic qualification.
Current CLI builds on Rust 1.91; adapter native library/tests and Wasm library
Clippy pass with warnings denied. Allocation peer provenance is refreshed for
this root lock; unchanged peer SHA-256:
`948756af6623cf8f1b3c39fb503c06034bd94b914ed0a8a89c035523a18abffb`.
Final read-only verification passes. Failed cache/socket/Candid probes and the
corrected capacity assertion retain their original logs and identities.

Earlier tooling qualification remains bound to root lock
`ccd253de2498028b21b1892c8dbfa7894f2ccdd97b8b38eb34b4c2a62e2f3a60`
(Backup 0.11.0). Its 78 Host ICP / 79 Backup persistence cases, strict affected
Clippy, Auth 202 cases, Rust 1.91 Host/Backup/full-feature facade checks and exact
managed Component Group child-lifecycle case passed. That case's complete output
is `target/test-runs/20261009T121633Z-3806957.qBWiNs/2.log`.
Canonical installers, catalog, hooks, worker cancellation/order/cache, Linux tool
setup/admission, governance, links and shell/CI lint also passed at those inputs.
Evidence and failed attempts: `target/review-validation/tooling-hard-cut-20261009/`.
Native macOS execution remains unrun. The optional exported installer fixture was
omitted after missing owner companions; its complete exact-revision owner test
passed. Feedback remains
[Shared Tooling #73](https://github.com/dragginzgame/shared-tooling/issues/73#issuecomment-6079827949).
No sibling files were mutated.

## Retained framework and adapter work

[#491](https://github.com/dragginzgame/canic/issues/491) delegates canonical Auth
encoding and eleven passive protocol declarations to published 0.1.14, preserving
Fleet bytes and checked roles through ops. Fallible hashes propagate through
configuration, issuance, signing and verification. Canic's key registry/policy
framing, verifier/cache, certification stores and durable session engine remain;
complete Auth adoption is WIP. Its 202 focused cases, Core Clippy and Rust 1.91
native/default/full-Wasm checks remain bound to root lock
`53c3a47e527dae3dd512ac401562cc52b1164141de88e11f835ae97463a34030`.
Evidence: `target/review-validation/ic-auth-adoption-20261009/`.
They retain their original identities. The same 202 owning encoding/wire/config
cases also pass on the current Auth/protocol-types 0.2.1 graph under the tooling
evidence directory; its runtime source is unchanged from 0.1.14. This does not
complete adoption of the remaining verifier/certification/session engines.

The earlier operator cleanup shares durable JSON publication, descriptor hashes
and foreground communication with Host, preserves typed failures and crash
barriers, skips unsupported Observatory capabilities, and uses the shared registry
observer and report digest helper. Owning issues are
[#458](https://github.com/dragginzgame/canic/issues/458),
[#439](https://github.com/dragginzgame/canic/issues/439),
[#462](https://github.com/dragginzgame/canic/issues/462) and
[#471](https://github.com/dragginzgame/canic/issues/471).
Independent checkpoint bounded-reader edits and their changelog were preserved;
#458 owns their behavioral acceptance. The tooling case does not qualify that
checkpoint behavior. Earlier artifact/process evidence retains its source identities under
`target/review-validation/host-088-consolidation-20261009/` and the earlier
shared-consolidation, report-digests and observatory-capabilities directories.

[#444](https://github.com/dragginzgame/canic/issues/444)'s independent Blob roots
now select published runtime/contracts 0.21.0 on the held Memory 0.33 graph;
local propagation and managed qualification above are complete. The adapter is
unpublished. Earlier 0.19.2/Memory 0.31 evidence and its prepared archive remain
under `target/review-validation/toko-miner-priorities-20261009/` and are not today's
registry-package proof. [Blob #41](https://github.com/dragginzgame/ic-blob-storage/issues/41)
is closed after compatible publication and exact archive verification.
An actual registry-only probe with published Canic 0.110.54 and Blob 0.21 plus the
intended Memory 0.33/Timers 0.16 resolves two Memory and Timers identities. Coherent
Canic library/CLI publication, adapter registry requirement/publication and
actual Toko Miner acceptance remain; no override hides that gap.

Published Canic 0.110.54 already delivers unchanged configuration-byte
preservation and exact Memory/Timers admission under
[#33](https://github.com/dragginzgame/canic/issues/33) and
[#34](https://github.com/dragginzgame/canic/issues/34).
Actual Toko Miner registry-only installation and recovery remain owned by
[Toko Miner #6](https://github.com/dragginzgame/toko-miner/issues/6).
The browser certificate/provider/GC journey, parent initialization and funding
acceptance remain distinct outcomes under #444, #493 and #494. Live Fleet backups
[#394](https://github.com/dragginzgame/canic/issues/394), early complete funding
forecasts [#37](https://github.com/dragginzgame/canic/issues/37) and progress detail
[#29](https://github.com/dragginzgame/canic/issues/29) remain separate owners.

## Delivery boundary

The tooling hard cut is qualified for Linux maintainer review. The whole accepted
release batch is not ready to push: coherent library/CLI and adapter publication, actual Toko Miner acceptance, and
[#459](https://github.com/dragginzgame/canic/issues/459)'s incompatible-patch/minor
boundary remain. The human must explicitly request and accept the exact 0.110
closeout audit before the next minor begins. No product-version or
release-validation receipt mutation, full workspace suite, staging, commit, push, package publication, deployment or
paid effect ran. The 0.110.55 draft is descriptive, not publication authority.

Preserve `.canic/local-work/`, `.canic/incident-repairs/canic188/`, old bundles,
archives, failed scratch and historical evidence. Predecessor state must not be
revived for compatibility; same-release recovery and cycle custody remain required.
