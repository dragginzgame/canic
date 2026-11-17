# Retained wire fixtures

Captured from published Platform 0.33.7 (source
`663cf913997490f448553119a6caf66b3cdecc81`, `crates/ic-platform`) before removing
that dependency. The operation and wallet comparisons passed against Protocol
0.3.5 before capture. Files are lowercase hex-encoded Candid messages, read only
by native tests; normal tests never regenerate them.

Management and creation request fixtures were independently decoded and
re-encoded with the old management/ledger types, with byte equality checked.
`upload-chunk` is Protocol's encoding independently decoded by the old codec:
its target and payload matched, while its type-table ordering intentionally
differs. The response and wallet fixtures were encoded with the old types.
The corresponding tests retain explicit inputs, expected responses and malformed
reply cases. These bytes preserve compatibility evidence without retaining an
obsolete Agent/codec dependency or implementing a consumer codec.

Creation responses use block 1 and canister `rrkah-fqaaa-aaaaa-aaaaq-cai`;
the negative cases are duplicate without principal, TooOld and zero funds.
Wallet fixtures cover 510925125, 405555115186, zero, u128::MAX,
u128::MAX + 1 and the wrong Candid text type.

The public mainnet key at `../../src/network/mainnet-root.der` is the 133-byte
DER trust anchor shipped in Topology 0.109.2 (`src/mainnet/root.der`), also used
by the prior Agent. It is public trust material, never an operator identity.
