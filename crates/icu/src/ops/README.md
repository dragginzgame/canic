Operations (ops)

- Purpose: High-level workflows built on lower-level interfaces (e.g., canister creation, pool ops).
- Scope: Compose multiple `interface` calls, add validation, and return typed responses.
- Stability: Public API; changes should be noted in CHANGELOG under Unreleased.
- Tip: Keep business logic here; keep raw canister calls in `interface/`.

