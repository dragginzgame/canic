# Audit Summary - 2026-06

## Included Run Days

| Day | Summary | Status |
| ---- | ---- | ---- |
| `2026-06-01` | reports retained under `docs/audits/reports/2026-06/2026-06-01/` | partial: day summary not present in current tree |
| `2026-06-02` | [2026-06-02/summary.md](2026-06-02/summary.md) | complete for reports retained on this day |
| `2026-06-04` | reports retained under `docs/audits/reports/2026-06/2026-06-04/` | partial: day summary not present in current tree |
| `2026-06-06` | [2026-06-06/summary.md](2026-06-06/summary.md) | complete for reports retained on this day |
| `2026-06-08` | reports retained under `docs/audits/reports/2026-06/2026-06-08/` | partial: day summary not present in current tree |
| `2026-06-13` | [2026-06-13/summary.md](2026-06-13/summary.md) | complete for reports retained on this day |
| `2026-06-19` | [2026-06-19/summary.md](2026-06-19/summary.md) | complete for reports retained on this day |
| `2026-06-22` | [2026-06-22/summary.md](2026-06-22/summary.md) | complete for reports retained on this day |
| `2026-06-28` | [2026-06-28/summary.md](2026-06-28/summary.md) | complete for reports retained on this day |

## Month-Level Status

Status: `partial`.

The month has retained report artifacts for `2026-06-01`, `2026-06-02`,
`2026-06-04`, `2026-06-06`, `2026-06-08`, `2026-06-13`, `2026-06-19`, and
`2026-06-22`, and `2026-06-28`.
This summary was created during the `2026-06-02` modular MSH run and updated as
additional audit reports were added; older reports without day summaries were
not rewritten.

## Carry-Forward Follow-up

- Preserve the DRY consolidation watchpoints from the retained June reports.
- Keep module-surface-hardening runs read-only unless the user explicitly asks
  for cleanup implementation.
- Carry forward the workflow-purity follow-up to keep workflow record carriers
  and Candid codecs behind ops/lower codec ownership. The workflow/API
  registry-record projection follow-up was closed after the 2026-06-13
  change-friction report.
- The audience-target-binding method drift note is closed as of 2026-06-22;
  carry forward the current watchpoint that active proof install issuer/root
  binding and root proof batch install metadata matching must remain in
  recurring coverage.
- Carry forward the change-friction watchpoints: the `workflow/pool` direct
  storage-record reference was fixed on 2026-06-13; prioritize host
  deployment-truth decomposition before adding more promotion/lifecycle report
  families. The first host decomposition follow-ups moved external lifecycle
  error/digest helpers plus promotion error/request, digest, identity, policy,
  guard, provenance, execution-receipt, materialization, wasm-store
  identity/catalog, transform/readiness, and artifact-plan/target lineage
  helpers out of the largest modules, then moved lifecycle authority-plan,
  external lifecycle pending/check/handoff/critical-fix reports, and
  external-upgrade report construction/validation into focused lifecycle child
  modules. Promotion and lifecycle parents are now small re-export modules, and
  install-root command/build, root-verification, and receipt IO support has
  started moving into child modules; remaining install-truth/report/text
  pressure remains open.
- Continue modular MSH down the CLI tree with focused low-risk modules before
  entering backup/restore recovery surfaces that require a larger Tier 2 pass.
- Keep `verify_token_material(...)` private and keep role-attestation/root
  proof provisioning out of delegated-token endpoint authorization. Rerun the
  supplemental PocketIC role-attestation verification path after the local
  PocketIC runner is known healthy.
- Keep capability DTOs passive, endpoint macros thin, and replay/
  authorization sequencing covered when the root capability surface changes.
- Carry forward the 2026-06-28 capability-surface watchpoint: retained global
  `canic_*` methods stayed stable, but root-managed renewal added six
  root-only service methods and the protocol export table continued to grow.
- Carry forward the 2026-06-28 change-friction watchpoints: keep the
  `api::blob_storage` hash/lifecycle/gateway/billing split intact, keep
  canonical blob root hash string conversion behind
  `ops::blob_storage::conversion`, and watch `crates/canic-cli/src/auth/mod.rs`
  after the 0.74 split.
- Carry forward the 2026-06-28 module-structure watchpoints: keep
  root-renewal schedule/retrieval/install/view children private and keep broad
  auth DTO/API plus host deployment-truth support surfaces under review.
- Carry forward the 2026-06-28 dependency-hygiene watchpoints: keep
  blob-storage billing, auth proof, sharding, and control-plane surfaces
  default-off, and keep blob-storage billing probes plus integration harnesses
  unpublished.
- Carry forward the 2026-06-28 DRY consolidation watchpoints: keep
  root-renewal and blob-storage runtime/API splits private and phase-specific,
  watch CLI auth/blob-storage command growth, and revisit evidence/proof
  helper extraction only if emitters or shell isolation rules converge.
- Carry forward the 2026-06-28 expiry/replay/single-use watchpoints: keep
  delegated-token verification stateless, keep root proof batch replay keyed by
  request id plus request fingerprint, keep exact-boundary expiry checks, and
  keep root-renewal scheduled retrieval/install gates covered as that line
  stabilizes.
