# Audit Summary - 2026-06

## Included Run Days

| Day | Summary | Status |
| ---- | ---- | ---- |
| `2026-06-01` | reports retained under `docs/audits/reports/2026-06/2026-06-01/` | partial: day summary not present in current tree |
| `2026-06-02` | [2026-06-02/summary.md](2026-06-02/summary.md) | complete for reports retained on this day |
| `2026-06-04` | reports retained under `docs/audits/reports/2026-06/2026-06-04/` | partial: day summary not present in current tree |
| `2026-06-06` | [2026-06-06/summary.md](2026-06-06/summary.md) | complete for reports retained on this day |
| `2026-06-08` | reports retained under `docs/audits/reports/2026-06/2026-06-08/` | partial: day summary not present in current tree |
| `2026-06-13` | [2026-06-13/summary.md](2026-06-13/summary.md) | complete for reports retained on this day |
| `2026-06-19` | [2026-06-19/summary.md](2026-06-19/summary.md) | complete for reports retained on this day |

## Month-Level Status

Status: `partial`.

The month has retained report artifacts for `2026-06-01`, `2026-06-02`,
`2026-06-04`, `2026-06-06`, `2026-06-08`, `2026-06-13`, and `2026-06-19`.
This summary was created during the `2026-06-02` modular MSH run and updated as
additional audit reports were added; older reports without day summaries were
not rewritten.

## Carry-Forward Follow-up

- Preserve the DRY consolidation watchpoints from the retained June reports.
- Keep module-surface-hardening runs read-only unless the user explicitly asks
  for cleanup implementation.
- Carry forward the workflow-purity follow-up to keep workflow record carriers
  and Candid codecs behind ops/lower codec ownership. The workflow/API
  registry-record projection follow-up was closed after the 2026-06-13
  change-friction report.
- Carry forward the audience-target-binding method drift note: retired or
  absent internal-invocation and delegated-grant surfaces should either be
  removed from the recurring definition or explicitly tracked as historical
  scope.
- Carry forward the change-friction watchpoints: the `workflow/pool` direct
  storage-record reference was fixed on 2026-06-13; prioritize host
  deployment-truth decomposition before adding more promotion/lifecycle report
  families. The first host decomposition follow-ups moved external lifecycle
  error/digest helpers plus promotion error/request, digest, identity, policy,
  guard, provenance, execution-receipt, materialization, wasm-store
  identity/catalog, transform/readiness, and artifact-plan/target lineage
  helpers out of the largest modules, then moved lifecycle authority-plan,
  external lifecycle pending/check/handoff/critical-fix reports, and
  external-upgrade report construction/validation into focused lifecycle child
  modules. Promotion and lifecycle parents are now small re-export modules, and
  install-root command/build, root-verification, and receipt IO support has
  started moving into child modules; remaining install-truth/report/text
  pressure remains open.
- Continue modular MSH down the CLI tree with focused low-risk modules before
  entering backup/restore recovery surfaces that require a larger Tier 2 pass.
