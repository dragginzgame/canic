# Wasm Detail: `user_hub`

| Metric | Value |
| --- | ---: |
| Kind | leaf-canister |
| Release Wasm bytes | 2369688 |
| Release gzip bytes | 789228 |
| Debug Wasm bytes | 5094882 |
| Debug gzip bytes | 1291598 |
| Debug delta | +2725194 (115.00%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 4674 |
| Data sections / bytes | 3 / 194456 |
| Exported methods | 25 |
| Largest shallow item | data[0] (193858 bytes) |
| Largest retained item | table[0] (1481564 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼──────────────────────
        1481564 ┊     62.52% ┊ table[0]
        1481558 ┊     62.52% ┊   ⤷ elem[0]
         383979 ┊     16.20% ┊       ⤷ code[24]
         274459 ┊     11.58% ┊           ⤷ code[4]
         204555 ┊      8.63% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v2` without duplicating raw data.
