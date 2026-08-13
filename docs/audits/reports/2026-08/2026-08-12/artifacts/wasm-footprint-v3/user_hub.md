# Wasm Detail: `user_hub`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3135440 |
| Release gzip bytes | 1026694 |
| Debug Wasm bytes | 6724477 |
| Debug gzip bytes | 1663661 |
| Debug delta | +3589037 (114.47%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 5743 |
| Data sections / bytes | 3 / 241440 |
| Exported methods | 32 |
| Largest shallow item | data[0] (240982 bytes) |
| Largest retained item | table[0] (1486455 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼──────────────────────
        1486455 ┊     47.41% ┊ table[0]
        1486449 ┊     47.41% ┊   ⤷ elem[0]
         412242 ┊     13.15% ┊       ⤷ code[13]
         278798 ┊      8.89% ┊           ⤷ code[3]
         267739 ┊      8.54% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v3` without duplicating raw data.
