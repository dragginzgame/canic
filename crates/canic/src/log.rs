use candid::CandidType;
use derive_more::Display;
use serde::{Deserialize, Serialize};

///
/// Debug
///

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, CandidType, Display, Serialize, Deserialize,
)]
pub enum Level {
    Debug, // least severe
    Info,
    Ok,
    Warn,
    Error, // most severe
}

#[macro_export]
macro_rules! log {
    // ============================================================
    // (1) topic, level, message
    //    log!("db", Level::Error, "failed {}", err)
    // ============================================================
    ($topic:literal, $level:expr, $fmt:expr, $($arg:tt)*) => {{
        $crate::log!(@inner $level, Some($topic), $fmt, $($arg)*);
    }};
    ($topic:literal, $level:expr, $fmt:expr) => {{
        $crate::log!(@inner $level, Some($topic), $fmt);
    }};

    // ============================================================
    // (2) topic, message
    //    log!("auth", "login {}", user)
    // ============================================================
    ($topic:literal, $fmt:expr, $($arg:tt)*) => {{
        $crate::log!(@inner $crate::log::Level::Info, Some($topic), $fmt, $($arg)*);
    }};
    ($topic:literal, $fmt:expr) => {{
        $crate::log!(@inner $crate::log::Level::Info, Some($topic), $fmt);
    }};

    // ============================================================
    // (3) level, message
    //    log!(Level::Warn, "bad input {}", id)
    // ============================================================
    ($level:expr, $fmt:expr, $($arg:tt)*) => {{
        $crate::log!(@inner $level, None, $fmt, $($arg)*);
    }};
    ($level:expr, $fmt:expr) => {{
        $crate::log!(@inner $level, None, $fmt);
    }};


    // ============================================================
    // INTERNAL IMPLEMENTATION
    // ============================================================
    (@inner $level:expr, $topic:expr, $fmt:expr $(, $($arg:tt)*)?) => {{
        let level = $level;
        let topic: Option<&str> = $topic;
        let message = format!($fmt $(, $($arg)*)?);

        // Persist log entry in stable memory
        let _ = $crate::memory::log::StableLog::append(level, topic, &message);

        // Compute canister type field
        let ty_raw = $crate::memory::Env::get_canister_type()
            .as_ref()
            .map_or_else(|| "...".to_string(), ::std::string::ToString::to_string);

        let ty_disp = $crate::utils::format::ellipsize_middle(&ty_raw, 9, 4, 4);
        let ty_centered = format!("{:^9}", ty_disp);

        // Optional topic rendering
        let final_msg = if let Some(t) = topic {
            format!("[{t}] {message}")
        } else {
            message
        };

        // ANSI color codes (Debug has no color)
        let color = match level {
            $crate::log::Level::Ok    => "\x1b[32m", // green
            $crate::log::Level::Info  => "\x1b[34m", // blue
            $crate::log::Level::Warn  => "\x1b[33m", // yellow
            $crate::log::Level::Error => "\x1b[31m", // red
            $crate::log::Level::Debug => "",         // no color
        };

        // Only apply reset if we actually colored the label
        let reset = if color.is_empty() { "" } else { "\x1b[0m" };

        // Final colored (or plain) label
        let label = format!("{color}{:^5}{reset}", level.to_string().to_uppercase());

        // Final log line
        let line = format!("{label}|{ty_centered}| {final_msg}");

        $crate::cdk::println!("{line}");
    }};
}
