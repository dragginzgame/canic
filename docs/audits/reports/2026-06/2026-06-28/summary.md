# Audit Summary - 2026-06-28

## Run Contexts

| Report | Type | Scope | Status |
| ---- | ---- | ---- | ---- |
| `capability-surface.md` | Recurring capability surface audit | endpoint macro bundles, retained-fleet DID service surface, protocol constants, RPC/capability DTO variants, root renewal/proof and issuer-local surfaces | PASS |
| `capability-scope-enforcement.md` | Recurring capability/scope invariant audit | delegated-token scope enforcement, root capability proof/envelope validation, replay and authorization ordering | PASS |
| `canonical-auth-boundary.md` | Recurring canonical auth boundary invariant audit | macro endpoint auth expansion, delegated-token verifier ordering, role attestation, root proof provisioning | PASS with supplemental validation blocked |
| `change-friction.md` | Recurring change-friction audit | recent `0.71` through `0.74` feature slices, CAF/locality, boundary leakage, enum shock radius, hub pressure, and dirty root-renewal split | PASS after cleanup |
| `complexity-accretion.md` | Recurring complexity accretion audit | `canic-core` enum growth, branch density, flow multiplicity, concept spread, and hub pressure | PASS with watchpoints |
| `dependency-hygiene.md` | Recurring dependency hygiene and feature/publish-surface audit | workspace manifests, public/support crate dependencies, feature aliases, publish fields, and internal harness seams | PASS |
| `dry-consolidation.md` | Recurring DRY consolidation audit | CLI/host/backup ownership, evidence envelope assembly, release-proof scripts, root proof provisioning, root-renewal, and blob-storage splits | PASS |
| `expiry-replay-single-use.md` | Recurring expiry/replay/single-use invariant audit | delegated-token bearer freshness, active proof status/install, root proof batch replay/get/install, root-renewal scheduled gates, and root replay capacity | PASS |
| `module-structure.md` | Recurring module-structure and visibility audit | public root surfaces, crate/subsystem direction, root-renewal and blob-storage splits, test/fleet seam containment | PASS |

## Risk Index Summary

| Report | Risk | Notes |
| ---- | ----: | ---- |
| `capability-surface.md` | 4 / 10 | Endpoint definitions and protocol export lines grew since 2026-06-19, with retained DID growth concentrated on six root-only delegation-renewal methods; no hard placement violation or over-bundled global family was found. |
| `capability-scope-enforcement.md` | 3 / 10 | No scope-as-identity or authorization-before-authentication break found; endpoint verify/bind/scope ordering and root capability replay/authorization coverage passed, including PocketIC checks outside the sandbox. Follow-up visibility cleanup reduced the counted capability-facing surface from 20 to 18 items. |
| `canonical-auth-boundary.md` | 4 / 10 | Required scans and focused tests passed; broad auth DTO/proof fan-in and recent auth API churn remain watchpoints. Supplemental PocketIC role-attestation parity check blocked on local runner health. |
| `change-friction.md` | 4 / 10 | Initial root-managed renewal and endpoint/cycles hardening slices raised p95 edit blast radius, while the root-renewal split improved locality. Follow-up cleanup moved canonical root-hash string conversion behind ops and split `api::blob_storage` into focused hash, lifecycle, gateway, billing, and test children. |
| `complexity-accretion.md` | 3 / 10 | Enum surfaces stayed flat, and the follow-up root-renewal split brought every renewal production file below the 600 logical LOC threshold. The later change-friction cleanup split the blob-storage API facade; remaining hub pressure is concentrated in older delegated-token, runtime-auth, nonroot-cycles, and CLI auth modules. |
| `dependency-hygiene.md` | 2 / 10 | Published crates still avoid runtime dependencies on unpublished workspace members, operator crates remain facade-free, and `canic` defaults stay narrow. The only notable delta is default-off blob-storage facade/core feature aliases and unpublished blob-storage billing fixtures. |
| `dry-consolidation.md` | 3 / 10 | No duplicate lifecycle owner was found. Root-renewal and blob-storage splits lowered runtime/API parent pressure, while CLI auth, blob-storage CLI, evidence emitters, and proof scripts remain low-risk watchpoints. |
| `expiry-replay-single-use.md` | 3 / 10 | No expiry/replay/single-use break was found. Exact-boundary expiry checks, request-id plus fingerprint replay, pending proof cleanup, root-renewal scheduled gates, and root replay capacity ordering passed; fan-in remains the main pressure. |
| `module-structure.md` | 3 / 10 | No public/internal seam leak, layer-direction breach, or module-discovery violation was found. Root-renewal and blob-storage API parents now delegate to focused private children; broad auth DTO/API and host deployment-truth surfaces remain watchpoints. |

## Method / Comparability Notes

- `canonical-auth-boundary.md` uses `canonical-auth-boundary/current` and is
  comparable with the 2026-06-19 run. No audit definition change was required
  for this run.
- `capability-scope-enforcement.md` uses
  `capability-scope-enforcement-current` and is comparable with the
  2026-06-19 run. No audit definition change was required for this run.
- `capability-surface.md` uses `capability-surface-current` and is comparable
  with the 2026-06-19 run. Retained DID counts are service-block scoped and the
  generated artifacts were refreshed from `fleets/test/canic.toml`.
- `change-friction.md` uses `change-friction-current-root-renewal` and is
  partially comparable with the 2026-06-19 run. CAF/locality, boundary
  leakage, enum shock radius, hub pressure, and release-sweep filtering remain
  comparable, while the feature sample shifted from the `0.68` root-proof
  provisioning line to `0.74` root-managed renewal plus recent `0.72` and
  `0.71` cross-layer feature slices.
- `complexity-accretion.md` uses
  `Method V4.5 / root-renewal scheduling/retrieval split refresh` and is
  partially comparable with the 2026-06-19 run. File, LOC, enum,
  branch-density, and large-file counts remain comparable; raw capability
  mention counts are noisy because the current code snapshot includes the
  root-renewal directory-module split and retained auth-renewal tests.
- `dependency-hygiene.md` uses `dependency-hygiene-current` and is comparable
  with the 2026-06-19 run. The manifest/package methodology is unchanged; the
  main delta is the addition of default-off blob-storage facade/core feature
  aliases.
- `dry-consolidation.md` uses
  `DRY Consolidation V6 / root-renewal and blob-storage split refresh` and is
  comparable with the 2026-06-19 run. The scan contract is unchanged; current
  readout is path-adjusted for the root-renewal directory-module split and
  blob-storage API child-module split.
- `expiry-replay-single-use.md` uses
  `Method V4.4 / root-renewal split refresh` and is comparable with the
  2026-06-19 run. The invariant and focused checks are unchanged; current
  readout is path-adjusted for the root-renewal split and added scheduled
  renewal retrieval/install gate tests.
- `module-structure.md` uses `module-structure-current` and is comparable with
  the 2026-06-19 run, with path-adjusted notes for the root-renewal
  directory-module split and blob-storage API child-module split.

## Follow-up

- Rerun the supplemental PocketIC role-attestation verification path after the
  local PocketIC runner is known healthy.
- Keep `verify_token_material(...)` private and keep role-attestation/root
  proof provisioning out of delegated-token endpoint authorization.
- Keep capability DTOs passive, endpoint macros thin, and replay/
  authorization sequencing covered when the root capability surface changes.
- Keep root-managed renewal endpoints root-only, keep blob-storage billing
  role-scoped when retained in future rosters, and watch protocol table fan-out
  before adding more constants.
- Keep the blob-storage API split intact. The parent facade is now small; new
  hash, lifecycle, gateway, and billing behavior should stay in the matching
  child module.
- Keep canonical blob root hash string conversion behind
  `ops::blob_storage::conversion`; do not reintroduce direct production
  API-to-model conversion references.
- Watch `crates/canic-cli/src/auth/mod.rs` after the 0.74 split; a follow-up
  module pass is warranted if auth command dispatch keeps growing.
- Keep the DRY consolidation watchpoints focused on CLI auth/blob-storage
  command growth, evidence emitter convergence, and proof-script isolation
  drift. Root-renewal and blob-storage runtime/API splits should stay private
  and phase-specific.
- Keep expiry/replay watchpoints focused on stateless delegated-token
  verification, root proof batch request-id plus fingerprint replay,
  exact-boundary expiry checks, root-renewal scheduled retrieval/install
  gates, and root replay per-caller capacity ordering.
- Keep root-renewal schedule/retrieval/install/view child modules private and
  route new lifecycle behavior to the matching owner. Keep broad auth DTO/API
  and host deployment-truth surfaces under recurring module-structure review.
- Keep `canic` defaults narrow; control-plane, sharding, auth proof, and
  blob-storage billing surfaces should remain explicit features. Keep
  blob-storage billing probes and integration harnesses `publish = false`.
