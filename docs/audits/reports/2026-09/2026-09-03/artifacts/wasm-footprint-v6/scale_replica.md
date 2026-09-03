# Wasm Detail: `scale_replica`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 2898207 |
| Release gzip bytes | 1044134 |
| Debug Wasm bytes | 6525480 |
| Debug gzip bytes | 1628006 |
| Debug delta | +3627273 (125.16%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3078355 → 2898207 |
| Optimizer gzip bytes | 1025182 → 1044134 |
| Optimizer code-section bytes | 2837328 → 2660428 |
| Optimizer data-section bytes | 202969 → 200385 |
| Optimizer defined functions | 5399 → 4700 |
| Functions | 4737 |
| Data sections / bytes | 180 / 198891 |
| Exported methods | 10 |
| Largest shallow item | code[932] (126498 bytes) |
| Largest retained item | table[0] (1197769 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1197769 ┊     41.33% ┊ table[0]
        1197763 ┊     41.33% ┊   ⤷ elem[0]
         329053 ┊     11.35% ┊       ⤷ code[12]
         225274 ┊      7.77% ┊           ⤷ code[3]
         199711 ┊      6.89% ┊ [161 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v6` without duplicating raw data.
