# Wasm Detail: `fleet_coordinator`

| Metric | Value |
| --- | ---: |
| Kind | fleet-coordinator |
| Release Wasm bytes | 3496090 |
| Release gzip bytes | 1214593 |
| Debug Wasm bytes | 7849875 |
| Debug gzip bytes | 1868539 |
| Debug delta | +4353785 (124.53%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3720063 → 3496090 |
| Optimizer gzip bytes | 1180746 → 1214593 |
| Optimizer code-section bytes | 3466506 → 3245678 |
| Optimizer data-section bytes | 201209 → 198867 |
| Optimizer defined functions | 5323 → 4463 |
| Functions | 4497 |
| Data sections / bytes | 233 / 196954 |
| Exported methods | 6 |
| Largest shallow item | code[147] (255287 bytes) |
| Largest retained item | table[0] (1240111 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1240111 ┊     35.47% ┊ table[0]
        1240105 ┊     35.47% ┊   ⤷ elem[0]
         197998 ┊      5.66% ┊       ⤷ code[3303]
         111374 ┊      3.19% ┊           ⤷ code[478]
           7450 ┊      0.21% ┊           ⤷ code[122]
           2945 ┊      0.08% ┊           ⤷ code[302]
         212811 ┊      6.09% ┊ [219 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v5` without duplicating raw data.
