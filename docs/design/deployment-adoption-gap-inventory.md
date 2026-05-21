# Deployment Adoption Gap Inventory

This is product-gap source material for the 0.41-0.50 roadmap. It is not
itself a release contract.

The normative design is in the per-line design docs and
`0.41-deployment-truth-model/0.41-0.50-deployment-roadmap.md`.

---

## A. Use cases the product does not serve today (and may still miss after 0.46)

### A.1 Multiple real IC deployments (no testnet)

**Need:** Run prod, staging, v2, canary, and destructive-rehearsal lanes on IC as separate deployments that share one topology template but differ in root, controllers, wasm per role, and embedded config per role.

**Gap today:** One install name, one `.canic/<network>/fleets/<name>.json`, global `controllers = []`, rediscovery on every install. Reusing a root id **deletes** other fleet install records (`install_root/state.rs`).

**Roadmap:** 0.41 truth model + 0.46 deployment groups partially target this. Still missing unless 0.46 ships operator commands people actually run daily, not only JSON schemas.

**Still missing after roadmap (propose explicitly):**

- **Deployment catalog** — list/search deployments by group, environment, owner team, not only fleet directory names.
- **Lane teardown** — retire a test deployment (controllers, pool release policy, `.canic` state) without manual IC archaeology.
- **Cost attribution per deployment** — cycles burned per lane for finance/ops (not in 0.41–0.46).

---

### A.2 Per-role wasm and per-role config on different lanes

**Need:** Staging runs `project_hub` v1 + `project_instance` v2 candidate; prod runs all v1; one role from a pinned `.wasm.gz`, others from workspace packages.

**Gap today:** One discovered package per role per fleet config; config variance requires different `CANIC_CONFIG_PATH` builds, not a host-side matrix.

**Roadmap:** 0.44 artifact overrides + promotion.

**Still missing:**

- **Mixed-lane install in one command** — install only roles `{X,Y}` on deployment `staging` without rebuilding the whole release set.
- **Config-independent roles** — explicit manifest flag for roles where sealed wasm can promote across trust domains (rare; must be deliberate).
- **Attestation-aware promotion** — after 0.40, promoting instance wasm without hub/root epoch alignment is a **runtime** failure; promotion UX must surface verifier/epoch prerequisites, not only wasm sha256.

---

### A.3 Brownfield: Canic wraps an existing IC project

**Need:** Team already has canisters on IC (dfx/icp legacy layout, hand-rolled controllers, SNS-upgraded canisters). They want root + topology **without** reinstalling everything from zero.

**Gap today:**

- No first-class **import canister into fleet registry** with `CanisterControlClass` = `ExternallyImported` / `JointlyControlled` (enum appears in 0.45 design only).
- Pool `import.*` is destructive (controllers reset, code uninstalled per `CONFIG.md`) — terrifying for brownfield.
- No guided “adopt this principal as role `foo`” flow.

**Propose (not in roadmap):**

- `canic fleet adopt <role> <principal>` — read-only inventory first, then optional registry entry + controller reconciliation plan (0.42).
- **Import dry-run** — show exactly which management calls would run before pool import mutates live canisters.
- **Partial fleet** — runtime topology lists roles where only some are Canic-managed; others are observe-only forever (0.45 touches user-owned, not general brownfield).

---

### A.4 Library-only or single-canister adoption

**Need:** Project wants `canic` macros + stable memory + auth on **one** canister, no root fleet, no wasm store, no thin-root staging.

**Gap today:**

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

**Gap today:**

- `CANIC_WORKSPACE_ROOT` / `CANIC_ICP_ROOT` exist but are easy to misconfigure; install state does not record them in a way CI can verify.
- No official hook: “after build, emit declarations to `frontend/src/declarations`.”
- Downstreams still wrap `canic` in shell scripts (common in large apps).

**Propose:**

- `canic build --emit-declarations <dir>` (or integrate with existing `generate_declarations` patterns as a documented contract).
- **Project manifest** — optional `canic.project.toml` at repo root mapping workspace root, icp root, declaration output, deployment group name (single file for CI).
- JSON output on all operator commands for CI parsers (`--json` is inconsistent across command families today).

---

### A.6 CI/CD and GitOps-shaped workflows

**Need:** Pipeline builds artifacts, pins digests, runs plan diff against prod inventory, applies only on approval; no interactive install prompts.

**Gap today:**

- Install can prompt for config choice when multiple `canic.toml` exist.
- No `canic deploy plan` / inventory in CLI (design only).
- No policy gate: “fail if `SafetyReport` contains `unsafe_blocked`.”

**Roadmap:** 0.41 inventory + safety report; 0.43 executor abstraction.

**Still missing:**

- **Stable machine-readable exit codes** per diff category (controller drift vs module hash vs config digest).
- **Signed plans / receipts** — optional Sigstore/cosign-style attestation for artifact promotion between CI stages (not in roadmap).
- **Concurrency lock** — two CI jobs must not install the same deployment; no lease/lock object in designs.
- Official **GitHub/GitLab Action** that wraps inventory/plan/diff (adoption friction for “any project”).

---

### A.7 User- or customer-controlled canisters in a fleet topology

**Need:** SaaS model: fleet provisions `project_instance`; end user is IC controller; fleet must propose upgrades, not push them.

**Gap today:** No lifecycle consent flow in CLI.

**Roadmap:** 0.45 `ExternalUpgradeProposalV1`, control classes.

**Still missing:**

- **Operator playbook** — document flows for security patch on user-controlled role (proposal → consent → verify → receipt).
- **Frontend/SDK contract** — how a web app requests user consent aligned with Canic receipts (0.45 is canister-side only).
- **Fleet upgrade must not block** on user-controlled roles — partial fleet upgrade semantics need to be default, not edge case.

---

### A.8 Operator teams, RBAC, and break-glass

**Need:** Platform team vs app team vs emergency break-glass; staging principals must not be IC controllers (design notes in 0.42) but operators need a story.

**Gap today:** Single operator identity via `icp`/`dfx` identity; `controllers = []` in config.

**Propose:**

- **Authority profiles** in repo config (0.42) plus **CLI identity binding** — `canic deploy plan --as-profile staging-operator` fails if current `dfx identity` not in profile.
- Audit log export: who ran install, which receipt, which principal (receipts need operator principal field — verify in 0.41 schema).
- **Break-glass** explicit in plans with TTL and mandatory post-incident inventory diff.

---

### A.9 Disaster recovery beyond backup/restore

**Need:** Backup/restore is subtree-oriented and journal-heavy (powerful, heavy). Many teams want: snapshot all roles, restore one role, or clone deployment to new root.

**Gap today:** Restore tied to backup layouts; no “clone deployment identity to new root” migration object (0.41 mentions migration but no operator command).

**Propose:**

- **Deployment clone plan** — new trust domain, replay artifact set from receipt, explicit migration receipt chain.
- **Role-scoped DR** — backup/restore selectors already have subtree; expose “restore only role X on deployment Y” as first-class doc + CLI examples.
- Lighter **snapshot-only DR** path for teams that refuse full backup journal complexity.

---

### A.10 Observability and post-deploy verification

**Need:** After deploy, verify: all roles ready, cycles above threshold, protected calls work hub→instance, metrics sane.

**Gap today:** `canic info list` + readiness; `canic metrics` exists but no deploy-gated **verification profile**.

**Propose:**

- `canic deploy verify <deployment>` — run checklist: readiness, `canic_metadata` version skew, sample protected call (where applicable), cycle floors.
- Export metrics snapshot to file after install for 0.46 comparison baselines.
- Integration with external monitoring (OpenTelemetry export is **not** present — missing use case for enterprises).

---

### A.11 Non-Rust and hybrid fleets

**Need:** Fleet includes Motoko, legacy wasm, or partner canisters; root still orchestrates registry.

**Gap today:** Rust-centric build pipeline; discovery assumes Cargo packages.

**Propose:**

- **Role artifact source** `external_wasm` with digest pin (0.44) plus **no Canic build** for that role — must be in 0.41 inventory.
- Candid/interface validation only mode for external roles.
- Document that root/registry can reference roles Canic does not build (topology without `build!` for that role).

---

### A.12 Supply chain and reproducible builds

**Need:** Prove artifact X was built from git sha Y with profile Z.

**Gap today:** Wasm hash in install table; no build provenance object.

**Propose:**

- Extend `RoleArtifact` with `source_revision`, `build_profile`, `cargo_hash`, `builder_version` (optional fields).
- **Reproducible build check** in plan — same inputs → same wasm sha256 within tolerance.
- Policy: refuse install if workspace dirty unless `--allow-dirty` recorded on receipt.

---

### A.13 Education / small-team simplification

**Need:** Two-canister app, no sharding/scaling/directory pools, no delegated auth.

**Gap today:** `canic fleet create` scaffolds minimal root+app but CONFIG.md and features expose enormous surface; delegated auth defaults enabled in scaffold TOML.

**Propose:**

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

**Change:**

- 0.41 inventory must query **live IC controllers** on every canister and diff vs plan (design says this — must ship).
- Until reconciliation (0.42), `canic install` should warn loudly when post-install controller set ≠ configured expectation.
- Stop implying `controllers` in toml alone is sufficient — document required `update_settings` reconciliation path.

---

### B.3 `wasm_store` as hidden special case

**Problem:** Not in fleet tree; excluded from `configured_install_targets`; path mismatches panic root build (`gabriel.md`); not in operator mental model.

**Change:**

- Treat as normal role in inventory/plan immediately (0.41 design says so — implement).
- Validate artifact paths in plan **before** root `build.rs` reads files (fail with `SafetyReport`, not panic).
- Single canonical artifact root env (`CANIC_ICP_ROOT` / `.icp`) — reject `.dfx` path drift in validation.

---

### B.4 Silent or partial build before install

**Problem:** Role listed as install target but orchestrated build skipped role → install panics on missing `wasm.gz` (`gabriel.md`).

**Change:**

- **Materialization gate** — mandatory in 0.41 post-build checks: every planned role has file + digest or install refuses.
- `canic build --all-roles-for <fleet>` that fails if any role missing.

---

### B.5 Install state model

**Problem:** Fleet-named JSON keyed by directory name; conflicts deleted on root reuse; no deployment identity.

**Change:**

- Persist under **deployment id**, not fleet folder name.
- Allow multiple deployment records per fleet template.
- Record: digests, profiles, workspace/icp roots, operator principal, phase receipts (when 0.41 lands).

---

### B.6 Nonlocal IC “managed externally”

**Problem:** README/INSTALLING defer mainnet workflows — adopters hit a wall exactly where multi-deployment matters.

**Change:**

- Short term: document required external steps honestly + provide **inventory-only** against `ic` network (read-only truth).
- Medium: 0.43 executor with `icp` backend still counts as “managed” if Canic drives it — clarify wording.
- Do not label 0.46 “multi-deployment” complete if `install` still local-first.

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

**Change:**

- Align UX vocabulary: `plan`, `receipt`, `row ref` across backup and deploy (0.41+).
- Offer **lightweight deploy receipt** before full backup-grade journal infrastructure.
- Document when backup is unnecessary for small fleets (reduce perceived mandatory complexity).

---

### B.10 Config validation only at compile time

**Problem:** Invalid topology fails at `canic::build!` — good for authors, bad for operators changing deploy overlays later.

**Change:**

- Host-side validator for deploy overlay + fleet template **before** build (0.41 role artifact manifest).
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
| Canonical embedded config digest algorithm | Without spec + tests, drift/promotion warnings become noise (predesign High 2). |
| Promotion artifact level: sealed vs source/build | Without it, staging→prod footgun remains (predesign High 1). |
| Automated resume vs resume report | Resume skipping phases is high-risk; default should be report-only first (predesign High 3). |
| Concurrent install protection | Teams with multiple operators/CI jobs. |
| CLI/library API for inventory/plan | Rust consumers (CI tools) need stable crate surface, not only `canic` binary. |
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

### P0 — Unblock honest multi-deployment (align with 0.41–0.42, ship in CLI)

1. Live **inventory** (IC controllers, module hashes, embedded config digest per role, pool state) — not self-reporting `canic.toml`.
2. **Safety report** that blocks install on mismatch — not post-hoc panic on missing wasm.
3. **Deployment receipt** on every install with digests and paths used.
4. **wasm_store** in inventory as normal role + path validation.
5. Rename operator concepts in CLI: **fleet template** vs **deployment target** (reduce `fleet` overload).

### P1 — Integration ergonomics

6. `canic.project.toml` (or equivalent) for split monorepos.
7. Consistent `--json` and exit codes for CI.
8. `canic fleet check` icp.yaml ↔ canic.toml role diff + suggestions.
9. Materialization gate before stage/install.
10. Post-install **verify** profile (readiness + metadata skew).

### P2 — Adoption paths

11. Brownfield **adopt** + non-destructive pool registration.
12. Standalone / leaf-only documented profile without root fleet.
13. External wasm role sources (hybrid fleets).
14. Declaration emit hook for frontends.
15. Protected-call migration scanner in inventory.

### P3 — Maturity

16. Deployment clone / trust-domain migration plans.
17. CI deploy lock + optional signed receipts.
18. Build provenance fields on role artifacts.
19. Config lint tiers (`minimal` / `platform`).
20. GitHub Action wrapping plan/diff/inventory.

---

## E. Use-case checklist (quick scan for product planning)

| Use case | Served today? | Roadmap | Still needed |
| --- | --- | --- | --- |
| Local dev single fleet | Partial | — | Better lost-state UX |
| IC prod + staging + v2 | No | 0.41–0.46 | CLI + deployment catalog |
| Per-role wasm override | No | 0.44 | Partial role install |
| Per-role config variant | No | 0.41 variants | Build-once vs per-env clarity |
| Canary one role | No | 0.44/0.46 | Same-root policy explicit |
| Promote tested wasm to prod | No | 0.44 | Attestation + config digest rules |
| User-controlled instance | No | 0.45 | SDK/frontend consent story |
| Import existing canister | Partial/destructive | 0.42/0.45 | Adopt + dry-run import |
| CI plan-only gate | No | 0.41 | Exit codes + Action |
| Split repo frontend+backend | Partial | 0.43 | Declarations + project manifest |
| Single canister, no root | Poor | — | Standalone profile |
| Hybrid Motoko/legacy wasm | No | 0.44 partial | External role docs |
| Brownfield dfx→canic | No | — | Migration guide + inventory |
| Operator RBAC / break-glass | No | 0.42 | CLI identity binding |
| DR clone deployment | No | 0.41 migration mention | Clone command |
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

---
