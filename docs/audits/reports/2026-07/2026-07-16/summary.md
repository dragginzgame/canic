# 2026-07-16 Audit Summary

## Scope

This run day closes the final non-deferred 0.92 finding by adding and executing
the dedicated secret scanner required by retained method
`CANIC-RELEASE-INTEGRITY-001/v1`, catches and fixes an unrelated
dependency-resolution side effect in the human release-version transaction,
completes the executable `v0.91.6` compatibility accounting, and closes the
0.92 release line at immutable `v0.92.12`. It also records the separately
accepted D14 auth-checkpoint follow-up, released in `v0.92.13`, without
rewriting that closeout, and records accepted working-tree D15 hardening for
the two final P2s.

## Result

[D12 dedicated secret scan](0.92-d12-dedicated-secret-scan.md) pins Gitleaks
8.30.1 and every supported installer archive digest in the repository tool
contract. The installer verifies the selected archive before extraction and
rejects a binary whose reported version differs from the pin.

The scan covers complete reachable Git history, uses the version-bound built-in
rules, fully redacts findings, retains no raw report, and executes in CI plus
the maintainer patch-release gate. Its initial run rejected 11 candidates. All
were confirmed generic-rule false positives in audit prose, audit method
identifiers, structured certificate construction, or stable-key test fixtures.
They are excluded only by exact historical finding fingerprints; no path or
rule is broadly allowed.

The admitted rerun reports zero findings. Unavailable or near-match versions,
environment or repository rule overrides, shallow history,
unexpected arguments, and scanner-operational failures reject
deterministically. All 18 changed or new D12 files also pass individual
candidate-file scans. This fixes `CANIC-092-RELEASE-003` without a waiver and
changes no runtime, public, serialized, stable-state, product-configuration,
package, Cargo dependency, or lockfile surface.

[D13 workspace-only release lock synchronization](0.92-d13-workspace-lock-sync.md)
records that the final 0.92.11 version bump re-resolved six external packages
despite having no direct dependency-declaration change. The released packages
are retained after locked/offline metadata and advisory validation; D13 does
not hide or reverse them. Future version bumps use Cargo's workspace-only
offline update, which a disposable 0.92.10-to-0.92.11 proof confirms changes
only workspace package versions and locks zero external packages.

The release transaction remains fail-closed and restores every version surface
after a later failure. Its integrity guard now requires the bounded update.
This fixes P2 `CANIC-092-RELEASE-005` without changing runtime, public,
serialized, stable-state, configuration, or dependency declarations.

[The `v0.91.6` compatibility accounting](0.92-v0916-compatibility-accounting.md)
then compares the published anchor to immutable `v0.92.11`. Independently
generated root Candid and the canonical Wasm-store DID are byte-identical;
production CLI, config, stable-state, backup/restore, and package-feature
owners are unchanged. A canister installed from `v0.91.6` Wasm upgrades to
`v0.92.11` in PocketIC while preserving its persisted environment, topology,
and state projection.

The accepted 0.92.7 provenance hard cut is also executed directly: an old
envelope lacking `transforms` is rejected by the current policy with
`policy.build_provenance.invalid_payload`, while a current envelope passes.
The documented migration remains rebuild/regenerate; no compatibility decoder
or alias is added. No new finding results from the accounting.

The [0.92 release-line closeout](../../../release-lines/0.92-closeout.md) binds
D13 and the compatibility evidence to `v0.92.12` at
`dd4d55df8a9c870707ecda62f91900df8c0f6c70`. Its explicit verdict is
`pass_with_limitations`: all 28 P1 findings are fixed, while three bounded P2
watchpoints remain deferred with recorded revisit conditions. There are no
waivers, blocked current runs, or unclassified compatibility deltas.

[D14 auth performance checkpoints](0.92-d14-auth-performance-checkpoints.md)
is an explicitly accepted post-closeout P2 slice. It adds stage-level
instruction checkpoints to root-proof provisioning and delegated-token
prepare, repair, cache, and full-verification paths. Focused auth tests,
instruction-audit regression proof, and strict targeted Clippy pass. This
fixes `CANIC-092-PERF-001` in released `v0.92.13` without rewriting the
immutable `v0.92.12` closeout evidence.

[D15 final P2 hardening](0.92-d15-final-p2-hardening.md) gives the four known
unmaintained transitives one exact advisory/package/checksum/immediate-owner
inventory. Pinned `cargo-audit` now fails CI and patch releases on any
vulnerability, warning drift, direct-dependency promotion, or introducer-set
change. Because the four packages still exist upstream, the dependency finding
moves from deferred to accepted limitation rather than being called fixed.

D15 also splits the three concrete auth/root-proof production hubs along
existing responsibilities. Delegated-token configuration, verification/cache,
typed-error projection, request admission, replay ownership, and chain-key
installation progress now have focused children. Every resulting production
owner is below 600 logical lines, while authorization, replay, proof, state,
diagnostic, public, and stable contracts remain unchanged. This fixes
`CANIC-092-COMPLEXITY-001` without a generic abstraction or duplicate flow.

## Live Ledger

- Retained methods attempted: 22 of 22.
- Valid active results: 22.
- Invalid active results: 0; superseded v1 attempts remain historical.
- Mandatory traces: current reruns 10 pass and 0 fail; frozen Phase C aggregate
  remains historical.
- Unresolved findings: 3 (0 P1 and 3 P2), all explicitly deferred watchpoints.
- Required partial or blocked current runs: 0.
- Final finding index: 43 canonical findings, 40 fixed and 3 deferred P2
  watchpoints; `CANIC-092-RELEASE-005` is fixed in released `v0.92.12`.
- `v0.91.6` contract: complete with explicit source/provenance hard cuts and
  no unclassified compatibility delta.
- Closeout verdict: `pass_with_limitations`.
- Post-closeout `v0.92.13` ledger: 41 fixed and 2 deferred P2 watchpoints; the
  immutable `v0.92.12` closeout ledger above remains 40 fixed and 3 deferred.
- Working-tree D15 ledger: 42 fixed, 1 accepted external P2 limitation, zero
  deferred, and zero blocked. The live verdict remains
  `pass_with_limitations`.

## Validation

- Checksum-bound Gitleaks 8.30.1 install and reported-version check: pass.
- Redacted full-history scan: pass with zero unreviewed findings.
- Release-integrity and release-validation matrix guards: pass.
- Gitleaks unavailable/near-match version, rule-configuration override,
  shallow-history, and installer-argument rejection: pass with deterministic
  causes.
- `make gitleaks-scan`: pass with the pinned binary.
- `actionlint`, Bash syntax, and changed-script ShellCheck: pass.
- Current locked graph: 524 packages, 484 external registry packages, zero Git
  packages, zero missing license declarations, and zero known vulnerabilities.
- Disposable workspace-only version/lock synchronization: pass with zero
  external package updates.
- Release transaction rollback and bounded-update integrity guard: pass.
- Tagged root Candid and canonical Wasm-store DID comparison: byte-identical.
- Tagged CLI command/help, exit, diagnostic, and normalized adoption JSON
  comparison: pass.
- `v0.91.6` app install followed by `v0.92.11` PocketIC upgrade: pass.
- Current stable-record suites: 34 core and 18 control-plane tests passed.
- Current protocol/package proofs: 19 protocol and 7 manifest tests passed.
- Current build-provenance/policy proof: 15 passed; old provenance rejected as
  expected and current provenance admitted.
- Current backup/restore domain proof: 195 passed.
- Final `v0.92.12` method-catalog/fingerprint, release-integrity,
  release-validation, recovery/runbook, and zero-violation layering guards:
  pass.
- Final release-flow, release-index, workspace-manifest, and changelog tests:
  6, 5, 7, and 1 passed respectively.
- Final pinned full-history secret scan: zero findings at
  `dd4d55df8a9c870707ecda62f91900df8c0f6c70`.
- D15 dependency gate and its vulnerability/addition/removal/identity
  rejection fixtures: pass; release guards, ShellCheck, and `actionlint` pass.
- D15 auth ownership validation: 165 auth tests and 11 auth-prepare workflow
  tests pass; strict all-target `canic-core` Clippy, formatting, and diff
  hygiene pass.

## Next

0.92 is closed and no deferred finding remains after accepted D15. Commit and
release-reconcile D15, including the frozen complexity-method rerun on the
committed identity. Then begin 0.93 discovery from operational use and current
evidence. Broad product, deployment, package, publish, and release gates remain
maintainer-owned.
