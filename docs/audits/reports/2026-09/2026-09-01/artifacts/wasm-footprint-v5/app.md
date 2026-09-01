# Wasm Detail: `app`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 2885999 |
| Release gzip bytes | 1038424 |
| Debug Wasm bytes | 6494693 |
| Debug gzip bytes | 1619528 |
| Debug delta | +3608694 (125.04%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3064368 → 2885999 |
| Optimizer gzip bytes | 1019692 → 1038424 |
| Optimizer code-section bytes | 2824674 → 2649529 |
| Optimizer data-section bytes | 202317 → 199763 |
| Optimizer defined functions | 5353 → 4657 |
| Functions | 4694 |
| Data sections / bytes | 179 / 198277 |
| Exported methods | 9 |
| Largest shallow item | code[923] (126498 bytes) |
| Largest retained item | table[0] (1190384 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1190384 ┊     41.25% ┊ table[0]
        1190378 ┊     41.25% ┊   ⤷ elem[0]
         328868 ┊     11.40% ┊       ⤷ code[13]
         225274 ┊      7.81% ┊           ⤷ code[3]
         198507 ┊      6.88% ┊ [160 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v5` without duplicating raw data.
