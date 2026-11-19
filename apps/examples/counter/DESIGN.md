# Counter

The starter fleet answers one question: can I change my canister’s code and keep my data? Root manages the counter, a separate Wasm Store holds its releases, and the application owns the persistent value, its fence and its readiness report.

The [current walkthrough](README.md) already exercises public increments and a managed upgrade. Extend it with the standard metrics endpoint. Define the first application metric as successful increments in the currently retained counter state, so it can be read from the same stored value without maintaining a second count.

1. Start at zero and increment three times. Read the value and the metric.
2. Record the canister ID and executable version. Upgrade the same canister from v1 to v2.
3. Show the same ID, value 3, metric 3 and the new executable version. Readiness and metrics queries must leave the value and fence untouched.

Keep this the first hands-on exercise. Snapshot recovery has its own [notebook walkthrough design](../notebook/DESIGN.md).
