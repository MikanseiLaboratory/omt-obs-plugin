//! Timestamp conversion between OBS nanoseconds and OMT 100 ns ticks.

/// OBS uses nanoseconds; OMT uses 100 ns ticks.
pub const NS_PER_OMT_TICK: u64 = 100;

/// Convert OMT 100 ns ticks to OBS nanoseconds.
pub fn omt_ticks_to_obs_ns(ticks: i64) -> u64 {
    if ticks <= 0 {
        0
    } else {
        (ticks as u64).saturating_mul(NS_PER_OMT_TICK)
    }
}

/// Convert OBS nanoseconds to OMT 100 ns ticks.
pub fn obs_ns_to_omt_ticks(ns: u64) -> i64 {
    (ns / NS_PER_OMT_TICK) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let ns = 33_333_333u64;
        let ticks = obs_ns_to_omt_ticks(ns);
        assert_eq!(omt_ticks_to_obs_ns(ticks), 33_333_300);
    }

    #[test]
    fn negative_ticks_become_zero() {
        assert_eq!(omt_ticks_to_obs_ns(-1), 0);
    }
}
