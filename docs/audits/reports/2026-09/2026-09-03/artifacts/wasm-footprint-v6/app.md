# Wasm Detail: `app`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 2887478 |
| Release gzip bytes | 1039059 |
| Debug Wasm bytes | 6503664 |
| Debug gzip bytes | 1620397 |
| Debug delta | +3616186 (125.24%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3067317 → 2887478 |
| Optimizer gzip bytes | 1020273 → 1039059 |
| Optimizer code-section bytes | 2827346 → 2650724 |
| Optimizer data-section bytes | 202577 → 200022 |
| Optimizer defined functions | 5369 → 4672 |
| Functions | 4709 |
| Data sections / bytes | 180 / 198528 |
| Exported methods | 9 |
| Largest shallow item | code[926] (126498 bytes) |
| Largest retained item | table[0] (1188945 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1188945 ┊     41.18% ┊ table[0]
        1188939 ┊     41.18% ┊   ⤷ elem[0]
         329084 ┊     11.40% ┊       ⤷ code[13]
         225274 ┊      7.80% ┊           ⤷ code[3]
         198767 ┊      6.88% ┊ [160 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v6` without duplicating raw data.
