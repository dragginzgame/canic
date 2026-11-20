# Counter

The starter fleet answers one question: can I change my canister’s code and keep my data? Root manages the counter, a separate Wasm Store holds its releases, and the application owns the persistent value, its fence and its readiness report.

The [current walkthrough](README.md) already exercises public increments and a managed upgrade. The value and fence survive a compatible upgrade. Endpoint traffic measurement has its own [metrics fleet](../metrics/README.md), and snapshot recovery has its own [notebook walkthrough](../notebook/DESIGN.md).
