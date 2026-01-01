• FILE: crates/canic-core/src/lib.rs
  ISSUE: Layer violation – core error conversion exposes PublicError outside api
  CURRENT: core (crate root)
  SHOULD BE: api
  REASON: Anything returning PublicError must live in api.

  FILE: crates/canic-core/src/access/guard.rs
  ISSUE: Layer violation – guard helpers return PublicError
  CURRENT: access
  SHOULD BE: api
  REASON: Anything returning PublicError must live in api.

  FILE: crates/canic-core/src/access/auth.rs
  ISSUE: Layer violation – auth rules return PublicError
  CURRENT: access
  SHOULD BE: api
  REASON: Anything returning PublicError must live in api.

  FILE: crates/canic-core/src/access/rule.rs
  ISSUE: Layer violation – rule helpers return PublicError
  CURRENT: access
  SHOULD BE: api
  REASON: Anything returning PublicError must live in api.

  FILE: crates/canic-core/src/domain/mod.rs
  ISSUE: Layer violation – DomainError exposes PublicError conversion
  CURRENT: domain/policy
  SHOULD BE: api
  REASON: Anything returning PublicError must live in api.

  FILE: crates/canic-core/src/ops/rpc/mod.rs
  ISSUE: Layer violation – ops error path carries PublicError
  CURRENT: ops
  SHOULD BE: api
  REASON: Anything returning PublicError must live in api.

  FILE: crates/canic-core/src/workflow/facade/read.rs
  ISSUE: Layer violation – workflow exposes pass-through read helpers
  CURRENT: workflow
  SHOULD BE: api
  REASON: Workflow must orchestrate, not expose thin wrappers; user-facing helpers belong in api.

  FILE: crates/canic-core/src/workflow/rpc/request/mod.rs
  ISSUE: Layer violation – workflow wraps ops RPC without orchestration
  CURRENT: workflow
  SHOULD BE: api
  REASON: Workflow must orchestrate; thin RPC façades belong in api, not workflow.

  FILE: crates/canic-core/src/api/endpoints/mod.rs
  ISSUE: Layer violation – endpoints call workflow directly
  CURRENT: api/endpoints
  SHOULD BE: api
  REASON: Workflow must not be user-callable; endpoints should delegate to api wrappers.

  FILE: crates/canic-core/src/ops/runtime/log.rs
  ISSUE: Layer violation – log retention policy decided in ops
  CURRENT: ops
  SHOULD BE: domain/policy
  REASON: Decisions about what should happen belong in domain/policy; ops should only apply them.

  FILE: crates/canic-core/src/ops/storage/pool.rs
  ISSUE: Layer violation – pool controller selection decided in ops
  CURRENT: ops
  SHOULD BE: domain/policy
  REASON: Decisions about who should control canisters belong in domain/policy.

  FILE: crates/canic-core/src/ops/storage/registry/subnet.rs
  ISSUE: Layer violation – singleton cardinality policy enforced in ops
  CURRENT: ops
  SHOULD BE: domain/policy
  REASON: Policy decisions (cardinality/role uniqueness) belong in domain/policy.

  FILE: crates/canic-core/src/storage/memory/log.rs
  ISSUE: Layer violation – retention policy and pruning in storage
  CURRENT: model/storage
  SHOULD BE: domain/policy
  REASON: Storage may store and retrieve only; policy decisions must live above it.

  FILE: crates/canic-core/src/storage/memory/pool.rs
  ISSUE: Layer violation – ready-canister selection policy in storage
  CURRENT: model/storage
  SHOULD BE: domain/policy
  REASON: Storage must not select or rank; selection policy belongs in domain/policy.

  FILE: crates/canic-core/src/storage/memory/registry/subnet.rs
  ISSUE: Layer violation – traversal and filtering logic in storage
  CURRENT: model/storage
  SHOULD BE: ops
  REASON: Storage may store and retrieve only; traversal/filtering belongs in ops.

  FILE: crates/canic-core/src/storage/memory/scaling.rs
  ISSUE: Layer violation – filtered query in storage
  CURRENT: model/storage
  SHOULD BE: ops
  REASON: Storage may store and retrieve only; filtering belongs in ops.

  FILE: crates/canic-core/src/storage/memory/sharding/registry.rs
  ISSUE: Layer violation – filtered/derived queries in storage
  CURRENT: model/storage
  SHOULD BE: ops
  REASON: Storage may store and retrieve only; query shaping belongs in ops.

  FILE: crates/canic-core/src/api/perf.rs
  ISSUE: Layer violation – API returns non-DTO perf types
  CURRENT: api
  SHOULD BE: api
  REASON: API boundary must return DTO-shaped types.

  FILE: crates/canic-core/src/api/endpoints/mod.rs
  ISSUE: Layer violation – API endpoints return non-DTO types (PerfEntry, CanisterStatusResult)
  CURRENT: api/endpoints
  SHOULD BE: api
  REASON: API boundary must return DTO-shaped types.