# Test Compile Consolidation

Date: 2026-09-08. Baseline: maintainer-pushed `v0.110.11`. Package versions
remain 0.110.11; this operator-regression batch uses the open 0.110.12 draft.

## Measured baseline

The retained release validation log is
`target/validation-runs/20260908T101525Z-25278.HAj9RC/0.log`.
Its workspace test runner took 4,497 seconds (74m57s), beyond the earlier
reported 67-minute release. These are different runs, not a regression estimate.
The retained test tiers were:

| Tier | Elapsed | Cargo compilation reported |
| --- | ---: | ---: |
| Workspace libraries/binaries | 232s | 186s |
| Separate pure internal harness | 185s | 184s |
| Ordinary integrations | 156s | 148s |
| Internal serial PocketIC | 3,524s | 14s |
| Private host PocketIC proof | 137s | 136s |
| Runtime integrations | 198s | 46s |
| Blob integrations | 55s | 4s |
| Payload integrations | 7s | 2s |

The tier sum excludes runner setup. Earlier preflight/check/Clippy/feature
barriers added 263 seconds; publication has its own timing records. Compilation
figures are reported Cargo durations rounded to seconds, not CPU time.

The internal tier's largest case was the generated mixed-topology journey at
9m17s, followed by generated reinstall at 7m32s. Artifact preparation is included
in those case totals and must not be added again. `sccache` is already selected
by Make when installed; adding another wrapper would not remove these journeys.

## Maintained change

Pure internal fixture tests now join the workspace library invocation. Their
separate `--no-default-features` compilation and aggregate harness are removed.
The ordinary internal library test binary excludes the stateful Fleet catalogue;
normal library consumers retain the default fixture exports. The serial lane
still explicitly enables the governed catalogue. This removes a Cargo pass that
cost 185 seconds in the retained run; it does not promise that the entire pass
becomes net wall-clock savings on every machine or cache state.

Each complete Fleet fixture uses one existing preflighted artifact builder for
Coordinator, Store, Root and application roles. The exact release ID, manifest,
cache inputs and output verification are preserved. Existing observation pacing
is retained. No runtime or fixture contract was weakened for speed.

## Qualification

- Default-feature internal library: six native tests pass, no PocketIC startup.
- Governed-feature inventory: passes required-case, uniqueness and order checks.
- Comparing the previous and current compiled libtest catalogues removes only
  the redundant aggregate harness; every substantive test identity is retained.
- Internal package all-target/all-feature Clippy with warnings denied: passes.
- Runner release-integrity contract and changed-script ShellCheck with governed
  exclusions: pass. Ordinary plan resolves to one workspace library invocation
  and one combined integration invocation; PocketIC remains a later barrier.
- Default-feature internal library and fixture-payload target: ten tests pass.
  Default-feature all-target Clippy also passes with warnings denied.
- A focused generated mixed-topology journey qualified shared builder preflight,
  exact artifact sealing, fifty reviewed effects, recovery, conservation and
  zero-effect replay. It passed in 830.40s (854s runner), including 332s of cold
  artifact preparation. The pacing experiment below was active during this run;
  the final source retains the original pacing already covered by the retained
  passing release. Final governed catalogue verification and all-target/all-feature
  Clippy pass after that revert. No expensive rerun was added for restored pacing.

## Rejected polling experiment

Fresh journeys were temporarily given the recovery fixtures' 25 ms observation
interval. The journey passed, but progress observations grew from 25 to 28, with
12 awaiting-progress reports versus nine. First-to-last reported progress took
477.322s versus the retained 471.126s. Different build/cache and system conditions
prevent attributing that difference to pacing, but it supplies no evidence of a
speedup. The original pacing was restored rather than retaining the experiment.
The 5m32s cold artifact preparation versus the retained 1m11s also prevents using
whole-case durations as a performance comparison. The one-builder change does
remove two repeated toolchain resolutions; its wall-time saving is not measured.

Focused logs are `/tmp/canic-internal-ordinary.log`,
`/tmp/canic-internal-inventory.log`, `/tmp/canic-internal-clippy.log`,
`/tmp/canic-turnaround-contract.log`, `/tmp/canic-turnaround-fleet.log`,
`/tmp/canic-internal-default-clippy.log`, and
`/tmp/canic-internal-default-tests.log`. Final-source checks are retained in
`/tmp/canic-internal-final-clippy.log` and
`/tmp/canic-internal-final-inventory.log`.

## Limits

No new full-release duration is established. The 58m44s serial internal tier
remains the major cost. Its fresh journeys still share process-local fixture
owners and some scratch paths; enabling parallel libtest threads would not
safely establish a speedup. A larger concurrency change needs measured isolated
fixtures and recovery qualification, rather than a global thread-count switch.
No substantive scenario or package verification was dropped, and no production
Fleet protocol, compatibility path or recovery mode was introduced.

The complete bounded source batch and its 0.110.12 changelog draft are ready
for release approval. No broad suite, version transaction, publication, deployment
or sibling edit was performed. The draft is not a new package version or release
approval. Larger test-concurrency work remains a separate measured follow-up.
