# Wasm Detail: `fleet_coordinator`

| Metric | Value |
| --- | ---: |
| Kind | fleet-coordinator |
| Release Wasm bytes | 3439803 |
| Release gzip bytes | 1075598 |
| Debug Wasm bytes | 7247791 |
| Debug gzip bytes | 1698688 |
| Debug delta | +3807988 (110.70%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 5136 |
| Data sections / bytes | 3 / 242940 |
| Exported methods | 28 |
| Largest shallow item | data[0] (242522 bytes) |
| Largest retained item | table[0] (1566403 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        1566403 ┊     45.54% ┊ table[0]
        1566397 ┊     45.54% ┊   ⤷ elem[0]
         210889 ┊      6.13% ┊       ⤷ code[8]
          89928 ┊      2.61% ┊           ⤷ code[191]
          46037 ┊      1.34% ┊           ⤷ code[17]
         273937 ┊      7.96% ┊ [5 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v3` without duplicating raw data.
