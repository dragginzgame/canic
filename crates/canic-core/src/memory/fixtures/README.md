# Storage test setup

Collection fixtures use the published Storage fixture runtime with explicit
namespace grants, stable keys and default resource budgets. They transfer an
exclusive lease to the selected collection. Framework tests own composed
admission, observation and same-release recovery; upstream owns allocation
formats and the collection engine.
