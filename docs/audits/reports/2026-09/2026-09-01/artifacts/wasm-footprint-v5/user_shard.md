# Wasm Detail: `user_shard`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3254719 |
| Release gzip bytes | 1160530 |
| Debug Wasm bytes | 7412893 |
| Debug gzip bytes | 1860589 |
| Debug delta | +4158174 (127.76%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 3458268 → 3254719 |
| Optimizer gzip bytes | 1139895 → 1160530 |
| Optimizer code-section bytes | 3197929 → 2997689 |
| Optimizer data-section bytes | 216889 → 214296 |
| Optimizer defined functions | 6080 → 5320 |
| Functions | 5363 |
| Data sections / bytes | 195 / 212680 |
| Exported methods | 12 |
| Largest shallow item | code[1048] (126498 bytes) |
| Largest retained item | table[0] (1453073 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1453073 ┊     44.65% ┊ table[0]
        1453067 ┊     44.64% ┊   ⤷ elem[0]
         254041 ┊      7.81% ┊       ⤷ code[13]
         150827 ┊      4.63% ┊           ⤷ code[4]
         215197 ┊      6.61% ┊ [172 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v5` without duplicating raw data.
