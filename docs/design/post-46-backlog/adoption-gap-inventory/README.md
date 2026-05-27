# Deployment Adoption Gap Inventory

This is reconciled product-gap source material for the completed 0.41-0.46
deployment foundation and the post-46 backlog. It is not itself a release
contract, an approved numbered follow-on line, or current implementation truth.

Last reconciled after the 0.46 closeout on 2026-05-27.

The normative design remains in the per-line design docs and
`0.41-deployment-truth-model/0.41-0.46-deployment-roadmap.md`. Active backlog
topics live under `post-46-backlog/`.

Status markers:

- **Resolved by 0.41-0.46**: the original adoption concern is now covered by
  the completed deployment foundation.
- **Partially covered**: the model or passive workflow exists, but the
  operator workflow is not complete.
- **Open backlog**: still product work, not part of the completed 0.41-0.46
  line.

---

## A. Adoption Use Cases And Current Status

### A.1 Multiple real IC deployments (no testnet)

**Need:** Run prod, staging, v2, canary, and destructive-rehearsal lanes on IC as separate deployments that share one topology template but differ in root, controllers, wasm per role, and embedded config per role.

**Status after 0.46:** Partially covered.

Resolved by 0.46:

- live local state is deployment-target state under
  `.canic/<network>/deployments/<deployment>.json`;
- fleet templates are reusable desired topology inputs, not live state;
- supplied-plan install requires exact deployment target identity;
- old fleet-named live state fails closed instead of being read as deployment
  truth;
- `canic deploy compare --left <file> --right <file>` compares two archived
  `DeploymentCheckV1` artifacts passively.

Still open backlog:

- **Deployment catalog** — list/search deployments by group, environment, owner team, not only fleet directory names.
- **Lane teardown** — retire a test deployment (controllers, pool release policy, `.canic` state) without manual IC archaeology.
- **Cost attribution per deployment** — cycles burned per lane for finance/ops (not in 0.41–0.46).

---

### A.2 Per-role wasm and per-role config on different lanes

**Need:** Staging runs `project_hub` v1 + `project_instance` v2 candidate; prod runs all v1; one role from a pinned `.wasm.gz`, others from workspace packages.

**Status after 0.46:** Partially covered.

0.44 added digest-pinned promotion artifacts, sealed-wasm vs source/build
identity, promotion readiness/provenance, materialization evidence, and
plan-mediated install for ready promotion envelopes. It remains deliberately
passive around live `wasm_store` catalog lookup and active dedupe policy.

Still open backlog:

- **Mixed-lane install in one command** — install only roles `{X,Y}` on deployment `staging` without rebuilding the whole release set.
- **Config-independent roles** — explicit manifest flag for roles where sealed wasm can promote across trust domains (rare; must be deliberate).
- **Attestation-aware promotion** — after 0.40, promoting instance wasm without hub/root epoch alignment is a **runtime** failure; promotion UX must surface verifier/epoch prerequisites, not only wasm sha256.

---

### A.3 Brownfield: Canic wraps an existing IC project

**Need:** Team already has canisters on IC (dfx/icp legacy layout, hand-rolled controllers, SNS-upgraded canisters). They want root + topology **without** reinstalling everything from zero.

**Status after 0.46:** Open backlog.

0.42 and 0.45 provide the control-class and external lifecycle vocabulary
needed for adoption planning, but Canic still does not provide a guided
brownfield adoption workflow.

Still open backlog:

- `canic fleet adopt <role> <principal>` — read-only inventory first, then optional registry entry + controller reconciliation plan (0.42).
- **Import dry-run** — show exactly which management calls would run before pool import mutates live canisters.
- **Partial fleet** — runtime topology lists roles where only some are Canic-managed; others are observe-only forever (0.45 touches user-owned, not general brownfield).

---

### A.4 Library-only or single-canister adoption

**Need:** Project wants `canic` macros + stable memory + auth on **one** canister, no root fleet, no wasm store, no thin-root staging.

**Status after 0.46:** Open backlog.

- Docs and scaffold push `fleets/<name>/root` + `app` + `canic install`.
- `build_standalone!` exists but is framed for probes/sandbox, not “production single canister.”
- No published pattern for “Canic runtime without control-plane feature.”

**Propose:**

- First-class **standalone fleet profile** in docs and `canic fleet create --profile standalone`.
- Explicit **feature matrix** in `INSTALLING.md`: which crates/features are required for which topology (root / hub / leaf / standalone).
- Cargo feature preset `canic = { features = ["leaf-only"] }` with compile-time guard that rejects `start_root!` in the same crate.

---

### A.5 Split-repo / polyglot monorepo (backend + frontend + infra)

**Need:** Rust canisters in `backend/`, `icp.yaml` at repo root, frontend needs Candid/declarations, CI builds from nested cwd.

**Status after 0.46:** Partially covered.

Deployment-target state now records workspace and ICP roots, and deployment
truth uses those roots for local plan/inventory. The broader split-repo
developer experience is still open.

Still open backlog:

- No official hook: “after build, emit declarations to `frontend/src/declarations`.”
- Downstreams still wrap `canic` in shell scripts (common in large apps).
- `canic build --emit-declarations <dir>` (or integrate with existing `generate_declarations` patterns as a documented contract).
- **Project manifest** — optional `canic.project.toml` at repo root mapping workspace root, icp root, declaration output, deployment group name (single file for CI).
- Stable machine-readable output on all operator commands for CI parsers
  (today some commands print raw JSON by default, while others use
  `--format json|text`).

---

### A.6 CI/CD and GitOps-shaped workflows

**Need:** Pipeline builds artifacts, pins digests, runs plan diff against prod inventory, applies only on approval; no interactive install prompts.

**Status after 0.46:** Partially covered.

Resolved or partially covered by 0.41-0.46:

- `canic deploy plan`, `inventory`, `check`, `diff`, `report`, and
  `compare` exist;
- safety reports and deployment checks can block unsafe install gates;
- promotion, authority, and external lifecycle artifacts have JSON surfaces;
- release-index guarding prevents mixed release commits.

Still open backlog:

- **Stable machine-readable exit codes** per diff category (controller drift vs module hash vs config digest).
- **Signed plans / receipts** — optional Sigstore/cosign-style attestation for artifact promotion between CI stages (not in roadmap).
- **Concurrency lock** — two CI jobs must not install the same deployment; no lease/lock object in designs.
- Official **GitHub/GitLab Action** that wraps inventory/plan/diff (adoption friction for “any project”).

---

### A.7 User- or customer-controlled canisters in a fleet topology

**Need:** SaaS model: fleet provisions `project_instance`; end user is IC controller; fleet must propose upgrades, not push them.

**Status after 0.46:** Partially covered.

0.45 added passive external lifecycle projection, plans, proposals, consent
evidence, completion reports, and deployment-truth inventory-backed
verification checks. It did not add consent delivery or external execution.

Still open backlog:

- **Operator playbook** — document flows for security patch on user-controlled role (proposal → consent → verify → receipt).
- **Frontend/SDK contract** — how a web app requests user consent aligned with Canic receipts (0.45 is canister-side only).
- **Fleet upgrade must not block** on user-controlled roles — partial fleet upgrade semantics need to be default, not edge case.

---

### A.8 Operator teams, RBAC, and break-glass

**Need:** Platform team vs app team vs emergency break-glass; staging principals must not be IC controllers (design notes in 0.42) but operators need a story.

**Status after 0.46:** Partially covered.

0.42 can report authority dry-run state and external-action cases, but operator
identity binding, profile-specific CLI enforcement, and break-glass workflow
remain open.

Still open backlog:

- **Authority profiles** in repo config (0.42) plus **CLI identity binding** — `canic deploy plan --as-profile staging-operator` fails if current `dfx identity` not in profile.
- Audit log export: who ran install, which receipt, which principal
  (`DeploymentReceiptV1.operator_principal` exists, but capture/export policy
  remains open).
- **Break-glass** explicit in plans with TTL and mandatory post-incident inventory diff.

---

### A.9 Disaster recovery beyond backup/restore

**Need:** Backup/restore is subtree-oriented and journal-heavy (powerful, heavy). Many teams want: snapshot all roles, restore one role, or clone deployment to new root.

**Status after 0.46:** Open backlog.

Backup/restore has deployment-shaped manifests, journals, and verification
coverage. Clone and lighter DR workflows remain separate post-46 work.

Still open backlog:

- **Deployment clone plan** — new trust domain, replay artifact set from receipt, explicit migration receipt chain.
- **Role-scoped DR** — backup/restore selectors already have subtree; expose “restore only role X on deployment Y” as first-class doc + CLI examples.
- Lighter **snapshot-only DR** path for teams that refuse full backup journal complexity.

---

### A.10 Observability and post-deploy verification

**Need:** After deploy, verify: all roles ready, cycles above threshold, protected calls work hub→instance, metrics sane.

**Status after 0.46:** Partially covered.

List, cycles, metrics, and deployment truth checks exist, but no deploy-gated
verification profile combines those checks into one post-deploy acceptance
surface.

Still open backlog:

- `canic deploy verify <deployment>` — run checklist: readiness, `canic_metadata` version skew, sample protected call (where applicable), cycle floors.
- Export metrics snapshot to file after install for future comparison
  baselines.
- Integration with external monitoring (OpenTelemetry export is **not** present — missing use case for enterprises).

---

### A.11 Non-Rust and hybrid fleets

**Need:** Fleet includes Motoko, legacy wasm, or partner canisters; root still orchestrates registry.

**Status after 0.46:** Partially covered.

0.44 added role artifact source and promotion identity vocabulary, but the
build/discovery path is still Rust/Cargo-centric.

Still open backlog:

- **Role artifact source** `external_wasm` with digest pin plus **no Canic
  build** for that role, represented in deployment truth inventory instead of
  the Rust build pipeline.
- Candid/interface validation only mode for external roles.
- Document that root/registry can reference roles Canic does not build (topology without `build!` for that role).

---

### A.12 Supply chain and reproducible builds

**Need:** Prove artifact X was built from git sha Y with profile Z.

**Status after 0.46:** Partially covered.

0.44 records source/build and materialization identity, but full reproducible
build policy, dirty-workspace enforcement, and signed provenance remain
post-46 work.

Still open backlog:

- Extend `RoleArtifact` with `source_revision`, `build_profile`, `cargo_hash`, `builder_version` (optional fields).
- **Reproducible build check** in plan — same inputs → same wasm sha256 within tolerance.
- Policy: refuse install if workspace dirty unless `--allow-dirty` recorded on receipt.

---

### A.13 Education / small-team simplification

**Need:** Two-canister app, no sharding/scaling/directory pools, no delegated auth.

**Status after 0.46:** Open backlog.

Still open backlog:

- **Tiered config profiles:** `minimal`, `standard`, `platform` with forbidden sections per tier.
- Lint: `canic config lint` rejects unused sections (e.g. sharding pools on leaf-only fleet).
- Default scaffold `auth.delegated_tokens.enabled = false` already in fleet create — extend to not imply platform features in examples.

---

## B. Critical changes to existing behavior (not only new features)

### B.1 `ICP_ENVIRONMENT` baked at compile time

**Problem:** Same source tree produces different wasm per `local` vs `ic` (`CONFIG.md`). You cannot promote **one sealed artifact** across environments unless config is identical — conflicts with real CI (build once) and with 0.44 promotion semantics.

**Change options (pick one and document breaking impact):**

1. **Runtime-selected environment** for non-security-critical config slices (large refactor).
2. **Require per-deployment rebuild** in all docs and plans — embed `deployment_name` + `network` in canonical config digest so “same wasm” across envs is never assumed.
3. **Split config:** security-invariant in wasm, environment-specific in init args (explicit pattern + macros).

Until fixed, “build once, deploy many” is a **footgun** for every adopter, not only Toko.

---

### B.2 `controllers = []` pretends to be policy but is not what IC shows

**Problem:** Documented as optional list appended to provisioned canisters; field reports (see `gabriel.md`) show installed controllers ≠ config, pool canisters not gaining root, metadata-only feel.

**Status after 0.46:** Partially covered.

0.41-0.42 added deployment truth and authority reconciliation reports. The
remaining adoption problem is active policy/application: controller mutation
remains outside the 0.42 dry-run boundary, and future docs must not imply that
`controllers` in config alone proves live IC controller state.

---

### B.3 `wasm_store` as hidden special case

**Problem:** Not in fleet tree; excluded from `configured_install_targets`; path mismatches panic root build (`gabriel.md`); not in operator mental model.

**Status after 0.46:** Partially covered.

0.41-0.44 moved `wasm_store` into deployment truth, artifact manifests,
promotion, and install-plan validation. Live catalog lookup and active
artifact-registry retention remain post-46 work.

---

### B.4 Silent or partial build before install

**Problem:** Role listed as install target but orchestrated build skipped role → install panics on missing `wasm.gz` (`gabriel.md`).

**Status after 0.46:** Partially covered.

Deployment truth and install-plan validation now catch missing materialized
artifacts before mutation in the current install path. A dedicated
`canic build --all-roles-for <fleet-template>` convenience command remains
open backlog, if still needed.

---

### B.5 Install state model

**Problem:** Fleet-named JSON keyed by directory name; conflicts deleted on root reuse; no deployment identity.

**Status after 0.46:** Resolved for current local deployment state.

0.46 stores live local state under deployment target identity and rejects old
fleet-named state as deployment truth. Richer catalog/history and operator
principal reporting remain separate backlog topics.

---

### B.6 Nonlocal IC “managed externally”

**Problem:** README/INSTALLING defer mainnet workflows — adopters hit a wall exactly where multi-deployment matters.

**Status after 0.46:** Open backlog.

0.46 is closed only as a deployment-target identity hard cut plus passive
two-target comparison line. Broader mainnet/operator workflows still need an
honest status page that distinguishes read-only inventory, explicit external
steps, and Canic-driven mutation.

Still open backlog:

- Document required external steps honestly and provide **inventory-only**
  against `ic` network as read-only truth.
- Clarify when an `icp` backend execution path counts as Canic-managed versus
  externally managed.
- Keep future "multi-deployment" claims scoped to the actual command surfaces
  that exist.

---

### B.7 Protected internal calls (0.40) raise migration cost for adopters

**Problem:** Sibling RPC requires envelope + root proofs; raw calls fail; hub/instance clients must migrate to `canic_internal_client!` / `CanicCall`.

**Change for adoption:**

- **Compatibility window** — detect and report roles still using raw internal calls in inventory (static + runtime probe).
- **Adopter guide:** minimal steps for one protected call path (not only test stubs).
- Considercodegen from `.did` for internal clients (missing — today macro-driven only).

---

### B.8 Pool import destructiveness

**Problem:** `pool.import` resets controllers and uninstalls code — catastrophic on brownfield.

**Change:**

- Default pool import to **dry-run** in inventory.
- Require explicit `--confirm-destructive-import` on install plan execute.
- Separate “register existing canister in pool” from “reset imported canister.”

---

### B.9 Backup/restore vs deploy asymmetry

**Problem:** Backup has plans, journals, row refs, integrity tests; deploy has `install` only. New adopters learn backup complexity before deploy honesty.

**Status after 0.46:** Partially covered.

0.41-0.46 added deployment plans, checks, receipts, promotion/external
evidence, plan-mediated install, and deployment-shaped backup terminology. A
simple operator guide that explains when backup is unnecessary remains open.

---

### B.10 Config validation only at compile time

**Problem:** Invalid topology fails at `canic::build!` — good for authors, bad for operators changing deploy overlays later.

**Change:**

- Host-side validator for deploy overlay + fleet template **before** build,
  reusing role artifact manifest evidence where available.
- `canic config validate --deployment staging` runnable in CI without compiling all wasm.

---

### B.11 `icp.yaml` read-only to Canic

**Problem:** Since 0.38.8, Canic does not sync canister entries from `canic.toml` — drift is manual.

**Change:**

- `canic fleet check` should diff **roles in toml** vs **canisters in icp.yaml** (if check exists, extend; if not, add).
- Emit icp.yaml patch suggestions (stdout or `--write` opt-in) — not auto-sync silently, but reduce integration friction.

---

### B.12 Memory ledger hard cut without operator migration tool

**Problem:** Old stable-memory layout fails closed; public docs mention destructive reset, not a guided migration.

**Change:**

- `canic memory migrate-plan` or document explicit unsupported path with inventory of affected canisters.
- Adopters on older Canic versions need a **version skew** section in install docs.

---

## C. Gaps in the 0.41–0.46 roadmap (should be explicit backlog)

| Topic | Why it matters for generic adoption |
| --- | --- |
| Canonical embedded config digest algorithm | Partially covered by deployment truth/promotion; keep spec and cross-role edge cases in the backlog. |
| Promotion artifact level: sealed vs source/build | Covered by 0.44; future work is operator workflow and live catalog/dedupe policy. |
| Automated resume vs resume report | Resume report exists; automated resume remains safety-sensitive backlog. |
| Concurrent install protection | Teams with multiple operators/CI jobs. |
| CLI/library API for inventory/plan | CLI surfaces exist; stable library/API contract is still backlog. |
| `canic-host` publish story | Downstream automation should not shell out for everything — document which APIs are stable. |
| Controller overlap validation | `staging_principals` must not appear in IC controllers (gabriel.md) — plan validation rule. |
| Spawned canister controller policy | AuthorityProfile vs per-spawn policy undefined (gabriel.md). |
| Frontend declaration pipeline | Not mentioned in deployment roadmap at all. |
| Brownfield adopt / partial fleet | 0.45 is user-owned only, not general import. |
| Single-canister / no-root mode | Not in deployment lines. |
| External observability | Metrics on-canister only. |
| Official CI Action | Adoption multiplier. |
| Dirty git / provenance | Supply chain adopters. |
| Version skew policy | `canic-cli` must match `canic` crate — enforce in install with clear error. |

---

## D. Proposed feature/changes summary (prioritized for “any project”)

### P0 — Keep Multi-Deployment Honest After 0.46

1. **Resolved by 0.41-0.46:** deployment-target local state, safety reports,
   deployment checks, receipts, `wasm_store` deployment-truth participation,
   plan-mediated install gates, and current operator terminology.
2. **Still open:** live comparison crawling, verified-root registration, and
   broader catalog/group UX.

### P1 — Integration ergonomics

3. `canic.project.toml` (or equivalent) for split monorepos.
4. Stable machine-readable exit codes for CI.
5. `canic fleet check` icp.yaml ↔ canic.toml role diff + suggestions.
6. Post-install **verify** profile (readiness + metadata skew).

### P2 — Adoption paths

7. Brownfield **adopt** + non-destructive pool registration.
8. Standalone / leaf-only documented profile without root fleet.
9. External wasm role sources (hybrid fleets).
10. Declaration emit hook for frontends.
11. Protected-call migration scanner in inventory.

### P3 — Maturity

12. Deployment clone / trust-domain migration plans.
13. CI deploy lock + optional signed receipts.
14. Build provenance fields on role artifacts.
15. Config lint tiers (`minimal` / `platform`).
16. GitHub Action wrapping plan/diff/inventory.

---

## E. Use-case checklist (quick scan for product planning)

| Use case | Served today? | Roadmap | Still needed |
| --- | --- | --- | --- |
| Local dev single fleet | Partial | — | Better lost-state UX |
| IC prod + staging + v2 | Partial | 0.41–0.46 complete | Deployment catalog/group UX |
| Per-role wasm override | Partial | 0.44 complete | Partial role install |
| Per-role config variant | No | 0.41 foundation complete | Build-once vs per-env clarity |
| Canary one role | No | 0.44/0.46 foundation complete | Same-root policy explicit |
| Promote tested wasm to prod | Partial | 0.44 complete | Live catalog/dedupe, attestation, config digest rules |
| User-controlled instance | Partial | 0.45 complete | SDK/frontend consent story |
| Import existing canister | Partial/destructive | 0.42/0.45 | Adopt + dry-run import |
| CI plan-only gate | Partial | 0.41 complete | Exit codes + Action |
| Split repo frontend+backend | Partial | 0.43 foundation complete | Declarations + project manifest |
| Single canister, no root | Poor | — | Standalone profile |
| Hybrid Motoko/legacy wasm | No | 0.44 partial | External role docs |
| Brownfield dfx→canic | No | — | Migration guide + inventory |
| Operator RBAC / break-glass | No | 0.42 foundation complete | CLI identity binding |
| DR clone deployment | No | Post-46 backlog | Clone command |
| Reproducible/supply chain | Partial | — | Provenance fields |
| Post-deploy verification | Partial | — | `deploy verify` |
| External metrics/OTel | No | — | Export pipeline |
| Library-only Canic | Poor | — | Features matrix |

---

## F. Doc hygiene (avoid wrong implementation)

| Document | Issue |
| --- | --- |
| `docs/design/0.41-deployment-flexibility/` | Superseded by `0.41-deployment-truth-model/` — remove or redirect to avoid duplicate truths. |
| INSTALLING vs deployment roadmap | Adopters read install guide, not 0.41 design — gap between “what works now” and “what’s planned” should be one page (`DEPLOYMENT-STATUS.md` or section in OBSERVATIONS). |
| This inventory | Must remain reconciled backlog source material under `post-46-backlog/`; do not treat it as an approved release design. |

---
