# Audience Target Binding Invariant Audit - 2026-05-29

## Report Preamble

- Scope: delegated-token audience binding, role-attestation audience binding,
  internal-invocation proof audience binding, root capability target binding,
  delegated grant target binding, and build-time role-to-manifest resolution
  used by the sharding trust-chain test flow.
- Definition path:
  `docs/audits/recurring/invariants/audience-target-binding.md`
- Compared baseline report path:
  `docs/audits/reports/2026-05/2026-05-07/audience-target-binding.md`
- Code snapshot identifier: `3e96578d`
- Method tag/version: `Method V4.1`
- Comparability status: `comparable with remediation`
- Auditor: `codex`
- Run timestamp: `2026-05-29`
- Worktree: `dirty`

## Executive Summary

Verdict: **PASS AFTER FIX**.

The verifier-side audience-target invariant still holds. Delegated-token
verification continues to require claim audience subset, local verifier
membership in both claim and cert audiences, and single-role cert audiences
when role hashing is used. Role attestations and internal invocation proofs
continue to bind `audience` to the local canister principal before endpoint
dispatch. Root capability hashes and delegated grants continue to bind the
target canister.

The audit did find a setup/build defect adjacent to the invariant: release
artifact manifest discovery could resolve a role from workspace-wide package
metadata instead of the selected fleet canister root. In a workspace containing
`root_probe`, demo roots, test roots, and test-only canisters with duplicated
roles, this could build the wrong canister artifact for a configured role. That
was fixed by making manifest discovery scoped, exact, and fail-closed.

Risk score after remediation: **3 / 10**.

## Findings

| ID | Status | Severity | Area | Finding | Resolution |
| --- | --- | --- | --- | --- | --- |
| ATB-2026-05-29-1 | PASS | High | Delegated-token verifier | Claims audience must remain a subset of cert audience, and the local verifier must be in both audiences. | Existing verifier and tests still pass. |
| ATB-2026-05-29-2 | PASS | High | Role/internal attestations | Role attestations and internal invocation proofs reject audience mismatch against the local canister. | Existing verifier and tests still pass. |
| ATB-2026-05-29-3 | PASS | High | Capability/grant target binding | Capability hashes and delegated grants bind the target canister before proof-specific acceptance. | Existing tests still pass. |
| ATB-2026-05-29-4 | FIXED | Medium | Build/setup role binding | Artifact builds could resolve role metadata from outside the selected fleet root, making duplicate workspace roles ambiguous. | Resolver now requires exactly one matching `[package.metadata.canic] role` under the selected canister root. |

## Remediation

- Removed package-name and workspace-wide fallback from host canister manifest
  resolution.
- Manifest lookup now searches only under the selected canister root and fails
  when zero or multiple packages declare the requested
  `[package.metadata.canic] role`.
- Root harness profile builds now set `CANIC_CANISTERS_ROOT` alongside
  profile-specific `CANIC_CONFIG_PATH` values so config files under
  `fleets/test/test-configs` do not move the canister root.
- Added `fleets/test/test` as the scoped fleet verifier canister for
  `role = "test"` instead of letting sharding tests reach into
  `canisters/test`.
- Added regression coverage for duplicate `role = "root"` packages where the
  selected fleet canister root must win.

## Verification Readout

Commands passed:

- `cargo +1.96.0 test -p canic-core --lib verify_delegated_token --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib resolve_root_key --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib cert_role_hash_rejects_multi_role_cert_audience --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib verify_root_delegated_grant_claims_rejects_audience_mismatch --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib verify_capability_hash_binding --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib root_capability_hash_binds_target_canister --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib role_attestation_claims_reject --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib internal_invocation_claims_reject --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib authorize_rejects_internal_invocation_proof_with_unknown_audience --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-core --lib authorize_accepts_internal_invocation_proof_from_app_index_role --locked -- --nocapture`
- `cargo +1.96.0 test -p canic-tests --test pic_role_attestation role_attestation_verification_paths --locked -- --test-threads=1 --nocapture`
- `cargo +1.96.0 test -p canic-host release_set --locked -- --nocapture`
- `cargo +1.96.0 check -p canic-host --locked`
- `cargo +1.96.0 check -p canister_test --locked`
- `cargo +1.96.0 test -p canic-tests --test root_suite delegated_token_verification_uses_cascaded_subnet_state_root_key --locked -- --nocapture`
- `cargo +1.96.0 fmt --all --check`
- `cargo +1.96.0 check -p canic-tests --locked`
- `cargo +1.96.0 clippy -p canic-host -p canic-tests -p canister_test --all-targets --locked -- -D warnings`
- `cargo +1.96.0 test -p canic --test changelog_governance --locked`
- `git diff --check`

Commands that intentionally found drift before remediation:

- `cargo +1.96.0 test -p canic-tests --test root_suite delegated_token_verification_uses_cascaded_subnet_state_root_key --locked -- --nocapture`

  Initial failure showed the root build resolving the wrong root artifact:
  delegated auth was configured, but the selected root artifact lacked
  `auth-crypto`. After scoped manifest resolution and the scoped `test`
  canister were added, the same command passed.

- `cargo +1.96.0 test -p canic-core --lib verify_role_attestation_claims --locked -- --nocapture`

  This stale filter matched zero tests; concrete replacement filters are listed
  above.

## Residual Risk

No blocker remains. The main watchpoint is operational: any fleet config that
lists a role must have exactly one package under the selected canister root with
matching `[package.metadata.canic] role`. That is now enforced by the artifact
resolver instead of inferred from package names or workspace-wide matches.
