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

## Month-Level Status

Status: `partial`.

The month has retained report artifacts for `2026-06-01`, `2026-06-02`,
`2026-06-04`, `2026-06-06`, `2026-06-08`, and `2026-06-13`. This summary was
created during the `2026-06-02` modular MSH run and updated as additional
audit reports were added; older reports without day summaries were not
rewritten.

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
  guard, provenance, execution-receipt, and materialization helpers out of the
  largest modules, with lifecycle and promotion internals under directory
  modules. Lifecycle and install-root/report-family pressure remain open.
