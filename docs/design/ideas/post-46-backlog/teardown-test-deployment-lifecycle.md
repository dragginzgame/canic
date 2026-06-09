# Teardown And Test Deployment Lifecycle

Status: optional idea.

This idea includes plan-driven teardown, authority-aware cleanup, test
deployment create/rebuild/teardown workflows, cleanup receipts, verified
postconditions, and safeguards against production authority leaking into test
plans.

Reason deferred: teardown is authority-sensitive and risky. It should be
implemented only when there is a clear test-deployment lifecycle requirement.

Constraint if revived: broad cleanup commands remain a non-goal.
