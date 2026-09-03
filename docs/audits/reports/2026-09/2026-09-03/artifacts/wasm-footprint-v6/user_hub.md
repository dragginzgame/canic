# Wasm Detail: `user_hub`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3330758 |
| Release gzip bytes | 1188786 |
| Debug Wasm bytes | 7589098 |
| Debug gzip bytes | 1903067 |
| Debug delta | +4258340 (127.85%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3538804 → 3330758 |
| Optimizer gzip bytes | 1171883 → 1188786 |
| Optimizer code-section bytes | 3276982 → 3072247 |
| Optimizer data-section bytes | 220849 → 218254 |
| Optimizer defined functions | 6256 → 5473 |
| Functions | 5512 |
| Data sections / bytes | 194 / 216646 |
| Exported methods | 14 |
| Largest shallow item | code[1097] (126498 bytes) |
| Largest retained item | table[0] (1585863 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1585863 ┊     47.61% ┊ table[0]
        1585857 ┊     47.61% ┊   ⤷ elem[0]
         250072 ┊      7.51% ┊       ⤷ code[4]
          38492 ┊      1.16% ┊           ⤷ code[31]
         217128 ┊      6.52% ┊ [171 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v6` without duplicating raw data.
