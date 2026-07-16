# 2026-07-16 Audit Summary

## Scope

This run day closes the final non-deferred 0.92 finding by adding and executing
the dedicated secret scanner required by retained method
`CANIC-RELEASE-INTEGRITY-001/v1`, catches and fixes an unrelated
dependency-resolution side effect in the human release-version transaction,
and completes the executable `v0.91.6` compatibility accounting.

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

## Live Ledger

- Retained methods attempted: 22 of 22.
- Valid active results: 22.
- Invalid active results: 0; superseded v1 attempts remain historical.
- Mandatory traces: current reruns 10 pass and 0 fail; frozen Phase C aggregate
  remains historical.
- Unresolved findings: 3 (0 P1 and 3 P2), all explicitly deferred watchpoints.
- Required partial or blocked current runs: 0.
- Additional current-tree finding: `CANIC-092-RELEASE-005` is fixed in the
  D13 candidate and requires release identity before final Slice E closeout.
- `v0.91.6` contract: complete with explicit source/provenance hard cuts and
  no unclassified compatibility delta.

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

## Next

Phase D has no unresolved P0 or P1 finding. Compatibility accounting and the
three deferred P2 dispositions are complete. D13 and this report must receive
an immutable release identity before Slice E publishes one explicit 0.92
closeout verdict. Broad product, package, deployment, publish, and release
gates remain maintainer-owned.
