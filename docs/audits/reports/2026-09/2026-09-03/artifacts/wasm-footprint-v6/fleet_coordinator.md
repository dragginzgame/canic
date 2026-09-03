# Wasm Detail: `fleet_coordinator`

| Metric | Value |
| --- | ---: |
| Kind | fleet-coordinator |
| Release Wasm bytes | 3497977 |
| Release gzip bytes | 1215581 |
| Debug Wasm bytes | 7864111 |
| Debug gzip bytes | 1871070 |
| Debug delta | +4366134 (124.82%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3721861 → 3497977 |
| Optimizer gzip bytes | 1182054 → 1215581 |
| Optimizer code-section bytes | 3468047 → 3247309 |
| Optimizer data-section bytes | 201465 → 199123 |
| Optimizer defined functions | 5324 → 4463 |
| Functions | 4497 |
| Data sections / bytes | 233 / 197210 |
| Exported methods | 6 |
| Largest shallow item | code[147] (255910 bytes) |
| Largest retained item | table[0] (1240111 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1240111 ┊     35.45% ┊ table[0]
        1240105 ┊     35.45% ┊   ⤷ elem[0]
         197998 ┊      5.66% ┊       ⤷ code[3304]
         111374 ┊      3.18% ┊           ⤷ code[478]
           7450 ┊      0.21% ┊           ⤷ code[122]
           2945 ┊      0.08% ┊           ⤷ code[302]
         213139 ┊      6.09% ┊ [219 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v6` without duplicating raw data.
