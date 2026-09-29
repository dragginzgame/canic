# Toko size follow-through

The maintainer's 2026-09-28 instruction explicitly selects role persistence,
optional observability and initial Canic blob preparation. This reopens those
narrow B3/B4 cuts after the September 25 deferral. It does not open the next
minor or authorize a release, broad validation or downstream edits.

The accepted batch comprises:

1. Separate ordinary, Root and Wasm Store activation codecs. Keep one durable
   memory owner, fail closed on role/schema mismatch, and reconstruct transient
   projections after restart. Preserve exact release identity, prepared evidence,
   interrupted transitions and effect-free replay. Do not add predecessor readers.
2. Select optional observation providers through the existing role contract and
   generated endpoint cfgs. Preserve default behavior, operational health,
   readiness, binding, cycle balance and recovery/accounting owners. Bind the
   selected surface into generated Candid and capability/profile identity.
3. Make passive blob billing contracts usable without local storage features.
   Retain the embedded implementation until the standalone library is ready.
   Do not introduce a second blob protocol or modify either sibling repository.

Qualification includes focused codec/transition and role-contract regressions,
blob Candid tests without storage features, generated endpoint compilation,
same-release lifecycle/recovery evidence and optimized size observations.
The complete extended batch is ready for review after those targeted checks,
repeated optimized measurements and both changelog updates. The
[audit follow-through](../../audits/reports/2026-09/2026-09-28/toko-wasm-size-audit.md#selected-implementation-follow-through)
records evidence and measurement limits. Package versions remain unchanged;
the maintainer-selected release gate remains separate.
