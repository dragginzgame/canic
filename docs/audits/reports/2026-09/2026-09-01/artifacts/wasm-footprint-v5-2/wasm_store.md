# Wasm Detail: `wasm_store`

| Metric | Value |
| --- | ---: |
| Kind | wasm-store |
| Release Wasm bytes | 2452234 |
| Release gzip bytes | 889967 |
| Debug Wasm bytes | 5563466 |
| Debug gzip bytes | 1394929 |
| Debug delta | +3111232 (126.87%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 2607459 → 2452234 |
| Optimizer gzip bytes | 870403 → 889967 |
| Optimizer code-section bytes | 2407282 → 2254883 |
| Optimizer data-section bytes | 179101 → 176894 |
| Optimizer defined functions | 4919 → 4239 |
| Functions | 4278 |
| Data sections / bytes | 164 / 175531 |
| Exported methods | 10 |
| Largest shallow item | code[817] (126542 bytes) |
| Largest retained item | table[0] (1001371 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1001371 ┊     40.84% ┊ table[0]
        1001365 ┊     40.83% ┊   ⤷ elem[0]
         217666 ┊      8.88% ┊       ⤷ code[60]
          52296 ┊      2.13% ┊           ⤷ code[838]
         143755 ┊      5.86% ┊ [146 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v5` without duplicating raw data.
