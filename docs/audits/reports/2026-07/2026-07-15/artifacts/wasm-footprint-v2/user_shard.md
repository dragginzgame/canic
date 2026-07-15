# Wasm Detail: `user_shard`

| Metric | Value |
| --- | ---: |
| Kind | leaf-canister |
| Release Wasm bytes | 2368860 |
| Release gzip bytes | 789006 |
| Debug Wasm bytes | 5079760 |
| Debug gzip bytes | 1287649 |
| Debug delta | +2710900 (114.44%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 4657 |
| Data sections / bytes | 3 / 194136 |
| Exported methods | 26 |
| Largest shallow item | data[0] (193554 bytes) |
| Largest retained item | table[0] (1484243 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼───────────────────────
        1484243 ┊     62.66% ┊ table[0]
        1484237 ┊     62.66% ┊   ⤷ elem[0]
         219187 ┊      9.25% ┊       ⤷ code[32]
         208735 ┊      8.81% ┊           ⤷ code[19]
         208267 ┊      8.79% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v2` without duplicating raw data.
