# Current Status

Last updated: 2026-07-14

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the source, design, audit, or changelog files needed for the current task.

Historical detail is archived at:

- [status through 2026-06-30](archive/2026-06-30-precompact.md); and
- [status through the 0.90.2 release](archive/2026-07-13-precompact.md).

## Current Release

- The workspace package version is `0.91.5`.
- `v0.91.5` is published at commit `c2ee1b3a`.
- The accepted line design is
  [0.91 role admission and complete-build manifest publication](../design/0.91-role-admission-and-manifest-publication/0.91-design.md).
- Detailed release notes are in the [0.91 changelog](../changelog/0.91.md).

## Current Decision

`0.91.5` is live. Post-release work remains on the 0.91 line and is limited to
concrete defects, dependency maintenance, and focused cleanup. No 0.92 line is
assigned. The 0.91.6 changelog is prepared for the current cleanup batch; the
workspace package version remains 0.91.5 until maintainer-owned release
preparation.

Pre-1.0 removals remain hard cuts. Do not add aliases, compatibility wrappers,
duplicate command paths, deprecated APIs, or fallback behavior unless the
maintainer explicitly requests it. Named build environments resolve through
`icp.yaml`; only `local` and `ic` are implicit, and no staging/mainnet aliases
exist.

Toko mint remains downstream-owned. Canic provides generic primitives only;
automated work must not edit the Toko repository or move mint-specific
requests, receipts, evidence, retry, cancellation, or tests into Canic.

## 0.91 Outcome

- `0.91.0` added canonical lowercase snake_case role admission and bound root
  release-set publication to one complete build's exact outputs.
- `0.91.1` added the root-only issuer-readiness facade
  `AuthApi::provision_chain_key_delegation_proof_for_issuer_root` without
  restoring retired delegation-proof APIs.
- `0.91.2` updated allocation governance to `ic-memory 0.11.1` as a destructive
  reinstall boundary and rejects unsafe release-set artifact paths at
  admission.
- `0.91.3` bounded the audit archive and removed redundant generated exports.
- `0.91.4` made cost-guard settlement atomic, preserved snapshot restart
  causes, and made a failing installed `ic-wasm` shrink command fatal.
- `0.91.5` made ICP refill admission atomic and fail-closed, added durable CLI
  retry identity, and specified direct verified refill output.

The accepted
[0.91 closeout audit](../audits/reports/2026-07/2026-07-13/0.91-closeout.md)
remains the release-line baseline.

## Prepared 0.91.6

- Live `cycles convert --json` now actually emits the verified refill object
  promised by 0.91.5 instead of wrapping serialized JSON in the removed
  command-context shape. `cycles_sent` is plain base-10 text.
- Deployment commands share one internal JSON/text selector and renderer. The
  distinct deployment-check evidence-envelope format remains separate.
- Backup and restore share one persistence-owned journal sidecar lock, one
  timestamp derivation owner, and one artifact path sanitizer. The direct and
  resumable backup workflows remain intentionally separate.
- A partially acquired journal lock is removed when its ownership marker
  cannot be written. Existing public backup and restore lock errors are
  preserved as typed domain causes.
- The package version remains `0.91.5`; the changelog is prepared as 0.91.6,
  but package and lockfile version preparation remains maintainer-owned.

## Focused Validation

- `cargo check -p canic-cli -p canic-backup` passes.
- All four typed ICP-refill response tests pass, including the exact maintained
  JSON projection.
- All 96 focused deployment command tests pass.
- Shared journal-lock exclusion/drop, backup lock rejection, restore lock
  rejection and pending recovery, timestamp, and artifact-path tests pass.
- `cargo clippy -p canic-cli -p canic-backup --all-targets -- -D warnings`
  passes.
- Targeted package formatting and diff-hygiene checks pass.
- Full workspace, broad PocketIC, deployment, and release suites remain
  maintainer-owned and were not run for this cleanup.

## Next Action

Review the prepared 0.91.6 changelog and run the maintainer-owned release flow.
Do not assign 0.92 or begin deferred multi-step claim orchestration without an
explicit maintainer request.
