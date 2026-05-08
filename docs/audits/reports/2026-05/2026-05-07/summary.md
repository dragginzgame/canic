# Audit Summary - 2026-05-07

## Run Contexts

| Report | Type | Scope | Snapshot | Worktree | Status |
| --- | --- | --- | --- | --- | --- |
| `dry-consolidation.md` | Ad hoc | CLI, host, backup, scripts DRY and ownership | `6e72960b` | dirty | complete |
| `audience-target-binding.md` | Recurring invariant | delegated auth, role attestations, capability proofs, delegated grants | `6e72960b` | dirty | complete |
| `token-trust-chain.md` | Recurring invariant | delegated-token root/shard trust chain and role-attestation key verification | `6e72960b` | dirty | complete |

## Risk Index Summary

| Report | Risk | Readout |
| --- | ---: | --- |
| `dry-consolidation.md` | 3 / 10 | Boundary is mostly coherent; remaining DRY work is low-risk test fixture cleanup. |
| `audience-target-binding.md` | 3 / 10 | Invariant holds; follow-up removed the optional-audience DTO risk, leaving only broad `RoleAttestation` reference radius as residual risk. |
| `token-trust-chain.md` | 3 / 10 | Invariant holds; residual risk is stale April audit/test naming after verifier implementation drift. |

## Method and Comparability Notes

- `dry-consolidation.md` is non-comparable because it is an ad hoc audit rather
  than a recurring definition.
- `audience-target-binding.md` is comparable to
  `docs/audits/reports/2026-04/2026-04-05/audience-target-binding.md` using
  `Method V4.1`.
- `token-trust-chain.md` is partially comparable to
  `docs/audits/reports/2026-04/2026-04-05/token-trust-chain.md` using
  `Method V4.2`; the implementation moved from removed proof-state trace
  helpers to the self-contained delegated-token verifier.

## Key Findings by Severity

### Medium

- `dry-consolidation.md`: CI target-list duplication was found and then
  resolved by routing configured fleet canister views through `canic fleet
  canisters`.

### Medium-Low

- `audience-target-binding.md`: the audit initially flagged optional
  role-attestation audience as residual risk; follow-up work hard-cut the DTO
  to require `audience: Principal`.

### Low

- `dry-consolidation.md`: backup/restore test fixtures still have some local
  duplication; no production boundary issue was found.
- `audience-target-binding.md`: two April audit test names now match zero tests,
  but replacement tests cover the behavior.
- `token-trust-chain.md`: April verifier hotspot and trace-test names were
  stale; the recurring definition has been updated to current verifier paths.

## Verification Rollup

| Report | PASS | BLOCKED | FAIL | Notes |
| --- | ---: | ---: | ---: | --- |
| `dry-consolidation.md` | 6 | 0 | 0 | Read-only audit commands plus follow-up validation were recorded in the report. |
| `audience-target-binding.md` | 14 | 2 | 0 | Two stale April test filters matched zero tests; replacement tests passed. Follow-up removed the optional-audience DTO risk. |
| `token-trust-chain.md` | 8 | 2 | 0 | Two stale April test filters matched zero tests; replacement unit and PocketIC trust-chain tests passed. |

## Follow-up Actions

1. `canic-core` auth/capability maintainers: keep role-attestation issuance and
   verification checks centralized; review again during the next
   `audience-target-binding` recurring run.
2. Audit maintenance: update future `audience-target-binding` runbooks to use
   current replacement test names.
3. Audit maintenance: keep `token-trust-chain` aligned with the current
   self-contained delegated-token verifier and root-key cascade tests.
4. `canic-backup` and `canic-cli`: move duplicated backup/restore fixture
   builders into crate-local `test_support` modules after functional backup
   testing identifies the stable fixtures.
