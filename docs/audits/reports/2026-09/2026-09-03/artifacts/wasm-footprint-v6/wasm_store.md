# Wasm Detail: `wasm_store`

| Metric | Value |
| --- | ---: |
| Kind | wasm-store |
| Release Wasm bytes | 2452131 |
| Release gzip bytes | 890086 |
| Debug Wasm bytes | 5566994 |
| Debug gzip bytes | 1395876 |
| Debug delta | +3114863 (127.03%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 2608460 → 2452131 |
| Optimizer gzip bytes | 870170 → 890086 |
| Optimizer code-section bytes | 2408151 → 2254620 |
| Optimizer data-section bytes | 179229 → 177039 |
| Optimizer defined functions | 4922 → 4244 |
| Functions | 4283 |
| Data sections / bytes | 163 / 175684 |
| Exported methods | 10 |
| Largest shallow item | code[820] (126542 bytes) |
| Largest retained item | table[0] (999590 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
         999590 ┊     40.76% ┊ table[0]
         999584 ┊     40.76% ┊   ⤷ elem[0]
         216779 ┊      8.84% ┊       ⤷ code[60]
          52296 ┊      2.13% ┊           ⤷ code[841]
         143960 ┊      5.87% ┊ [145 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v6` without duplicating raw data.
