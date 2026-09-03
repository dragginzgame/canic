# Wasm Detail: `test`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3202037 |
| Release gzip bytes | 1139866 |
| Debug Wasm bytes | 7271644 |
| Debug gzip bytes | 1818010 |
| Debug delta | +4069607 (127.09%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3403236 → 3202037 |
| Optimizer gzip bytes | 1119669 → 1139866 |
| Optimizer code-section bytes | 3142734 → 2944773 |
| Optimizer data-section bytes | 216629 → 214090 |
| Optimizer defined functions | 5940 → 5189 |
| Functions | 5228 |
| Data sections / bytes | 195 / 212473 |
| Exported methods | 11 |
| Largest shallow item | code[1023] (126498 bytes) |
| Largest retained item | table[0] (1467065 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1467065 ┊     45.82% ┊ table[0]
        1467059 ┊     45.82% ┊   ⤷ elem[0]
         328839 ┊     10.27% ┊       ⤷ code[14]
         225154 ┊      7.03% ┊           ⤷ code[3]
         215933 ┊      6.74% ┊ [174 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v6` without duplicating raw data.
