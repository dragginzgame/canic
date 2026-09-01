# Wasm Detail: `user_hub`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3326921 |
| Release gzip bytes | 1188287 |
| Debug Wasm bytes | 7618236 |
| Debug gzip bytes | 1914980 |
| Debug delta | +4291315 (128.99%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3533534 → 3326921 |
| Optimizer gzip bytes | 1170636 → 1188287 |
| Optimizer code-section bytes | 3272186 → 3068953 |
| Optimizer data-section bytes | 220589 → 217932 |
| Optimizer defined functions | 6231 → 5450 |
| Functions | 5489 |
| Data sections / bytes | 195 / 216316 |
| Exported methods | 13 |
| Largest shallow item | code[1091] (126498 bytes) |
| Largest retained item | table[0] (1586486 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1586486 ┊     47.69% ┊ table[0]
        1586480 ┊     47.69% ┊   ⤷ elem[0]
         250072 ┊      7.52% ┊       ⤷ code[4]
          38492 ┊      1.16% ┊           ⤷ code[31]
         216667 ┊      6.51% ┊ [172 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v5` without duplicating raw data.
