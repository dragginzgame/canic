# Audit overlay: Canic retained and exposed module authority

## Method Contract

- Audit ID: `CANIC-MODULE-SURFACE-001`
- Method version: `2.4`
- Disposition: `revise`
- Owner: Canic retained and exposed module authority
- Kind/profile: `manual`
- Trace mode: `code_trace`
- Prerequisites: named scope, exact source and dirty-state evidence, caller inventory
- False-positive boundary: signals require a present contract or maintenance consequence
- Shared contract: [common audit contract](../../../audits/README.md)
- Shared method: [module-surface-hardening](../../../audits/module-surface-hardening.md)
- Snapshot revision: `a37771f1b6b5fc9a88ed6ab3b705bdda35cd8fa3`

## Product obligations

Review a named module across Core, facade, Macros, Control Plane, Host, CLI,
Backup, testing and canisters where its contract is consumed. Inspect
`canic::__internal`, `canic::__build`, Core re-exports, macro expansions,
feature-selected runtime registrations and Host-generated Fleet packages.

Preserve endpoint authentication, explicit subject/caller/audience/subnet/parent
bindings, model storage invariants, same-release lifecycle restore, backup,
interrupted paid effects and cycle conservation. Pre-1.0 hard cuts do not erase
retained-effect evidence. AGENTS.md owns reinstall-only release transitions;
cleanup authorizes no live stop/delete/reset or release operation.

Record cold/warm/hot, encode/decode-sensitive, Wasm-sensitive and test-only shape.
Public, generated, persisted, recovery and deployment-truth boundaries need
expanded proofs. Test-only support may remain useful; tests must not unnecessarily
widen production visibility. Use targeted caller/expansion/feature tests;
canister lifecycle and inter-canister effects require PocketIC. Instruction/raw
Wasm changes require the local measured methods' comparable artifacts.

Use [the local cleanup overlay](module-cleanup-runner.md) only when implementation
is authorized. The shared method owns classification and finding-based verdicts;
there is no separate numeric risk score or mandatory whole-workspace build.

## Evidence and execution

[AGENTS.md](../../../AGENTS.md) owns product invariants and execution authority.
Run only the smallest relevant checks after checking build ownership; inspection
is not repair authority. Do not run broad gates, mutate services or deploy.
Reports stay under Canic's existing `docs/audits/reports/YYYY-MM/YYYY-MM-DD/`
hierarchy, with exact method, source, host, dependency and execution identities.

The shared finding-based contract governs new runs. Prior scores and this
superseded [local definition](../historical/shared-adoption-20261006/module-surface-hardening.md)
remain historical, ineligible for new reviews. Affected comparisons are
`N/A (method change)`; unchanged domain/measurement evidence retains only its
own proven identity. This adoption does not itself execute a product audit.
