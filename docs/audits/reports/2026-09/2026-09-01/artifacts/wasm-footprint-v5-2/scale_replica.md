# Wasm Detail: `scale_replica`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 2896719 |
| Release gzip bytes | 1043641 |
| Debug Wasm bytes | 6514713 |
| Debug gzip bytes | 1626481 |
| Debug delta | +3617994 (124.90%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3075406 → 2896719 |
| Optimizer gzip bytes | 1024747 → 1043641 |
| Optimizer code-section bytes | 2834656 → 2659233 |
| Optimizer data-section bytes | 202709 → 200117 |
| Optimizer defined functions | 5383 → 4685 |
| Functions | 4722 |
| Data sections / bytes | 180 / 198623 |
| Exported methods | 10 |
| Largest shallow item | code[929] (126498 bytes) |
| Largest retained item | table[0] (1199208 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1199208 ┊     41.40% ┊ table[0]
        1199202 ┊     41.40% ┊   ⤷ elem[0]
         328837 ┊     11.35% ┊       ⤷ code[12]
         225274 ┊      7.78% ┊           ⤷ code[3]
         199443 ┊      6.89% ┊ [161 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v5` without duplicating raw data.
