use crate::{
    dto::metrics::timer::TimerMetricEntry,
    model::metrics::timer::{TimerMetricKey, TimerMode},
};

#[must_use]
pub fn timer_metrics_to_view(
    raw: impl IntoIterator<Item = (TimerMetricKey, u64)>,
) -> Vec<TimerMetricEntry> {
    raw.into_iter()
        .map(|(key, count)| TimerMetricEntry {
            mode: mode_to_string(key.mode),
            delay_ms: key.delay_ms,
            label: key.label,
            count,
        })
        .collect()
}

fn mode_to_string(mode: TimerMode) -> String {
    match mode {
        TimerMode::Once => "once",
        TimerMode::Interval => "interval",
    }
    .to_string()
}
