# Current handoff — 2026-10-01

## Active work and release boundary

The open batch is deployment reliability plus the maintainer-requested removal
of superseded completed-Fleet reset machinery. The complete batch is **not yet
push-ready**. Package versions remain `0.110.48`; extend the existing open
`0.110.49` changelog. HEAD at this handoff is `bdc3ddaab`; the maintainer committed
the first eight deployment-audit fixes. Subsequent work is uncommitted.

[Deployment reliability audit](../audits/reports/2026-09/2026-09-30/deployment-reliability.md)
findings own the current delivery work:

- Findings 1–8 are implemented and qualified: complete test failure feedback,
  native/gated/doctest selection, bootstrap budget admission, held-source funding
  credit, generated lock ownership and exact Cargo artifact capture.
- Findings 11–16 and 18 have implemented release/publication/runner corrections
  and focused shell/package evidence. Native macOS execution remains for CI.
- Finding 17 has native CLI evidence for build JSON and Fleet automation phases,
  exact argument-array next actions, approval versus resume, and successor review.
  Packaged consumer/recovery evidence remains outstanding.
- Findings 9 and 10 remain open: isolated installed-package consumer qualification
  and structured embedded-Wasm freshness. Another process is working on these;
  preserve its current edits and verify its latest evidence before claiming closure.

Existing qualification logs live under `target/review-validation/`: `deployment-*`,
`bootstrap-admission-*`, `import-funding-*`, `generated-lock-*`, `artifact-drift-*`,
`cli-*`, `release-content-native.log` and `release-core-package.log`. These qualify
their recorded source states, not subsequent changes or the complete workspace.
Do not run the entire historical release-flow/remote-state test files: some create
real fixture commits. New release fixtures use fake Git.

## Completed-Fleet cleanup

The maintainer accepted removal of the superseded completed-source preparation,
receipt/interface reconstruction, seal/publication and reset flows. Implementation
and focused qualification are in progress. Current clean reinstall is the sole
completed-Fleet reset route. Shared certified-controller observation belongs to
bootstrap/import; unfinished activation and paid-import reconciliation remain.
Remove obsolete tests and guards with their old paths, then qualify current
bootstrap/import, reset recovery and terminal replay. Keep current CLI automation
and the other process's package-consumer changes intact.

## Retained incident and operating constraints

The exact CANIC-188 issued `.48` import has its own
[repair decision](../design/0.110-fleet-runtime-contraction/issued-import-recovery.md)
and [incident instructions](../../scripts/dev/canic188/README.md). Its qualified
candidate and original Root are retained under
`.canic/incident-repairs/canic188/ed3084b6908a04b28effa21e00ec425aaf382d1423849fcbb4f3b12414d61f48/`.
Publishing current packages does not resume that operation. Preserve its original
journal, status, artifacts, ceilings and restoration sequence. Live repair,
convergence, replay and frontend acceptance remain outstanding. A separate local
`.48` import with an issued uninstall is outside that exact repair.

No live deployment, incident execution, sibling mutation, version transaction,
commit or Git publication is authorized by this cleanup. Toko Miner and other
repositories remain read-only. Keep release build artifacts and retained paid
operation evidence. Agents must never create commits. Check shared `target/`
ownership before each targeted build/test; use focused checks during implementation.

Fresh live backup execution remains unavailable because Coordinator-backed
Component Registry topology preflight is unimplemented. Preserve existing backup
and same-release restore/recovery machinery; see the
[backup availability guide](../features/backup-and-restore/README.md#current-availability).
Blob extraction remains separate accepted future work, not this cleanup's scope.

## Accepted follow-up and history

The [0.110 tracker](../design/0.110-fleet-runtime-contraction/status.md#accepted-code-review-corrections--2026-09-30)
owns remaining R2–R8 work. Open outcomes include exhausted/older-unknown imports,
remaining funding accounting, allocation-scoped caller/issuer/funding authority,
backup upload/capture/restore authority, background-driver trap recovery and
operation-specific convergence. Its conservative count is 27 of 401 original
findings; do not treat partial corrections as closed or the full queue as a gate
for every bounded corrective release.

After the accepted deployment/cleanup outcomes and their direct evidence finish,
report complete-batch readiness and extend the open changelog. Broad validation,
versioning and publication retain the maintainer-selected release boundary.
The final 0.110 closeout audit must be explicitly requested and accepted before
0.111 implementation; generic continuation does not cross that boundary.

Earlier checkpoints, superseded next steps and detailed timings are retained in
[historical handoffs](archive/2026-10-01-prior-fleet-handoffs.md). They are evidence,
not current instructions. The
[0.110 design](../design/0.110-fleet-runtime-contraction/0.110-design.md), tracker,
audits and governance own their respective contracts; status grants no release authority.
