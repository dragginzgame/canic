# Audit Summary - 2026-05-13

## Run Contexts

| Report | Type | Scope | Snapshot | Worktree | Status |
| --- | --- | --- | --- | --- | --- |
| `instruction-footprint.md` | Recurring system | Canic instruction footprint across internal leaf/root/scaling probes plus selected root update flows | `c533afd6` plus current 0.35.4 worktree | dirty | complete |
| `audience-target-binding.md` | Recurring invariant | Delegated-token, role-attestation, RPC capability, and delegated-grant audience/target binding | `c533afd6` plus current 0.35.4 worktree | dirty | complete |

## Risk Index Summary

| Report | Risk | Readout |
| --- | ---: | --- |
| `instruction-footprint.md` | 3 / 10 | Query visibility and endpoint measurement are working for the sampled matrix. Remaining risk is first-run `0.35` comparability plus limited deeper attribution for delegated auth. |
| `audience-target-binding.md` | 3 / 10 | Audience/target binding still fails closed across delegated tokens, role attestations, and capability proofs. Residual risk is structural fan-out and stale external or branch-local config snippets. |

## Method / Comparability Notes

- `instruction-footprint.md` uses `Method V1`.
- `audience-target-binding.md` uses `Method V4.1`.
- This is the first `0.35` instruction-footprint baseline, so baseline deltas
  are `N/A` until a comparable rerun exists.
- Query lanes use local-only `QueryPerfSample` probe endpoints because query
  perf rows are not committed.
- `audience-target-binding.md` is comparable with the 2026-05-07 run; the
  current run replaces stale blocked filters with current passing filters.

## Key Findings by Severity

### Medium-Low

- `root::canic_request_delegation:fresh-shard` is the highest sampled endpoint
  at `800834` average local instructions.
- `root::canic_response_capability_v1:cycles-request` is the second sampled
  hotspot at `506816` average local instructions.
- Auth/capability audience correctness remains spread across DTO, delegated
  auth ops, RPC capability verification, outbound RPC attestation caching, and
  PocketIC support.

### Low

- `delegated auth issuance/verification` currently lacks deeper checkpoint
  attribution, so it remains an endpoint-total lane unless that cost becomes a
  concrete optimization target.
- The audit scaling probe config had drifted behind the current
  `scale_replica` role name and was corrected before the successful run.
- Obsolete per-canister verifier tables are rejected by the live config schema;
  docs now point at `[auth.delegated_tokens]` and per-canister `auth` flags.

## Verification Rollup

| Report | PASS | BLOCKED | FAIL | Notes |
| --- | ---: | ---: | ---: | --- |
| `instruction-footprint.md` | 6 | 1 | 0 | PocketIC runner completed; first-run baseline comparison is blocked until a comparable rerun exists. |
| `audience-target-binding.md` | 6 | 0 | 0 | Unit and PocketIC coverage passed for delegated grant audience mismatch, delegated-token audience narrowing, role-audience local-role/hash checks, role-attestation mismatch paths, and capability proof hash/audience rejection. |

## Follow-up Actions

1. Rerun the instruction-footprint audit after the next concrete performance
   change so this baseline gets real drift deltas.
2. If root delegation remains the highest sampled endpoint after rerun, add
   focused delegated-auth checkpoints before attempting optimization.
3. Keep current auth config docs and examples focused on
   `[auth.delegated_tokens]`, `[auth.role_attestation]`, and per-canister
   `auth` flags.
