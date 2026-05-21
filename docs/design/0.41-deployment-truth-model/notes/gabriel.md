# Source Notes: Deployment Pain Points

These are archival source notes captured during 0.41 roadmap planning. They are
not normative design text; the normative design is in `../0.41-design.md` and
the cross-line roadmap is in `../0.41-0.50-deployment-roadmap.md`.

---

1) When I installed yesterday the new root the controllers in canic.toml were just metadata but only my controller was in the list after install, not sure if that's a bug or you need to run update-settings after the wasm is installed. How do you propagate the list of controller onto all canisters? do you add them in projects/ledger as well?

2) Pool ownership, do you remove all controllers and re-add them once the canisters are installed? currently the controllers stay the same, not even root is added, I had to manually add it yesterday.
How does plan execute reconcile and validate pool canister ownership/ controllers on reused pool canisters?

3) wasm_store should be a normal role.

wasm_store is auto-generated, built by a separate  binary, hardcoded to write to .dfx/local/canisters/wasm_store/, and excluded from configured_install_targets. The artifact-path mismatch (build path .dfx/local/..., root's build.rs reads .dfx/<DFX_NETWORK>/...) panics root's  build.rs with no graceful failure. As a first-class role the mismatch becomes a plan-validation error.
4) the current installer is dfx dependent, is the plan to make agnostic so we can easily move to icp-cli or whatever?

5) RoleArtifact needs an embedded-config digest, not just a wasm digest.
 Every Canic canister bakes canic.compiled.rs into its wasm at build time based  on CANIC_CONFIG_PATH. Two builds of the same source can embed different topology if CANIC_CONFIG_PATH differs between invocations. The current wasm_sha256 won't catch mixed-config fleets. Suggest RoleArtifact { ..., wasm_sha256,  embedded_config_sha256 }, and refuse to execute a plan where any role's embedded_config_sha256 doesn't match the fleet's runtime config digest.


Plan execute needs post-build artifact materialization checks.
We hit this twice this week: canic-list-install-targets returned a role, but the orchestrated dfx build sequence silently skipped it (project_registry in our case). The install proceeded to emit_release_set_manifest which then panicked with os error 2 from fs::read. Plan execute should verify every role's  <artifact_root>/<role>/<role>.wasm.gz materialized with the expected digest before staging, and emit a clear error if not.


Resumability pick a model, don't leave it open."How much of plan execution should be resumable after partial failure?" can't be open IMO. Each retry on staging cost ~2 min of chunk uploads  (stage_release_set) before we even got back to wait_ready. Without phase-level  checkpoints, every retry pays the full cost. Suggest each phase (create → build  → stage → install_root → resume_bootstrap → wait_ready) records a receipt before the next runs, and execute is --resume-from <last_completed_phase>-capable.
8 Agreed on read-only inventory as the first slice. To make it useful for catching the gaps above, inventory should fingerprint implicit state, current  IC controllers on every canister, pool canister IDs and their controllers, embedded-config sha256 per role artifact, current module_hash per role, not just restate canic.toml. Otherwise inventory becomes a self confirming oracle and the issues only surface at install.


Authority profile categories vs IC's single notion of "controller".
State explicitly that staging_principals / emergency_controllers must NOT also appear in any IC controllers field — otherwise the role grant is bypassable via direct management canister calls. Plan validation should fail on overlap.


Does AuthorityProfile.managed_canister_controllers apply to spawned canisters,
 or do they have a per-spawn controller policy?

If the user is the sole IC controller of their project_instance, that canister is in a different trust domain than the rest of the fleet — fleet upgrades  can't push there without user consent.
How does promotion / upgrade work for user-owned canisters? Does the plan distinguish fleet-controlled from user-controlled canisters? If a critical bug fix needs to land in project_instance, what's the flow?
