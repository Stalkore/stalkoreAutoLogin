//! Client-independent decisions. The SDK supplies observations and native actions.
mod dispatch;
mod pin;
mod recovery;
use crate::{
    recovery::{Recovery, RecoverySettings},
    ClientOperations, LoginAction, PluginConfiguration,
};
use stalkore_sdk_api::client::*;
use std::{ffi::CString, time::Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Dialog {
    token: u32,
    kind: u32,
}

pub struct LoginPolicy {
    user: CString,
    password: CString,
    server: u32,
    character: Option<u8>,
    character_error: Option<&'static str>,
    pin: pin::PinPolicy,
    recovery: Recovery,
    dialog_seen: Option<Dialog>,
    dialog_acknowledged: Option<Dialog>,
    login_mode: Option<u32>,
    retry_limit_logged: bool,
    sso_wait_logged: bool,
    dispatched_revision: Option<u32>,
    credentials_sent: bool,
    dry_run: bool,
    last_progress_revision: Option<u32>,
    last_observed_phase: Option<u32>,
    next_action_at: Instant,
}

impl LoginPolicy {
    /// Compatibility constructor; explicit dry_run retains its existing priority.
    pub fn from_environment(
        dry_run: bool,
        config: &str,
        account_capacity: u32,
    ) -> Result<Self, &'static str> {
        let mut configuration = PluginConfiguration::parse(config)?;
        configuration.override_dry_run(dry_run);
        Self::from_configuration(&configuration, account_capacity)
    }

    pub fn from_configuration(
        configuration: &PluginConfiguration,
        account_capacity: u32,
    ) -> Result<Self, &'static str> {
        Self::from_sources(configuration, account_capacity, |name| {
            std::env::var_os(name)
        })
    }

    fn from_sources(
        configuration: &PluginConfiguration,
        account_capacity: u32,
        environment: impl Fn(&str) -> Option<std::ffi::OsString>,
    ) -> Result<Self, &'static str> {
        let inputs = configuration.resolve_login(account_capacity, environment)?;
        Self::new(
            (inputs.user, inputs.password),
            inputs.server,
            Some(inputs.character),
            inputs.pin.as_deref(),
            configuration.recovery,
            configuration.dry_run(),
            Instant::now(),
        )
    }

    fn new(
        credentials: (CString, CString),
        server: u32,
        character: Option<u8>,
        pin: Option<&str>,
        settings: RecoverySettings,
        dry_run: bool,
        now: Instant,
    ) -> Result<Self, &'static str> {
        let (user, password) = credentials;
        Ok(Self {
            user,
            password,
            server,
            character,
            character_error: None,
            pin: pin::PinPolicy::new(pin)?,
            recovery: Recovery::new(settings, now),
            dialog_seen: None,
            dialog_acknowledged: None,
            login_mode: None,
            retry_limit_logged: false,
            sso_wait_logged: false,
            dispatched_revision: None,
            credentials_sent: false,
            dry_run,
            last_progress_revision: None,
            last_observed_phase: None,
            next_action_at: now,
        })
    }
    pub fn tick(&mut self, client: &impl ClientOperations, now: Instant) {
        if self.next_action_at > now {
            return;
        }
        let Ok(snapshot) = client.snapshot() else {
            return;
        };
        if snapshot.has(FLAG_LOGIN_STAGE_VALID)
            && self.last_progress_revision != Some(snapshot.progress_revision)
        {
            self.recovery.progress(now);
        }
        if self.character.is_some() && !self.dry_run && self.recover(client, &snapshot, now) {
            return;
        }
        if snapshot.mode == MODE_MAP && self.last_observed_phase != Some(PHASE_MAP) {
            client.log(format_args!("autoLogin: map mode reached"));
            self.recovery.reached_map(now);
            self.pin.reached_map();
            self.reset_dispatch();
            self.retry_limit_logged = false;
            self.login_mode = None;
            self.last_progress_revision = Some(snapshot.progress_revision);
            self.last_observed_phase = Some(PHASE_MAP);
        }
        if snapshot.mode != MODE_LOGIN
            || !snapshot.has(FLAG_LOGIN_READY)
            || !snapshot.has(FLAG_LOGIN_STAGE_VALID)
            || snapshot.mode_token == 0
        {
            return;
        }
        self.observe_mode(snapshot.mode_token, now);
        if self.last_progress_revision != Some(snapshot.progress_revision) {
            if snapshot.phase == PHASE_CREDENTIALS && self.credentials_sent {
                self.recovery.failed(now);
                self.reset_dispatch();
            }
            self.recovery.progress(now);
            client.log(format_args!(
                "autoLogin: phase {} revision {}",
                snapshot.phase, snapshot.progress_revision
            ));
            self.last_progress_revision = Some(snapshot.progress_revision);
            self.last_observed_phase = Some(snapshot.phase);
        }
        // Native actions can finish asynchronously. One action per observed
        // revision prevents a fast SDK tick from submitting it again.
        if self.dispatched_revision == Some(snapshot.progress_revision) {
            return;
        }
        let action = match snapshot.phase {
            PHASE_ACCOUNT_PROMPT => LoginAction::OpenAccountSelection,
            PHASE_ACCOUNT_SELECTION => LoginAction::SelectAccount,
            PHASE_CREDENTIALS => LoginAction::SubmitCredentials,
            PHASE_SERVERS
                if snapshot.has(FLAG_SERVER_COUNT)
                    && (1..=100).contains(&snapshot.server_count)
                    && self.server < snapshot.server_count as u32 =>
            {
                LoginAction::SelectServer
            }
            PHASE_CHARACTERS => LoginAction::SelectCharacter,
            _ => return,
        };
        if matches!(
            action,
            LoginAction::SubmitCredentials | LoginAction::SelectCharacter
        ) && self.dry_run
        {
            client.log(format_args!("autoLogin: dry run reached {action:?}"));
            self.dispatched_revision = Some(snapshot.progress_revision);
            return;
        }
        self.dispatch_action(client, &snapshot, action, now);
    }
}

#[cfg(test)]
mod tests;
