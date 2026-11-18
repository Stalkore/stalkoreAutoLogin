//! Bounded login recovery independent of native windows and wall-clock time.
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoverySettings {
    pub close_delay: Duration,
    pub retry_delay: Duration,
    pub timeout: Duration,
    pub max_attempts: u32,
    pub dialogs: bool,
}

impl Default for RecoverySettings {
    fn default() -> Self {
        Self {
            close_delay: Duration::from_secs(5),
            retry_delay: Duration::from_secs(5),
            timeout: Duration::from_secs(45),
            max_attempts: 3,
            dialogs: true,
        }
    }
}

impl RecoverySettings {
    pub fn parse(config: &str) -> Result<Self, &'static str> {
        let mut result = Self::default();
        for line in config.lines() {
            let Some((key, value)) = line.split('#').next().unwrap_or_default().split_once('=')
            else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "autologin_dialog_close_seconds" => {
                    result.close_delay = seconds(value, 0, 3600)?;
                }
                "autologin_retry_delay_seconds" => {
                    result.retry_delay = seconds(value, 1, 3600)?;
                }
                "autologin_timeout_seconds" => {
                    result.timeout = seconds(value, 1, 3600)?;
                }
                "autologin_max_attempts" => {
                    result.max_attempts = value
                        .parse()
                        .ok()
                        .filter(|value| (1..=100).contains(value))
                        .ok_or("autologin_max_attempts must be between 1 and 100")?;
                }
                "autologin_dialog_enabled" => {
                    result.dialogs = match value {
                        "true" | "1" => true,
                        "false" | "0" => false,
                        _ => return Err("autologin_dialog_enabled must be true or false"),
                    };
                }
                _ => {}
            }
        }
        Ok(result)
    }
}

fn seconds(value: &str, minimum: u64, maximum: u64) -> Result<Duration, &'static str> {
    value
        .parse()
        .ok()
        .filter(|value| (minimum..=maximum).contains(value))
        .map(Duration::from_secs)
        .ok_or("autologin recovery seconds are outside the supported range")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attempt {
    Wait,
    Start(u32),
    Exhausted,
}

pub struct Recovery {
    pub settings: RecoverySettings,
    attempts: u32,
    next_attempt: Instant,
    last_progress: Instant,
    in_flight: bool,
    pending: Option<(usize, Instant)>,
}

impl Recovery {
    pub fn new(settings: RecoverySettings, now: Instant) -> Self {
        Self {
            settings,
            attempts: 0,
            next_attempt: now,
            last_progress: now,
            in_flight: false,
            pending: None,
        }
    }

    pub fn begin_attempt(&mut self, now: Instant) -> Attempt {
        if self.attempts >= self.settings.max_attempts {
            return Attempt::Exhausted;
        }
        if now < self.next_attempt || self.pending.is_some() || self.in_flight {
            return Attempt::Wait;
        }
        self.attempts += 1;
        self.in_flight = true;
        self.last_progress = now;
        Attempt::Start(self.attempts)
    }

    pub fn progress(&mut self, now: Instant) {
        self.last_progress = now;
    }

    pub fn timed_out(&self, now: Instant) -> bool {
        self.in_flight
            && self.pending.is_none()
            && now.saturating_duration_since(self.last_progress) >= self.settings.timeout
    }

    /// A dialog is a timer, not an attempt. Repeated ticks cannot spend retries.
    pub fn dialog_due(&mut self, window: usize, now: Instant) -> bool {
        if !self.settings.dialogs {
            return false;
        }
        if self.pending.is_none_or(|(previous, _)| previous != window) {
            self.pending = Some((window, now));
        }
        let (_, opened) = self.pending.expect("pending dialog");
        now.saturating_duration_since(opened) >= self.settings.close_delay
    }

    pub fn has_dialog(&self) -> bool {
        self.pending.is_some()
    }

    pub fn failed(&mut self, now: Instant) {
        self.pending = None;
        self.in_flight = false;
        self.next_attempt = now + self.settings.retry_delay;
    }

    pub fn reached_map(&mut self, now: Instant) {
        self.attempts = 0;
        self.pending = None;
        self.in_flight = false;
        self.next_attempt = now;
        self.last_progress = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_delays_closure_and_t_separately_delays_the_next_login() {
        let start = Instant::now();
        let mut state = Recovery::new(RecoverySettings::default(), start);
        assert_eq!(state.begin_attempt(start), Attempt::Start(1));
        assert!(!state.dialog_due(42, start));
        assert!(!state.dialog_due(42, start + Duration::from_secs(4)));
        let closed = start + Duration::from_secs(5);
        assert!(state.dialog_due(42, closed));
        state.failed(closed);
        assert_eq!(
            state.begin_attempt(closed + Duration::from_secs(4)),
            Attempt::Wait
        );
        assert_eq!(
            state.begin_attempt(closed + Duration::from_secs(5)),
            Attempt::Start(2)
        );
    }

    #[test]
    fn three_attempts_include_the_first_and_never_allow_a_fourth() {
        let mut now = Instant::now();
        let mut state = Recovery::new(RecoverySettings::default(), now);
        for number in 1..=3 {
            assert_eq!(state.begin_attempt(now), Attempt::Start(number));
            for _ in 0..100 {
                // Reentrant/timer ticks do not create another in-flight request.
                assert_ne!(state.begin_attempt(now), Attempt::Start(number + 1));
            }
            state.failed(now);
            now += state.settings.retry_delay;
        }
        assert_eq!(state.begin_attempt(now), Attempt::Exhausted);
        assert_eq!(
            state.begin_attempt(now + Duration::from_secs(3600)),
            Attempt::Exhausted
        );
    }

    #[test]
    fn reaching_map_renews_the_budget_for_a_later_disconnect() {
        let mut now = Instant::now();
        let mut state = Recovery::new(RecoverySettings::default(), now);
        for _ in 0..3 {
            state.begin_attempt(now);
            state.failed(now);
            now += state.settings.retry_delay;
        }
        assert_eq!(state.begin_attempt(now), Attempt::Exhausted);
        state.reached_map(now);
        assert_eq!(state.begin_attempt(now), Attempt::Start(1));
    }

    #[test]
    fn timeout_follows_progress_and_does_not_run_over_a_visible_dialog() {
        let now = Instant::now();
        let mut state = Recovery::new(RecoverySettings::default(), now);
        assert!(!state.timed_out(now + Duration::from_secs(100)));
        state.begin_attempt(now);
        assert!(!state.timed_out(now + Duration::from_secs(44)));
        state.progress(now + Duration::from_secs(40));
        assert!(!state.timed_out(now + Duration::from_secs(84)));
        assert!(state.timed_out(now + Duration::from_secs(85)));
        state.dialog_due(42, now + Duration::from_secs(85));
        assert!(!state.timed_out(now + Duration::from_secs(200)));
    }

    #[test]
    fn replacement_dialog_starts_its_own_close_timer_and_disabled_dialogs_stay_open() {
        let now = Instant::now();
        let mut state = Recovery::new(RecoverySettings::default(), now);
        state.dialog_due(1, now);
        assert!(!state.dialog_due(2, now + Duration::from_secs(4)));
        assert!(!state.dialog_due(2, now + Duration::from_secs(8)));
        assert!(state.dialog_due(2, now + Duration::from_secs(9)));
        state.settings.dialogs = false;
        assert!(!state.dialog_due(2, now + Duration::from_secs(100)));
    }

    #[test]
    fn configuration_is_bounded_and_independent_of_credentials() {
        let settings = RecoverySettings::parse("autologin_dialog_close_seconds=2\nautologin_max_attempts=3\nautologin_retry_delay_seconds=8\nautologin_timeout_seconds=30").unwrap();
        assert_eq!(settings.close_delay, Duration::from_secs(2));
        assert_eq!(settings.max_attempts, 3);
        assert_eq!(settings.retry_delay, Duration::from_secs(8));
        assert_eq!(settings.timeout, Duration::from_secs(30));
        for invalid in [
            "autologin_max_attempts=0",
            "autologin_max_attempts=101",
            "autologin_retry_delay_seconds=0",
            "autologin_timeout_seconds=3601",
            "autologin_dialog_close_seconds=-1",
            "autologin_dialog_enabled=maybe",
        ] {
            assert!(RecoverySettings::parse(invalid).is_err());
        }
        assert_eq!(
            RecoverySettings::parse("").unwrap(),
            RecoverySettings::default()
        );
    }
}
