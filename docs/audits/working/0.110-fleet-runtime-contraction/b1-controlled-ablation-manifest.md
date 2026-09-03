# B1 Controlled-Ablation Manifest

Date: 2026-09-03
State: experiment and artifact manifests plus frozen function counter
executable; rows 2 and 11 measured, row 3 qualified, six source switches
specified and remaining immutable measurements open
Design owner: [0.110 Fleet runtime contraction](../../../design/0.110-fleet-runtime-contraction/0.110-design.md)
Baseline authority: immutable `v0.110.5` at
`50f40171d6177c3d1e490b1fdb5f6163323b2cd5`

## Verdict

B1 now has one machine-checked catalog for all eighteen required experiments
and one artifact roster for the eleven canonical roles plus four Canic-owned
capability fixtures. The runner can execute the unchanged baseline, the fixed
`Page<T>` cohort, the measured row 2 global-storage-registration and row 11
payload-adapter switches, and the qualified row 3 activation-persistence
switch. Rows 4 through 6, row 8, row 10 and row 12 have exact audit-only
patches against the unchanged `v0.110.5` source, but remain non-runnable until
their selected build sets are qualified. The runner refuses every other
experiment until its exact one-switch patch or compatible cross-commit input
exists.

The immutable
[row 2 report](../../reports/2026-09/2026-09-03/wasm-ablation-b1-02.md)
attributes 273,554 optimized code-section bytes and 662 replica-limited
defined functions across the eleven separately deployed canonical artifacts to
global memory declarations, authority ranges and eager TLS registration. Fleet
Coordinator accounts for 192,340 code bytes and 166 functions of that total.
The build-only switch makes no storage-bootstrap or lifecycle-parity claim, so
it advances explicit role-selected registration into B2 evidence but authorizes
no direct production deletion.

The immutable
[row 11 report](../../reports/2026-09/2026-09-03/wasm-ablation-b1-11.md)
attributes 967 optimized code-section bytes and zero replica-limited defined
functions to the raw payload-limited update adapter in its owning fixture. The
adapter remains required for canister-origin payload bounds; this measurement
authorizes no production deletion. The function-limit quantity remains bound
to frozen primary source, and the current working tree is not an immutable
measurement source.

## Canic-Owned Qualification Matrix

The binding artifact manifest is
`scripts/ci/wasm-ablation-artifacts.tsv`:

| Group | Artifacts | Purpose |
| --- | --- | --- |
| canonical | `app`, `index_hub`, `test`, `user_hub`, `scale_hub`, `index_child`, `user_shard`, `scale_replica`, `root`, `fleet_coordinator`, `wasm_store` | exact current role and infrastructure projections |
| fixture | `runtime_probe` | both authentication verifiers, lifecycle, application endpoints, timers and same-release recovery |
| fixture | `payload_limit_probe` | ordinary and explicitly payload-limited update adapters |
| fixture | `blob_storage_probe` | opt-in blob-storage and billing endpoints and state |
| fixture | `leaf_probe` | fixed five-width `Page<T>` generic cohort |

This matrix is deliberately Canic-owned and decomposed by capability. A
consumer's application protocol, generated model, source commit, lockfile or
discard/reseed policy is not part of B1 authority. The retained 10,275,629-byte
downstream observation remains non-binding pressure evidence only.

## Experiment Catalog

The executable source is `scripts/ci/wasm-ablation-experiments.tsv`. Every row
names its immediate baseline, artifact selectors, instruction disposition and
exact source owners.

| Row | Experiment | Primary Canic owner | Current switch state |
| ---: | --- | --- | --- |
| 1 | current baseline | canonical configs and four capability fixtures | ready; no source switch |
| 2 | global stable-storage registration | `memory_macros.rs`, `storage/stable` | measured; material role-wide result, lifecycle parity open |
| 3 | activation records/codecs | activation model, stable storage and mapper | ready; all eleven canonical release artifacts qualified |
| 4 | authorization records/codecs | auth model, stable storage, ops and workflow | specified patch; build qualification open |
| 5 | bounded relevant-CBOR stub | shared CBOR adapter and its reachable callers | specified patch; build qualification open |
| 6 | unconditional recovery dispatch | timer API/workflow and Root pool watchdog | specified patch; build qualification open |
| 7 | current versus exact role/capability expansion | build, start and endpoint macros | planned patch |
| 8 | endpoint Candid type construction | endpoint procedural expansion | specified patch; build qualification open |
| 9 | Candid type documentation | reachable DTOs, IDs and endpoint types | planned patch |
| 10 | Candid serialization/newtype adapters | IDs, DTOs and endpoint adapters | specified patch; build qualification open |
| 11 | payload-limited async adapters | endpoint expansion and ingress payload owner | ready; immutable selected-fixture measurement retained |
| 12 | metrics providers | role endpoint projection and runtime metrics ops | specified patch; build qualification open |
| 13 | configuration/provisioning providers | Root projection and control-plane providers | planned patch |
| 14 | command providers | managed, Root, Coordinator and Store projections | planned patch |
| 15 | timer/watchdog providers | lifecycle macros, timer authority and Root watchdog | planned patch |
| 16 | status projection | managed, Root, Coordinator and Store projections | planned patch |
| 17 | `Page<T>` generic cohort | `leaf_probe`, `Page<T>` and generated role status | ready; `CANIC_GENERIC_COHORT_WIDTH=1..5` |
| 18 | pool-Ledger recovery hard cut | former helper, Store, Root, DTO and host family | planned compatible cross-commit comparison |

`planned` is fail-closed and means no source switch exists. `specified` means
the exact patch exists and applies to its source anchor, but the runner still
refuses it. A row becomes `ready` only when its patch is reviewable, changes
exactly one causal family, compiles every selected artifact and cannot enter a
published Cargo feature or runtime option. None of these states means that the
named family is removable.

Row 7 deliberately remains `planned`. Immutable `v0.110.5` already derives
exact role capabilities and emits compile-time cfg selection, so a current-
versus-exact comparison first needs a separately frozen expanded-source or
projection counterfactual. The working overlay changes selected capability and
inspect-message projections and cannot define the immutable baseline. A patch
that merely removes metrics, command, status, timer or recovery providers would
also duplicate rows 12 through 16 rather than measure role expansion itself.

Row 9 also remains `planned`. The pinned `candid_derive` implementation emits
each derived `_ty_doc()` body directly from Rust `#[doc]` attributes and offers
no Canic-local suppression control. The named Canic DTO, ID and endpoint owner
directories currently contain 1,451 such attributes. Deleting them would be a
broad source-documentation mutation, while stripping comments from the rendered
`.did` would measure metadata bytes after generation rather than the generated
type-documentation bodies. Row 9 needs a frozen derivation-level counterfactual
before it can make the one-cause claim.

## Measurement-Switch Rules

Source ablations are retained as patches under
`scripts/ci/wasm-ablation-patches/` and applied only to a clean disposable
linked worktree. They are not compiled into ordinary Canic builds. The runner:

1. binds one exact source commit and clean linked-worktree path;
2. builds each selected artifact twice through `canic-host`'s release artifact
   authority with offline Cargo and disabled incremental compilation, removing
   and recreating the same fixed absolute target path before each repetition;
3. applies at most one named patch after capturing the unchanged pair;
4. requires deterministic Wasm, gzip, Candid and complete metric vectors;
5. reverses the exact patch and rejects unexpected source mutation; and
6. writes evidence outside the product worktree so generated artifacts cannot
   become product inputs.

The generic cohort is the only environment-matrix row. The build script
accepts exactly widths `1..=5`; all five widths retain the same endpoint,
variant and wire surface. The pool-Ledger row is not a one-source ablation and
therefore remains a separately frozen compatible-predecessor comparison.

Row 2's patch has SHA-256
`9b66e1d344c4f73df993e50fba243dbf6f601fe876ead62b155868649cce7c13`.
It removes the static memory-declaration constructors, authority-range
constructors and eager TLS-touch registrations from `memory_macros.rs`. The
zero-sized marker-type reference moves into the ordinary store-open expansion,
so denied dead-code lints stay truthful without recreating a global root. The
patch applies to the byte-identical current and `v0.110.5` source file and
passes this narrow isolated compile:

```text
CARGO_NET_OFFLINE=true cargo check --offline --locked -p canister_root
```

That Root check exercises both `canic-core` and `canic-control-plane` macro
expansions. The subsequent complete qualification builds all eleven selected
canonical roles once through `canic-host`'s authoritative release builder with
one isolated target, then accepts each Wasm with `wasm-validate`, each gzip with
`gzip -t` and each Candid file with `didc check`. The exact patch is reversed
and the disposable `v0.110.5` worktree is clean afterward. The subsequent
governed two-pair
[measurement](../../reports/2026-09/2026-09-03/wasm-ablation-b1-02.md)
passes determinism and artifact validation for every canonical role. It removes
273,554 code-section bytes and 662 defined functions in artifact-summed
attribution, including 192,340 code bytes and 166 functions from Fleet
Coordinator. Row 2 is therefore `measured`; its material result supports B2's
role-selected storage wiring, but the destructive switch still makes no
bootstrap, restore or lifecycle-parity claim and is not production code.

Row 3's patch has SHA-256
`0b26dde775b5babd7bc7347623e6a09ceeca1c4a5f820870ad904ce557e69a12`.
It disconnects the combined activation stable record, CBOR codec, mapper and
storage-operation implementation while retaining the existing DTO/view-facing
call signatures, error surface and state-manifest projection. The replacement
returns opaque fail-closed results through `core::hint::black_box`; this avoids
making the result discriminant an invitation to constant-fold dependent caller
control flow. The endpoint-mode model remains compiled, while disconnected
install validators are explicitly expected dead code.

This is deliberately an inclusive activation-persistence-family attribution,
not an isolated codec experiment, activation-compatible implementation or
production deletion proposal. The patch applies to the byte-identical current
and `v0.110.5` owner sources and passes this narrow isolated role-shape compile:

```text
CARGO_NET_OFFLINE=true cargo check --offline --locked \
  -p canister_root -p canister_app -p canic-wasm-store
```

That check covers Root, ordinary non-Root and Store consumers. The subsequent
complete qualification builds all eleven canonical roles once through the
authoritative release builder using one isolated target, and accepts every
Wasm, gzip and Candid artifact. The exact patch is reversed and the disposable
worktree is clean afterward. Row 3 is therefore `ready`; it still proves no
activation behavior or persistence parity and has no optimized delta until the
governed two-pair measurement completes.

Row 4's patch has SHA-256
`42ed7228ef4fa9c09ffa6c67acde6a7e8dd766bc0c950b0299c0c5e04c434be9`.
It replaces only the authorization stable cell with an audit-local heap cell
that retains the same `get`/`set` operation shape. The authorization model,
records, mappers, storage-ops API, crypto operations, workflows and callers stay
compiled; removing `impl_storable_unbounded!(AuthStateRecord)` disconnects the
record's stable CBOR path without replacing the surrounding security workflow
with a constant-result stub.

The switch retains same-execution heap behavior but intentionally destroys
stable persistence, so it makes neither persistence nor authorization parity
claims. It is a stable-codec attribution rather than evidence that the
authorization record or workflow can be removed. The patch applies to the
byte-identical current and `v0.110.5` owner source and passes this narrow
isolated auth-feature compile:

```text
CARGO_NET_OFFLINE=true cargo check --offline --locked \
  -p canister_root -p canister_app -p runtime_probe
```

Those packages cover Root signing/issuance, delegated-token verification and
the runtime probe's verifier combination. They do not qualify the complete
canonical plus runtime-probe selector, run an auth journey or provide an
optimized delta, so row 4 remains `specified`.

Row 5's patch has SHA-256
`2e61881a6d9441162345360fca58db1ffe511823e096c368b11d57040214a343`.
It retains the shared helper's generic signatures and serde trait bounds while
replacing CBOR serialization with an opaque one-byte result and deserialization
with an opaque typed failure. The fixed encoder output is bounded for every
current Storable declaration. `core::hint::black_box` prevents callers from
treating either result discriminant or encoded value as a compile-time
constant solely because this is a measurement stub.

The switch affects every reachable user of `canic_core::cdk::serialize`, not
only activation, authorization or blob storage. Direct `ciborium` users remain
unchanged. This is therefore shared-helper attribution with intentional overlap
against rows 3 and 4, not a total-CBOR result or a viable persistence codec. It
makes no codec, stable-state, hash or runtime parity claim. The patch applies
to the byte-identical current and `v0.110.5` helper and passes this narrow
isolated capability compile:

```text
CARGO_NET_OFFLINE=true cargo check --offline --locked \
  -p canister_root -p runtime_probe -p blob_storage_probe
```

Those packages cover the control-plane-heavy Root, combined runtime auth and
opt-in blob-storage billing shapes. They do not qualify every selected artifact
or provide an optimized delta, so row 5 remains `specified`.

Row 6's patch has SHA-256
`59a746714aa62050db91f4c9323a1129ec49746e5bb5010bec1c39ef80f9b417`.
It keeps the shared timer runtime, identities, claims, registrations, inventory,
suspension and ordinary Root pool maintenance intact. Only watchdog takeover
dispatch is replaced: core auth-renewal, placement-acknowledgement and automatic-
top-up dispatchers return an opaque zero count, while Root pool takeover returns
an opaque `false`. Recovery-only helpers made unreachable by those exact edges
carry audit-patch-local `dead_code` expectations so the build does not recreate
reachability merely to satisfy lints.

This is dispatch attribution, not evidence that async-job state, timer custody
or same-release recovery can be removed. It makes no watchdog recovery parity
claim. The patch applies to byte-identical current and `v0.110.5` owner sources
and passes this narrow isolated role-shape compile:

```text
CARGO_NET_OFFLINE=true cargo check --offline --locked \
  -p canister_root -p runtime_probe
```

Those packages cover the Root pool watchdog and non-Root core watchdog. They do
not qualify the complete canonical plus runtime-probe selector, run watchdog
ownership/recovery journeys or provide an optimized delta, so row 6 remains
`specified`.

Row 8's patch has SHA-256
`9d24a7fdeb6b35901b9ea1da6097356871acdc520667980d3fea2a2d0e16ac8a`.
It marks ordinary IC CDK query/update expansions as Candid-hidden and omits the
manual Candid annotation used only by the raw payload-limited update adapter.
The endpoint implementations, Wasm exports, access checks, payload registration
and bounds, raw decode/encode adapter, dispatch and wire serialization remain
compiled. Only their signatures stop feeding the dedicated declaration pass;
the lifecycle constructor remains represented.

This switch intentionally produces a different Candid declaration and therefore
a different protocol-profile digest and embedded Candid metadata. It is
declaration-construction attribution, not a pure code-section result and not a
Candid, metadata or wire-compatibility proposal. Type construction that remains
reachable from runtime serialization is outside this switch and belongs to row
10. The exact patch applies to the byte-identical current and `v0.110.5` macro
expander. Its macro unit suite passes, as does this narrow isolated role-shape
compile:

```text
CARGO_NET_OFFLINE=true cargo check --offline --locked \
  -p canister_root -p runtime_probe -p payload_limit_probe \
  -p blob_storage_probe
```

A governed release build of the payload-limit fixture produced a `didc`-valid
27-byte `service : (opt blob) -> {}` declaration while the final Wasm retained
all four fixture update exports plus the selected Canic role methods. That
single fixture proves the declaration and runtime-export separation, but it
does not qualify the complete canonical plus three-fixture selector or provide
an immutable optimized before/after delta, so row 8 remains `specified`.

Row 10's patch has SHA-256
`0bacf5c226709024bdd18ac51163ecc6c0612bde133a712ff853af0826797a13`.
For ordinary endpoints it makes the IC CDK runtime wrapper Candid-hidden, gives
that wrapper a generated fixed opaque reply encoder and separately registers
the original typed function signature for the declaration pass. The raw
payload-limited adapter likewise drops its typed result into an opaque one-byte
reply instead of calling `encode_one` or `encode_args`. Typed request decoding,
endpoint execution, authorization, payload limits, dispatch and the public
Candid declaration remain compiled.

This is bounded endpoint-reply serialization attribution. It is not a valid
reply codec, makes no reply or wire-parity claim and does not remove request
deserialization or direct inter-canister Candid encoders. It measures only the
reachable outbound `CandidType::idl_serialize` and transparent-newtype
machinery that becomes unnecessary at the endpoint reply boundary. The exact
patch applies to the byte-identical current and `v0.110.5` macro expander. Its
macro unit suite passes, as does this narrow isolated role-shape compile:

```text
CARGO_NET_OFFLINE=true cargo check --offline --locked \
  -p canister_root -p runtime_probe -p payload_limit_probe \
  -p blob_storage_probe
```

A governed baseline/patch build pair for the payload-limit fixture produced
byte-identical 9,225-byte Candid with SHA-256
`fb5a55c930325f32d26ae91a49a6e47ebd3db4ea79290d07a21bb54d7ff6d0a9`
and identical Wasm export lists. The Wasm artifacts differed, establishing
that the switch reaches runtime code without changing the declared protocol.
Those single builds are qualification evidence only: they do not satisfy the
runner's two-build determinism, complete selected-artifact matrix or immutable
delta requirements, so row 10 remains `specified` and no savings value is
retained.

Row 11's patch has SHA-256
`5807cfe5496f8b4ca4cf965475ad19777221311afd6f378ad3f2c51741bd4abd`.
It retains the endpoint signature and body, payload-limit registration,
inspect-message registry lookup, ordinary dispatch and Candid declaration, but
routes explicitly limited updates through the ordinary IC CDK adapter instead
of generating Canic's raw predecode/copy/reply adapter. The dedicated
`msg_arg_data_size`, bounded allocation, `msg_arg_data_copy`, configured Candid
decode and manual reply path therefore become unreachable from this fixture.

This is raw-adapter attribution, not a production simplification proposal.
Ingress inspect-message still rejects oversized ingress payloads, but the IC
does not invoke that hook for canister-origin calls; without the raw adapter,
those calls no longer receive the endpoint-local predecode bound. The switch
therefore makes no complete payload-safety, canister-call or runtime parity
claim. The exact patch applies to the byte-identical current and `v0.110.5`
macro expander. Its targeted 12-test endpoint-expansion suite passes, as does
this narrow isolated capability compile:

```text
CARGO_NET_OFFLINE=true cargo check --offline --locked \
  -p payload_limit_probe
```

The selected `payload_limit_probe` also completes one governed release build
under the patched immutable source. `wasm-validate`, `didc check` and `gzip -t`
accept its complete artifact set, and its Candid hash remains
`fb5a55c930325f32d26ae91a49a6e47ebd3db4ea79290d07a21bb54d7ff6d0a9`.
That qualifies row 11 as `ready`; it does not replace the runner's two-build
determinism comparison, the payload-limit PocketIC journey or an immutable
before/after delta.

The subsequent immutable
[two-build measurement](../../reports/2026-09/2026-09-03/wasm-ablation-b1-11.md)
passes exact Wasm, gzip, Candid and complete-metric determinism for both
conditions. Removing the adapter changes final Wasm by -1,055 bytes, gzip by
-316 bytes, code section by -967 bytes, data section by -88 bytes and defined
functions by zero. Inspect-message does not cover canister-origin calls, so the
small footprint is accepted and the raw adapter remains production behavior.

Row 12's patch has SHA-256
`03fa9ecaf96e7edc2506a34859a65dafbe02b3f064322044bfa6919e508a0c9e`.
It retains each role's Metrics request and response variants, endpoint
authorization and dispatch, typed request consumption, response-page shape and
all metric recording sites. The role-facing metrics helper returns an opaque
empty page instead of selecting a compiled family and invoking its snapshot,
row-conversion, sorting and pagination providers.

This is inclusive read-side provider attribution. It does not remove metric
recording, DTO/Candid types or the surrounding status endpoint, and it overlaps
row 16 only at that endpoint's Metrics branch. The empty result is deliberately
not behaviorally equivalent, so the switch makes no metrics or runtime parity
claim and is not a production deletion proposal. The exact patch applies to
the byte-identical current and `v0.110.5` helper anchor and passes this narrow
isolated role-shape compile:

```text
CARGO_NET_OFFLINE=true cargo check --offline --locked \
  -p canister_root -p canister_app -p canic-wasm-store
```

Those packages cover control-plane-heavy Root, an ordinary application role
and Store projection. They do not qualify all eleven canonical artifacts, run
a metrics journey or provide an immutable optimized delta, so row 12 remains
`specified` and no savings value is retained.

## Artifact Vector And Validator Boundary

For every selected artifact and condition, the runner records:

- final Wasm, deterministic gzip, code-section and data-section bytes;
- `ic-wasm` imports-plus-definitions total and optimizer-defined functions;
- the repository-owned replica-limited local/defined-function count;
- table minimum, element entries, Wasm exports and `ic-wasm` exported methods;
- Candid bytes, service methods and artifact hashes; and
- the complete governed optimizer before/after vector.

`wasm-validate` and `didc check` must also accept every artifact. The runner
compiles `scripts/ci/wasm-replica-function-count.rs` into invocation-local
scratch and rejects any identity that is not tied to DFINITY `ic` commit
`2f8dc21e2e5c37a4cae7f65d2a4230ac8f143e5a` and the 50,000 local-function
limit. That IC validator filters `module.functions` with `is_local()`; imported
functions are checked elsewhere and do not consume this limit. The counter
therefore reads the function-section vector, cross-checks its code-body count,
and must equal Canic's independently emitted optimizer-defined count. Its
source hash, executable hash, identity and frozen IC commit enter every run.
The `ic-wasm` total remains attribution only and cannot substitute for the
binding quantity.

Each run also retains private copies of the runner and counter source under its
method directory, records both hashes and rejects either repository source
changing during the long build. The executable is compiled from the retained
counter copy, not from a mutable pathname.

The current counter source SHA-256 is
`bcab127a0a188a1f013eb70c09e595b023fecd6ff55cddaa9a2eb1e40abb6e01`.
It emits identity `canic-b1-replica-function-count/v1`, the frozen IC commit,
quantity `local-defined-functions` and limit `50000`; the runner checks all
four fields before measuring an artifact.

Representative instruction evidence remains a separate workload result. Rows
that intentionally remove command, status, authorization or codec semantics
are build-only attribution and make no parity claim. Production contractions
must later preserve their matched workload within the design's proposed 1%
maximum regression. No production contraction may increase indirect table
entries without an exact, separately accepted compiler explanation; the
generic cohort reports its intentional nonlinear growth rather than applying
that allowance.

## Commands

Manifest-only validation is targeted and build-free:

```text
bash scripts/ci/wasm-ablation-report.sh --check
bash scripts/ci/test-wasm-ablation-report.sh
```

A retained run will use an exact clean linked worktree and external output
root; the runner owns and compiles the frozen counter:

```text
bash scripts/ci/wasm-ablation-report.sh \
  --experiment b1-01-current-baseline \
  --source <candidate-commit> \
  --product-root <clean-linked-worktree> \
  --output-root <external-evidence-directory>
```

## Next Step

Measure qualified row 3, then qualify rows 4 through 6, row 8, row 10 and row
12 against every selected artifact on a clean candidate and measure them
through the governed runner. Measured row 2 confirms the direct hypothesis
behind B2 but does not supply its required lifecycle parity; rows 3 through 5
provide differently
scoped and intentionally overlapping persistence/codec attribution needed to
choose B3 work; row 6 isolates the recovery dispatch rooted by shared watchdog
registration. None is a behavior-preserving result. Keep B2 blocked until the
full immutable B1 ledger, generic mapping and allowances are accepted.
