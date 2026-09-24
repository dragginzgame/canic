# Issue #28 qualification follow-up

Date: 2026-09-24. Published baseline: Canic 0.110.40,
`1ee0f742f79af572f134ed10aa893f0b5aea3492`.

[Issue #28](https://github.com/dragginzgame/canic/issues/28) confirms the published
lock presentation, read-only inspection, waiter cancellation and native cycle
parsing fixes. Its remaining work is acceptance evidence: the complete launcher
recovery matrix and live retry-deadline/per-child timing observations. This review
found no additional production defect. The issue remains open.

## Added regression coverage

The existing `canic-cli` integration target now invokes the real CLI inspector
against disposable subprocesses holding the production durable kernel lock.
Those fixtures publish typed advisory metadata; they are not application builds.

| Boundary | Evidence |
| --- | --- |
| Normal release and owner crash | A successor contends before release, then holds the same inode with its own matched identity; normal cleanup empties metadata. Only the fixture child is killed. |
| Wrong birth or namespace, missing identity and malformed metadata | JSON reports mismatch/unavailable, omits process observations, leaves metadata intact and preserves kernel exclusion. |
| Exited-owner metadata | The inspector cannot report a matched owner or child observations; it leaves the retained bytes unchanged. Numeric PID reuse does not require a particular unavailable/mismatch classification. |
| Hidden process view | The host fixture covers missing process data and an inspector namespace different from the visible holder's namespace. The kernel observation remains separate from identity and child visibility. |
| Artifact admission after reacquisition | A fresh `CompleteBuildReuse` guard accepts the exact retained release, then a later guard rejects tampered Wasm with the typed evidence error. |

The executable cases validate structured fields, kernel exclusion and inode/file
state. They do not freeze human diagnostic wording or impose a performance
threshold. The subprocess readiness deadline bounds a broken test fixture.
Existing tests retain cancellation, bounded process snapshots, quiet/active
children, terminal repaint and redirected-output coverage.

## Focused validation

- Host: 13 selected lock, exact-artifact and request-timing tests pass.
- CLI integration: three executable scenarios pass, plus the registered
  subprocess fixture entry.
- CLI library: 29 selected progress, retry-deadline, receipt, rendering and
  cancellation tests pass. Receipt coverage includes failed/interrupted request
  pairs, child subjects distinct from Root targets, and deadlines that do not
  imply remote advancement.
- Strict Clippy for host/CLI test targets, scoped formatting, whitespace and
  the workspace test inventory pass. Recovery stays within the existing
  `build_lock_inspection` target; no new top-level target needs registration.

Commands and full logs are retained in `.tmp/issue-28-20260924/`. Host-visible
execution permits the Linux inspector to observe its own fixture processes.
No full workspace suite, application build, PocketIC suite or deployment ran.

## Remaining downstream acceptance

Keep #28 open until an authorized Toko run supplies both of these:

1. Complete the recovery cases through the ordinary Toko build launcher on a
   disposable workspace: normal handoff, controlled holder crash, limited
   process visibility and stale identity metadata. Bind the CLI/source and
   retained artifact hashes; verify a survivor acquires normally and rejects
   changed output. Existing published feedback already covers real contention,
   terminal/redirection behavior and waiter-only cancellation.
2. Retain a .40 live convergence and terminal replay receipt through that
   launcher. Check observed retry deadlines (including unknown/expired values),
   child `request.subject` versus Root `target`, parent pairing and incomplete
   requests on interruption. Confirm replay is effect-free. A deployment-speed
   claim would additionally need matched inputs and timing conditions; summing
   nested or concurrent spans cannot establish wall-time improvement.

Toko's repository was inspected read-only. Its current feedback leaves those
checks open and staging on .38. This native qualification does not replace its
launcher acceptance or authorize a deployment. Earlier local evidence remains
in [lock qualification](canic-176-lock-wait.md),
[deadline qualification](canic-150-retry-deadlines.md) and
[timing attribution](toko-performance-followup.md).

This is test-only hardening and an acceptance handoff, with no maintained runtime
or CLI behavior change. No patch draft or version bump is allocated for it;
it does not independently justify a release. All edits remain uncommitted.
