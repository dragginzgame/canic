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
