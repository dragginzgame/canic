# Qualifying public metric publication

Publication settings are compiled into each Wasm. Supplying different text to
`ManagedComponentGroupQualificationInput` does not change those settings.
The fixture deliberately keeps this a caller-validated build precondition: its
portable input contains role bytes, source text and a release identity, without
access to the caller's finalized build directory. The release manifest binds
artifact bytes and Component topology; topology identity alone does not bind
every runtime setting, including the public family selection. It therefore
cannot establish publication parity by itself.

Before constructing PocketIC or installing any canister, the caller must retain
and compare the exact configuration bytes used by the build. Reuse the existing
finalized release and artifact manifests for artifact identity. A source digest
in a test receipt records the compared build input; it is evidence, not another
runtime configuration authority.

## Matching enabled and disabled builds

Perform the following sequence separately for an enabled and a disabled build.
Use separate output/evidence directories and separate finalized release IDs.

1. Select the configuration file actually consumed by each role's
   `canic::build!` declaration or its supported build configuration input. For
   the enabled build, select the families and the real application measurement
   participant being qualified. For the disabled build, omit `public_metrics`
   or select an empty array. Keep that source unchanged for the complete build
   and test. Copy its exact bytes into the run's evidence directory and record
   `sha256sum` before building.
2. Run the normal complete `canic build <app> --environment local --profile
   release`. Retain its printed finalized release-build ID and logs. Do not reuse
   the other run's Wasms, release ID or build cache entries that were not
   validated against this run's configuration input.
3. Recompare the build input with the retained bytes using `cmp`. Reject a
   mismatch before creating the fixture. Load the current release manifest with
   `load_persisted_current_release_set_manifest`; validate it with
   `validate_finalized_release_build_manifest`. Load the application artifact
   union, compare its digest with the current manifest, and verify every selected
   role's release ID, size and SHA-256 against the exact bytes passed to the
   fixture. These are the existing `canic_host::release_build` and
   `canic_host::release_set` owners.
4. Construct `ManagedComponentGroupQualificationInput::new` using the retained
   source string, this run's finalized release ID, and these verified role bytes.
   Never substitute a string with a different publication selection. A missing
   source receipt is insufficient evidence; rebuild from the selected source.
5. On the disabled build, check `Disabled`, empty public metrics/history and no
   sampling timer. On the enabled build, exercise the real participant, advance
   through at least three five-minute slots and check values, source timestamps,
   units, counter windows, history and missing/stale observations. Confirm an
   anonymous caller can read the public cache and cannot read protected
   observability. Measure first allocation, repeat sampling and the entire
   scheduled sampling callback in Wasm, at representative maximum cardinality.
6. Inject a participant/family failure and confirm retained data ages while
   other families and ordinary cycle tracking continue. Retain both build IDs,
   source and artifact hashes, assertions and measured instruction costs.

Canic's runtime probe and disabled fixture are separate builds with their own
embedded configuration. Their timer qualification covers the framework's actual
participant, cache, history and complete sampling cost. An application's provider
and database cardinality still require that application's matching build run;
framework fixture measurements cannot establish application-specific cost.
