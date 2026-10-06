# Audit overlay: Host/CLI/runtime flow convergence

## Method Contract

- Audit ID: `CANIC-DUPLICATION-001`
- Method version: `3`
- Disposition: `revise`
- Owner: Host/CLI/runtime flow convergence
- Kind/profile: `manual`
- Trace mode: `code_trace`
- Prerequisites: named scope, exact source and dirty-state evidence, caller inventory
- False-positive boundary: signals require a present contract or maintenance consequence
- Shared contract: [common audit contract](../../../../audits/README.md)
- Shared method: [flow-convergence-and-duplication](../../../../audits/flow-convergence-and-duplication.md)
- Snapshot revision: `a37771f1b6b5fc9a88ed6ab3b705bdda35cd8fa3`

## Product obligations

Trace endpoint/macro/facade, workflow, policy, ops, model and DTO boundaries under
`crates/`, generated Fleet packages under `canic-host`, and maintained shell
entrypoints under `scripts/`. Include command dispatch/help/output, ICP response
parsing, installed-Fleet discovery, build provenance, evidence envelopes,
backup/restore and root proof provisioning when the selected change reaches them.

Retain separation for network/credential selection, controller and spending
authority, paid-effect intent and lost-response reconciliation, exact artifact
admission, same-release recovery and independently measured hot paths. The
layering and individual auth methods own their violations; link their evidence
rather than duplicating a finding. Different public entrypoints need not merge.

Use caller searches and exact producer-to-result traces, followed by targeted
owning-package tests only within repair authority. Local test setup and display
formatting alone do not establish competing semantic ownership. Record existing
convergence and intentional retention as well as evidenced duplication.

## Evidence and execution

[AGENTS.md](../../../../AGENTS.md) owns product invariants and execution authority.
Run only the smallest relevant checks after checking build ownership; inspection
is not repair authority. Do not run broad gates, mutate services or deploy.
Reports stay under Canic's existing `docs/audits/reports/YYYY-MM/YYYY-MM-DD/`
hierarchy, with exact method, source, host, dependency and execution identities.

The shared finding-based contract governs new runs. Prior scores and this
superseded [local definition](../../historical/shared-adoption-20261006/dry-consolidation.md)
remain historical, ineligible for new reviews. Affected comparisons are
`N/A (method change)`; unchanged domain/measurement evidence retains only its
own proven identity. This adoption does not itself execute a product audit.
