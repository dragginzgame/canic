# CANIC-185 unpaid-review readiness follow-up

Toko Miner's September 29 feedback adopts the .47 bootstrap reserve correction,
but records a null infrastructure forecast after its earlier unpaid review
replaced completed-Fleet authority. Source/seed admission succeeds at this
boundary; the completed-Fleet forecast no longer selects any targets.

Readiness now explains that boundary through
`funding.clean_reinstall_infrastructure_unavailable`. A retained infrastructure
plan without an execution journal reports `retained_infrastructure_review` with
the exact operation and plan identifiers. The metadata reader checks the selected
environment, Fleet and hash shapes without reopening executable contracts.
Missing paired inputs and other non-completed operation states have separate
typed reasons. A present forecast has a null unavailable reason and retains its
existing per-target observation diagnostics.

Plain output gives the same reason and directs the operator to preserve authority
and obtain exact funding through the supported current-release review flow. The
unavailable quote itself does not prevent compiling replacement artifacts needed
for review. Neither an empty blocker list nor a null forecast establishes funding
sufficiency. No current estimate substitutes for an older review's quoted amount;
readiness does not replace that review, approve payment or calculate a conversion
amount from unavailable funding.

## Qualification boundary

Focused coverage includes a generated, compiled infrastructure review retained
through the production plan writer. Paired policy/seed admission and the readiness
forecast/report path return the typed reason and exact identifiers without opening
the observation executable. A recursive before/after byte comparison covers all
retained files, including the plan content and clean-reinstall selection. This is
a native diagnostic/persistence proof, not a live canister reset.

Additional regressions cover malformed or foreign review identities, an absent
operation, an existing execution journal, known forecast shortfalls, JSON reason
serialization and plain-output guidance.

Focused validation passes:

- Host readiness, review metadata and generated retained-estate selection:
  18 passed, two governed PocketIC cases ignored by the native filter.
- CLI readiness parsing and reason rendering: two passed.
- Recursive CLI command ordering and concise help: two passed.
- Scoped formatting and diff whitespace checks pass.

Logs are retained at `/tmp/canic-185-readiness-native.log`,
`/tmp/canic-185-readiness-cli.log` and `/tmp/canic-185-readiness-help.log`.
These are the initial diagnostic checks. The
[CANIC-186 cancellation qualification](toko-unpaid-review-cancellation.md)
records the final combined targeted checks and the public-CLI PocketIC reset
journey. No broad workspace gate was run.

Toko remains read-only. Its actual unpaid-review acceptance, matching release
qualification and live CANIC-185 funding/conservation proof remain downstream
work. This follow-up does not change the bootstrap funding algorithm or the
retained review's approval and payment authority.
