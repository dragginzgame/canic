# Wasm Detail: `root`

| Metric | Value |
| --- | ---: |
| Kind | bundle-canister |
| Release Wasm bytes | 3845263 |
| Release gzip bytes | 1742162 |
| Debug Wasm bytes | 7858899 |
| Debug gzip bytes | 2772940 |
| Debug delta | +4013636 (104.38%) |
| Compatible predecessor delta | N/A (N/A) |
| Functions | 6030 |
| Data sections / bytes | 3 / 890540 |
| Exported methods | 44 |
| Largest shallow item | data[0] (889806 bytes) |
| Largest retained item | table[0] (2129488 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼───────────────────────
        2129488 ┊     55.38% ┊ table[0]
        2129482 ┊     55.38% ┊   ⤷ elem[0]
         117869 ┊      3.07% ┊       ⤷ code[152]
         100833 ┊      2.62% ┊           ⤷ code[10]
         915711 ┊     23.81% ┊ [3 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v2` without duplicating raw data.
