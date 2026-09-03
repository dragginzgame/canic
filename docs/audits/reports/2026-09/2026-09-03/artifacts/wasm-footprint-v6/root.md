# Wasm Detail: `root`

| Metric | Value |
| --- | ---: |
| Kind | fleet-subnet-root |
| Release Wasm bytes | 7097112 |
| Release gzip bytes | 2523593 |
| Debug Wasm bytes | 15943177 |
| Debug gzip bytes | 3952520 |
| Debug delta | +8846065 (124.64%) |
| Compatible predecessor delta | N/A (N/A) |
| Optimizer raw bytes | 7542918 → 7097112 |
| Optimizer gzip bytes | 2454567 → 2523593 |
| Optimizer code-section bytes | 7100961 → 6659744 |
| Optimizer data-section bytes | 336997 → 333748 |
| Optimizer defined functions | 10983 → 9596 |
| Functions | 9641 |
| Data sections / bytes | 280 / 331442 |
| Exported methods | 11 |
| Largest shallow item | code[7662] (241913 bytes) |
| Largest retained item | table[0] (4658570 bytes) |

## Bounded Dominator Evidence

```text
 Retained Bytes │ Retained % │ Dominator Tree
────────────────┼────────────┼────────────────────────
        4658570 ┊     65.64% ┊ table[0]
        4658564 ┊     65.64% ┊   ⤷ elem[0]
         405183 ┊      5.71% ┊       ⤷ code[427]
          17225 ┊      0.24% ┊           ⤷ code[121]
          16436 ┊      0.23% ┊           ⤷ code[154]
          11876 ┊      0.17% ┊           ⤷ code[42]
         302926 ┊      4.27% ┊ [258 Unreachable Items]
```

## Bounded Monomorphization Evidence

```text
 Apprx. Bloat Bytes │ Apprx. Bloat % │ Bytes │ %     │ Monomorphizations
────────────────────┼────────────────┼───────┼───────┼──────────────────
                  0 ┊          0.00% ┊     0 ┊ 0.00% ┊ Σ [0 Total Rows]
```

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by `CANIC-WASM-001/v6` without duplicating raw data.
