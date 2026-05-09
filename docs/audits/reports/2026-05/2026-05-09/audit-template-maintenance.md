# Audit Template Maintenance

## Preamble

- Scope: recurring audit definitions under `docs/audits/recurring/`, plus
  `docs/audits/AUDIT-HOWTO.md` and `docs/audits/META-AUDIT.md`.
- Compared baseline report path: N/A.
- Code snapshot identifier: working tree, 0.33.1 docs cleanup line.
- Method tag/version: `audit-template-maintenance-2026-05-09`.
- Comparability status: non-comparable maintenance pass.
- Auditor: Codex.

## Summary

The recurring audit suite is broadly usable after the ICP CLI hard cut. The
remaining improvements were mostly template hygiene: remove old review-note
prose, keep the audit layout docs aligned with the domain-scoped directory
structure, and bring the freshness invariant up to date with delegated-token
single-use and root replay reservation limits.

## Findings

### Low - Audit how-to still described the old flat recurring layout

Evidence:

- `docs/audits/AUDIT-HOWTO.md` described recurring definitions as
  `docs/audits/recurring/<focus>.md`, while the maintained tree uses
  `docs/audits/recurring/invariants/` and `docs/audits/recurring/system/`.

Resolution:

- Updated the canonical tree and recurring-definition path to the domain-scoped
  layout.

### Low - Bootstrap lifecycle audit had embedded review prose

Evidence:

- `docs/audits/recurring/system/bootstrap-lifecycle-symmetry.md` ended with
  notes phrased as review commentary, including "What I would improve..." and
  targeted wording suggestions.

Resolution:

- Removed the commentary tail.
- Promoted the useful parts into direct audit requirements: non-goals and a
  `(path:line-range) observed behavior -> implication` evidence standard.

### Low - Freshness invariant lagged recent replay hardening

Evidence:

- `docs/audits/recurring/invariants/expiry-replay-single-use.md` described
  generic nonce/replay semantics but did not call out update-call delegated
  token consumption, query statelessness, or per-caller root replay capacity.

Resolution:

- Added update/query delegated-token replay expectations, current suggested
  tests, consumed-token hotspots, and root replay per-caller capacity red flags.

### Low - Some system audit wording still used demo/test/audit terminology

Evidence:

- `docs/audits/recurring/system/module-structure.md` and
  `docs/audits/recurring/system/README.md` still used "demo/test/audit"
  phrasing after the fleet layout cleanup.

Resolution:

- Updated those references to "fleet/test/audit".

## Verification Readout

| Command | Result | Notes |
| --- | --- | --- |
| `rg "dfx|DFX|dfx\\.json|\\.dfx|test-canisters|audit-canisters|demo/reference"` | PASS | No stale ICP-hard-cut or crate-local canister layout references remain in current recurring audit docs. |
| `rg "I would|Your findings|A few targeted|your current version" docs/audits/recurring ...` | PASS | Found review prose only in `bootstrap-lifecycle-symmetry.md`; removed it. |
| `git diff --check` | PASS | Whitespace check passed after cleanup. |

## Follow-up Actions

No follow-up actions required.
