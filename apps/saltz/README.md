# Saltz Waveform Application

Saltz is an experimental standalone canister for rendering the selected
mountain trace in one exact ICP Subnet's public Cycle Burn Rate series. The
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
45 pre-roll pulses followed by 864 drawing pulses, one every 100 seconds. The
build fails if its digest, duration, per-pulse rate or total ceiling drifts.
The removed image-authoring pipeline does not exist in the workspace, so the
CSV does not independently establish source provenance.

The executable mapping is deliberately Subnet-scoped: a visible target of
`1..=2.5 Bcycles/second` against a fixed dated `0.625 Bcycles/second`
background model. It is not intended to dominate the volatile global series.
The inferred rectangular `4,531`-second response model remains dated and
provisional; a successful timer execution proves the burn plan, not artistic
fidelity on the public Dashboard.

The exact embedded envelope is:

- authorization digest `491cd73eb597ca4586fd33516d0390160df0b51111fb388d96843b21552a86c9`;
- pre-roll burn `5,766,930,000,000` cycles;
- drawing burn `90,038,364,472,300` cycles;
- total intentional burn `95,805,294,472,300` cycles;
- retained reserve `1,000,000,000,000` cycles; and
- execution allowance `100,000,000,000` cycles.

The authorized staged trial binds its first 42 pulses, covering 70 minutes and
`5,382,468,000,000` intentional cycles. Arming therefore requires at least
`6,482,468,000,000` cycles: that initial burn allocation, the retained reserve
and the execution allowance. Funding is never autonomous. If an external
top-up does not arrive before a later pulse needs it, the first balance
shortfall stops permanently without retry or catch-up. Additional balance
cannot increase the immutable schedule or total ceiling.

The application-owned Candid surface is exactly two methods:

- `burner_command(variant { Arm; Abort })` — controller-only update;
- `burner_status(variant { Summary; Receipts })` — controller-only query.

There is no generic amount argument, start/stop toggle, forecast endpoint,
funding endpoint, Fleet endpoint or application timer facade. `Abort` is the
only stop command and never resumes.

The staged-trial release Wasm with module hash
`728edf4a7d652cc1ffa79e7dda5e96e4a91e42c67eaabb9cc7e2e240f325294b`
was installed on IC-mainnet canister `w47na-gaaaa-aaaad-qmclq-cai` on
2026-08-16. Its authorization digest is
`491cd73eb597ca4586fd33516d0390160df0b51111fb388d96843b21552a86c9`.
The controller then funded only the 70-minute trial envelope and armed a first
deadline at `2026-08-16T23:45:00+02:00`, with the one-hour decision point at
`00:45` and the next-funding deadline before `00:55`.

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
integer compiler emits the exact 909 amounts embedded by the burner build;
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
and inert deployment of the standalone burner; a mint, top-up and `Arm` remain
separate exact external effects.

The preview, calibration probe and burner remain excluded from the broad `ic`
environment. Their deliberately named environments do not themselves
authorize an effect. In particular, `waveform-burner-ic` authorizes only an
explicit install command unless a separate exact funding and arm envelope is
recorded.
