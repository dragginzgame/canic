# Prior Audit Notes: Deployment Roadmap

These are archival audit notes captured during 0.41 through 0.46 roadmap
planning. They are not normative design text; the normative design is in
`../0.41-design.md`, the later per-line design docs, and
`../0.41-0.50-deployment-roadmap.md`.

---

Overall Audit: Canic 0.41–0.46 Deployment Roadmap

Baseline reviewed:

repo: dragginzgame/canic
commit: 43b06d4b36c3bedf85815c58fa948a9eee38dea7
message: for codex
scope: docs/design/0.41 through docs/design/0.46
Verdict

Accept this as the baseline roadmap, with revisions before implementation.

Risk score:

4/10 design risk

The sequencing is right. The docs now form a coherent dependency chain:

0.41 = deployment truth and current-install safety
0.42 = authority reconciliation
0.43 = backend-agnostic execution
0.44 = artifact promotion
0.45 = external/user-owned lifecycle
0.46 = multi-deployment operations

The key architectural improvement is that Canic no longer tries to become more flexible before it becomes more honest. The 0.41 doc explicitly makes plan, inventory, receipt, and diff separate objects, states that receipts are evidence rather than truth, and requires live inventory before install/resume decisions. That is the right foundation.

The remaining risks are mostly around boundary precision: resumability, canonical config digests, authority taxonomy, promotion semantics, and concurrent operation protection.

Executive Summary

The roadmap is directionally strong and should be kept.

The most important thing it gets right is the layering:

observe truth
-> refuse unsafe states
-> reconcile authority
-> abstract execution
-> promote artifacts
-> coordinate external lifecycle
-> compare deployments

That ordering prevents the major failure mode: building a sophisticated deployment system that faithfully automates hidden or false assumptions.

The 0.41 doc is especially improved. It states that 0.41 is not a new executor, but a truth and safety layer around the current install path; it names the current pain points directly, including controller drift, wasm_store, missing materialized artifacts, mixed embedded config, repeated expensive phases, and the 0.40 protected-call coupling.

The downstream docs are mostly consistent with that foundation. 0.42 consumes 0.41 objects and adds authority reconciliation rather than inventing a second drift model. 0.43 correctly delays executor abstraction until after truth and authority are stable. 0.44 treats promotion as a plan transformation and says “promote bytes, not authority.” 0.45 handles canisters Canic cannot unilaterally upgrade. 0.46 builds comparison and drift reports from the earlier stable objects.

No architectural rewrite is needed. I would fix the findings below before letting Codex start implementation.

Strengths
1. The truth/authority/execution split is correct

The docs now avoid a common deployment-system mistake: combining observation, authority mutation, and execution abstraction into one “deployment flexibility” feature.

The 0.41 doc gives truth its own model:

DeploymentPlanV1
DeploymentInventoryV1
DeploymentReceiptV1
DeploymentDiffV1

That is exactly right. It lets Canic answer four distinct questions:

What did we intend?
What exists?
What did we try?
What differs?

Those should never be collapsed into one object.

2. Receipts are correctly treated as evidence

The line:

Receipts are evidence, not truth.

is one of the most important decisions in the roadmap. It prevents local state from becoming an authority source. The docs also correctly require live inventory to prove a phase postcondition before a receipt can help skip work.

3. 0.40 protected-call coupling is properly surfaced

The docs now recognize that deployment state affects authorization, not just installation. Root identity, embedded config, role assignment, verifier key material, role epochs, and controllers all affect whether protected internal RPC works safely.

That is essential. After 0.40, deployment tooling is no longer just “install some wasm.” It is managing the authority state that makes protected calls valid.

4. 0.42 is framed as reconciliation, not application

The 0.42 doc correctly says controller policy becomes reconciled deployment authority, and that Canic should prove controller state is correct or explain why it cannot make it correct.

That framing is much better than “apply controller policy.” Reconciliation naturally includes:

already correct
can apply automatically
requires external action
unsafe / blocked
unknown

That is the right operator model.

5. 0.43 is placed at the right point

The 0.43 doc explicitly says backend abstraction before truth and authority is a rewrite trap. Correct. It also says executor operations must return enough observation data to verify phase postconditions without trusting local process state.

That is the right executor contract.

6. 0.44 gets the promotion invariant right

The rule:

Promote bytes, not authority.

is correct. The doc also correctly refuses to copy source controllers, source root principal, source authority profile, source network, pool canister IDs, or deployment operation epoch by default.

That protects against the staging-to-prod footgun where “tested staging artifact” accidentally carries staging authority.

Findings
High 1 — Promotion semantics need a sharper distinction between “tested bytes” and “target-configured bytes”
Issue

0.44 says promotion should copy selected source artifact byte identity while also recomputing the target embedded config digest.

That is directionally right, but it hides a crucial distinction:

If embedded config is compiled into the wasm,
then recomputing target embedded config changes the final wasm bytes.

So there are two different promotion modes:

A. promote sealed wasm bytes
   -> final wasm is byte-identical
   -> embedded config must already be valid for target

B. promote source/build identity
   -> rebuild or relink for target config
   -> final wasm bytes may differ
   -> source artifact identity is promoted, not final installed bytes

The current doc risks saying both at once:

copy selected source artifact byte identity
and
recompute target embedded config

Those are only compatible if the embedded config is outside the copied bytes or if the source artifact is pre-configuration.

Why it matters

This is the staging/prod footgun in its most subtle form:

tested staging wasm
-> copied into prod
-> bytes are identical
-> but embedded root/topology is staging

0.44 already names this risk, but the data model needs to encode the distinction.

Recommended fix

Add a section to 0.44:

## Promotion Artifact Levels

Promotion may operate at one of two levels:

1. SealedWasmArtifact
   - exact wasm/wasm.gz bytes
   - may be installed only if embedded config identity is valid for the target

2. SourceOrBuildArtifact
   - package/source/build recipe/object identity
   - target deployment recomputes embedded config
   - resulting sealed wasm has a new target artifact digest

Then add a rule:

A promotion plan must explicitly say whether it is promoting sealed bytes
or source/build identity. It must not describe a target-config-recomputed
artifact as byte-identical to the source sealed wasm.
Suggested test

Create a staged source receipt with:

same source package
different target root
different embedded config digest

Assert that promotion produces:

same source identity
different target sealed wasm digest
different embedded config digest

unless the role is declared config-independent.

High 2 — Canonical embedded config digest is still underspecified
Issue

0.41 records embedded_config_sha256 and embedded_topology_sha256 in RoleArtifact, and later docs rely heavily on embedded config comparison for promotion and drift.

But the roadmap does not yet define what the digest is computed over.

This is not a small detail. The digest must be canonical.

Why it matters

If the digest is over raw config bytes, these could falsely differ:

same semantic config
different whitespace
different key ordering
different include expansion order
different default omission
different path spelling

Operators will learn that config drift warnings are noisy, and then ignore them. Once that happens, 0.44 promotion safety and 0.46 drift reporting lose credibility.

Recommended fix

Add a section to 0.41:

## Canonical Embedded Config Identity

embedded_config_sha256 is computed over CanonicalEmbeddedConfigV1, not raw
TOML/JSON/YAML bytes.

Canonicalization includes:
- resolved includes
- stable map ordering
- normalized principal text
- normalized role ordering
- explicit defaults
- normalized network/root/trust-domain references
- no incidental whitespace
- schema_version included in the hash input

Define two digests if needed:

raw_config_sha256
canonical_embedded_config_sha256

Use the canonical digest for safety decisions.

Suggested test

Two configs with different ordering/whitespace but identical resolved semantics should produce the same canonical digest.

Two configs with different root principal, role topology, authority-relevant config, or verifier assumptions should produce different canonical digests.

High 3 — Automated resume is still too large for the 0.41 safety line
Issue

0.41 is supposed to be the truth and safety layer, but it still includes canic deploy resume, --resume, --resume-from <phase>, and a multi-phase resume model.

The document is careful: it requires live inventory, matching identities, and verified postconditions. That is good.

But automated resume is still the highest-risk part of 0.41 because it is the only part that turns receipts into control flow across partially completed mutation phases.

Why it matters

Resume bugs are usually not obvious in happy-path testing. They appear when:

a phase partially succeeded
controller state changed externally
artifact digest matches but module hash changed
root changed
pool state moved
another operator resumed from another machine
staging succeeded but bootstrap did not

A receipt format is low-risk. Acting on receipts to skip phases is higher-risk.

Recommended fix

Split the 0.41 wording into two levels:

0.41 required:
- phase receipts
- verified postconditions
- resume safety report
- explain what would be resumable
- refuse unsafe continuation

0.41 optional / 0.41.x:
- automated --resume that skips completed phases

Update 0.43’s dependency text from:

0.41 can produce ... verified resume safety

to:

0.41 can produce phase receipts and verified postcondition evidence;
automated resume may be implemented only where the safety report proves
phase skipping is valid.

The 0.43 doc currently assumes verified resume safety as an input. That is fine long-term, but it should not force 0.41 to ship automated resume before receipts have been exercised.

Suggested test

Model a resume after each phase with a changed live condition:

root changed
artifact digest changed
authority profile changed
observed module hash changed
pool controller changed
deployment epoch changed

Every case should refuse resume.

Medium 1 — 0.42 example blurs emergency controllers with normal IC controllers
Issue

0.42 says:

staging_principals and emergency_controllers are authority categories,
not normal IC controllers.

and says overlap with ordinary IC controller sets should fail unless break-glass is explicit.

But the example immediately shows:

root:
  desired controllers: [deploy-principal, emergency-principal]
  observed controllers: [deploy-principal]
  action: add emergency-principal
  can_apply: yes

That reads like emergency-principal is simply a desired normal IC controller.

This also conflicts with the 0.41 validation that staging/emergency principals must not overlap normal IC controllers.

Why it matters

This is exactly the boundary the roadmap needs to keep sharp:

normal controller
staging authority
emergency authority
break-glass authority

If examples blur it, implementation will too.

Recommended fix

Change the example to use a normal managed controller:

root:
  desired controllers: [deploy-principal, ops-principal]
  observed controllers: [deploy-principal]
  action: add ops-principal
  can_apply: yes

Then add a separate break-glass example:

root:
  emergency controller requested: emergency-principal
  state: requires_explicit_break_glass_flow
  can_apply: no in normal reconciliation
Medium 2 — Canister control classification taxonomy drifts across docs
Issue

0.41 defines classifications as:

fleet_controlled
canic_managed_pool
externally_imported
user_controlled
unknown_or_unsafe

0.45 then introduces:

FleetControlled
JointlyControlled
UserControlled
ExternallyControlled
UnknownUnsafe

These are similar but not identical.

The gap is especially visible here:

externally_imported
ExternallyControlled
JointlyControlled
canic_managed_pool

Those are not obviously the same axis.

Why it matters

Classification is foundational. It affects:

whether Canic may mutate
whether Canic may propose
whether external consent is required
whether the canister belongs in pool logic
whether 0.45 lifecycle workflows apply

If 0.41 and 0.45 use different enums, later code will develop translation glue and edge-case drift.

Recommended fix

Define one canonical enum in 0.41:

CanisterControlClassV1 {
  FleetControlled,
  CanicManagedPool,
  ExternallyImported,
  JointlyControlled,
  UserControlled,
  UnknownUnsafe,
}

Then define lifecycle authority separately:

LifecycleAuthorityV1 {
  control_class,
  allowed_upgrade_modes,
  required_controllers,
  consenting_principals,
  verification_requirements,
}

0.45 should extend lifecycle behavior, not rename the classification model.

Medium 3 — Concurrent deployment protection is underpowered for real multi-operator use
Issue

0.41 requires:

local deployment lock
operation ID
receipt identity checks
post-validation that detects if observed deployment identity changed

and reserves fields for future remote protection.

This is good as schema preparation, but a local lock only protects one machine. It does not protect against:

two laptops
CI plus local operator
two CI jobs
manual controller change during install
root-side mutation by another actor
Why it matters

0.41 still wraps the current install path. That path mutates real IC state. Without a remote epoch or pre-phase live checks, two operators can interleave:

A builds and stages
B changes controllers
A resumes bootstrap
B installs another role
A post-validates too late

Post-validation catches some damage but does not prevent all bad interleavings.

Recommended fix

Keep remote locking for later if needed, but strengthen 0.41 with explicit best-effort race detection:

Before every mutating phase:
- re-inventory relevant canisters
- compare root trust anchor
- compare controller fingerprint
- compare module hash if phase depends on installed state
- compare deployment epoch if observable
- refuse if the mutation base changed

Rename the section from:

Concurrent Deployment Protection

to:

Concurrent Deployment Detection And Local Locking

unless there is an actual remote lock.

Medium 4 — DeploymentIdentity has potentially ambiguous aggregate digests
Issue

0.41 proposes:

DeploymentIdentityV1 {
  deployment_name,
  network,
  root_principal,
  authority_profile_hash,
  role_topology_hash,
  embedded_config_digest,
  artifact_set_digest,
  pool_identity_set_digest,
  canic_version,
  ic_memory_version,
}

This is good directionally, but embedded_config_digest is ambiguous at deployment level because RoleArtifact also has per-role embedded config identity.

Why it matters

A deployment can have:

same root config
different role embedded config
same artifact set
different role topology

A single embedded_config_digest could mean:

root config only
canonical whole-deployment config
aggregate of per-role configs
the exact file embedded into each role

Those are different.

Recommended fix

Rename and split:

deployment_manifest_digest
canonical_runtime_config_digest
role_embedded_config_set_digest
artifact_set_digest

Then define each one.

The most important one is:

deployment_manifest_digest =
  canonical hash over root, network, runtime variant, role topology,
  authority profile hash, role artifact identities, pool identities,
  and expected verifier assumptions
Medium 5 — 0.43 should explicitly retire wasm_store special materialization
Issue

0.41 correctly says wasm_store should be modeled as a normal role artifact even if special-case builder code remains.

0.43 separates artifact roles from artifact transport and says wasm_store as a role is bootstrapped by the executor.

That is right. But the docs should explicitly schedule the removal of the special materialization path.

Why it matters

“Special internally for now” tends to become permanent unless removal is written into the roadmap.

If wasm_store remains special inside artifact materialization, the 0.43 executor abstraction inherits an exception that the model claims does not exist.

Recommended fix

Add a 0.43 slice:

### Slice N: Retire wasm_store Special Materialization

wasm_store remains special only in bootstrap ordering.
It must be materialized through the same RoleArtifactManifest pipeline as
every other role.
Low 1 — Directory name still says deployment-flexibility

The 0.41 document is now titled:

0.41 Design: Deployment Truth Model

but the path remains:

docs/design/0.41-deployment-flexibility/0.41-design.md

That is stale naming.

Recommended fix:

docs/design/0.41-deployment-truth-model/0.41-design.md

This matters because “deployment flexibility” was the old framing. The new framing is better and should be reflected in the path.

Low 2 — Add a roadmap index / cross-line invariants document

Each doc has good local scope, but there is no single compact roadmap contract.

Add:

docs/design/0.41-deployment-truth-model/0.41-0.50-deployment-roadmap.md

with:

0.41 tells truth and refuses unsafe installs.
0.42 reconciles authority.
0.43 abstracts execution.
0.44 promotes artifacts, not authority.
0.45 coordinates lifecycle without pretending Canic has authority.
0.46 compares deployments using stable truth artifacts.

Then list cross-line invariants:

Receipts are never truth.
Live inventory wins over local state.
Promotion never copies authority.
Root trust anchor defines trust domain.
Config digests are canonical.
Executor backends do not change plan meaning.
Unknown control classification blocks mutation.

This will make future patch review much easier.

Cross-Document Consistency Matrix
Area	Status	Notes
Truth before mutation	Strong	0.41 anchors this well.
Receipts vs live inventory	Strong	Correctly says receipts are evidence, not truth.
Controller mutation boundary	Mostly strong	0.42 owns reconciliation; fix emergency-controller example.
Backend abstraction timing	Strong	0.43 is correctly delayed.
wasm_store modeling	Good	Add scheduled removal of special materialization path.
Promotion safety	Good but needs precision	Distinguish sealed wasm bytes from source/build identity.
Embedded config identity	Underspecified	Needs canonicalization in 0.41.
External lifecycle	Good	Unify classification taxonomy with 0.41.
Multi-deployment comparison	Good	Depends heavily on canonical digests and stable diff categories.
Concurrent operation handling	Partial	Local lock plus post-validation is not enough to call “protection.”
Recommended Edits Before Implementation

I would make these edits before any code work starts:

Add canonical embedded config digest semantics to 0.41.
Split automated resume from phase receipts in 0.41, or mark automated resume as optional/0.41.x.
Fix the 0.42 emergency-controller example.
Define one CanisterControlClassV1 taxonomy and reuse it in 0.41, 0.42, and 0.45.
Clarify 0.44 promotion artifact levels: sealed bytes vs source/build identity.
Strengthen 0.41 concurrent-operation language to “detection and local locking” unless a remote epoch/lock exists.
Add 0.43 slice to retire wasm_store special materialization.
Rename the 0.41 directory from deployment-flexibility to deployment-truth-model.
Add a short roadmap index with cross-line invariants.
Implementation Priorities

For actual code work, I would start with a very narrow 0.41 MVP:

1. DeploymentPlanV1 / DeploymentInventoryV1 / DeploymentReceiptV1 / DeploymentDiffV1 types
2. Canonical digest model
3. RoleArtifactManifest
4. Read-only inventory
5. Materialization checks
6. Safety report
7. Receipt emission from current installer
8. Hard refusal gates before dangerous phases
