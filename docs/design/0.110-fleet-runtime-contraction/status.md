# Canic 0.110 Implementation Status

## Maintained scope

The .47 package set is published. CANIC-185 target-local bootstrap funding,
role-owned activation persistence, optional observability, passive blob
contracts, artifact reuse/admission and IcyDB composition corrections are
implemented. The [current handoff](../../status/current.md) owns the active
cleanup batch; [release notes](../../changelog/0.110.md) describe shipped behavior.

The normative [design](0.110-design.md) and independent
[size follow-through amendment](2026-09-28-toko-size-follow-through.md) define
accepted scope. The amendment reopened only the bounded .47 activation,
observability and blob-contract work; it did not restart the full B3/B4 matrix.

## Batch dispositions

| Batch | Disposition |
| --- | --- |
| B1 | Accepted by the maintainer on 2026-09-23; retained controlled measurements and source/interface dispositions remain evidence. |
| B2 | Complete with the accepted cold-query tradeoff; role-selected storage/restoration evidence remains retained. |
| B3 | General record/codec restructuring stopped; the bounded .47 activation amendment is implemented. |
| B4 | Remaining general pruning deferred and unscheduled; bounded .47 observability and passive blob-contract changes are implemented. |
| FI1 | Bootstrap and capacity-import implementation/qualification shipped in .43; subsequent reinstall/recovery corrections shipped through .47. |
| B5 | The .42 checkpoint is qualified; final closeout must cover FI1 and subsequent corrections and receive human acceptance. |
| Cleanup | Complete and ready for maintainer review: unused runtime/Host paths, obsolete helpers/tests and status-owned release flows removed, duplicate evidence consolidated, focused checks passed. |
| Reinstall review corrections | Complete in the open .48 batch: typed unavailable funding diagnostics and exact-digest unpaid-review cancellation, with native recovery and public-CLI PocketIC qualification. Publication and Toko live acceptance remain pending. |

## Evidence and remaining acceptance

- [B1 review packet](../../audits/working/0.110-fleet-runtime-contraction/b1-input-evidence.md#complete-b1-review--accepted-2026-09-23).
- [B2 reachability ledger](../../audits/working/0.110-fleet-runtime-contraction/b2-storage-reachability.md).
- [CANIC-185 funding qualification](../../audits/reports/2026-09/2026-09-29/toko-bootstrap-funding.md).
- [CANIC-185/186 review readiness and cancellation qualification](../../audits/reports/2026-09/2026-09-29/toko-unpaid-review-cancellation.md).
- [Bounded size qualification](../../audits/reports/2026-09/2026-09-28/toko-wasm-size-audit.md#selected-implementation-follow-through).
- [Final closeout audit owner](../../audits/release-lines/0.110-closeout-audit.md).

Downstream live/performance acceptance and complete cycle-attribution evidence
remain separate from the passing Canic implementation checks. Downstream source,
application state and launcher choices are not Canic release gates. Necessary
correctness, security and recovery corrections remain in scope for this line.

The next minor is not authorized by this cleanup or by a passing gate. The
maintainer must explicitly request and accept the final 0.110 closeout audit
before 0.111 implementation begins. Do not restart completed B1 ablations,
stopped B3 work or deferred B4 work without a new scope decision.

## History

The [dated implementation history](../../status/archive/2026-09-29-fleet-runtime-contraction.md)
retains every prior checkpoint, measurement, review and historical queue.
Its old “next action” statements are historical; the dispositions above govern
current work.
