# Canic 0.41-0.50 Program State Audit

## Executive Summary

Verdict: **MOSTLY COMPLETE / CLOSEOUT DEBT REMAINS**.

Canic has become a deployment-evidence and fleet-lifecycle tool rather than a
simple root installer. The 0.41-0.50 arc is coherent: deployment truth, dry-run
authority evidence, execution boundaries, artifact promotion, external
lifecycle reporting, deployment-target identity, verified registered-root
recovery, metadata-driven setup, role lifecycle, and passive adoption reporting
now form one model.

What Canic can safely do today:

- represent deployment truth as plans, inventories, safety reports, receipts,
  artifact manifests, comparison reports, and root-verification evidence;
- keep authority, external lifecycle, and adoption workflows passive unless a
  deliberately active command boundary is used;
- enforce fleet-scoped package identity with `[package.metadata.canic] fleet`
  plus `role`;
- let roles be declared before topology attachment while keeping
  deploy/build/install/truth surfaces attached-role strict;
- generate read-only adoption reports from config and supplied evidence without
  importing, attaching topology, mutating controllers, or installing code.

What Canic still cannot safely do:

- treat adoption reports as executable import plans;
- mutate controller authority from the 0.42 authority reports;
- execute external lifecycle actions from the 0.45 proposal/handoff artifacts;
- use `wasm_store` as a provenance-rich registry with retention/GC semantics;
- provide a stable public JSON envelope, signing, CI lock, or GitOps contract;
- broadly verify live deployments beyond the specific registered-root
  verification path.

The main implementation risk found during this audit is that
`cargo test --workspace --locked` currently fails in
`canic-tests::pic_ingress_payload_limits`: `payload_limit_probe` installs with
a standalone declared-only config and traps because subnet `prime` is missing.
That is not a 0.50 adoption-report defect, but it is release-closeout debt for
the overall 0.41-0.50 arc.

The main documentation risk is that older design docs remain marked
"Tentative". The 0.49 role-only metadata follow-up identified during this audit
has since been resolved in the 0.49 design and closeout audit. Active setup
docs and 0.50 docs are aligned; remaining design-source closeout is mostly
older implemented-line status wording.

Recommendation: fix the workspace test failure and design-doc closeout debt,
then make the next numbered line **0.51 - CI/GitOps provenance and stable
evidence envelopes**. That line best fits the architecture Canic now has:
many strong evidence artifacts, but no stable automation contract tying them
together.

## One-Paragraph Product Description

Canic is now a fleet-scoped IC deployment evidence system. Canister packages
declare `fleet` and `role`, fleets declare roles independently of topology, and
only attached roles become artifact-build, install, or deployment-truth
targets. Deployment truth is represented through passive plans, inventories,
diffs, safety reports, receipts, artifact manifests, authority evidence, root
verification evidence, and comparison reports. Setup is metadata-driven through
`canic::build!` and `canic::start!()`. Adoption is currently report-only:
operators can classify brownfield, partial, standalone, leaf-only, hybrid
external-Wasm, and minimal states from supplied evidence, but Canic does not
import, mutate controllers, attach topology, install Wasm, or bless observed
state from adoption reports.

## Timeline: 0.41 Through 0.50

| Line | Intended theme | What actually landed | Main capability gained | Passive or active? | Evidence | Caveats |
| --- | --- | --- | --- | --- | --- | --- |
| 0.41 | Deployment truth model | Plans, inventories, diffs, safety reports, receipts, install gates, local live-root inventory, duplicate-evidence hardening. | Operators can inspect and gate installs against deployment truth. | Mostly passive; install gate is active because it blocks/receipts current install phases. | `docs/design/0.41-deployment-truth-model/status.md`; `crates/canic-host/src/deployment_truth/*`; `install_truth_*` tests. | Authority, executor semantics, promotion, locks, and catalogs deferred. |
| 0.42 | Authority reconciliation | Dry-run authority plans, reports, evidence bundles, receipts, text/JSON output, mutation source guards. | Controller drift can be classified without changing controllers. | Passive/read-only. | `docs/design/0.42-authority-reconciliation/status.md`; `deployment_truth/authority.rs`; `deployment_truth/receipt.rs`; authority CLI tests. | No controller apply path. |
| 0.43 | Backend-agnostic execution boundary | Executor/result boundaries, preflight evidence, operation receipts, current-install runner bridge. | Mutating install phases have clearer execution artifacts and receipts. | Active only through explicit install execution; reports/preflights are passive. | `docs/changelog/0.43.md`; `install_root` operation/receipt tests. | Stable public CI JSON contract deferred. |
| 0.44 | Artifact promotion | Promotion readiness, artifact identity, plan transforms, provenance, wasm_store identity/catalog verification, promotion execution receipts. | Artifact movement can be planned, checked, and receipted. | Mix: promotion reports are passive; plan-mediated install/promotion receipt path can be active. | `deployment_truth/promotion.rs`; `docs/changelog/0.44.md`; promotion tests in `deployment_truth/tests.rs` and `install_root/tests.rs`. | `wasm_store` is still not a full registry/retention system. |
| 0.45 | External lifecycle | Passive external lifecycle plans, proposals, consent evidence, verification policies/checks, completion, handoff, pending, critical-fix reports. | User/external-controlled lifecycle work can be represented without taking ownership. | Passive/report-only. | `docs/design/0.45-external-lifecycle/status.md`; `deployment_truth/lifecycle.rs`; `canic deploy external ...` parser/tests. | No consent delivery or external execution. |
| 0.46 | Multi-deployment operations | Deployment-target state hard cut, archived check comparison, explicit unverified root registration, old fleet state rejection, target-wording cleanup. | Concrete deployments are separate from reusable fleet templates. | Mostly passive; `deploy register` writes unverified local state. | `docs/design/0.46-multi-deployment-operations/status.md`; `deployment_truth/multi.rs`; `install_root` deployment-state tests. | Verified-root registration deferred to 0.47; groups/catalog/teardown deferred. |
| 0.47 | Verified deployment registration | Root observation evidence, passive root inspection, explicit root verification command, guarded local-state promotion, receipts, digest/source guards. | A registered root can become verified only from bound deployment-truth evidence. | `deploy root inspect` passive; `deploy root verify` mutates local trust state only. | `docs/design/0.47-verified-deployment-registration/status.md`; `deployment_truth/root.rs`; `verify_registered_deployment_root_*` tests. | No broad live deployment verification or root rotation. |
| 0.48 | Setup/macros cleanup | Metadata-driven `build!`/`start!`, removed `start_root!`, single normal startup macro, canister artifact boundary, token/cycles wrappers, singular delegated-token audience. | Canister setup surface became smaller and stricter. | Build/start macros active at compile/runtime; token/cycles wrappers call external ICP tools. | `crates/canic/src/macros/{build,start}.rs`; `crates/canic/src/build_support/config.rs`; `docs/changelog/0.48.md`; setup docs. | Some historical audit docs still mention `start_root!`; active public API does not. |
| 0.49 | Role lifecycle foundation | Fleet-scoped role declarations, declared-only roles, attach/list/inspect/rename, strict `canic build <fleet> <role>`, deployable selector hardening. | Developers can stage roles before topology attachment without weakening deploy/build truth. | Role declare/attach/rename mutate config/package files; list/inspect are passive; build is active artifact generation. | `docs/audits/release-lines/0.49-closeout.md`; `release_set/config.rs`; `fleets/mod.rs`; `workspace_manifest.rs`. | Original design-doc role-only wording follow-up is resolved. |
| 0.50 | Adoption profiles and safe onboarding | Host adoption model, read-only CLI, evidence inputs, deployment-check/cargo-metadata consumption, authority-gated recommendations, missing/stale evidence, evidence-conflict, JSON experimental decision. | Operators can understand brownfield/partial/standalone/leaf-only/hybrid states without Canic taking ownership. | Passive/read-only. | `docs/audits/release-lines/0.50-closeout.md`; `crates/canic-host/src/adoption.rs`; `crates/canic-cli/src/fleets/mod.rs`; adoption tests. | No import/mutation behavior by design. |

## Architecture Map

- Package metadata identity: `PackageCanicMetadata { fleet, role }` in
  `crates/canic/src/build_support/config.rs` reads
  `[package.metadata.canic] fleet` and `role`. `required_package_metadata`
  fails if either is absent.
- Fleet/role declaration model: `FleetRoleRefV1` and `RoleDeclaration` live in
  `crates/canic-core/src/config/schema/role.rs`; declarations are stored under
  `[roles.<role>]` inside a single-fleet `canic.toml` with `[fleet] name`.
- Declared-only vs attached roles: `ConfigModel::declares_role`,
  `attached_roles`, and `attached_fleet_roles` distinguish declaration from
  topology reachability. `attached_roles` traverses direct topology canisters
  and role-bearing child roles.
- Topology attachment: `canic fleet role attach` calls
  `attach_fleet_role_source` in `crates/canic-host/src/release_set/config.rs`.
  Topology references unknown roles fail validation.
- Build/start macro identity: `canic::build!` validates package `fleet.role`
  and emits `CANIC_FLEET`, `CANIC_CANISTER_ROLE`, `CANIC_FLEET_ROLE`,
  `CANIC_CANISTER_ROLE_DECLARED`, and `CANIC_CANISTER_ROLE_ATTACHED`.
  `canic::start!()` dispatches root vs non-root from build cfg/env emitted by
  `build!`.
- Deployment truth: `crates/canic-host/src/deployment_truth` owns plans,
  inventories, diffs, safety reports, authority, promotion, lifecycle,
  comparison, receipts, and root verification.
- Inventories and artifact manifests: local deployment inventories and role
  artifact manifests are generated from deployable roles, not declared-only
  roles.
- Deployment checks: `DeploymentCheckV1` binds plan, inventory, diff, and
  safety report. Install gates and adoption evidence inputs consume this
  object.
- Authority evidence: 0.42 authority reports/evidence/receipts are dry-run
  artifacts derived from deployment checks; they do not mutate controllers.
- External lifecycle: 0.45 objects in `deployment_truth/lifecycle.rs` model
  proposals, consent evidence, verification checks, completion, handoff, and
  pending reports without executing external work.
- Adoption profiles: `crates/canic-host/src/adoption.rs` owns
  `AdoptionProfileV1`, `AdoptionReportV1`, classifications, supplied evidence,
  recommendations, blocked actions, and summary counts.
- Adoption evidence inputs: `crates/canic-cli/src/fleets/mod.rs` reads
  `--inventory`, `--artifact-manifest`, `--package-metadata`,
  `--deployment-check`, and `--cargo-metadata`, then passes values to host
  adoption code.
- Recommendation/blocking model: adoption reports render
  `suggested_action_preview`, `status: not executed by adoption report`, and
  blocked actions. Recommendations may describe future actions but never
  execute them.

## Passive vs Active Boundary

| Surface | Current behavior | Boundary | Evidence | Safety invariant | Gaps |
| --- | --- | --- | --- | --- | --- |
| `canic build <fleet> <role>` | Builds selected attached role artifact. | Active artifact generation. | `crates/canic-cli/src/build.rs`; tests `build_preflight_rejects_declared_only_role`, `build_resolves_config_from_selected_fleet`. | Role must be declared and attached before Cargo artifact build starts. | No issue found. |
| `canic fleet role declare` | Adds config-only ordinary role declaration. | Active config mutation. | `declare_fleet_role_source`; tests `declare_fleet_role_*`. | Rejects root and duplicates; does not attach topology. | No issue found. |
| `canic fleet role attach` | Adds direct topology attachment. | Active config mutation. | `attach_fleet_role_source`; tests `attach_fleet_role_*`. | Source role must exist; root/duplicates/unknown kind fail closed. | Direct attachment only; broader placement UX remains future work. |
| `canic fleet role rename` | Renames selected role in config/topology/package metadata where editable. | Active config/package mutation. | `rename_fleet_role_source`; tests `rename_fleet_role_*`. | Exact selected fleet role only; root, missing, duplicate, same-role fail. | Non-editable manifest behavior remains intentionally conservative. |
| `canic fleet role list/inspect` | Shows declared/attached lifecycle state and next action. | Passive/read-only. | CLI render tests in `fleets/tests.rs`. | Inspection must not change deployability. | No issue found. |
| `deploy check/report/evidence` | Produces deployment-truth, authority, or evidence artifacts from config/state/checks. | Passive/read-only unless explicitly inside install/check gate. | `deployment_truth/*`; source guard tests; `deploy` parser tests. | Reports do not rewrite local trust state or controllers. | Full live verification remains future work. |
| External lifecycle commands | Generate plans, proposals, handoff, pending, verification, completion reports. | Passive/read-only. | `deployment_truth/lifecycle.rs`; `canic deploy external ...` tests. | Consent/external execution is not performed. | Active external execution intentionally absent. |
| Adoption report commands | Classify configured/observed/evidence states and render recommendations. | Passive/read-only. | `adoption_report_from_config_source`; CLI smoke/unit tests; 0.50 closeout. | No config/topology/controller/artifact/install mutation. | Import/adoption apply intentionally absent. |
| Token/cycles wrappers | Wrap ICP token/cycles commands with Canic deployment-target resolution. | Active wallet/ICP command wrapper. | `crates/canic-cli/src/token.rs`, `cycles/wallet.rs`; parser tests. | Resolution is explicit; network-specific wallet behavior is not hidden in status. | Operational safety depends on external `icp` command semantics. |
| `canic::build!` | Build-time config/package validation and env/cfg emission. | Compile-time active. | `macros/build.rs`; build-support tests. | Metadata is source of truth; role-only metadata fails. | Standalone generated configs are declared-only; one workspace test failure indicates at least one test fixture still needs topology-aware setup. |
| `canic::start!()` | Single normal startup macro; root/non-root dispatch from metadata/build cfg. | Runtime lifecycle entrypoint. | `macros/start.rs`; setup docs; `start_root!` search. | No public `start_root!` active surface; root endpoints only under `canic_is_root`. | Historical audit docs mention old `start_root!` fixtures. |

## Claim Verification

| Claim | Status | Evidence | Notes |
| --- | --- | --- | --- |
| Package metadata is fleet-role strict. | Verified | `PackageCanicMetadata { fleet, role }`; workspace governance test. | Active package manifests include both fields. |
| Role-only package metadata compatibility is gone. | Verified | `required_package_metadata` requires both fields; 0.50 docs say role-only invalid; 0.49 design follow-up now requires both fields. | No active compatibility path remains. |
| Roles are fleet-scoped. | Verified | `FleetRoleRefV1 { fleet, role }`; `[fleet] name`; role CLI takes `<fleet> <role>`. | One fleet per config is the implemented model. |
| Declared-only roles are first-class. | Verified | `ConfiguredRoleLifecycle`; `canic fleet role list/inspect`; 0.49 tests. | They are compile-eligible but not deployable. |
| Declared-only roles are excluded from build/deploy/install/truth mutation surfaces. | Verified | `configured_deployable_roles`; tests `configured_deployable_surfaces_exclude_declared_only_roles`; build preflight test. | Adoption reports preserve this boundary. |
| Attached roles remain explicit. | Verified | `canic fleet role attach`; config validation. | Attachment is not inferred from package names. |
| `canic build <fleet> <role>` is attached-role strict. | Verified | CLI usage/tests; old role-only shape rejected. | Active docs aligned. |
| `build!` / `start!` derive identity from package metadata. | Verified | `build_support/config.rs`; `macros/build.rs`; `macros/start.rs`. | `CANIC_CANISTER_ROLE` comes from `build!`. |
| `start_root!` is gone. | Verified for active API | `rg "start_root!" crates canisters fleets docs README* CHANGELOG.md` finds only docs/changelog/audit history and 0.49 statements. | Historical audit docs are stale but not active API. |
| Deployment truth can be represented and checked passively. | Verified | 0.41-0.47 status docs; `DeploymentCheckV1`; comparison/root-inspect reports. | Current install uses truth as a gate. |
| Authority evidence is preserved and surfaced. | Verified | `deployment_truth/authority.rs`; `receipt.rs`; authority CLI docs/tests. | Still dry-run. |
| External lifecycle is report/evidence oriented unless explicitly active elsewhere. | Verified | `deployment_truth/lifecycle.rs`; 0.45 status. | No external execution path found. |
| Adoption reporting is read-only. | Verified | 0.50 closeout; `adoption_report_from_config_source`; CLI write boundary. | Report recommendations are non-executing. |
| Adoption reporting never mutates config/topology/controllers/artifacts/deployments. | Verified | Host builder takes values; CLI only writes optional output artifact. | Smoke tests in 0.50 closeout verified file boundary. |
| Evidence inputs are optional and safely handled. | Verified | CLI option parsing and tests for conflicts/invalid paths. | JSON schema remains experimental in 0.50. |
| `evidence-conflict` and `missing_or_stale_evidence` are implemented. | Verified | `artifact_conflict_roles`, `missing_evidence`; 0.50.13/14/15 tests. | Both directions of artifact conflict are tested. |
| Docs and changelogs reflect current behavior. | Partially Verified | Active README/setup/adoption docs aligned; changelogs through 0.50.15. | 0.43-0.46 design docs still say "Tentative"; 0.49 role-only wording follow-up is resolved. |

## User Journey Audit

| Journey | Commands that exist | Evidence available | Safe today | Blocked/confusing |
| --- | --- | --- | --- | --- |
| New greenfield project | `canic scaffold`, `canic scaffold canister <fleet> <role>`, `canic fleet role attach`, `canic build <fleet> <role>`, deploy/install flows. | Config role lifecycle, build artifacts, deployment checks, receipts. | Yes for managed fleet work. | Full workspace test failure suggests standalone/test fixture setup needs cleanup. |
| Existing brownfield deployment | `canic fleet adoption report <fleet> --profile brownfield`, optional evidence inputs. | Observed canisters, authority, artifact, package metadata evidence when supplied. | Safe because report-only. | No active import/apply; operators must manually declare/attach after review. |
| Partial adoption | `adoption report --profile partial`, deployment-check/cargo-metadata evidence. | Missing/stale evidence, observed-only, declared-only, external classifications. | Safe for assessment. | No guided mutation beyond recommended previews. |
| Standalone canister | `start_local!` or standalone generated config paths; adoption standalone profile. | Declared-only standalone reports. | Compile-only is supported. | Workspace test failure shows at least one standalone install fixture expects topology not present in generated standalone config. |
| Leaf-only adoption | `adoption report --profile leaf-only`. | Authority-sensitive observed rows and warnings. | Safe: no declaration recommendations for authority-sensitive leaf cases. | No import path. |
| Hybrid external-Wasm | `adoption report --profile hybrid-external-wasm` plus artifact evidence. | Module hashes, external artifact evidence, blocked artifact registry import. | Safe: artifact registry import is blocked. | Real artifact registry remains future work. |
| External-controller deployment | `deploy external ...` reports; adoption authority-gated recommendations. | External action/authority evidence, proposals, verification checks. | Safe as passive evidence. | No controller or external execution workflow. |
| Declared-only staged role | `scaffold canister`, `fleet role declare`, `fleet role list/inspect`, then `fleet role attach`. | Lifecycle output shows compile eligibility and deploy-artifact block. | Yes. | Build/deploy remains blocked until explicit attach, by design. |

## Test Coverage Map

| Capability | Test evidence | Gaps |
| --- | --- | --- |
| Fleet-role metadata and governance | `crates/canic/tests/workspace_manifest.rs::canic_package_metadata_resolves_to_declared_fleet_roles`; build-support tests. | 0.49 design-doc follow-up is resolved; code coverage is strong. |
| Declared-only lifecycle | `configured_role_lifecycle_lists_declared_and_attached_roles`; `configured_deployable_surfaces_exclude_declared_only_roles`; CLI render tests. | No major gap. |
| Role declare/attach/rename | `declare_fleet_role_*`, `attach_fleet_role_*`, `rename_fleet_role_*`. | No major gap. |
| Build strictness | `build_rejects_old_role_only_shape`, `build_preflight_rejects_declared_only_role`, `build_resolves_config_from_selected_fleet`. | No major gap. |
| Deployment truth/install gates | Many `install_truth_*`, local inventory, artifact manifest, plan, diff tests in `canic-host`. | Broad workspace test currently fails in a separate PocketIC fixture. |
| Authority dry-run | Authority report/evidence/receipt tests in `deployment_truth/tests.rs` and CLI parser/render tests. | No active controller apply tests because no apply path exists. |
| External lifecycle | Proposal, receipt, policy, verification, completion, handoff tests in `deployment_truth/lifecycle.rs`/tests. | No active external execution tests by design. |
| Root verification | `verify_registered_deployment_root_*` tests. | No broad live deployment verification. |
| Setup macro surface | Build-support tests, workspace manifest tests, setup docs. | Historical audit docs mention `start_root!`; active API clean. |
| Adoption reporting | 0.50 closeout lists host/CLI tests through 0.50.15. | No import/apply tests because no apply path exists. |
| Non-mutation/read-only behavior | Source-guard tests for authority/deploy check; adoption smoke tests preserve config/output boundary. | Could add a recurring docs/source guard against stale design-doc active claims. |

## Documentation State

Aligned:

- `README.md`, `INSTALLING.md`, `docs/architecture/build-artifacts.md`,
  `docs/getting-started/minimal-managed-fleet.md`, crate READMEs, and
  `docs/architecture/adoption-profiles.md` describe fleet/role metadata,
  `canic::start!()`, `canic build <fleet> <role>`, and passive adoption
  reporting correctly.
- `CHANGELOG.md` and `docs/changelog/0.50.md` are current through 0.50.15.
- `docs/audits/release-lines/0.50-closeout.md` is PASS.

Stale or risky:

- `docs/design/0.43-*`, `0.44-*`, `0.45-*`, and `0.46-*` design docs still
  say "Tentative design notes" even though their status docs mark those lines
  closed. This is less risky than 0.49 because status docs are clear, but it is
  still design-source debt.
- The historical post-46 adoption backlog status was marked superseded by
  0.50 passive adoption. Active adoption/import remains optional future idea
  material, not active release scope.
- Historical changelog/audit entries mention `start_root!` and old
  `canic build <role>` forms. Those are acceptable as history, but stale dated
  audit docs from May still describe old `start_root!` fixtures as current.

## Backlog Reconciliation

| Item | Classification | Rationale |
| --- | --- | --- |
| Adoption profiles | Now complete for passive reporting | Implemented in 0.50; post-46 backlog status is obsolete. Active adoption/import remains a separate future topic. |
| Adoption gap inventory | Partially complete | Passive classification/reporting landed; mutation/import/controller transfer did not. |
| CI/GitOps provenance | Partially complete; remaining scope moved to ideas | 0.51-0.53 now cover envelopes, exit classes, provenance, policy gates, and manifests. Signing, provider wrappers, release evidence, and locks remain optional ideas. |
| DR clone verification | Moved to ideas | Depends on stable evidence/provenance and maybe catalogs/groups. 0.46 only compares two archived checks. |
| `wasm_store` artifact registry | Moved to ideas | 0.44/0.50 expose artifact evidence and block registry import; no registry/provenance/retention model exists. |
| Deployment catalog/groups | Still relevant soon/later | Deployment-target identity exists, but no group/catalog UX. Could follow CI evidence or run in parallel if scoped read-only. |
| Teardown/test-deployment lifecycle | Still relevant later | Requires catalogs/groups, authority policy, and stronger verification/receipts. |
| Controller mutation/import | Premature | Authority/adoption reports are passive. Needs apply contract, locks, live verification, and receipts. |
| Active adoption/import | Premature | Passive reports are complete, but import semantics and authority handoff are not designed/implemented. |
| External lifecycle follow-ups | Still relevant later | Consent delivery and external execution remain deferred. |

## What Can Be Cut

- Mark post-46 adoption-profile backlog docs as superseded by 0.50 passive
  adoption reporting.
- Mark implemented design docs 0.43-0.46 and 0.49 as implemented/closed, or
  make the top of each doc point readers to the corresponding `status.md`.
- Treat historical `start_root!` audit notes as archival only; do not keep
  them in active audit scope examples.
- Avoid any future doc examples that use `canic build <role>` except as
  historical changelog context.

## What Must Not Be Built Yet

| Feature | Why premature | Missing prerequisite |
| --- | --- | --- |
| Active adoption/import | Passive reports do not define ownership transfer, topology mutation receipts, or controller safety. | Import design, explicit operator consent, authority apply, deployment truth postconditions. |
| Controller mutation/apply | 0.42 intentionally stops at dry-run authority evidence. | Stable apply plan, locks, live recheck, receipts, rollback/repair semantics. |
| Artifact registry import/GC | `wasm_store` is visible evidence, not a registry authority. | Registry entry model, provenance, pins, retention policy, plan/apply receipts. |
| Teardown | Destructive operations need deployment catalog, authority checks, comparison, and receipts. | Catalog/groups, teardown plan, authority preflight, protected state evidence. |
| Broad live deployment verification | 0.47 only verifies registered roots from check artifacts. | Verification profiles, live inventory crawling, freshness policy, protected-call probes. |
| Stable GitOps automation on raw DTOs | Current JSON uses internal V1 objects without a public envelope/exit-code contract. | CI/GitOps provenance line. |

## Recommended Next Line

Primary recommendation:

**0.51 - CI/GitOps provenance and stable evidence envelopes**

Why it should come next:

- 0.41-0.50 produced many evidence artifacts but intentionally left raw JSON
  and CI contracts unstable.
- 0.50 explicitly keeps adoption JSON experimental.
- Automation is the natural next consumer of deployment truth, authority,
  promotion, external lifecycle, root verification, and adoption reports.
- A stable envelope and exit-code contract will make later registry, catalog,
  DR, and active apply work safer.

Minimum first slice:

- Add a read-only `JsonEnvelopeV1` and `ExitCodeClassV1` around one or two
  existing passive commands, likely `canic deploy check <deployment>` and
  `canic fleet adoption report <fleet> --profile <profile> --format json`.
- Document that the envelope is stable while payload DTOs may remain
  command/version-specific unless explicitly promoted.
- Add tests for envelope schema, command identity, generated timestamp,
  warnings/hard failures, and exit classification.

Must stay out of scope:

- controller mutation;
- active adoption/import;
- artifact registry import/GC;
- teardown;
- broad live deployment verification;
- requiring GitOps for local development.

Risks:

- Freezing the wrong JSON surface too early.
- Letting CI wrappers imply live truth without fresh inventory.
- Expanding into signing/locks before the envelope/exit semantics are crisp.

Acceptance criteria:

- At least one deployment-truth command and one adoption command can emit a
  stable envelope.
- Exit classes are documented and tested.
- The envelope references source command, target identity, generated time,
  warnings, hard failures, and payload schema.
- No command starts mutating more state because of CI support.

Alternatives:

1. **0.51 - Documentation and workspace-test closeout.** This is safer if the
   maintainer wants zero debt before new feature work, but it is too small for
   a full strategic line unless combined with evidence hardening.
2. **0.51 - Deployment catalog/groups.** This fits deployment-target identity
   but benefits from stable evidence envelopes first.

## Risk Register

| Risk | Severity | Area | Evidence | Impact | Mitigation | Blocks next line? |
| --- | --- | --- | --- | --- | --- | --- |
| Workspace test failure in `pic_ingress_payload_limits` | High | Tests/setup | `cargo test --workspace --locked` fails installing `payload_limit_probe` due missing subnet `prime`. | Broad CI confidence is not clean; standalone config behavior may have broken a PocketIC fixture. | Fix fixture/config expectation or use an attached test config for topology-dependent probe. | Yes, before release/merge; no, for design-only planning. |
| 0.49 design doc contradicted hard cut | Resolved | Docs | Original audit found role-only metadata example and "role alone valid" text in 0.49 design. | Contributors could have reintroduced fallback behavior. | Resolved on 2026-05-31 by updating 0.49 design to implemented hard-cut language. | No. |
| Older design docs still say tentative | Low | Docs | 0.43-0.46 design docs start with "Tentative design notes". | Design-source confusion. | Mark implemented/closed or point to status logs. | No. |
| Post-46 adoption backlog status obsolete | Resolved | Roadmap | The archived backlog status now says passive adoption was superseded by 0.50. | Roadmap readers can distinguish passive adoption from active import. | Active adoption/import is optional idea material. | No. |
| Raw JSON artifacts lack stable public envelope | Resolved | Automation | 0.51 added the stable evidence envelope and exit-class contract. | Automation no longer needs to branch on command-specific raw DTOs for envelope-enabled surfaces. | Remaining locks/signing/provider wrappers are optional idea material. | No. |
| Active adoption/import temptation | High | Product safety | 0.50 reports only; recommendations are non-executing. | Premature import could bless foreign state or mutate topology unsafely. | Keep active adoption out until authority/apply/receipt design exists. | Yes for any import feature. |
| `wasm_store` registry temptation | Medium | Artifacts | 0.44/0.50 expose artifact evidence but no registry model. | Registry metadata could be mistaken for install truth. | Build provenance/envelope first; registry later with pins/GC plan. | No. |

## Final Closeout Checklist

- [x] Changelogs consistent through 0.50.15.
- [ ] Design docs implemented/closed: 0.50 yes; 0.49 and older design docs need status wording cleanup.
- [x] Active setup/docs match CLI.
- [x] Tests cover core role/adoption/deployment-truth invariants.
- [x] Passive/active boundaries are clear in implementation.
- [x] Passive adoption work is complete.
- [ ] Backlog reconciled in source docs: this report reconciles it, but backlog docs need updates.
- [x] Next line selected: 0.51 CI/GitOps provenance and stable evidence envelopes.
- [ ] No release-blocking ambiguity: implementation model is clear, but broad workspace test failure must be fixed before release/merge confidence.

## Validation Results

Passed:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test -p canic --test changelog_governance --locked`
  - Covered by `cargo test --workspace --locked`; passed 1 test before the
    later workspace failure.
- `cargo test -p canic --test workspace_manifest --locked`
  - Covered by `cargo test --workspace --locked`; passed 4 tests before the
    later workspace failure.

Failed:

- `cargo test --workspace --locked`
  - Failed in `canic-tests --test pic_ingress_payload_limits`.
  - Failure: installing `payload_limit_probe` trapped with
    `init: subnet prime not found in configuration`.
  - Interpretation: the full workspace suite is not clean. The failure appears
    related to a topology-dependent PocketIC fixture using a standalone
    declared-only config.

Searches:

- `rg -n "start_root!" crates canisters fleets docs README* CHANGELOG.md`
  found no active source call sites; matches are historical changelog/audit
  entries and 0.49 design statements that `start_root!` must remain absent.
- `rg -n "canic build <role>|canic adoption report|report or plan|Tentative design notes" ...`
  found no active `canic build <role>` instructions, one rejected alternative
  for `canic adoption report`, and remaining tentative markers in older design
  docs.
- `rg -n "[package.metadata.canic]|fleet =|role =" ...`
  showed active package metadata examples use both `fleet` and `role`; the
  0.49 design-doc role-only exception has since been resolved.

Not run:

- Networked deployment/install flows.
- Packaged-downstream shell probes.
- `make test`, because `cargo test --workspace --locked` already failed and
  this audit does not patch source code.
