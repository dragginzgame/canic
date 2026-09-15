# Toko Miner feedback after B1 qualification

Date: 2026-09-14. Read-only downstream refresh after rows 10 and 12 passed.

Toko Miner HEAD at inspection is
`dc58fb253cdd543b0141796c3b9260d1ccf00303`. Its working
`docs/upstream/canic.md` has SHA-256
`6cd89c001fe8db25435628fa088ef5e293921658c979c9c0fc6969cf6275f46d`.
The ledger now ends at CANIC-171. Canic remains based on published .16 commit
`a875c6498721bd89ca98549e38389b8280910d68`, with existing unpublished .17
work preserved. This supersedes the earlier scan's no-new-feedback finding.

## Disposition

| Feedback | Canic assessment | Next work |
| --- | --- | --- |
| CANIC-139 | The absent dependency-inventory defect is fixed and qualified in the open .17 draft. | Publish/adopt separately; this refresh does not rerun Toko's staging build. |
| CANIC-166 | Confirmed apply-entry selection defect. The CLI calls `retained_reinstall_apply_plan` first; that function decodes the old active plan before considering the separately staged review. Existing later workflow paths select the staged review correctly. | Reuse the validated staged-review owner at the public apply selector, preserving digest, environment, Fleet and reinstall checks; qualify the source-bound cases through that selector. |
| CANIC-171 | Confirmed accounting gap. Terminal conservation adds initial controlled cycles and operator funding, but not bounded positive movements already observed in exact preparation/Root-reset Stop receipts. | Add an explicit observed-credit term limited to those exact source-bound Stop receipts; qualify both prerequisite scopes and the complete conservation equation. |
| CANIC-170 | The command context lacks an explicit signing identity, but a global-default race has not been reproduced as the cause of the reported password prompt. | Resolve/bind an operation identity while preserving reviewed Principal checks; reproduce a default-identity change between subprocesses before claiming the incident's cause. |

## Review boundaries

The CANIC-166 direction is consistent with the maintained hard cut: the staged
review already owns current executable authority and the old source remains
receipt-only evidence. No older active-plan decoder or counter backfill is
needed. Toko's retained local patch adds the correct selection precedence and
extends four source-bound cases; it is review input, not a published Canic fix.

For CANIC-171, the observed credits are real receipt differences; their origin
is not independently proven to be a refund. The proposed local patch scopes
credits to source-bound preparation and Root-reinstall prerequisites, requires
Applied receipts with exact action hashes, and uses the reviewed observation
bound. Install/Start movements grant no credit. That direction is sound, but
upstream completion needs end-to-end conservation and replay regressions in
addition to receipt-helper tests. Missing receipts, excess credit, arithmetic
overflow and unexplained growth outside the boundary must fail. Starting
balances, funding authority and completed effects must not be rewritten.

Inspected Canic owners: `fleet_ensure/workflow/mod.rs`, the conservation model,
staged-review adoption, CLI retained-plan selection and `icp/command.rs`.
Downstream evidence includes the September 14 CANIC-166/170/171 sections,
`docs/upstream/scan-log.md`, and the retained `cli-review-selection.patch` and
`cli-staging-recovery.patch` under
`docs/audits/reports/2026/09/14/staging-recovery/01/`.

No Toko or live estate mutation ran. The new recovery and identity findings
were reviewed, not implemented in this B1 qualification batch. Prioritize the
bounded CANIC-166/171 recovery corrections before another contraction build
campaign; keep CANIC-170's cause qualification explicit. B1 itself still needs
matched measurements and the remaining evidence described in its tracker.
