 High-Level Architectural Assessment

  - Overall strengths: clear module separation in canic-core, stable-memory primitives in canic-memory, and a
    consistent macro pipeline (guard/auth/rule/dispatch) that keeps endpoints thin; the ops/policy/workflow
    split is mostly respected and readable.
  - Systemic risks: layering drift in docs and a few code paths (workflow reaches into infra/model), plus
    example canisters that bypass workflow/policy, create ambiguity about the intended usage patterns; public
    API surface in canic-core is broad and may ossify quickly.
  - Architectural maturity level: mid-maturity—strong scaffolding and conventions are in place, but there are
    still real boundary leaks and documentation mismatches that need cleanup before long-term stability.

  Layer-by-Layer Review

  Model

  - Does well: stable memory ownership is clean and localized; canonical *Data snapshots are used and model is
    not exported outside the crate, e.g. crates/canic-core/src/model/memory/directory/app.rs.
  - Leaks or overreaches: workflow consumes model data types directly in crates/canic-core/src/workflow/
    directory/builders.rs, bypassing ops adapters and weakening the boundary.
  - Concrete suggestions: route directory building through ops adapters (or a workflow-local DTO/view) and keep
    model types crate-local to ops only.

  Ops

  - Does well: ops::ic centralizes allowed IC surface area; adapters clearly map model data to DTO/view types;
    read vs write ops are separated into storage/runtime/view modules.
  - Leaks or overreaches: doc claims “perform orchestration” in crates/canic-core/src/ops/mod.rs, which
    conflicts with the architecture; some public ops (e.g., RPC helpers) are easy to call directly from
    endpoints, enabling bypass of workflow/policy.
  - Concrete suggestions: update ops docs to remove orchestration language and explicitly mark “single-step”
    semantics; consider a public “read-only ops” facade for policy consumers to prevent accidental mutation.

  Policy

  - Does well: pure, deterministic logic; no serialization or async; reads state only via ops; planning APIs are
    explicit and composable.
  - Leaks or overreaches: policy reads registry internals via ops exports that return model-shaped data (e.g.,
    crates/canic-core/src/policy/placement/sharding/metrics.rs), tying policy to storage structure.
  - Concrete suggestions: define policy-facing view types or read-only ops exports that abstract the storage
    layout.

  Workflow

  - Does well: multi-step orchestration is centralized; policy decisions are invoked explicitly; ops are used
    for mutations and IC calls; timer scheduling is handled in lifecycle and runtime flows.
  - Leaks or overreaches: workflow uses infra::ic directly in crates/canic-core/src/workflow/runtime/mod.rs and
    crates/canic-core/src/workflow/bootstrap/root.rs; model data types are used directly in crates/canic-core/
    src/workflow/directory/builders.rs; naming like ShardingOps inside workflow (in crates/canic-core/src/
    workflow/placement/sharding/assign.rs) blurs layer identity.
  - Concrete suggestions: route all IC access through ops::ic, move model-typed builder outputs behind ops
    adapters, and rename workflow-level “Ops” to “Workflow/Orchestrator” to preserve mental model.

  Endpoints / macros

  - Does well: macros enforce guard/auth/rule and dispatch instrumentation; root endpoints delegate to workflow
    for multi-step operations; query endpoints are thin.
  - Leaks or overreaches: macro docs in crates/canic-core/src/macros/start.rs say “must not schedule timers” but
    do; example canisters call policy/ops directly (e.g., crates/canisters/scale_hub/src/lib.rs, crates/
    canisters/shard_hub/src/lib.rs, crates/canisters/blank/src/lib.rs) which can teach the wrong layering.
  - Concrete suggestions: align macro docs with actual behavior and either route example endpoints through
    workflow or label them as test-only deviations from the layering rules.

  DTO

  - Does well: DTOs are pure data with no logic and no Default impls; serialization stays at the boundary.
  - Leaks or overreaches: view types live alongside DTOs and are sometimes returned directly, which can blur the
    “view vs DTO” boundary.
  - Concrete suggestions: split views into a dedicated module or explicitly document which *View types are
    intended as boundary DTOs.

  Supporting crates (memory, utils, cdk, etc.)

  - Does well: canic-memory provides a clear stable-memory registry and macros; canic-cdk isolates IC types/
    spec; canic-utils keeps generic helpers out of core.
  - Leaks or overreaches: workflow still bypasses ops to use infra directly; tests and examples use custom
    helpers (canic-testkit::Fake) rather than the AGENTS-specified p pattern, which is a consistency gap.
  - Concrete suggestions: enforce “infra only via ops” and reconcile test helper guidance (document one
    canonical helper).

  Cross-Cutting Concerns

  - Error handling: canic_core::Error collapses layered errors into strings for Candid stability, which is
    stable but loses structured error detail; consider error codes or a typed error DTO for external consumers
    in the future (crates/canic-core/src/lib.rs).
  - Logging & observability: logging is centralized and metrics views are well-defined; readiness gating avoids
    traps during early init; good coverage via ops metrics and access logging (crates/canic-core/src/log.rs,
    crates/canic-core/src/ops/runtime/metrics.rs).
  - Stable memory strategy: explicit range reservations and registry summaries give deterministic layout; model/
    memory separation aligns with stable-structures usage (crates/canic-memory/src/ops.rs, crates/canic-core/
    src/ops/runtime/memory.rs).
  - Upgrade safety: lifecycle adapters schedule async bootstraps and trap on root failures; however init vs
    post-upgrade behavior diverges in runtime (payload import is only on init), so this should be explicitly
    justified against the AGENTS “same execution model” rule (crates/canic-core/src/workflow/runtime/mod.rs).
  - Testing ergonomics: unit tests exist for policy/ops, and testkit provides deterministic principals; there
    are few tests that assert layer boundaries or detect infra/model leaks from workflow.

  Integration Feedback

  - The canic facade + prelude is approachable and keeps entrypoints small, but the sheer width of canic-core
    makes it easy for consumers to bypass workflow/policy unintentionally.
  - Intuitive: macro pipeline and ops adapters are clear; surprising: “Ops” naming inside workflow and example
    canisters that directly call policy/ops.
  - Missing: a concise “when to use workflow vs ops vs policy” guide for downstream canisters, and explicit
    guidance for dry-run endpoints to avoid layer bypass.

  Actionable Recommendations

  Short-term fixes (non-breaking)

  - Update layering docs in crates/canic-core/src/lib.rs, crates/canic-core/src/ops/mod.rs, and crates/canic-
    core/src/macros/start.rs to match AGENTS.
  - Replace infra::ic usage in workflow with ops::ic re-exports in crates/canic-core/src/workflow/runtime/mod.rs
    and crates/canic-core/src/workflow/bootstrap/root.rs.
  - Move crates/canic-core/src/workflow/directory/builders.rs behind ops adapters or refactor to output view
    types only.
  - Mark example canisters that call ops/policy directly as test-only or reroute them through workflow (crates/
    canisters/scale_hub/src/lib.rs, crates/canisters/shard_hub/src/lib.rs, crates/canisters/blank/src/lib.rs).

  Medium-term refactors

  - Add a policy-facing read-only ops facade that returns views rather than model data.
  - Normalize naming across workflow modules (e.g., rename ShardingOps to ShardingWorkflow).
  - Separate DTOs vs views into distinct modules or provide explicit module-level contracts.
  - Add a CI lint or module-level check to prevent workflow from importing infra or model directly.

  Long-term architectural bets

  - Enforce layer boundaries via crate-level features or a separate policy/workflow crate split.
  - Reduce public surface area of canic-core and stabilize only the facade exports.
  - Introduce architecture tests that parse module dependencies and fail on upward imports.

  If you want, I can:

  1. Produce a concrete dependency violation report (file-level list with proposed patches).
  2. Draft the refactors for workflow infra usage and directory builders.
  3. Align example canisters with the layering rules and update docs accordingly.

 
› Implement {feature}
 
  77% context left · ? for shortcuts

