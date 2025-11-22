use candid::CandidType;
use derive_more::Display;
use serde::{Deserialize, Serialize};

///
/// Level
///

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, CandidType, Display, Serialize, Deserialize, Copy,
)]
pub enum Level {
    Debug, // least severe
    Info,
    Ok,
    Warn,
    Error, // most severe
}

/// Emit a structured log line with consistent coloring and headers.
///
/// Accepts an optional [`Log`](crate::Log) level followed by a format string
/// and arguments, mirroring `format!`. When the level is omitted the macro
/// defaults to [`Log::Debug`](crate::Log::Debug).
#[macro_export]
macro_rules! log {
    // Explicit level, no args
    ($level:expr, $fmt:expr) => {{
        $crate::log!(@inner $level, $fmt);
    }};

    // Explicit level, with args
    ($level:expr, $fmt:expr, $($arg:tt)*) => {{
        $crate::log!(@inner $level, $fmt, $($arg)*);
    }};

    // No level → default to Debug
    ($fmt:expr) => {{
        $crate::log!(@inner $crate::log::Level::Info, $fmt);
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::log!(@inner $crate::log::Level::Info, $fmt, $($arg)*);
    }};

    // -------------------------------------------------------
    // Inner expansion
    // -------------------------------------------------------
    (@inner $level:expr, $fmt:expr $(, $($arg:tt)*)?) => {{
        let level = $level;
        let message = format!($fmt $(, $($arg)*)?);

        // Canister type label (truncated)
        let ty = $crate::memory::Env::get_canister_type();
        let ty_raw = ty.as_ref()
            .map_or("...".to_string(), ::std::string::ToString::to_string);

        let ty_disp = $crate::utils::format::ellipsize_middle(&ty_raw, 9, 4, 4);
        let ty_col = format!("{:^width$}", ty_disp, width = 9);

        // Level label based on Display impl
        let level_label = level.to_string().to_uppercase();

        // The plain/raw line (stored in stable memory, no ANSI)
        let plain_line = format!("{:<5}|{ty_col}| {message}", level_label);

        // Colored console output
        let final_line = match level {
            $crate::log::Level::Ok =>
                format!("\x1b[32m{:<5}\x1b[0m|{ty_col}| {message}", level_label),
            $crate::log::Level::Info =>
                format!("\x1b[34m{:<5}\x1b[0m|{ty_col}| {message}", level_label),
            $crate::log::Level::Warn =>
                format!("\x1b[33m{:<5}\x1b[0m|{ty_col}| {message}", level_label),
            $crate::log::Level::Error =>
                format!("\x1b[31m{:<5}\x1b[0m|{ty_col}| {message}", level_label),
            $crate::log::Level::Debug =>
                format!("{:<5}|{ty_col}| {message}", level_label),
        };

        // Output to replica console
        $crate::cdk::println!("{final_line}");

        // Store the plain log line in stable memory (no ANSI)
        let _ = $crate::memory::log::StableLog::append_line(level, &plain_line);
    }};
}
