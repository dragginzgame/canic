# Audit overlay: Current decision axes and ownership spread

## Method Contract

- Audit ID: `CANIC-COMPLEXITY-001`
- Method version: `4`
- Disposition: `revise`
- Owner: Current decision axes and ownership spread
- Kind/profile: `manual`
- Trace mode: `code_trace`
- Prerequisites: named scope, exact source and dirty-state evidence, caller inventory
- False-positive boundary: signals require a present contract or maintenance consequence
- Shared contract: [common audit contract](../../../../audits/README.md)
- Shared method: [complexity-and-technical-debt](../../../../audits/complexity-and-technical-debt.md)
- Snapshot revision: `a37771f1b6b5fc9a88ed6ab3b705bdda35cd8fa3`

## Product obligations

Start with the selected Core/Control Plane/Host owner under `crates/`; include
facade, macros, generated Fleet entrypoints, CLI, backup and tests only where
those contracts propagate. Inventory the complete selected production/test roots,
including diagnostics, features, state/data variants and generated consumers.

Map independent admitted axes, invalid combinations, semantic/plumbing consumers,
and at most three concrete near-term change rehearsals. Distinguish same-release
paid-effect recovery, stable memory, cycle conservation and public facade
coordination from unnecessary state space. Layering, authentication, lifecycle
and build-integrity methods retain their domain authority.

The frozen `measure-complexity-v2.sh` and `measure-complexity-v3.sh` outputs and
older scored reports remain historical evidence, ineligible as this method's
verdict. Raw counts may guide inspection if their exact scope and source are
recorded; do not convert them into an aggregate debt/health score. Runtime cost
claims require the instruction/Wasm method's comparable execution proof.

## Evidence and execution

[AGENTS.md](../../../../AGENTS.md) owns product invariants and execution authority.
Run only the smallest relevant checks after checking build ownership; inspection
is not repair authority. Do not run broad gates, mutate services or deploy.
Reports stay under Canic's existing `docs/audits/reports/YYYY-MM/YYYY-MM-DD/`
hierarchy, with exact method, source, host, dependency and execution identities.

The shared finding-based contract governs new runs. Prior scores and this
superseded [local definition](../../historical/shared-adoption-20261006/complexity-accretion.md)
remain historical, ineligible for new reviews. Affected comparisons are
`N/A (method change)`; unchanged domain/measurement evidence retains only its
own proven identity. This adoption does not itself execute a product audit.
