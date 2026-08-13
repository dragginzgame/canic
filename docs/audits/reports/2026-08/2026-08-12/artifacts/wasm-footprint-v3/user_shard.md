# Wasm Detail: `user_shard`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3140027 |
| Release gzip bytes | 1028056 |
| Debug Wasm bytes | 6720789 |
| Debug gzip bytes | 1663219 |
| Debug delta | +3580762 (114.04%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 5728 |
| Data sections / bytes | 3 / 242296 |
| Exported methods | 33 |
| Largest shallow item | data[0] (241854 bytes) |
| Largest retained item | table[0] (1492867 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1492867 ┊     47.54% ┊ table[0]
        1492861 ┊     47.54% ┊   ⤷ elem[0]
         243950 ┊      7.77% ┊       ⤷ code[15]
         189254 ┊      6.03% ┊           ⤷ code[131]
         272623 ┊      8.68% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v3` without duplicating raw data.
