# Audit Summary - 2026-05

## Included Run Days

| Day | Summary | Status |
| --- | --- | --- |
| `2026-05-01` | `docs/audits/reports/2026-05/2026-05-01/summary.md` | complete |
| `2026-05-07` | `docs/audits/reports/2026-05/2026-05-07/summary.md` | complete |

## Month-Level Status

Status: **complete**.

May has day summaries for the currently recorded audit days.

## Carry-Forward Follow-up List

1. `canic-core` auth/capability maintainers: keep role-attestation issuance and
   verification checks centralized; review again during the next
   `audience-target-binding` recurring run.
2. Audit maintenance: keep future `audience-target-binding` runbooks aligned
   with current test names and the required role-attestation audience DTO shape.
3. Audit maintenance: keep `token-trust-chain` aligned with the current
   self-contained delegated-token verifier and root-key cascade tests.
4. `canic-backup` and `canic-cli`: move duplicated backup/restore fixture
   builders into crate-local `test_support` modules after functional backup
   testing identifies the stable fixtures.
