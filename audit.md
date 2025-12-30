High-Level Architectural Assessment

  - Overall strengths: layering is explicit and mostly enforced in crates/canic-core/src/lib.rs, crates/canic-
    core/src/ARCHITECTURE.md, and module layout; DTOs are pure (crates/canic-core/src/dto/mod.rs), and workflows
    avoid direct model access.
  - Systemic risks: a few boundary rules are enforced by convention rather than structure, especially in ops
    (control vs view) and naming (export/import) for non-*Data types (crates/canic-core/src/ops/runtime/env.rs,
    crates/canic-core/src/ops/runtime/memory.rs).
  - Architectural maturity level: strong foundation with consistent layering, but still in the “boundary
    hardening” phase; a handful of drift points can accumulate into cross-layer ambiguity.
  - Most immediate pressure points: ops deciding “whether” (not just “how”) in crates/canic-core/src/ops/ic/
    mgmt.rs and duplicated env defaulting logic between workflow and ops.

  Layer-by-Layer Review

  model

  - Does well: stable-memory ownership and explicit ID ranges are cleanly centralized (crates/canic-core/src/
    model/memory/mod.rs, crates/canic-core/src/model/memory/*); *Data types are used for canonical exports.
  - Leaks/overreach: no direct upward dependencies found; serialization here is storage-only and not exposed.
  - Improvements: add explicit schema/version markers to *Data snapshots for future migrations.
  - API shape: model is pub(crate) in crates/canic-core/src/lib.rs, which keeps state authoritative and
    internal.

  ops

  - Does well: clear split between storage and adapter (crates/canic-core/src/ops/storage/*, crates/canic-core/
    src/ops/adapter/*); ops::ic wraps infra with metrics (crates/canic-core/src/ops/ic/*).
  - Leaks/overreach: upgrade_canister decides whether to upgrade based on module hash (policy-ish) in crates/
    canic-core/src/ops/ic/mgmt.rs; LogOps mixes control and view in crates/canic-core/src/ops/runtime/log.rs.
  - Improvements: split control vs view ops (or enforce *_view naming), and move “should upgrade” decisions to
    policy/workflow.
  - API shape: EnvOps::export/import use EnvView (not *Data) in crates/canic-core/src/ops/runtime/env.rs, which
    violates the export/import naming rule.

  policy

  - Does well: pure, deterministic rules with no async or serialization usage (crates/canic-core/src/policy/*);
    depends on ops for reads only.
  - Leaks/overreach: some decisions are still embedded outside policy (upgrade skip logic in ops, env defaulting
    in workflow).
  - Improvements: move eligibility decisions like “upgrade required” into policy, expose as typed plans.
  - API shape: policy errors are composable and clean, but collapse into string variants at canic::Error in
    crates/canic-core/src/lib.rs.

  workflow

  - Does well: orchestration is centralized and uses ops/policy only (crates/canic-core/src/workflow/*); no
    direct model access found.
  - Leaks/overreach: env defaulting logic is duplicated in workflow runtime (crates/canic-core/src/workflow/
    runtime/mod.rs) and ops (crates/canic-core/src/ops/runtime/env.rs).
  - Improvements: centralize env normalization in ops, keep workflow strictly orchestration.
  - API shape: workflow modules are public and broad; consider narrower exports for extension seams.

  endpoints / macros

  - Does well: canic_query/canic_update enforce guard/auth/rule pipeline (crates/canic-macros/src/lib.rs);
    lifecycle macros remain thin (crates/canic-core/src/macros/start.rs).
  - Leaks/overreach: canic_endpoints! exposes EnvOps::export and MemoryRegistryOps::export with non-*Data naming
    (crates/canic-core/src/macros/endpoints/mod.rs).
  - Improvements: allow opt-in/out endpoint bundles or documented profiles; align export naming in exposed
    endpoints.
  - API shape: macro validation for auth/rules is strong, but there’s no structural guard preventing control-ops
    use from query endpoints.

  dto

  - Does well: DTOs are passive, no impls or invariants (crates/canic-core/src/dto/*); dto::abi::v1 versioning
    is a good stability affordance.
  - Leaks/overreach: *View types live in the DTO module, which blurs “view vs boundary contract” for consumers.
  - Improvements: split view types into their own module or namespace, or explicitly document “view types are
    DTOs.”
  - API shape: Default usage is constrained and documented (crates/canic-core/src/dto/snapshot.rs).

  supporting crates (memory, utils, cdk, etc.)

  - Does well: canic-cdk provides a stable SDK facade (crates/canic-cdk/src/lib.rs); canic-memory centralizes
    stable-memory registry (crates/canic-memory/src/lib.rs); canic-testkit improves host-side testing (crates/
    canic-testkit/src/lib.rs).
  - Leaks/overreach: canic-core still depends directly on ic-cdk (crates/canic-core/Cargo.toml), undermining the
    facade intent.
  - Leaks/overreach: canic-memory uses export() on view types (crates/canic-memory/src/ops.rs), mirroring the
    naming inconsistency.
  - Improvements: tighten dependency direction so core uses canic-cdk only; align naming conventions across
    crates.

  Cross-Cutting Concerns

  - Error handling: layered errors are present, but canic::Error collapses many variants to strings (crates/
    canic-core/src/lib.rs), reducing typed composability and API stability.
  - Logging & observability: strong runtime logging and metrics endpoints (crates/canic-core/src/log.rs, crates/
    canic-core/src/macros/endpoints/mod.rs), but ANSI coloring may be noisy and logs lack structured correlation
    IDs.
  - Stable memory strategy: explicit IDs and a registry are solid (crates/canic-core/src/model/memory/mod.rs,
    crates/canic-memory/src/ops.rs), but there’s no explicit schema/version migration mechanism.
  - Upgrade safety: lifecycle adapters are synchronous and schedule async bootstrap correctly (crates/canic-
    core/src/lifecycle/*), but env validation differs between init and post-upgrade paths (crates/canic-core/
    src/workflow/runtime/mod.rs).
  - Testing ergonomics: test helpers exist (crates/canic-testkit/src/lib.rs), but global config/env dependencies
    force test-specific setup patterns (crates/canic-core/src/config/mod.rs).

  Integration Feedback

  - External consumers get a clean facade and prelude (crates/canic/src/lib.rs), which is intuitive for canister
    authors.
  - The auto-exposed endpoint bundle in canic_endpoints! is powerful but can be surprising without explicit opt-
    in documentation (crates/canic-core/src/macros/endpoints/mod.rs).
  - DTO/view distinction isn’t obvious at the module level; consumers may treat views as stable API without
    realizing they are projections.
  - Build-time requirements like CANIC_CONFIG_PATH and DFX_NETWORK are correct but easy to miss unless
    explicitly surfaced in quick-start docs (crates/canic-core/src/macros/start.rs).

  Actionable Recommendations

  Short-term fixes (non-breaking)

  1. Rename EnvOps::export/import and MemoryRegistryOps::export to export_view/import_view (or introduce *Data)
     and update crates/canic-core/src/macros/endpoints/mod.rs.
  2. Split upgrade_canister into needs_upgrade (policy) and upgrade_canister (ops) in crates/canic-core/src/ops/
     ic/mgmt.rs.
  3. Separate control vs view ops (module split or naming) for LogOps in crates/canic-core/src/ops/runtime/
     log.rs.
  4. Remove direct ic-cdk dependency from crates/canic-core/Cargo.toml to enforce the canic-cdk facade.

  Medium-term refactors

  1. Move env defaulting/normalization into EnvOps and have workflow call it exclusively (crates/canic-core/src/
     ops/runtime/env.rs, crates/canic-core/src/workflow/runtime/mod.rs).
  2. Introduce a dedicated view module or crate so view types are distinct from DTOs (crates/canic-core/src/dto/
     *).
  3. Add error codes or stable error enums to avoid string-only APIs in crates/canic-core/src/lib.rs.
  4. Add explicit *Data versioning to stable-memory types for upgrade safety (crates/canic-core/src/model/
     memory/*).

  Long-term architectural bets

  1. Define explicit extension seams as traits for policy and workflow to enable pluggable strategies without
     cross-layer bleed.
  2. Formalize ABI versioning beyond dto::abi::v1 to support long-lived consumers.
  3. Build a migration framework for stable-memory schema evolution tied to *Data versions.

  If you want, I can drill down into a specific crate or produce a “diff-first” checklist for bringing code into
  full alignment with AGENTS.md.