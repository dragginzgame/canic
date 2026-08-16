//! Module: saltz_burner
//!
//! Responsibility: reserve the Saltz Component role behind an inert Canic lifecycle shell.
//! Does not own: cycle burning, waveform scheduling, run state, funding, or authorization.
//! Boundary: destructive behavior remains absent until every Saltz promotion gate closes.

#![expect(clippy::unused_async)]

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

canic::finish!();
