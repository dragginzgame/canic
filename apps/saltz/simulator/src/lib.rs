//! Crate: saltz_simulator
//!
//! Responsibility: model bounded waveform control through a dated smoothing approximation.
//! Does not own: burn execution, deployment, funding, timers, or mainnet authorization.
//! Boundary: floating-point results are analysis evidence and never executable authority.

mod model;
mod waveform;

pub use model::{ChartPoint, SimulationConfig, SimulationError, SimulationReport, simulate};
pub use waveform::{Waveform, WaveformError, waveform};
