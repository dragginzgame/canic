# Docs DRY Consolidation Audit

## Preamble

- Scope: maintained docs under `docs/`, plus root `README.md`, `CONFIG.md`,
  `TESTING.md`, and `AGENTS.md`.
- Exclusions: generated audit reports, changelog detail files, and archived
  design docs unless they leak stale guidance into maintained docs.
- Compared baseline report path: N/A.
- Code snapshot identifier: working tree, pre-push 0.33.1 line.
- Method tag/version: `docs-dry-consolidation-2026-05-09`.
- Comparability status: non-comparable baseline run.
- Auditor: Codex.

## Summary

The current docs are mostly consistent around the new ICP CLI direction, but
the maintained audit templates still duplicate old DFX-era assumptions. The
highest-value cleanup is not more operator docs; it is consolidating the build
artifact vocabulary and canister-layout vocabulary so recurring audits do not
keep re-teaching stale paths.

## Findings

### Medium - Recurring audit templates still hardcode DFX artifact paths

Evidence:

- `docs/audits/recurring/system/capability-surface.md` still names generated
  `.did` files under `.dfx/local/canisters/**` and uses `.dfx` in suggested
  scans.
- `docs/audits/recurring/system/wasm-footprint.md` still says `dfx` is the
  canonical canister builder and shrink/gzip owner, names `dfx.json` as the
  default scope source, and treats `.dfx/local/canisters/<name>/<name>.wasm` as
  the canonical shipped artifact.

Impact:

- Recurring audits can produce false negatives or blocked reports after the
  0.33 ICP CLI hard cut.
- The same path vocabulary now exists in at least three forms:
  `.dfx/local/canisters`, `.icp/local/canisters`, and direct Cargo target
  artifacts.

Recommended consolidation:

- Add a short canonical artifact-path section in one maintained doc, probably
  `docs/architecture/README.md` or a new `docs/governance/build-artifacts.md`.
- Update recurring audit templates to reference that section rather than
  restating DFX/ICP-specific paths inline.

### Medium - Canister layout guidance is split and partially stale

Evidence:

- `README.md` describes current top-level `fleets/` plus non-fleet
  `canisters/audit`, `canisters/sandbox`, and `canisters/test`.
- `TESTING.md` says correctness fixtures should live under
  `crates/*/test-canisters/` and audit probes under `crates/*/audit-canisters/`.
- `docs/audits/recurring/system/module-structure.md` and
  `docs/audits/recurring/system/dependency-hygiene.md` still describe
  `canisters/**` as demo/reference surface, with separate crate-local
  test/audit canister paths.

Impact:

- A contributor has no single reliable answer for whether a new probe belongs
  under `canisters/test`, `canisters/audit`, or crate-local test/audit canister
  directories.
- Audit templates may flag the current hard-cut layout as drift because their
  canonical maps lag the repo.

Recommended consolidation:

- Make `TESTING.md` own test/audit canister placement.
- Point `README.md` and recurring audit canonical maps at `TESTING.md` for
  non-fleet canister layout.
- Keep `README.md` limited to operator-facing fleet and repo overview language.

### Low - Historical operations docs are not clearly separated from current flow

Evidence:

- `docs/operations/0.30-release-audit.md` and
  `docs/operations/0.30-backup-restore-smoke.md` contain DFX restore/snapshot
  commands and old `canic restore status` style references.
- `docs/design/0.30-canister-snapshots/0.30-design.md` and
  `docs/design/0.31-snapshot-cleanup/0.31-design.md` retain DFX-era design
  language.

Impact:

- These are versioned historical docs, so the content is not inherently wrong.
  The risk is discoverability: a reader can land on them without realizing
  current operator commands have moved to ICP CLI and named fleet arguments.

Recommended consolidation:

- Add a one-line historical banner to old operations/design docs that predate
  0.33, pointing readers to the 0.33 changelog and current README for live
  commands.
- Do not rewrite historical design decisions unless they are explicitly promoted
  back into current operator docs.

### Low - Current operator docs are mostly aligned

Evidence:

- `README.md` uses `canic install test`, `canic config test`,
  `canic list test`, `canic status`, and ICP CLI terminology in the current
  install/operator sections.
- `CONFIG.md` uses current config vocabulary for `initial_cycles`,
  `topup_policy`, `auto_create`, and fleet config semantics.
- `docs/status/current.md` correctly records the hard cut away from persisted
  fleet/network defaults.

Impact:

- No immediate operator-doc blocker for push.
- The remaining cleanup is structural DRY work in audit definitions, not
  command help or README correctness.

## Recommended Order

1. Update recurring audit templates for ICP CLI artifact paths and current
   build ownership.
2. Consolidate non-fleet canister layout ownership into `TESTING.md`, then make
   audit canonical maps reference it.
3. Add historical banners to old operations/design docs that still show DFX-era
   command flows.

## Follow-up Cleanup

- Added `docs/architecture/build-artifacts.md` as the current artifact
  vocabulary for `.icp`, `.canic`, generated Candid sidecars, Cargo wasm
  outputs, and Canic `.wasm.gz` artifacts.
- Updated recurring audit templates to use ICP CLI artifact paths and the
  current `fleets/` plus repo-level `canisters/{test,audit,sandbox}/` layout.
- Consolidated non-fleet canister placement in `TESTING.md`.
- Added historical banners to old 0.30/0.31 backup/restore operations and
  design docs that still intentionally preserve DFX-era command examples.

## Verification Readout

| Command | Result | Notes |
| --- | --- | --- |
| `find docs ... wc -l` | PASS | Captured maintained-doc size hotspots while excluding generated reports and changelog detail files. |
| `rg "dfx|DFX|dfx\\.json|canisters/|defaults|network use|canic network|canic defaults|canic scaffold|--dfx|fabricate|fabrication"` | PASS | Found stale DFX and layout vocabulary in maintained docs and audit templates. |
| `rg "canic (install|list|config|medic|status|replica|fleet|snapshot|restore)|icp ..."` | PASS | Confirmed current README/status docs use the new named-fleet and ICP CLI command shape. |
