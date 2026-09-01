# Wasm Detail: `scale_hub`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3250522 |
| Release gzip bytes | 1157418 |
| Debug Wasm bytes | 7401719 |
| Debug gzip bytes | 1856119 |
| Debug delta | +4151197 (127.71%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3454082 → 3250522 |
| Optimizer gzip bytes | 1140913 → 1157418 |
| Optimizer code-section bytes | 3196131 → 2995832 |
| Optimizer data-section bytes | 218145 → 215593 |
| Optimizer defined functions | 6046 → 5280 |
| Functions | 5319 |
| Data sections / bytes | 194 / 213985 |
| Exported methods | 11 |
| Largest shallow item | code[1047] (126498 bytes) |
| Largest retained item | table[0] (1515773 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1515773 ┊     46.63% ┊ table[0]
        1515767 ┊     46.63% ┊   ⤷ elem[0]
         250105 ┊      7.69% ┊       ⤷ code[4]
          38492 ┊      1.18% ┊           ⤷ code[30]
         213897 ┊      6.58% ┊ [173 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v5` without duplicating raw data.
