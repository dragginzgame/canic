# Wasm Detail: `scale_hub`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3047098 |
| Release gzip bytes | 994075 |
| Debug Wasm bytes | 6517862 |
| Debug gzip bytes | 1605304 |
| Debug delta | +3470764 (113.90%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 5531 |
| Data sections / bytes | 3 / 238624 |
| Exported methods | 29 |
| Largest shallow item | data[0] (238206 bytes) |
| Largest retained item | table[0] (1407085 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼──────────────────────
        1407085 ┊     46.18% ┊ table[0]
        1407079 ┊     46.18% ┊   ⤷ elem[0]
         411770 ┊     13.51% ┊       ⤷ code[13]
         278276 ┊      9.13% ┊           ⤷ code[3]
         264422 ┊      8.68% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v3` without duplicating raw data.
