# CI Locks, Signing, And Provider Wrappers

Status: optional idea.

This idea includes CI lock acquire/refresh/release behavior, optional signing
or attestation for saved evidence, provider-specific wrappers such as GitHub
Actions integration, and additional machine-readable release evidence beyond
existing manifests or envelopes.

Reason deferred: 0.51 through 0.53 already added the evidence, provenance,
policy-gate, and manifest foundations. Locks, signing, and wrappers are
automation features, not historical closeout requirements.

Constraint if revived: locks and signing must not create deployment authority
or make stale evidence fresh.
