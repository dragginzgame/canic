# Wasm Detail: `root`

| Metric | Value |
| --- | ---: |
| Kind | fleet-subnet-root |
| Release Wasm bytes | 7149541 |
| Release gzip bytes | 2539590 |
| Debug Wasm bytes | 16032374 |
| Debug gzip bytes | 3983834 |
| Debug delta | +8882833 (124.24%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 7593756 → 7149541 |
| Optimizer gzip bytes | 2470734 → 2539590 |
| Optimizer code-section bytes | 7149166 → 6709592 |
| Optimizer data-section bytes | 337873 → 334578 |
| Optimizer defined functions | 11026 → 9633 |
| Functions | 9678 |
| Data sections / bytes | 283 / 332247 |
| Exported methods | 11 |
| Largest shallow item | code[7684] (244339 bytes) |
| Largest retained item | table[0] (4660365 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        4660365 ┊     65.18% ┊ table[0]
        4660359 ┊     65.18% ┊   ⤷ elem[0]
         414329 ┊      5.80% ┊       ⤷ code[437]
          26202 ┊      0.37% ┊           ⤷ code[12]
          16500 ┊      0.23% ┊           ⤷ code[125]
          11876 ┊      0.17% ┊           ⤷ code[47]
         304284 ┊      4.26% ┊ [261 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v5` without duplicating raw data.
