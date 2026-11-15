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
