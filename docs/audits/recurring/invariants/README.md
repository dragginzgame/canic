# Auth Invariant Audit Suite

## Purpose

This suite defines the security invariants that must hold across the authentication and authorization stack.

Unlike refactor checklists or subsystem-specific audits, these audits are organized around properties that must remain true regardless of implementation details.

Use these audits to detect regressions caused by:

- auth refactors
- macro / DSL changes
- dispatcher changes
- token DTO changes
- trust-chain changes
- environment or runtime changes affecting caller identity
- introduction of alternate execution paths

## How to Use

- Run the full suite before every release.
- Re-run the affected invariant audits after any auth or access-control change.
- Treat each audit as a property check, not as architecture documentation.
- If a check fails, either restore the invariant or explicitly redesign the security model and update the suite.

## Required Report Sections

Each invariant report generated from this suite must include:

- `Report Preamble`
- `Audit Checklist` findings
- `Structural Hotspots` (real source files/modules/structs)
- `Hub Module Pressure` (import/coupling pressure table)
- `Early Warning Signals` (predictive architecture-decay indicators)
- `Dependency Fan-In Pressure` (module/type dependency fan-in table)
- `Risk Score` (`X / 10`, normalized scale)
- `Verification Readout` (`PASS`/`FAIL`/`BLOCKED`)
- `Follow-up Actions`

## Invariant Index

1. Subject-Caller Binding Invariant
2. Canonical Auth Boundary Invariant
3. Capability Scope Enforcement Invariant
4. Token Trust Chain Invariant
5. Expiry / Replay / Single-Use Invariant
6. Auth Abstraction Equivalence Invariant
7. Audience / Target Binding Invariant

## Invariant Map

Auth Pipeline Invariants

```text
Token Acceptance
|
|- Token Trust Chain
|
|- Subject-Caller Binding
|
|- Canonical Auth Boundary
|
|- Capability Scope Enforcement
|
|- Auth Abstraction Equivalence
|
|- Audience / Target Binding
|
`- Expiry / Replay / Freshness
```

## Token Acceptance Invariants

A token is accepted only if:

1. trust chain is valid
2. subject matches caller
3. audience matches runtime target
4. credential is fresh

## Audit Files

- [subject-caller-binding.md](subject-caller-binding.md)
- [canonical-auth-boundary.md](canonical-auth-boundary.md)
- [capability-scope-enforcement.md](capability-scope-enforcement.md)
- [token-trust-chain.md](token-trust-chain.md)
- [expiry-replay-single-use.md](expiry-replay-single-use.md)
- [auth-abstraction-equivalence.md](auth-abstraction-equivalence.md)
- [audience-target-binding.md](audience-target-binding.md)
