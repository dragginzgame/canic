# Wasm Detail: `scale_replica`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3016265 |
| Release gzip bytes | 985294 |
| Debug Wasm bytes | 6454190 |
| Debug gzip bytes | 1592950 |
| Debug delta | +3437925 (113.98%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 5480 |
| Data sections / bytes | 3 / 236656 |
| Exported methods | 27 |
| Largest shallow item | data[0] (236238 bytes) |
| Largest retained item | table[0] (1385466 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼──────────────────────
        1385466 ┊     45.93% ┊ table[0]
        1385460 ┊     45.93% ┊   ⤷ elem[0]
         411719 ┊     13.65% ┊       ⤷ code[14]
         278275 ┊      9.23% ┊           ⤷ code[3]
         261951 ┊      8.68% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v3` without duplicating raw data.
