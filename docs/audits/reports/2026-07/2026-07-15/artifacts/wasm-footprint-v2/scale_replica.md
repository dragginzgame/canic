# Wasm Detail: `scale_replica`

| Metric | Value |
| --- | ---: |
| Kind | leaf-canister |
| Release Wasm bytes | 2260688 |
| Release gzip bytes | 752095 |
| Debug Wasm bytes | 4835444 |
| Debug gzip bytes | 1225414 |
| Debug delta | +2574756 (113.89%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 4438 |
| Data sections / bytes | 3 / 189372 |
| Exported methods | 22 |
| Largest shallow item | data[0] (188794 bytes) |
| Largest retained item | table[0] (1389740 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼──────────────────────
        1389740 ┊     61.47% ┊ table[0]
        1389734 ┊     61.47% ┊   ⤷ elem[0]
         384001 ┊     16.99% ┊       ⤷ code[23]
         274481 ┊     12.14% ┊           ⤷ code[4]
         198766 ┊      8.79% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v2` without duplicating raw data.
