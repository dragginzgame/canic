# Wasm Detail: `scale_hub`

| Metric | Value |
| --- | ---: |
| Kind | leaf-canister |
| Release Wasm bytes | 2290404 |
| Release gzip bytes | 759760 |
| Debug Wasm bytes | 4902642 |
| Debug gzip bytes | 1237989 |
| Debug delta | +2612238 (114.05%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 4486 |
| Data sections / bytes | 3 / 191796 |
| Exported methods | 24 |
| Largest shallow item | data[0] (191218 bytes) |
| Largest retained item | table[0] (1408624 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼──────────────────────
        1408624 ┊     61.50% ┊ table[0]
        1408618 ┊     61.50% ┊   ⤷ elem[0]
         383984 ┊     16.76% ┊       ⤷ code[23]
         274464 ┊     11.98% ┊           ⤷ code[4]
         201693 ┊      8.81% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v2` without duplicating raw data.
