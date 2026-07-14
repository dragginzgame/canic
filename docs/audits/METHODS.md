# Active Audit Method Catalog

This is the canonical catalog for reusable Canic audit methods. The catalog
assigns one owner and one Phase B disposition to every candidate found by the
0.92 inventory. A definition is active only when this catalog names its method
ID and current definition path.

Global run, state, safety, comparison, evidence, and retention rules are in
[AUDIT-HOWTO.md](AUDIT-HOWTO.md). Cross-method authority is in
[META-AUDIT.md](META-AUDIT.md). Prepared/frozen content identities are recorded
in [method-fingerprints-v1.md](method-fingerprints-v1.md).

## Ownership Rules

- `CANIC-LAYERING-001` owns hard dependency-direction and layer-responsibility
  decisions, including access, workflow, policy, ops, model, DTO, record, view,
  conversion, and side-effect placement.
- Auth invariant methods own individual trust properties.
  `CANIC-AUTH-ORDERING-001` owns only cross-stage enforcement order and does
  not rescore the individual invariant.
- Structure, dependency, publish, capability, duplication, complexity, and
  change-friction methods own distinct pressure/contract dimensions; their
  scans may overlap, but a hard finding is assigned to the canonical owner.
- Measured instruction and Wasm methods own only their frozen metrics and
  cannot establish functional correctness.
- Release validation decisions come from the current validation matrix and a
  dated release-line closeout, never from a standing readiness verdict.

## System Methods

| Candidate definition | Disposition | Active method | Kind/profile | Canonical owner and trigger |
| --- | --- | --- | --- | --- |
| `access-purity.md` | `merge` | `CANIC-LAYERING-001/v1` | invariant | Layering owns access-boundary placement; run after access or endpoint-auth changes. |
| `bootstrap-lifecycle-symmetry.md` | `revise` | `CANIC-LIFECYCLE-001/v1` | invariant/manual | Lifecycle boundary; run after lifecycle, restore, bootstrap, timer, or start-macro changes. |
| `capability-surface.md` | `revise` | `CANIC-CAPABILITY-SURFACE-001/v1` | trend/invariant | Public capability and generated endpoint surface; run after endpoint bundle/Candid changes. |
| `change-friction.md` | `revise` | `CANIC-CHANGE-FRICTION-001/v1` | trend/manual | Empirical edit blast radius; run for hardening/refactor planning with a frozen commit sample. |
| `complexity-accretion.md` | `revise` | `CANIC-COMPLEXITY-001/v1` | trend/manual | Structural complexity; run after cross-cutting model/control-flow growth. |
| `dependency-hygiene.md` | `revise` | `CANIC-DEPENDENCY-001/v1` | invariant/trend | Cargo graph, feature, advisory, license, and lockfile posture; run after dependency/package graph changes and before closeout. |
| `dry-consolidation.md` | `revise` | `CANIC-DUPLICATION-001/v1` | manual | Duplicate behavior/authority; run after broad host/CLI/runtime workflow work. |
| `instruction-footprint.md` | `revise` | `CANIC-INSTRUCTION-001/v1` | measured | Runtime instruction measurement; run after relevant hot-path changes or explicit perf review. |
| `layer-violations.md` | `revise` | `CANIC-LAYERING-001/v1` | invariant/manual | Canonical architecture owner; run after layer, data-shape, conversion, endpoint, workflow, policy, ops, or model changes. |
| `module-structure.md` | `revise` | `CANIC-STRUCTURE-001/v1` | invariant/trend | Module topology and visibility; run after crate/module/public-surface changes. |
| `ops-purity.md` | `merge` | `CANIC-LAYERING-001/v1` | invariant | Layering owns ops responsibility and side-effect placement. |
| `publish-surface.md` | `revise` | `CANIC-PUBLISH-001/v1` | invariant/trend | Published package and downstream contract; run after features, packaging, docs.rs, examples, or public crate changes. |
| `security-boundary-ordering.md` | `revise` | `CANIC-AUTH-ORDERING-001/v1` | invariant/manual | Cross-stage auth/replay/capability order; run after security-boundary sequencing changes. |
| `wasm-footprint.md` | `revise` | `CANIC-WASM-001/v1` | measured | Built/shrunk/gzip Wasm metrics; run after Wasm-affecting changes or explicit size review. |
| `workflow-purity.md` | `merge` | `CANIC-LAYERING-001/v1` | invariant | Layering owns workflow responsibility, records, conversions, effects, and typed error placement. |
| `build-integrity.md` | `retain` | `CANIC-BUILD-INTEGRITY-001/v1` | invariant/measured | Build scripts, macros, generated code, unsafe inventory, and reproducibility; run before closeout and after build-pipeline changes. |
| `release-integrity.md` | `retain` | `CANIC-RELEASE-INTEGRITY-001/v1` | invariant/manual | CI permissions, action pinning, secret scanning, artifact provenance/checksums, host/target matrix; run before closeout and after CI/release changes. |

## Authentication Invariant Methods

| Definition | Disposition | Active method | Canonical property |
| --- | --- | --- | --- |
| `audience-target-binding.md` | `revise` | `CANIC-AUTH-AUDIENCE-001/v1` | Signed audience/target/local-role binding. |
| `auth-abstraction-equivalence.md` | `revise` | `CANIC-AUTH-EQUIVALENCE-001/v1` | Equivalent auth abstractions enforce the same property set. |
| `canonical-auth-boundary.md` | `revise` | `CANIC-AUTH-BOUNDARY-001/v1` | Authenticated entrypoints converge on the canonical verifier boundary. |
| `capability-scope-enforcement.md` | `revise` | `CANIC-AUTH-CAPABILITY-001/v1` | Capability and local-role scopes are enforced before execution. |
| `expiry-replay-single-use.md` | `revise` | `CANIC-AUTH-REPLAY-001/v1` | Expiry, replay identity, and single-use/domain-receipt boundaries. |
| `subject-caller-binding.md` | `revise` | `CANIC-AUTH-SUBJECT-001/v1` | Verified subject is bound to the transport caller. |
| `token-trust-chain.md` | `revise` | `CANIC-AUTH-TRUST-001/v1` | Root, issuer, proof, and token trust chain. |

All seven are `invariant` profile methods owned by the authentication boundary.
Run the affected method after a local property change and the full set before
release-line closeout.

## Modular Candidates

| Candidate | Disposition | Outcome |
| --- | --- | --- |
| `module-surface-hardening.md` | `manual_only` | `CANIC-MODULE-SURFACE-001/v2.0`; versioned reviewer protocol for requested module-surface work. |
| `module-cleanup-runner.md` | `retire` | Retired as an independent audit. It remains only as a finding-backed implementation workflow and cannot issue a separate audit verdict. |

## Operational Candidates

| Candidate | Disposition | Outcome |
| --- | --- | --- |
| `docs/operations/diagnostic-consistency-audit.md` | `retire` | Historical conclusion remains at `v0.91.6`; current diagnostic contract belongs to maintained operator docs and finding-backed audits. |
| `docs/operations/upgrade-state-compatibility-audit.md` | `retire` | Historical conclusion remains at `v0.91.6`; stable-state evidence belongs to current methods and dated closeout. |
| `docs/operations/rc-readiness-audit.md` | `retire` | No standing readiness verdict. Current line status plus a dated release-line closeout own the decision. |

The corresponding three literal historical-verdict guards are retired. No
alias or wrapper remains.

## Executable Components

| Component | Disposition | Method ownership |
| --- | --- | --- |
| `scripts/ci/check-audit-method-catalog.sh` | `retain` | Mechanical conformance guard for `CANIC-META-001`; it validates current method metadata but cannot issue a product verdict. |
| `scripts/ci/audit-product-tree-hash.sh` | `retain` | Snapshot helper for the versioned product-path scope; it hashes committed Git objects only. |
| `scripts/ci/instruction-audit-report.sh` | `revise` | `CANIC-INSTRUCTION-001`; same-day baseline and isolated execution contract. |
| `scripts/ci/run-layering-guards.sh` | `retain` | Mechanical subset of `CANIC-LAYERING-001`; a passing guard is not the complete manual verdict. |
| `scripts/ci/wasm-audit-report.sh` | `revise` | `CANIC-WASM-001`; isolated execution and evidence manifest required. |

## Holistic Coverage Ownership

| Required topic | Canonical method |
| --- | --- |
| Dependency advisories, licenses, lockfile integrity | `CANIC-DEPENDENCY-001` |
| `build.rs`, procedural macro, generated-code trust | `CANIC-BUILD-INTEGRITY-001` |
| Unsafe code inventory and justification | `CANIC-BUILD-INTEGRITY-001` |
| Reproducible/explained Wasm builds | `CANIC-BUILD-INTEGRITY-001` with `CANIC-WASM-001` measurements |
| CI permissions and action pinning | `CANIC-RELEASE-INTEGRITY-001` |
| Secret scanning | `CANIC-RELEASE-INTEGRITY-001` |
| Release artifact provenance/checksums | `CANIC-RELEASE-INTEGRITY-001` |
| Supported host/target matrix | `CANIC-RELEASE-INTEGRITY-001` |

## Follow-Up Ownership

- Method failure: the catalog's canonical repository owner triages the finding.
- Partial or blocked invariant: product baselining stops until resolved.
- Partial or blocked trend/measured method: record an explicit limitation;
  closeout is at best `pass_with_limitations` when the method is informational.
- P0/P1 manual finding: maintainer review plus a second reviewer or explicit
  single-review waiver.
