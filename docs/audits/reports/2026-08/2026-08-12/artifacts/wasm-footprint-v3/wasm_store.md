# Wasm Detail: `wasm_store`

| Metric | Value |
| --- | ---: |
| Kind | wasm-store |
| Release Wasm bytes | 2597251 |
| Release gzip bytes | 855667 |
| Debug Wasm bytes | 5553475 |
| Debug gzip bytes | 1377059 |
| Debug delta | +2956224 (113.82%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 5046 |
| Data sections / bytes | 3 / 216224 |
| Exported methods | 31 |
| Largest shallow item | data[0] (215710 bytes) |
| Largest retained item | table[0] (1203715 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1203715 ┊     46.35% ┊ table[0]
        1203709 ┊     46.35% ┊   ⤷ elem[0]
         188059 ┊      7.24% ┊       ⤷ code[2]
          85218 ┊      3.28% ┊           ⤷ code[133]
          35873 ┊      1.38% ┊           ⤷ code[13]
         228655 ┊      8.80% ┊ [4 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v3` without duplicating raw data.
