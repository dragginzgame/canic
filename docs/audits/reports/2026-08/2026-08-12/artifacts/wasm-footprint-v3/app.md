# Wasm Detail: `app`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3006400 |
| Release gzip bytes | 980885 |
| Debug Wasm bytes | 6433987 |
| Debug gzip bytes | 1585714 |
| Debug delta | +3427587 (114.01%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 5449 |
| Data sections / bytes | 3 / 236516 |
| Exported methods | 26 |
| Largest shallow item | data[0] (236102 bytes) |
| Largest retained item | table[0] (1377137 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼──────────────────────
        1377137 ┊     45.81% ┊ table[0]
        1377131 ┊     45.81% ┊   ⤷ elem[0]
         411769 ┊     13.70% ┊       ⤷ code[14]
         278275 ┊      9.26% ┊           ⤷ code[3]
         261704 ┊      8.70% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v3` without duplicating raw data.
