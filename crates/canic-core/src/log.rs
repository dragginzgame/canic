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

///
/// Topic
///

#[derive(Clone, Copy, Display, Eq, PartialEq)]
#[remain::sorted]
pub enum Topic {
    App,
    Auth,
    CanisterLifecycle,
    CanisterReserve,
    CanisterState,
    Cycles,
    Init,
    Memory,
    Perf,
    Sharding,
    Sync,
    Topology,
    Wasm,
}

#[macro_export]
macro_rules! log {
    // =========================================
    // (1) With topic (normal + trailing comma)
    // =========================================
    ($topic:expr, $level:ident, $fmt:expr $(, $arg:expr)* $(,)?) => {{
        $crate::log!(@inner Some(&$topic.to_string()), $crate::log::Level::$level, $fmt $(, $arg)*);
    }};

    // =========================================
    // (2) No topic (normal + trailing comma)
    // =========================================
    ($level:ident, $fmt:expr $(, $arg:expr)* $(,)?) => {{
        $crate::log!(@inner None::<&str>, $crate::log::Level::$level, $fmt $(, $arg)*);
    }};

    // =========================================
    // INTERNAL
    // =========================================
    (@inner $topic:expr, $level:expr, $fmt:expr $(, $arg:expr)*) => {{
        let level = $level;
        let topic_opt: Option<&str> = $topic;
        let message = format!($fmt $(, $arg)*);

        // append entry
        let crate_name = env!("CARGO_PKG_NAME");
        let _ = $crate::log::__append_to_stable_log(crate_name, topic_opt, level, &message);

        let ty_raw = $crate::log::__canister_role_string().unwrap_or_else(|| "...".to_string());

        let ty_disp = $crate::utils::format::ellipsize_middle(&ty_raw, 9, 4, 4);
        let ty_centered = format!("{:^9}", ty_disp);

        let final_msg = if let Some(t) = topic_opt {
            format!("[{t}] {message}")
        } else {
            message
        };

        let (color, reset) = match level {
            $crate::log::Level::Ok    => ("\x1b[32m", "\x1b[0m"),
            $crate::log::Level::Info  => ("\x1b[34m", "\x1b[0m"),
            $crate::log::Level::Warn  => ("\x1b[33m", "\x1b[0m"),
            $crate::log::Level::Error => ("\x1b[31m", "\x1b[0m"),
            $crate::log::Level::Debug => ("", ""),
        };

        let label = format!("{color}{:^5}{reset}", level.to_string().to_uppercase());
        let line = format!("{label}|{ty_centered}| {final_msg}");

        $crate::cdk::println!("{line}");
    }};
}

///
/// Helpers
///

#[doc(hidden)]
pub fn __append_to_stable_log(
    crate_name: &str,
    topic: Option<&str>,
    level: Level,
    message: &str,
) -> Result<u64, crate::Error> {
    crate::model::memory::log::StableLog::append(crate_name, topic, level, message)
}

#[doc(hidden)]
#[must_use]
pub fn __canister_role_string() -> Option<String> {
    crate::model::memory::Env::get_canister_role().map(|role| role.to_string())
}
