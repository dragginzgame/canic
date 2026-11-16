# Retained Root executable

`root-single-upgrade.wasm` is the retained pre-0.204 experimental greeting Root
used before serial-upgrade implementation. SHA-256:
`e4a4127abd525f59378503dcdb98fcb13f8eb9895673d4d8a4dea41d85e24296`.

The corresponding single-slot source and endpoint contract are inspected at
`v0.203.1` (`a2ecaa3e0f278637db76d6b5430975491b561617`). This is a retained
binary witness, not a claim that rebuilding that tag reproduces these bytes.
It exports controller-only `observe` (ID 21) and unit `observe_upgrade` (ID 25),
has no `observe_custody`, and has no fault-injection export. The slot cannot be
reset or replaced. Qualification installs these exact bytes and exercises the
transition through the current CLI. The artifact is retained here so the normal
Root suite does not depend on an ignored test or an untracked local build cache.

The Host's narrowly admitted predecessor hash names this contract. It does not
authorize other old binaries, query-based custody, endpoint fallback or rewriting
old approvals. The replacement must satisfy the current full-custody postflight.

## One-child history predecessor

`root-one-child-history.wasm` is the retained Root from the 0.204.1 release
qualification at `57a1e2e74d373088ed336210cfa719942edc91ac`. SHA-256:
`3de14eb8422bb99c57f9c63870523460fb9d7b3d4c5f0ef5f7a75cc0c1333c3a`.

It writes CUJ3 journals with immutable upgrade history and CUA2 artifact payloads
inside the original two allocations/three-bucket directory. It has no second child
or fault-injection endpoint. The two-child runtime fixture installs these exact
bytes, populates creation and serial upgrades, then verifies the full history
survives CUJ4 migration before admitting another child. This is a retained binary
witness, not a reproducible-build claim or another Host predecessor fallback.

## Store quiescence predecessor

`root-store-quiescence.wasm` is the normal Root built during the 0.209.0 release
gate, released at `e3e7be86301c377db8b064d2ef914f5cde936265`. SHA-256:
`f032cb36ce3599aa5141f7d6cf4673463720a634fd81aaf193ac120db590c785`.

It writes CUJ5 child/Store-maintenance journals and CUA3 artifact payloads. The
quiescence fixture creates real Closed and Stopped attempts in this executable,
upgrades to CUJ6, and verifies that all authority and stop evidence survive while
missing catalog-scan evidence stays absent. A still-closed Store can be scanned;
an already-stopped attempt cannot gain an invented commitment. This is retained
binary evidence, not a reproducible-build claim or a Host compatibility fallback.

## Single Store-maintenance predecessor

`root-store-single-maintenance.wasm` is the maintenance-candidate Root retained
from the 0.215.0 qualification, released at
`b7f39a3598cb84236a98320847245a47d4c79eca`. SHA-256:
`85f7081e96acb978617ac7cdac0262a4ab44d13ebda73b8fcfc7b1171d1c2573`.

It writes CUJ6 with one Store attempt and CUA3 artifact custody. The repeat-upgrade
fixture completes a real Store replacement with these bytes, upgrades Root, then
checks that the original outcome survives before admitting successors. The older
executable must refuse CUJ7 rather than discard history. This is retained binary
evidence, not a reproducible-build claim or a Host compatibility fallback.

## Positional child-storage predecessor

`root-positional-children.wasm` was retained on 2026-11-16 before replacing runtime
array access with identity-keyed collections. SHA-256:
`a00ce6fcac8687c9c350e317c53d2d653665c1c531639af9d07961ade96cc90d`.

Its 1,326,609 bytes match `target/root/slot-owner-root.json`, built from 0.216.4
plus the pending shared-slot-list cleanup. This is a qualified retained binary,
not the tagged release binary or a reproducible-build claim. It writes CUJ7/CUA3
in the existing checked leases and exports both child and complete Store custody.
The child witness compares every stable byte across pending/completed work and
both executable directions. The publication witness carries two children's Store
pins and upgrade histories through Host-driven Root replacement. No Host hash
fallback or new state format is introduced.

## Historical release-scan Root

`root-release-scan.wasm` was retained on 2026-11-16 before versioning Store scan
evidence. Its 1,453,680 bytes have SHA-256
`2f099c8d8d5bd9919963900b4002cf25ca50518bd38df9463ef35a19e595d24d`.
It is the `fault-injection` Root built by `test-store.sh preservation` from
Canic `37270cb` plus the pending Store 0.2.3 dependency adoption. The runtime
source was unchanged; this is retained executable evidence, not a tagged-release
or reproducible-build claim. It writes CUJ8/CUA4 and uses Store method 113 with
the metadata-v1 digest. Tests retain an actual partial scan, upgrade to CUJ9,
and verify its evidence and refusal survive; a downgrade from CUJ9 must fail.
The older-Store witness also starts its legacy scan before upgrading this owner.
No Host code-hash fallback is added.

## Pre-registry Store predecessor

`store-0.1.0.wasm` is the normal standalone Store retained from Canic's extraction
qualification and subsequent 0.215/0.216 gates. It was built from supplier commit
`541d911410dae0f38eefc371693024bf2068c55b`. Its 501,823 bytes have SHA-256
`e8a74f2155d91da7964525984c0d4e83d53d037f11190a71ce0c664b5933a3ad`, matching the
retained `target/evidence/store-extraction/supplier-build.json` and Store runner
artifact. This is an exact binary witness, not a reproducible-build claim.

The maintenance fixture initializes these bytes, creates shared releases, pins
and an incomplete upload, then replaces Store under sole Root control with the
selected registry executable. It reuses the current replacement/recovery proof,
including all stable bytes, callback loss, Root maintenance, explicit activation,
expired-operation recovery and rejection of stale activation. The fixture does
not add a Git build fallback, runtime migration API or live upgrade authorization.
