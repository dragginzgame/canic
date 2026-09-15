# Toko Miner feedback after Canic 0.110.16

Reviewed 2026-09-14. No new confirmed Canic defect was found in the downstream
ledger. It contains 169 unique indexed issues with matching section bodies and
still ends at CANIC-169. Its nine Confirmed labels describe earlier releases;
they do not establish missing implementation in .16.

## Inspected identities

- Canic HEAD and `v0.110.16` resolve to
  `a875c6498721bd89ca98549e38389b8280910d68`. The worktree was clean at inspection.
  The release snapshot records package .16, complete validation and source
  `81b71246fccb7e1defb949d0617a388fa9b08b9f`. The maintainer reports the push complete.
- Toko Miner HEAD is `010126581ac49dccb83822ec342329129c413bae`; its working
  Cargo manifest and lock already select Canic .16 at the exact release above.
  This is working-tree adoption, not a claim that downstream qualification or
  installation completed. Its active build uses the published Git checkout.
- The inspected `../toko-miner/docs/upstream/canic.md` has SHA-256
  `99fe182105144c5bea150133139351fd14b72378e0dad0a28e5d75428cb50ccd`.
  The latest issue updates inspected are dated September 13. Its status,
  adoption/alignment reports and lockfile were read independently because their
  release boundaries differ. No sibling file or live Fleet was changed.

## Current disposition

| Feedback | Canic .16 disposition | Remaining evidence or owner |
| --- | --- | --- |
| CANIC-002/008/010/017 | The promoted observatory, frontend handoff, Component commands and persistent multi-subnet local Fleet are shipped. The combined proof also covers ordinary inventory, no-op review, lost reply and imported local capacity. | Toko profiles, frontend integration and its representative multi-subnet journey. |
| CANIC-014 | The tagged status now exposes package .16 and its validated source, explicitly distinguishing the older handoff. | No new source defect; retain the generated summary on subsequent releases. |
| CANIC-141 | .16 implements the requested pre-effect rejection alternative using `MixedSubnetCreationFees`. | Exact per-subnet fee support is still unavailable; the accepted request does not require replacing its safe rejection alternative. |
| CANIC-166 | Raw retained-source inspection precedes executable journal admission; the exact 30-row omission shape reaches the governed diagnostic without inventing paid-attempt counters. | Fresh authorized staging review and execution under exact observed authority. A package update does not complete recovery. |
| CANIC-168 | Implicit compiler-cache startup is probed; the original error and explicit empty-wrapper override are reported. | Downstream verification of its affected build environment. |
| CANIC-169 | Reconciliation owns PendingReset/Failed funding; completed reinstall receipts survive the same-operation continuation. Live reconciliation is distinct from unavailable stopped-Root observations. | The original downstream local operation was superseded by an authorized fresh reset. That fresh success does not prove interrupted reinstall recovery; use a new disposable qualification if needed. |
| CANIC-003/135/165 | Scoped authorization, selected auth-store restoration and fixture provisioning are available in the tagged code. | Application authorization/configuration, installed-role evidence and application-specific fixture acceptance. Available labels alone do not reopen implementation. |
| CANIC-161 | The documented exact-role opt-in, protected diagnosis and manual-funding boundaries stand. Existing fixture coverage proves bounded automatic grants, partial final allowance and terminal budget exhaustion. | Incident consumption and installed policy are unproven; bounded fixture funding is not representative long-running Toko evidence. No Canic loop defect is established. |
| CANIC-162 | .16 ships the smaller manager bucket default and reviewed store consolidation, with protected per-memory reports and public aggregate summaries. | Fresh installed Game Hub attribution, workload growth and payload occupancy. The historical 232 MiB cannot be retroactively assigned to individual stores. |
| CANIC-087/139 | Exact unchanged-release reuse and intermediate caching remain the accepted scope. | Comparable downstream measurements. Finalized Wasm reuse across release identities stays excluded. |

Published implementation and previous qualification are linked from the
[0.110 changelog](../../../../changelog/0.110.md),
[operator tracker](../../../../design/0.110-fleet-runtime-contraction/status.md),
[funding runbook](../../../../operations/fleet-funding.md#workload-funding-and-application-failures)
and [local Fleet guide](../../../../features/operations/local-development-fleet.md#qualification).
The bounded automatic-funding evidence is the existing
`pending_fixture_automatically_funds_within_configured_allowance` case; it was
source-reviewed here, not rerun or relabelled as a long-session measurement.

## Continuation and limits

The maintainer explicitly continues 0.110 after .16. Additional same-minor
correctness follow-ups may be grouped when concrete feedback arrives; no
seventeenth patch is allocated merely for this scan. This remains appropriate
despite the advisory twelve-release guideline because the affected minor is
still maintained and no next-minor closeout has been requested or accepted.

The active tracker had two stale directions: it called the shipped generic
observatory deferred and described .14 as the next release. Those are corrected.
The remaining accepted Canic development sequence is the existing B1 Wasm
attribution work: complete row 8, then rows 10/12, with immutable matched
measurements and their stated limitations. .16's storage savings and ICYDB-033's
discarded outlining experiment do not complete B1 or authorize unrelated state
families. Role-specific stable initialization remains parked under ideas.

This review does not claim broad testing, live recovery, completed Toko
adoption, new performance measurements or a minor closeout. Only local
documentation/identity checks are required for this feedback reconciliation.
