# Audit overlay: Canic crate/module topology and visibility

## Method Contract

- Audit ID: `CANIC-STRUCTURE-001`
- Method version: `3`
- Disposition: `revise`
- Owner: Canic crate/module topology and visibility
- Kind/profile: `manual`
- Trace mode: `code_trace`
- Prerequisites: named scope, exact source and dirty-state evidence, caller inventory
- False-positive boundary: signals require a present contract or maintenance consequence
- Shared contract: [common audit contract](../../../../audits/README.md)
- Shared method: [module-surface-hardening](../../../../audits/module-surface-hardening.md)
- Snapshot revision: `a37771f1b6b5fc9a88ed6ab3b705bdda35cd8fa3`

## Product obligations

Inspect the selected package's module roots, re-exports, facade/generated seams,
feature-gated consumers, tests and Cargo dependencies. Runtime/facade ownership
is `canic`, Core and Macros; Control Plane owns canister orchestration; Host owns
generated Fleet packages; CLI/Host/Backup own operator workflows. Keep flat
`crates/`, edition 2024 and directory modules with `mod.rs` under AGENTS.md.

Verify intentional public exposure, narrow restricted visibility, ordinary module
discovery and test-support containment. Do not classify `pub` counts or file size
as a defect. Hard endpoint/workflow/policy/ops/model direction and data-shape
violations belong to the layering method. Inspect macro expansion and generated
callers before recommending deletion or narrowing.

Use exact caller evidence, relevant Cargo metadata and focused owning-package
checks. New visibility or dependency claims require their actual feature/target
coverage; canister runtime shape and cost remain with the measured methods.

## Evidence and execution

[AGENTS.md](../../../../AGENTS.md) owns product invariants and execution authority.
Run only the smallest relevant checks after checking build ownership; inspection
is not repair authority. Do not run broad gates, mutate services or deploy.
Reports stay under Canic's existing `docs/audits/reports/YYYY-MM/YYYY-MM-DD/`
hierarchy, with exact method, source, host, dependency and execution identities.

The shared finding-based contract governs new runs. Prior scores and this
superseded [local definition](../../historical/shared-adoption-20261006/module-structure.md)
remain historical, ineligible for new reviews. Affected comparisons are
`N/A (method change)`; unchanged domain/measurement evidence retains only its
own proven identity. This adoption does not itself execute a product audit.
