# 2026-07-16 Audit Summary

## Scope

This run day closes the final non-deferred 0.92 finding by adding and executing
the dedicated secret scanner required by retained method
`CANIC-RELEASE-INTEGRITY-001/v1`.

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

## Live Ledger

- Retained methods attempted: 22 of 22.
- Valid active results: 22.
- Invalid active results: 0; superseded v1 attempts remain historical.
- Mandatory traces: current reruns 10 pass and 0 fail; frozen Phase C aggregate
  remains historical.
- Unresolved findings: 3 (0 P1 and 3 P2), all explicitly deferred watchpoints.
- Required partial or blocked current runs: 0.

## Validation

- Checksum-bound Gitleaks 8.30.1 install and reported-version check: pass.
- Redacted full-history scan: pass with zero unreviewed findings.
- Release-integrity and release-validation matrix guards: pass.
- Gitleaks unavailable/near-match version, rule-configuration override,
  shallow-history, and installer-argument rejection: pass with deterministic
  causes.
- `make gitleaks-scan`: pass with the pinned binary.
- `actionlint`, Bash syntax, and changed-script ShellCheck: pass.

## Next

Phase D has no unresolved P0 or P1 finding. Slice E must now execute the
documented `v0.91.6` compatibility accounting, confirm the three deferred P2
dispositions, and publish one explicit 0.92 closeout verdict. Broad product,
package, deployment, publish, and release gates remain maintainer-owned.
