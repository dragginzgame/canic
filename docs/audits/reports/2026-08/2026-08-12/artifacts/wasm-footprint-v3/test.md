# Wasm Detail: `test`

| Metric | Value |
| --- | ---: |
| Kind | component |
| Release Wasm bytes | 3037685 |
| Release gzip bytes | 991291 |
| Debug Wasm bytes | 6497057 |
| Debug gzip bytes | 1602718 |
| Debug delta | +3459372 (113.88%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 5502 |
| Data sections / bytes | 3 / 238124 |
| Exported methods | 29 |
| Largest shallow item | data[0] (237702 bytes) |
| Largest retained item | table[0] (1400367 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼───────────────────────
        1400367 ┊     46.10% ┊ table[0]
        1400361 ┊     46.10% ┊   ⤷ elem[0]
         190261 ┊      6.26% ┊       ⤷ code[127]
          44708 ┊      1.47% ┊           ⤷ code[53]
         266539 ┊      8.77% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v3` without duplicating raw data.
