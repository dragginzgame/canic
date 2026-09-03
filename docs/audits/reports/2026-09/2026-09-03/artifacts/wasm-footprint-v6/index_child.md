# Wasm Detail: `index_child`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 2443547 |
| Release gzip bytes | 885997 |
| Debug Wasm bytes | 5591079 |
| Debug gzip bytes | 1396431 |
| Debug delta | +3147532 (128.81%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 2599776 → 2443547 |
| Optimizer gzip bytes | 868576 → 885997 |
| Optimizer code-section bytes | 2378085 → 2224620 |
| Optimizer data-section bytes | 186873 → 184710 |
| Optimizer defined functions | 4653 → 3996 |
| Functions | 4035 |
| Data sections / bytes | 154 / 183429 |
| Exported methods | 9 |
| Largest shallow item | code[778] (126497 bytes) |
| Largest retained item | table[0] (841415 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼─────────────────────────
         841415 ┊     34.43% ┊ table[0]
         841409 ┊     34.43% ┊   ⤷ elem[0]
         185677 ┊      7.60% ┊       ⤷ code[3151]
          22796 ┊      0.93% ┊           ⤷ code[376]
           7527 ┊      0.31% ┊           ⤷ code[1076]
           4883 ┊      0.20% ┊           ⤷ code[233]
           4111 ┊      0.17% ┊           ⤷ code[196]
         184540 ┊      7.55% ┊ [138 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v6` without duplicating raw data.
