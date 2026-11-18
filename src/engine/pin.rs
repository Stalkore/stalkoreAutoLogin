use super::ClientOperations;
use stalkore_sdk_api::client::{ClientSnapshotV1, FLAG_PIN_PRESENT, FLAG_PIN_READY, PIN_CONFIRM};

pub(super) struct PinPolicy {
    digits: Option<[u8; 4]>,
    attempted: bool,
    notice: Option<&'static str>,
}

impl PinPolicy {
    pub fn new(value: Option<&str>) -> Result<Self, &'static str> {
        let digits = value
            .map(|value| {
                if value.len() != 4 || !value.bytes().all(|digit| digit.is_ascii_digit()) {
                    return Err("RO_PIN must contain exactly four ASCII digits");
                }
                Ok(value.as_bytes().try_into().expect("four validated bytes"))
            })
            .transpose()?;
        Ok(Self {
            digits,
            attempted: false,
            notice: None,
        })
    }
    pub fn reached_map(&mut self) {
        self.attempted = false;
        self.notice = None;
    }
    fn report(&mut self, client: &impl ClientOperations, message: &'static str) {
        if self.notice != Some(message) {
            client.log(format_args!("autoLogin: {message}"));
            self.notice = Some(message);
        }
    }
    fn submit_once(&mut self, mut send: impl FnMut(u32, u32) -> bool) -> bool {
        if self.attempted {
            return false;
        }
        let Some(digits) = self.digits else {
            return false;
        };
        self.attempted = true;
        for (count, digit) in digits.into_iter().enumerate() {
            if !send(u32::from(digit - b'0'), count as u32) {
                return false;
            }
        }
        send(PIN_CONFIRM, 4)
    }
    pub fn tick(&mut self, client: &impl ClientOperations, snapshot: &ClientSnapshotV1) -> bool {
        if !snapshot.has(FLAG_PIN_PRESENT) {
            return false;
        }
        if self.attempted {
            return true;
        }
        if !snapshot.has(FLAG_PIN_READY) {
            self.report(
                client,
                "PIN window requires manual input or is not ready for existing-PIN entry",
            );
            return true;
        }
        if self.digits.is_none() {
            self.report(
                client,
                "PIN required; set RO_PIN before starting the launcher or enter it manually",
            );
            return true;
        }
        let requested = self.submit_once(|key, count| {
            client.pin_ready(snapshot.mode_token, count)
                && client.pin_key(snapshot.mode_token, key, count)
        });
        self.report(
            client,
            if requested {
                "native PIN confirmation requested; automatic retries are disabled"
            } else {
                "PIN dispatch stopped; automatic retries are disabled; continue manually"
            },
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leading_zero_and_confirmation_keep_one_attempt_until_map_entry() {
        let mut pin = PinPolicy::new(Some("0123")).unwrap();
        let mut keys = Vec::new();
        assert!(pin.submit_once(|key, count| {
            keys.push((key, count));
            true
        }));
        assert_eq!(keys, [(0, 0), (1, 1), (2, 2), (3, 3), (PIN_CONFIRM, 4)]);
        assert!(!pin.submit_once(|_, _| panic!("PIN must not repeat")));
        pin.reached_map();
        assert!(pin.submit_once(|_, _| true));
    }
    #[test]
    fn partial_failure_also_consumes_the_attempt_and_invalid_pin_is_rejected() {
        for invalid in ["", "123", "12345", "12a4", "１２３４"] {
            assert!(PinPolicy::new(Some(invalid)).is_err());
        }
        let mut pin = PinPolicy::new(Some("0123")).unwrap();
        assert!(!pin.submit_once(|_, count| count < 2));
        assert!(!pin.submit_once(|_, _| panic!("partial PIN must not retry")));
    }
}
