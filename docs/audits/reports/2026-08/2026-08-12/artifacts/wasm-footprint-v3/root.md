# Wasm Detail: `root`

| Metric | Value |
| --- | ---: |
| Kind | fleet-subnet-root |
| Release Wasm bytes | 7539746 |
| Release gzip bytes | 2430627 |
| Debug Wasm bytes | 15666083 |
| Debug gzip bytes | 3777667 |
| Debug delta | +8126337 (107.78%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 10977 |
| Data sections / bytes | 3 / 446252 |
| Exported methods | 126 |
| Largest shallow item | data[0] (445254 bytes) |
| Largest retained item | table[0] (5217079 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼───────────────────────
        5217079 ┊     69.19% ┊ table[0]
        5217073 ┊     69.19% ┊   ⤷ elem[0]
         206849 ┊      2.74% ┊       ⤷ code[4728]
         206711 ┊      2.74% ┊           ⤷ code[25]
         161587 ┊      2.14% ┊       ⤷ code[4867]
         161448 ┊      2.14% ┊           ⤷ code[10]
         538935 ┊      7.15% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v3` without duplicating raw data.
