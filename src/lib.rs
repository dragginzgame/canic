pub mod cycles;
pub mod helper;
pub mod ic;
pub mod macros;
pub mod serialize;
pub mod state;
pub mod wasm;

pub mod export {
    pub use defer;
}

///
/// Log
///

pub enum Log {
    Ok,
    Perf,
    Info,
    Warn,
    Error,
}
