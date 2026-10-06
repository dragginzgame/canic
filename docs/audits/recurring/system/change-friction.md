# Audit overlay: Observed edit blast radius and change rehearsals

## Method Contract

- Audit ID: `CANIC-CHANGE-FRICTION-001`
- Method version: `4`
- Disposition: `revise`
- Owner: Observed edit blast radius and change rehearsals
- Kind/profile: `manual`
- Trace mode: `code_trace`
- Prerequisites: named scope, exact source and dirty-state evidence, caller inventory
- False-positive boundary: signals require a present contract or maintenance consequence
- Shared contract: [common audit contract](../../../../audits/README.md)
- Shared method: [complexity-and-technical-debt](../../../../audits/complexity-and-technical-debt.md)
- Snapshot revision: `a37771f1b6b5fc9a88ed6ab3b705bdda35cd8fa3`

## Product obligations

Review demonstrated changes under `crates/canic-core/src/`, with exact first
parents, classified source/test/generated paths and semantic versus plumbing
edits. Include all selected subsystem roots, including diagnostics. Link the
complexity owner's decision-axis/consumer map and the layering owner's results;
do not create another import parser or infer defects from edit counts.

The frozen five-row sample in `fixtures/change-friction-v2-sample.tsv` and v2/v3
measurement scripts remain available solely to reproduce historical scored
reports. Preserve their commit ancestry, exact population, formulas and raw
output when reproducing them; identify that execution as the historical method.
New reviews may cite the original per-change evidence with its exclusions, and
rehearse at most three current changes. Do not reuse the scripts' aggregate
velocity/risk score as a current product verdict or silently change the sample.

Generated output, release sweeps, formatting, moves, test propagation and public
contract coordination are classified separately. A finding needs present
friction, an owning boundary and a concrete simpler alternative; a high path
count is not sufficient. No performance claim follows from structural evidence.

## Evidence and execution

[AGENTS.md](../../../../AGENTS.md) owns product invariants and execution authority.
Run only the smallest relevant checks after checking build ownership; inspection
is not repair authority. Do not run broad gates, mutate services or deploy.
Reports stay under Canic's existing `docs/audits/reports/YYYY-MM/YYYY-MM-DD/`
hierarchy, with exact method, source, host, dependency and execution identities.

The shared finding-based contract governs new runs. Prior scores and this
superseded [local definition](../../historical/shared-adoption-20261006/change-friction.md)
remain historical, ineligible for new reviews. Affected comparisons are
`N/A (method change)`; unchanged domain/measurement evidence retains only its
own proven identity. This adoption does not itself execute a product audit.
