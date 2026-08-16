# Saltz Waveform Application

Saltz is an experimental standalone canister for rendering the selected
mountain trace through public Cycle Burn Rate telemetry. Its first compiled
executor targeted one exact ICP Subnet; that scope was rejected during live
pre-roll because the intended canvas is the global Dashboard homepage. The
burner is destructive only after a controller explicitly arms its immutable
compiled plan; install and ordinary status calls cannot burn the waveform.

The implementation boundary contains:

- one standalone burner package with no Fleet or Canic runtime dependency;
- one pinned numeric waveform CSV consumed at build time; and
- one separately deployable read-only preview.

It also contains an independent `saltz_preview` canister. The preview is not a
Fleet role and has no Canic runtime dependency. It exposes one read-only
`http_request` query that serves:

- `/` — the exact 860-point waveform as a code-native T2-style SVG graph;
- `/waveform.csv` — the digest-verified reference CSV; and
- `/api/status.json` — bounded machine-readable runtime, qualification,
  observation and provenance facts.

The preview has no update call, timer, stable state, scheduler, authorization
surface or `cycles_burn` path. The repository and preview contain no source
photograph, raster derivative, graphical overlay or image route. Ordinary
canister execution consumes normal platform cycles; the displayed zero refers
specifically to intentional waveform burn.

The page makes no network-status claim merely because it responded. It labels
itself as an inert served preview, distinguishes the static proposed Dashboard
total from the exact dated global observation band, and states both that burn
capability is absent and that ordinary query execution still consumes cycles.
The footer's `Canic` label links to the authoritative repository.

The public HTTP surface is intentionally anonymous. Its HTML, status JSON and
CSV contain no `Saltz`, `neon` or publisher-name token, and the source-article
link is absent. The long German document title is the only textual clue to the
artistic source. Internal package and design names retain their truthful
implementation identity.

The checked-in CSV preserves the numeric geometry and exact rational 24-hour
time axis. The burner resamples that geometry into one immutable integer plan:
35 pre-roll pulses followed by 864 drawing pulses, one every 100 seconds. The
build fails if its digest, duration, per-pulse rate or total ceiling drifts.
The removed image-authoring pipeline does not exist in the workspace, so the
CSV does not independently establish source provenance.

The current local candidate is deliberately global-homepage-scoped. Seven
pre-run days put the global mean at `37.983 Bcycles/second`, its maximum at
`53.915 Bcycles/second`, and its 95th-percentile absolute adjacent ten-minute
change at `4.255 Bcycles/second`. The controller conservatively credits only
`30 Bcycles/second` of unrelated background and targets a visible
`100 Bcycles/second` floor plus `50 Bcycles/second` relief.

The terminal 26-pulse mainnet rise fits a `4,200.842`-second gain denominator
with `R² = 0.999475`. The observed trailing edge gives a distinct
`3,600`-second visible support and a conservative `100`-second control-grid
phase lead. The
complete post-Abort tail remains the final response gate; local evidence is
not mainnet installation or execution authority.

The exact local candidate envelope is:

- authorization digest `dc1cc6ba53470e0f4abf8045224c8a9bb92516b86e458e9238d4428def3e13d9`;
- pre-roll burn `409,320,934,169,000` cycles;
- drawing burn `9,072,189,520,950,000` cycles;
- total intentional burn `9,481,510,455,119,000` cycles;
- maximum per-step rate `500,000,000,000 cycles/second`;
- peak compiled rate `297,654,853,334 cycles/second`;
- immutable total ceiling `10,000,000,000,000,000 cycles`;
- retained reserve `1,000,000,000,000` cycles; and
- execution allowance `100,000,000,000` cycles.

The candidate's first 35 pulses are its complete pre-roll and burn exactly
`409,320,934,169,000 cycles`. Arming would therefore require at least
`410,420,934,169,000 cycles`: that pre-roll, retained reserve and execution
allowance. The existing terminal canister has more than that balance, but this
fact is not reinstall or Arm authority. Funding is never autonomous. If an
external top-up does not arrive before a later pulse needs it, the first
balance shortfall stops permanently without retry or catch-up. Additional
balance cannot increase the immutable schedule or total ceiling.

`Arm` authorizes only the 35-pulse pre-roll. Surplus balance cannot cross the
pre-roll-to-waveform boundary: the first drawing pulse fails terminally unless
the controller separately submits `AuthorizeWaveform` with the exact plan
digest. That command also rejects unless the current balance covers every
remaining embedded burn plus the retained reserve; the externally funded
execution allowance absorbs transient message reservation and run costs.

The application-owned Candid surface is exactly two methods:

- `burner_command(variant { Abort; Arm; AuthorizeWaveform })` — controller-only
  update;
- `burner_status(variant { Summary; Receipts })` — controller-only query.

There is no generic amount argument, start/stop toggle, forecast endpoint,
funding endpoint, Fleet endpoint or application timer facade. Continuation is
a variant, not another endpoint; it cannot change any amount or deadline.
`Abort` is the only stop command and never resumes.

The 10× staged-trial release Wasm with module hash
`2388f3f4e38274999682da7a3525d6fbc41724c073c61d16b7c9b253ebecbfc9`
was installed on IC-mainnet canister `w47na-gaaaa-aaaad-qmclq-cai` on
2026-08-16. Its authorization digest is
`e5977055cf691d29353c6649bd464a821475efd66432ff56ea93d76de419ff8d`.
Two earlier 1× attempts were aborted: the first exposed a staged-balance guard
defect after two exact pulses, and the corrected second produced five exact
pulses but remained too close to the public noise floor. The 10× trial starts
at `2026-08-17T00:15:00+02:00`. After 16 exact receipts, the owning Subnet's
successive exact 600-second observations rose from `0.389` to `0.508`, `1.496`,
`3.378` and `5.157 Bcycles/second`. That proportional public response passed
the continuation gate. An exact `904,073,274,118,831-cycle` top-up then funded
the immutable remainder; it did not change the schedule or maximum burn.
Before the 864-step drawing began, direct operator observation established
that the intended canvas was the global homepage graph rather than the
qualified Subnet graph. The controller aborted terminally at 26 exact receipts
and `33,320,040,000,000 cycles`; no waveform step executed. The remaining
approximately `925.798 Tcycles` stays inert in the canister for a separately
designed reinstall.

Build and test the standalone burner without creating a mainnet canister:

```sh
cargo test -p saltz_simulator
POCKET_IC_BIN=/path/to/pocket-ic cargo test -p saltz_burner --test pic_waveform
icp build saltz_burner -e waveform-burner-local
```

Deploying through `waveform-burner-local` or `waveform-burner-ic` installs an
inert `Prepared` canister. It does not fund or arm the schedule. Inspect the
exact envelope with:

```sh
icp canister call saltz_burner burner_status \
  '(variant { Summary })' \
  -e waveform-burner-local \
  --candid apps/saltz/burner/saltz_burner.did
```

Build and test the standalone preview without creating a canister:

```sh
cargo test -p saltz_preview
icp build saltz_preview -e saltz-preview-local
```

For a local browser proof, start the configured local network and deploy only
the preview environment:

```sh
icp network start -e saltz-preview-local -d
icp deploy saltz_preview -e saltz-preview-local
```

The preview intentionally serves an uncertified query response and therefore
uses a raw gateway URL. Locally, open the deployed principal as:

```text
http://<canister-id>.raw.localhost:8002/
```

The repository also contains a separate `cycle_burn_probe` qualification
canister. It is not the Burner Component and cannot schedule or replay a
waveform. Its current Wasm hard-binds at most eighteen `200 Bcycle` steps,
retains at least `1 Tcycle` plus a `100 Bcycle` execution allowance and has no
timer. The host must submit every exact index; stopping host calls stops new
burns. One controller-only `probe_command` update composes `Start`, `Step` and
terminal `Abort`, while `probe_status` makes all committed receipts
recoverable. Deploy it locally with explicit synthetic funding:

```sh
icp identity new cycle-burn-local
icp deploy cycle_burn_probe -e cycle-burn-local --cycles 6t \
  --identity cycle-burn-local
icp canister call cycle_burn_probe probe_status '()' \
  -e cycle-burn-local \
  --identity cycle-burn-local \
  --candid canisters/test/cycle_burn_probe/cycle_burn_probe.did
```

Keep the CLI default identity separate and pass `--identity cycle-burn-local`
explicitly. On a fresh deployment the creating identity becomes the probe
controller. Existing deployments require a deliberate controller transfer
before use.

The returned receipt proves the local burn primitive and accounting path. It
does not populate or reproduce the public ICP Dashboard, whose cycle-burn-rate
series observes mainnet Subnets only.

Model a bounded proposal without any network effect:

```sh
cargo run --locked -p saltz_simulator -- \
  --max-total-burn-cycles 130000000000000
```

The simulator binds the checked-in waveform digest and reports the current
144-point `1D` fit. Its floating-point report remains analysis only. A separate
integer compiler emits the exact 899 amounts embedded by the burner build;
that digested array, not the floating-point report, is execution authority.

The separately authorized B0b mainnet calibration is now complete. Its former
one-shot installation burned one exact `4 Tcycle` pulse that was clearly
visible in the owning Subnet's public series, but
the attributable peak was only approximately `0.883 Bcycles/second` and
remained spread across the observed tail. A naive independent 100-second
bucket would have been `40 Bcycles/second`. The API `step` parameter therefore
does not provide independent approximately 100-second drawing buckets. The
embedded controller instead uses their overlap under the dated inferred
`4,531`-second rectangular response. That inference is a bounded proposal, not
a completed platform contract. See the
[bounded calibration report](../../docs/audits/working/saltz-b0b-calibration/mainnet-calibration.md).

That B0c plateau is also complete: eighteen exact host-driven `200 Bcycle`
steps raised the owning Subnet series from approximately `0.312` to
`1.303 Bcycles/second` while retaining more than `1 Tcycle`. The result proves
that bounded repeated input produces a clean accumulated signal. It does not
qualify the complete decay kernel. The maintainer later promoted implementation
and inert deployment of the standalone burner; a mint, top-up, `Arm` and
waveform continuation remain separate exact external effects.

The preview, calibration probe and burner remain excluded from the broad `ic`
environment. Their deliberately named environments do not themselves
authorize an effect. In particular, `waveform-burner-ic` authorizes only an
explicit install command unless a separate exact funding and arm envelope is
recorded.
