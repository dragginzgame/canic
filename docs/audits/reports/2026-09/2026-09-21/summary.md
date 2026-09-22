# Investigation summary — 2026-09-21

- [CANIC-148 sampler cost](sampler-cost-investigation.md): exploratory real Wasm
  phase probes identify scattered history-ring writes and IC page metering as
  the principal measured optimization target. Three targeted runs passed 10
  cases each; temporary probes were removed. Structured results and independent
  reproduction patches are linked from the report.
- Result: partial investigation evidence, valid for attribution; CANIC-148
  remains open. No production savings, exact Toko regression cause, frozen-source
  audit or release-closeout verdict is claimed. Next candidate: improve history
  write locality, then qualify unchanged semantics and the real producer.
- At that investigation checkpoint, B1 remained the accepted implementation
  queue. No downstream edits or Git publication occurred.

- [Deployment timing and preparation](deployment-timing.md), with
  [structured IPC/cache measurements](deployment-timing.json): retained command
  evidence, one-action install-status reuse and controlled cache invalidation.
  Native checks and both matched PocketIC runs pass; all 24 payload hashes
  match. The narrow read reduction is qualified; broad mainnet speedup and full
  downstream feedback closure are not claimed.

- [Post-.35 history locality qualification](history-locality.md): grouped history
  and direct chronological reads pass native, targeted PocketIC and scoped lint
  checks. Matched control/width evidence retains improvements and sparse/gap
  tradeoffs. The separately authorised Toko advisory correction passes with the
  same 20,062,824 measurement. Real producer savings require downstream adoption;
  no B1 acceptance, broad validation or Git publication is inferred.
