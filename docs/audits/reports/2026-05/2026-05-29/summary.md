# Audit Summary - 2026-05-29

## Included Reports

| Report | Status | Summary |
| --- | --- | --- |
| `audience-target-binding.md` | PASS AFTER FIX | Audience binding still holds in verifier paths; artifact role resolution now fails closed to exactly one scoped package metadata role under the selected canister root. |
| `capability-scope-enforcement.md` | PASS WITH TEMPLATE CLEANUP | Capability and scope authorization still runs after authentication and identity binding; the recurring audit template now points at the current endpoint auth hotspot. |
| `change-friction.md` | PASS | 0.48 routine slices are narrower than the 0.33 hard-cut baseline; no cross-layer leakage was confirmed, with remaining pressure in setup-surface, root-capability, and deployment-truth change paths. |
| `dependency-hygiene.md` | PASS | Published crates still avoid unpublished workspace-member dependencies; operator crates remain facade-free, `ic-testkit` stays test-only, and `canic` defaults are narrow again. |
| `expiry-replay-single-use.md` | PASS AFTER FIX | Delegated grants and role/internal attestations now reject at the exact expiry timestamp, aligning them with delegated tokens, replay metadata, consumed-token state, and sessions. |
| `module-structure.md` | PASS | Module discovery, dependency direction, public/runtime seam containment, and testkit separation still hold; current risk is coordination pressure in host deployment-truth, CLI deploy, and install-root modules. |
| `subject-caller-binding.md` | PASS | Delegated-token subject binding still routes through the canonical access-auth verifier; generated endpoint wrappers preserve transport-caller and authenticated-subject lanes. |
| `token-trust-chain.md` | PASS | Delegated-token trust-chain validation still requires verifier-local root trust, root-certified shard authority, canonical cert/claim hashes, shard signatures, and endpoint guard ordering. |

## Findings

- No remaining blockers.
- Focused code remediation was applied for exclusive expiry-boundary alignment
  and scoped canister role manifest resolution.
- Audit-template remediation was applied for the capability-scope recurring
  hotspot table after endpoint auth ownership moved into `access/auth/token.rs`.
- Residual risk remains low and unchanged from the previous comparable run:
  `3 / 10` for subject-caller binding, expiry/replay/single-use,
  capability-scope enforcement, token-trust-chain, and post-remediation
  audience/target binding.
- Dependency hygiene risk remains low at `2 / 10`; the public facade default
  feature pressure improved because `control-plane`, `sharding`, and
  `auth-crypto` are no longer default-on.
- Module structure risk is moderate at `4 / 10`, driven by broad host/CLI
  coordination files rather than dependency direction, circularity, or
  product/test seam leakage.
- Change-friction risk is moderate at `4 / 10`, improved from the `5 / 10`
  0.33 hard-cut baseline. Sampled 0.48 routine slices average `28.43` touched
  files, down from `63.17`.

## Follow-ups

- Keep `access/auth/token.rs`, `access/expr/mod.rs`, and endpoint macro
  expansion aligned whenever authenticated endpoint syntax or delegated-session
  identity resolution changes.
- Keep `AuthApi::verify_token_material(...)` private unless a future public
  helper performs the full endpoint authorization boundary.
- Keep delegated-token verification, delegated grants, role attestations,
  replay metadata, sessions, and consumed-token markers on the same exclusive
  `now >= expires_at` boundary.
- Keep every configured fleet role backed by exactly one package under the
  selected canister root with matching `[package.metadata.canic] role`.
- Keep delegated-token verifier orchestration, pure chain verification,
  root-key resolution, endpoint access guards, and role-attestation refresh
  behavior aligned whenever delegated issuer formats or subnet-state
  propagation change.
- Keep capability proof DTOs, endpoint auth ordering, and root capability
  workflow authorization/replay changes coordinated across API, ops, workflow,
  metrics, and tests.
- Keep `ic-testkit` restricted to internal test harnesses and test/audit
  canisters, and keep `canic` default features narrow unless a release line
  explicitly chooses broader defaults.
- Split `canic-host::deployment_truth` implementation files before adding
  another broad deployment phase family.
- Split `crates/canic-cli/src/deploy/mod.rs` before adding more
  deployment-truth commands.
- Treat metadata-driven startup/build changes as public setup-surface changes
  and keep future edits behind `build_support`, scaffold, and macro tests.
- Budget root request or proof-mode additions as coordinated DTO, API,
  workflow, replay, metrics, Candid/docs, and test slices.
