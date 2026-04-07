# canic-testkit

Public PocketIC-oriented test utilities for projects that use Canic.

Use this crate when you want generic host-side test infrastructure that is
still publishable and reusable outside the Canic workspace.

What it owns:
- PocketIC startup and builder helpers
- generic call/install helpers
- ready polling and diagnostics
- standalone non-root canister fixtures
- generic prebuilt-wasm install helpers
- cached PocketIC baseline primitives
- workspace/wasm artifact helpers used by host-side tests

Current API shape:
- `Pic` is the intentional host-side wrapper surface for PocketIC calls used by this crate
- cached baseline guards expose explicit accessors instead of transparently derefing into raw `PocketIc`
- tests should prefer the wrapper methods and fixture helpers here instead of reaching through to the underlying PocketIC client directly

What it intentionally does not own:
- Canic's full root-topology harness
- attestation-specific fixture policy
- repo-only audit probes
- broad Canic self-test orchestration

Those repo-specific seams belong in the unpublished
`canic-testing-internal` crate instead of widening this public surface.

If you are writing downstream PocketIC tests, start here.
If you are editing Canic's own root/auth integration harnesses, you probably
want `canic-testing-internal` instead.
