# Wasm Detail: `test`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3200574 |
| Release gzip bytes | 1139639 |
| Debug Wasm bytes | 7268887 |
| Debug gzip bytes | 1817133 |
| Debug delta | +4068313 (127.11%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3400312 → 3200574 |
| Optimizer gzip bytes | 1119420 → 1139639 |
| Optimizer code-section bytes | 3140087 → 2943588 |
| Optimizer data-section bytes | 216369 → 213837 |
| Optimizer defined functions | 5924 → 5174 |
| Functions | 5213 |
| Data sections / bytes | 194 / 212228 |
| Exported methods | 11 |
| Largest shallow item | code[1020] (126498 bytes) |
| Largest retained item | table[0] (1468514 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1468514 ┊     45.88% ┊ table[0]
        1468508 ┊     45.88% ┊   ⤷ elem[0]
         328623 ┊     10.27% ┊       ⤷ code[14]
         225154 ┊      7.03% ┊           ⤷ code[4]
         215680 ┊      6.74% ┊ [173 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v5` without duplicating raw data.
