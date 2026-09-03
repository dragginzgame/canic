# Wasm Detail: `scale_hub`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3252022 |
| Release gzip bytes | 1157869 |
| Debug Wasm bytes | 7410414 |
| Debug gzip bytes | 1858900 |
| Debug delta | +4158392 (127.87%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3457034 → 3252022 |
| Optimizer gzip bytes | 1141493 → 1157869 |
| Optimizer code-section bytes | 3198807 → 2997048 |
| Optimizer data-section bytes | 218405 → 215853 |
| Optimizer defined functions | 6062 → 5295 |
| Functions | 5334 |
| Data sections / bytes | 194 / 214245 |
| Exported methods | 11 |
| Largest shallow item | code[1050] (126498 bytes) |
| Largest retained item | table[0] (1514356 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1514356 ┊     46.57% ┊ table[0]
        1514350 ┊     46.57% ┊   ⤷ elem[0]
         250105 ┊      7.69% ┊       ⤷ code[4]
          38492 ┊      1.18% ┊           ⤷ code[30]
         214157 ┊      6.59% ┊ [173 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v6` without duplicating raw data.
