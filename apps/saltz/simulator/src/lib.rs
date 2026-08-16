//! Crate: saltz_simulator
//!
//! Responsibility: model bounded waveform control through a dated smoothing approximation.
//! Does not own: burn execution, deployment, funding, timers, or mainnet authorization.
//! Boundary: floating-point results are analysis evidence and never executable authority.

mod executable;
mod model;
mod waveform;

pub use executable::{
    BACKGROUND_CYCLES_PER_SECOND, CHART_STEP_SECONDS, CONTROL_STEP_SECONDS, ExecutablePlan,
    ExecutablePlanError, INITIAL_FUNDING_STEP_COUNT, KERNEL_WINDOW_SECONDS,
    MAX_BURN_RATE_CYCLES_PER_SECOND, MAX_TOTAL_BURN_CYCLES, PRE_ROLL_STEP_COUNT,
    TARGET_AMPLITUDE_CYCLES_PER_SECOND, TARGET_FLOOR_CYCLES_PER_SECOND, WAVEFORM_STEP_COUNT,
    compile_executable_plan,
};
pub use model::{ChartPoint, SimulationConfig, SimulationError, SimulationReport, simulate};
pub use waveform::{Waveform, WaveformError, waveform};
