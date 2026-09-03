# Wasm Detail: `user_shard`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3259113 |
| Release gzip bytes | 1161338 |
| Debug Wasm bytes | 7410085 |
| Debug gzip bytes | 1858964 |
| Debug delta | +4150972 (127.37%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3464190 → 3259113 |
| Optimizer gzip bytes | 1140899 → 1161338 |
| Optimizer code-section bytes | 3203202 → 3001399 |
| Optimizer data-section bytes | 217277 → 214712 |
| Optimizer defined functions | 6103 → 5341 |
| Functions | 5384 |
| Data sections / bytes | 195 / 213096 |
| Exported methods | 13 |
| Largest shallow item | code[1053] (126498 bytes) |
| Largest retained item | table[0] (1453247 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1453247 ┊     44.59% ┊ table[0]
        1453241 ┊     44.59% ┊   ⤷ elem[0]
         254257 ┊      7.80% ┊       ⤷ code[13]
         150827 ┊      4.63% ┊           ⤷ code[4]
         215795 ┊      6.62% ┊ [172 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v6` without duplicating raw data.
