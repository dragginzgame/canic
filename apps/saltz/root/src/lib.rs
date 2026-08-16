//! Module: saltz_root
//!
//! Responsibility: provide the ordinary Canic Fleet Subnet Root for Saltz.
//! Does not own: waveform compilation, cycle burning, or experiment authorization.
//! Boundary: Saltz-specific behavior belongs to the dedicated application packages.

#![expect(clippy::unused_async)]

canic::start!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

canic::finish!();
