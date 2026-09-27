# Completed approval retirement and clean reinstall

## Requirement

The maintainer requires pre-1.0 clean reinstall from explicit inventory, current
controller authority and a qualified replacement build. Retain selected physical
IDs and controlled cycles; discard application and framework state. Completed
historical deployments must not require their original CLI, executable schema,
application migration or repair. Only unfinished paid effects need reconciliation.

This is the accepted current-minor correction in the
[design amendment](../../../../design/0.110-fleet-runtime-contraction/0.110-design.md#clean-reinstall-correction--accepted-2026-09-27).
It is not permission to ignore an interrupted current operation or lose cycles.

## Confirmed Toko evidence

Read-only source: `../toko-miner/docs/upstream/canic.md`, CANIC-184, and
`../toko-miner/.canic/fleet-ensure/staging/toko-miner-staging-001/`.

- Preparation release: `0.110.43`.
- Preparation review: `433a4f62da63e20b69f453eff7260a31640be724bf78817b49661733c8dc2263`.
- Completed publication: `89baaffb69873bb1782fd0124c6e9c597bb13d909dbf8c978a9d0016ec5d33f8`.
- Current operation: `e79d8265c3d6e3a15bb84354c9eda3b7fb741867ad8e5b56aef144e40bf327eb`.
- Current journal is `converged`; retained state contains 27 distinct physical IDs.

An offline comparison verified the publication object's SHA-256, its archived
replacement object's SHA-256, and equality of the consumed preparation/journal
with those embedded in the replacement. It also matched the source-document
binding and preparation review digest. No source file was modified and no IC
request was made. This verifies the selection diagnosis, not live estate custody
or a successful new deployment.

## Implemented boundary

Completed publication identity can be inspected without decoding its archived
replacement into the current `FleetEnsurePlan`. Exact consumed preparation is
excluded before its current-release admission check. Unrelated CLI selection
skips committed reset authority before loading its executable plan. Pending
preparation, pending publication and unrelated current approvals retain their
existing checks.

An explicit new reinstall retires consumed local approvals only after the active
journal reports convergence. Original files are retained by content hash, with a
`completed-authority-history/<review_sha256>.json` index. A removal intent is
durable before any active approval file is removed. Restart verifies the entire
archive and every remaining active file before finishing; it refuses changed
bytes. Current plan, journal, state, terminal accounting and canister state are
not rewritten by this transaction. Publication completion alone cannot retire an
unfinished replacement's paid work.

Forty distinct focused host regressions pass across completed-operation,
publication/retirement and retained-selection filters (the filters overlap).
Native regression covers unreadable retired executable payloads, consumed
approvals from another CLI release, interrupted publication, unrelated pending
preparation, every local removal interruption boundary, byte preservation and
effect-free retirement replay. Host/CLI warning-denied Clippy passes. These are
local state tests; they do not simulate successful canister management effects.

## Completion-envelope implementation

The completion-envelope selector now precedes executable decoding in readiness,
in-progress lookup and unrelated apply-digest lookup. It rejects a converged label
with an explicitly unresolved effect; a distinct newly reviewed plan remains
with its current owner. An explicit reinstall archives opaque operation files
and available shared publication objects under
`.canic/fleet-ensure/history/<environment>/<fleet>/`. The manifest records missing
historical objects without requiring old artifact qualification. Objects and the
manifest use atomic create-only publication; no source file is rewritten.

Verification: eight selector/archive regressions and six readiness regressions
pass. They cover opaque completed execution payloads, unresolved effects,
distinct pending plans, interrupted archive capture, immutable replay, corrupt
archive rejection, symlink rejection and unavailable old publication blobs. The
explicit read-only native audit also passed against the supplied Toko directory:
191 operation files/publication objects were archived into Canic-owned scratch,
with current operation `e79d8265c3d6e3a15bb84354c9eda3b7fb741867ad8e5b56aef144e40bf327eb`
and unchanged source plan/journal/state. It ran no signer, management request or
build of an old release. The initial test filter selected zero tests; the exact
fully qualified audit name was then run and passed one test. No zero-test run is
counted as evidence.
Both existing consumed-approval/retirement regressions passed as well. Scoped
host/CLI warning-denied Clippy passed in 27.66 seconds. Workspace validation and
PocketIC were not run for these local evidence changes.

## Connected current reset

Completed-Fleet `fleet generate` now compiles current artifacts without consulting
predecessor runtime contracts. `fleet ensure --reinstall` retires the completed
operation and reviews certified custody, current typed infrastructure setup,
Root-owned child clearing and workload convergence. The frozen current selection
persists through retries. Convergence identity binds its unique setup review, so
a second reset of identical artifacts receives distinct operation/payment identity.

Current Root import accepts running Root-owned children: stop intent precedes
controller normalization, the actual stopped management version is retained, and
uninstall history remains exact. The isolated real import case passed both direct
operator and Root-owned cases in 203.53 seconds. Supplied setup now certifies held
child custody without inventing management versions or balances; the new Root
samples them before import. Infrastructure receives reviewed Ledger top-ups before
stop/wipe, and an external operator Ledger credit can satisfy the original review
without increasing its debit authority.

A separate retirement intent archives all original active bytes and removes their
execution ownership while preserving the lock inode. Every removal boundary can
resume; changed bytes, symlinked parents and unfinished paid work reject. Final
current convergence retains exact state/journal/accounting evidence before
publishing completion, permitting the remaining local write and immediate replay
without resolving a signer or contacting the IC.

## Qualification status

Thirteen native selection/archive/retirement regressions pass, including selection
of a `.38`-shaped completed envelope without desired decoding. One external-evidence
audit remains explicitly ignored in that filter. The previously recorded read-only
Toko audit remains evidence for archive correctness only.

The normal-route real PocketIC case passes a changed-build reset followed by a
same-build reset on the existing small estate. It checks lost install replies on
both resets, distinct operation identities, exact retained IDs, cleared modules
and stable memory, exact Ledger debit, bounded native loss, interrupted final
publication and replay with an unavailable IC executable. Test duration: 197.99s;
complete targeted runner: 215s. This is a correctness result on a small estate,
not a full-suite speedup or a live Toko deployment result.

Successful log:
`target/test-runs/20260927T193921Z-56353.s4QDba/1.log`.
The existing native generation/recovery omnibus also passes (1.42s test), including
bootstrap funding ordering, version advancement and operator-credit admission.
All 27 focused Fleet CLI parser/report tests pass in 0.05s. The first CLI
invocation selected the executable wrapper and ran zero tests; only the corrected
library invocation counts as evidence. Final scoped host/CLI/internal-test
warning-denied Clippy passes in 7.04s. Scoped formatting and whitespace checks pass.

Two earlier connected attempts are retained as failure evidence. The first rejected
an insufficient fixture operator reserve before any reset effect
(`target/test-runs/20260927T191738Z-20690.Ws2kd3/1.log`). The second completed child
clearing but the test called PocketIC's stable-memory read API on an empty canister,
which that API rejects (`target/test-runs/20260927T193601Z-43871.FicmOd/1.log`). The
fixture now provisions sufficient Ledger credit and verifies zero stable-memory
size through management status. Production debit limits were not relaxed.

No full workspace gate, version transaction, commit, publication, live IC effect
or sibling mutation was performed. The complete accepted `.45` batch and changelog
are ready for maintainer review and the chosen release gate. Package versions
remain `.44`; changes remain uncommitted.

## Push-readiness follow-up

The local release-integrity guard caught an internal suite entrypoint using the
`governed_pocketic_` prefix reserved for host ignored tests. The internal entrypoint
is now `pic::governed_suite::governed_internal_pocketic_suite`; compile, execution,
worker and targeted runner selections use that same identifier. No case membership,
ordering, deployment behavior or assertion changed.

The release-integrity guard, workspace-runner regression, isolated-worker regression,
scoped ShellCheck with repository exclusions, formatting and whitespace checks pass.
The complete command plan resolves with separate internal and host filters; plan
resolution executes no suite. Local `.45` release-notes preflight passes, Canic
package versions remain `.44`, and `ic-testkit 0.10.1` / `icydb 0.261.12` agree with
the lockfile. This check does not establish remote branch/tag state or full-release
validation. Changes, including new source files, still need the maintainer's commit
before the normal release command.

The targeted compiled registry/runner check passes all four native tests, with the
two real PocketIC entrypoints ignored. It confirms the renamed entrypoint exists,
case partition/order remains exact and failure stops subsequent cases. No broad
PocketIC suite was rerun for this identifier-only correction.

## Maintainer ordinary-gate failure and correction

The run at `target/test-runs/20260927T202301Z-4656.I9OKSf/1.log` passed 1103 host
tests and failed three. The completion-envelope preflight preceded pending local
publication inspection, which could encounter a partly replaced document set.
Pending publication is now selected first and preserves its exact review/plan
recovery diagnostic. Existing crash-boundary tests exercise all four publication
checkpoints without rewriting records during preflight.

The consumed-publication fixture now installs its current replacement documents
before asserting that its completed publication consumed the opaque preparation.
The unreadable-readiness assertion checks the exact decode error and journal path
from the earlier retained-contract preflight. Both retain their no-mutation and
no-network requirements.

All 32 tests across publication/retirement, readiness, retained-contract and
operation-selection groups pass; the explicitly external audit remains ignored.
Scoped host library/test Clippy with warnings denied passes in 58.82s. Formatting
and whitespace checks pass. This correction stays in `.45`; it adds no predecessor
execution adapter and does not rerun the complete gate or PocketIC.
