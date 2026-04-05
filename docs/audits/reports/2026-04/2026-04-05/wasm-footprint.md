# Wasm Footprint Audit - 2026-04-05

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: `docs/audits/recurring/system/wasm-footprint.md`
- Retained summary policy: `0.25` keeps one retained summary per audit and drops same-day duplicates/artifacts by default.
- Code snapshot identifier: `c027b8df`
- Method tag/version: `Method V1`
- Comparability status: `comparable`
- Auditor: `codex`
- Run timestamp (UTC): `2026-04-05T16:21:11Z`
- Branch: `main`
- Worktree: `dirty`
- Profile: `release`
- Target canisters in scope: `app` `minimal` `user_hub` `user_shard` `scale_hub` `scale` `test` `root`

## Summary

- This retained summary keeps the final same-day `0.25` wasm readout after the demo-vs-test cleanup and testkit/public-surface trimming work.
- Same-day reruns earlier on `2026-04-05` were used for comparison during the audit sweep, but only this final retained summary is kept.
- The shared leaf floor remains tight, while `root` remains a separate control-plane outlier.

## Findings

| Check | Result | Evidence |
| --- | --- | --- |
| Current scope rebuilt in `release` profile | PASS | Final retained size snapshot records the current `release` canister set. |
| Shared leaf floor still easy to read | PASS | `minimal` remains close to `app`, `scale`, and `scale_hub`, so common runtime pressure is still visible. |
| Root still treated separately | PASS | `root` remains a control-plane bundle outlier and is not compared directly to leaf peers. |
| Same-day duplicate reports removed | PASS | This file is the single retained `wasm-footprint` summary for `2026-04-05`. |

## Current Size Snapshot

| Canister | Shrunk wasm | Shrink delta | Same-day delta | Note |
| --- | ---: | ---: | ---: | --- |
| `app` | 1668761 | 115271 | +1071 | role-specific leaf |
| `minimal` | 1668763 | 115269 | +15013 | shared runtime floor |
| `user_hub` | 1831164 | 126316 | +15027 | role-specific leaf |
| `user_shard` | 1779585 | 124141 | -15833 | role-specific leaf |
| `scale_hub` | 1729805 | 119390 | +11120 | role-specific leaf |
| `scale` | 1686593 | 116352 | +15001 | role-specific leaf |
| `test` | 1727326 | 120012 | -35816 | role-specific leaf |
| `root` | 3710278 | 217700 | -9525 | control-plane bundle outlier |

## Structural Readout

- `minimal` remains the shared-runtime floor at `1668763` shrunk bytes.
- `app` is effectively identical at `1668761`, which means most remaining pressure is still common runtime rather than app-specific logic.
- `user_hub` remains the heaviest non-root leaf at `1831164`.
- `test` and `user_shard` were the main same-day winners in the final rerun.
- `root` remains materially larger at `3710278` because it still bundles control-plane behavior and the bootstrap `wasm_store` artifact path.

## Conclusion

- The `0.25` wasm audit is retained as a single summary at this canonical path.
- The leaf floor is materially below the earlier `0.24` high-water mark, but shared-runtime pressure is still obvious because `minimal` stays close to feature canisters.
- The next meaningful wasm work should continue attacking shared runtime/auth/control fan-in rather than role-local logic.
