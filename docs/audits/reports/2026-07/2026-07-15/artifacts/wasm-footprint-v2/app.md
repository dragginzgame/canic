# Wasm Detail: `app`

| Metric | Value |
| --- | ---: |
| Kind | leaf-canister |
| Release Wasm bytes | 2251324 |
| Release gzip bytes | 747279 |
| Debug Wasm bytes | 4816015 |
| Debug gzip bytes | 1219145 |
| Debug delta | +2564691 (113.92%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 4411 |
| Data sections / bytes | 3 / 189240 |
| Exported methods | 21 |
| Largest shallow item | data[0] (188666 bytes) |
| Largest retained item | table[0] (1381904 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼──────────────────────
        1381904 ┊     61.38% ┊ table[0]
        1381898 ┊     61.38% ┊   ⤷ elem[0]
         384051 ┊     17.06% ┊       ⤷ code[24]
         274481 ┊     12.19% ┊           ⤷ code[4]
         198527 ┊      8.82% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v2` without duplicating raw data.
