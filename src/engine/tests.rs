use super::*;
use crate::recovery::Attempt;
use std::{
    cell::{Cell, RefCell},
    ffi::CStr,
    fmt,
    time::Duration,
};

#[test]
fn character_environment_distinguishes_missing_and_invalid_without_echoing_values() {
    let configuration = PluginConfiguration::parse("").unwrap();
    let resolve = |slot: Option<&str>| {
        LoginPolicy::from_sources(&configuration, 50, |name| match name {
            "RO_CHAR" => slot.map(Into::into),
            "RO_USERNAME" => Some("fixture-user".into()),
            "RO_PASSWORD" => Some("fixture-password".into()),
            "RO_SERVER" => Some("0".into()),
            _ => None,
        })
    };
    assert!(resolve(None).err().unwrap().contains("RO_CHAR is missing"));
    for value in ["", "-1", "256", "slot 1", "private-fixture-value"] {
        let error = resolve(Some(value)).err().unwrap();
        assert!(error.starts_with("RO_CHAR must be a zero-based character slot"));
    }
    assert_eq!(resolve(Some("0")).unwrap().character, Some(0));
    assert_eq!(resolve(Some("255")).unwrap().character, Some(255));
}

#[test]
fn invalid_pin_override_does_not_use_the_file_pin_or_start_native_work() {
    let configuration = PluginConfiguration::parse("[login]\nusername=fixture-user\npassword=fixture-password\nserver=0\ncharacter=0\npin=0123\n").unwrap();
    for pin in ["123", "12345", "bad-fixture", "１２３４"] {
        assert!(
            LoginPolicy::from_sources(&configuration, 50, |name| (name == "RO_PIN")
                .then(|| pin.into()))
            .err()
            .unwrap()
            .contains("RO_PIN")
        );
    }
    let configuration = PluginConfiguration::parse("[login]\nusername=fixture-user\npassword=fixture-password\nserver=0\ncharacter=0\npin=bad-fixture\n").unwrap();
    assert!(
        LoginPolicy::from_sources(&configuration, 50, |name| (name == "RO_PIN")
            .then(|| "0123".into()))
        .is_ok()
    );
}

struct Client {
    state: Cell<ClientSnapshotV1>,
    calls: RefCell<Vec<String>>,
    login_ok: Cell<bool>,
    pin_ok: Cell<bool>,
}
impl Client {
    fn new() -> Self {
        Self {
            state: Cell::new(ClientSnapshotV1::default()),
            calls: RefCell::new(Vec::new()),
            login_ok: Cell::new(true),
            pin_ok: Cell::new(true),
        }
    }
    fn phase(&self, phase: u32, revision: u32) {
        self.state.set(ClientSnapshotV1 {
            mode: MODE_LOGIN,
            mode_token: 1,
            phase,
            progress_revision: revision,
            flags: FLAG_LOGIN_READY | FLAG_LOGIN_STAGE_VALID | FLAG_SSO_IDLE | FLAG_SERVER_COUNT,
            server_count: 2,
            ..Default::default()
        });
    }
    fn record(&self, action: String) -> bool {
        self.calls.borrow_mut().push(action);
        true
    }
    fn count(&self, action: &str) -> usize {
        self.calls
            .borrow()
            .iter()
            .filter(|call| call.as_str() == action)
            .count()
    }
}
impl ClientOperations for Client {
    fn snapshot(&self) -> Result<ClientSnapshotV1, &'static str> {
        Ok(self.state.get())
    }
    fn open_account_selection(&self, _: u32) -> bool {
        self.record("open".into())
    }
    fn select_account(&self, _: u32) -> bool {
        self.record("account".into())
    }
    fn request_login(&self, _: u32, _: &CStr, _: &CStr) -> bool {
        self.record("login".into());
        self.login_ok.get()
    }
    fn select_server(&self, _: u32, server: u32) -> bool {
        self.record(format!("server:{server}"))
    }
    fn select_character(&self, _: u32, slot: u8) -> Result<bool, &'static str> {
        Ok(self.record(format!("character:{slot}")))
    }
    fn pin_ready(&self, _: u32, _: u32) -> bool {
        self.pin_ok.get()
    }
    fn pin_key(&self, _: u32, key: u32, count: u32) -> bool {
        self.record(format!("pin:{key}:{count}"));
        self.pin_ok.get()
    }
    fn acknowledge_dialog(&self, _: u32, _: bool) -> bool {
        self.record("ack".into())
    }
    fn return_to_login(&self, _: u32) -> bool {
        self.record("return".into())
    }
    fn log(&self, _: fmt::Arguments<'_>) {}
}
fn policy(now: Instant, pin: Option<&str>, dry_run: bool) -> LoginPolicy {
    LoginPolicy::new(
        (c"fixture-user".into(), c"fixture-password".into()),
        1,
        Some(2),
        pin,
        RecoverySettings::default(),
        dry_run,
        now,
    )
    .unwrap()
}

#[test]
fn complete_flow_uses_sso_then_server_pin_character_and_resets_at_map() {
    let now = Instant::now();
    let client = Client::new();
    let mut policy = policy(now, Some("0123"), false);
    for (second, phase) in [
        PHASE_ACCOUNT_PROMPT,
        PHASE_ACCOUNT_SELECTION,
        PHASE_CREDENTIALS,
        PHASE_AUTHENTICATING,
        PHASE_SERVERS,
    ]
    .into_iter()
    .enumerate()
    {
        client.phase(phase, second as u32 + 1);
        policy.tick(&client, now + Duration::from_secs(second as u64));
    }
    client.phase(PHASE_CHARACTERS, 6);
    let mut state = client.state.get();
    state.flags |= FLAG_PIN_PRESENT | FLAG_PIN_READY;
    client.state.set(state);
    policy.tick(&client, now + Duration::from_secs(5));
    assert_eq!(
        *client.calls.borrow(),
        [
            "open", "account", "login", "server:1", "pin:0:0", "pin:1:1", "pin:2:2", "pin:3:3",
            "pin:10:4"
        ]
    );
    // A still-open PIN window never receives a second attempt.
    policy.tick(&client, now + Duration::from_secs(6));
    assert_eq!(client.count("pin:0:0"), 1);
    state.flags &= !(FLAG_PIN_PRESENT | FLAG_PIN_READY);
    client.state.set(state);
    policy.tick(&client, now + Duration::from_secs(7));
    policy.tick(&client, now + Duration::from_secs(8));
    assert_eq!(client.count("character:2"), 1);
    client.state.set(ClientSnapshotV1 {
        mode: MODE_MAP,
        phase: PHASE_MAP,
        progress_revision: 7,
        ..Default::default()
    });
    policy.tick(&client, now + Duration::from_secs(9));
    assert_eq!(
        policy.recovery.begin_attempt(now + Duration::from_secs(9)),
        Attempt::Start(1)
    );
}

#[test]
fn dialog_close_and_retry_are_separate_timers_and_ack_does_not_use_stale_state() {
    let now = Instant::now();
    let client = Client::new();
    let mut policy = policy(now, None, false);
    client.phase(PHASE_CREDENTIALS, 1);
    policy.tick(&client, now);
    client.phase(PHASE_AUTHENTICATING, 2);
    let mut state = client.state.get();
    state.dialog_token = 9;
    state.dialog_kind = DIALOG_CONNECTION_FAILURE;
    client.state.set(state);
    policy.tick(&client, now + Duration::from_secs(1));
    policy.tick(&client, now + Duration::from_secs(5));
    assert_eq!(client.count("ack"), 0);
    policy.tick(&client, now + Duration::from_secs(6));
    policy.tick(&client, now + Duration::from_secs(7));
    assert_eq!(client.count("ack"), 1);
    client.phase(PHASE_CREDENTIALS, 3);
    policy.tick(&client, now + Duration::from_secs(10));
    assert_eq!(client.count("login"), 1);
    policy.tick(&client, now + Duration::from_secs(11));
    assert_eq!(client.count("login"), 2);
}

#[test]
fn failures_exhaust_three_attempts_until_map_and_normal_notices_are_not_acknowledged() {
    let now = Instant::now();
    let client = Client::new();
    let mut policy = policy(now, None, false);
    client.phase(PHASE_CREDENTIALS, 1);
    client.login_ok.set(false);
    for second in 0..30 {
        policy.tick(&client, now + Duration::from_secs(second));
    }
    assert_eq!(client.count("login"), 3);
    client.state.set(ClientSnapshotV1 {
        mode: MODE_MAP,
        phase: PHASE_MAP,
        progress_revision: 2,
        dialog_kind: DIALOG_CONNECTION_FAILURE,
        dialog_token: 10,
        ..Default::default()
    });
    policy.tick(&client, now + Duration::from_secs(30));
    assert_eq!(client.count("ack"), 0);
    client.phase(PHASE_CREDENTIALS, 3);
    policy.tick(&client, now + Duration::from_secs(31));
    assert_eq!(client.count("login"), 4);
}

#[test]
fn timeout_waits_for_sso_and_progress_revisions_restart_the_timer() {
    let now = Instant::now();
    let client = Client::new();
    let mut policy = policy(now, None, false);
    client.phase(PHASE_CREDENTIALS, 1);
    policy.tick(&client, now);
    client.phase(PHASE_AUTHENTICATING, 2);
    policy.tick(&client, now + Duration::from_secs(1));
    client.phase(PHASE_AUTHENTICATING, 3);
    policy.tick(&client, now + Duration::from_secs(40));
    policy.tick(&client, now + Duration::from_secs(46));
    assert_eq!(client.count("return"), 0);
    let mut state = client.state.get();
    state.flags &= !FLAG_SSO_IDLE;
    client.state.set(state);
    policy.tick(&client, now + Duration::from_secs(90));
    assert_eq!(client.count("return"), 0);
    state.flags |= FLAG_SSO_IDLE;
    client.state.set(state);
    policy.tick(&client, now + Duration::from_secs(91));
    assert_eq!(client.count("return"), 1);
}

#[test]
fn dry_run_and_invalid_server_counts_never_authenticate_or_select() {
    let now = Instant::now();
    let client = Client::new();
    let mut policy = policy(now, None, true);
    client.phase(PHASE_CREDENTIALS, 1);
    policy.tick(&client, now);
    client.phase(PHASE_CHARACTERS, 2);
    policy.tick(&client, now + Duration::from_secs(1));
    client.phase(PHASE_SERVERS, 3);
    let mut state = client.state.get();
    state.server_count = 1;
    client.state.set(state);
    policy.tick(&client, now + Duration::from_secs(2));
    assert!(client.calls.borrow().is_empty());
}
