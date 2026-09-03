# Wasm Detail: `index_hub`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 2672270 |
| Release gzip bytes | 960608 |
| Debug Wasm bytes | 6111310 |
| Debug gzip bytes | 1519597 |
| Debug delta | +3439040 (128.69%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 2843118 → 2672270 |
| Optimizer gzip bytes | 943256 → 960608 |
| Optimizer code-section bytes | 2612651 → 2444607 |
| Optimizer data-section bytes | 193761 → 191602 |
| Optimizer defined functions | 5056 → 4346 |
| Functions | 4385 |
| Data sections / bytes | 164 / 190241 |
| Exported methods | 9 |
| Largest shallow item | code[857] (126497 bytes) |
| Largest retained item | table[0] (1041436 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼─────────────────────────
        1041436 ┊     38.97% ┊ table[0]
        1041430 ┊     38.97% ┊   ⤷ elem[0]
         189905 ┊      7.11% ┊       ⤷ code[3447]
          22796 ┊      0.85% ┊           ⤷ code[415]
           7527 ┊      0.28% ┊           ⤷ code[1177]
           4883 ┊      0.18% ┊           ⤷ code[253]
           4111 ┊      0.15% ┊           ⤷ code[216]
         191887 ┊      7.18% ┊ [148 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v6` without duplicating raw data.
