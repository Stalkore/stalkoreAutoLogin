//! Execute one selected logical action while preserving attempt/PIN lifetimes.
use super::LoginPolicy;
use crate::{recovery::Attempt, ClientOperations, LoginAction};
use stalkore_sdk_api::client::*;
use std::time::{Duration, Instant};

impl LoginPolicy {
    pub(super) fn dispatch_action(
        &mut self,
        client: &impl ClientOperations,
        snapshot: &ClientSnapshotV1,
        action: LoginAction,
        now: Instant,
    ) {
        self.next_action_at = now + Duration::from_secs(1);
        let mode = snapshot.mode_token;
        let dispatched = match action {
            LoginAction::OpenAccountSelection => client.open_account_selection(mode),
            LoginAction::SelectAccount => client.select_account(mode),
            LoginAction::SubmitCredentials if !self.credentials_sent => {
                if !snapshot.has(FLAG_SSO_IDLE) {
                    return;
                }
                match self.recovery.begin_attempt(now) {
                    Attempt::Wait => return,
                    Attempt::Exhausted => {
                        if !self.retry_limit_logged {
                            client.log(format_args!("autoLogin: stopped after {} login attempts; automatic recovery exhausted", self.recovery.settings.max_attempts));
                            self.retry_limit_logged = true;
                        }
                        return;
                    }
                    Attempt::Start(number) => client.log(format_args!(
                        "autoLogin: login attempt {number}/{}",
                        self.recovery.settings.max_attempts
                    )),
                }
                self.credentials_sent = true;
                let requested = client.request_login(mode, &self.user, &self.password);
                if requested {
                    client.log(format_args!("autoLogin: native SSO login requested"));
                } else {
                    self.recovery.failed(now);
                    self.reset_dispatch();
                    client.log(format_args!(
                        "autoLogin: login preparation failed; waiting before next attempt"
                    ));
                }
                requested
            }
            LoginAction::SelectServer => client.select_server(mode, self.server),
            LoginAction::SelectCharacter if self.pin.tick(client, snapshot) => false,
            LoginAction::SelectCharacter => match self.character {
                Some(slot) => match client.select_character(mode, slot) {
                    Ok(value) => value,
                    Err(error) => {
                        if self.character_error != Some(error) {
                            client.log(format_args!(
                                "autoLogin: character selection waiting: {error}"
                            ));
                            self.character_error = Some(error);
                        }
                        false
                    }
                },
                None => false,
            },
            LoginAction::SubmitCredentials => false,
        };
        if dispatched {
            client.log(format_args!("autoLogin: dispatched {action:?}"));
            self.dispatched_revision = Some(snapshot.progress_revision);
        }
    }
}
