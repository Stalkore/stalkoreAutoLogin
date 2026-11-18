//! Dialog acknowledgment, timeout guards and recovery state transitions.
use super::{Dialog, LoginPolicy};
use crate::ClientOperations;
use stalkore_sdk_api::client::*;
use std::time::Instant;

impl LoginPolicy {
    pub(super) fn reset_dispatch(&mut self) {
        self.dispatched_revision = None;
        self.credentials_sent = false;
        self.character_error = None;
        self.sso_wait_logged = false;
    }
    pub(super) fn observe_mode(&mut self, mode: u32, now: Instant) {
        if self.login_mode != Some(mode) {
            if self.login_mode.is_some() {
                self.recovery.failed(now);
            }
            self.reset_dispatch();
            self.login_mode = Some(mode);
        }
    }

    pub(super) fn recover(
        &mut self,
        client: &impl ClientOperations,
        snapshot: &ClientSnapshotV1,
        now: Instant,
    ) -> bool {
        let phase = snapshot
            .has(FLAG_LOGIN_STAGE_VALID)
            .then_some(snapshot.phase);
        let failed = (self.credentials_sent
            && matches!(
                phase,
                Some(PHASE_CREDENTIALS | PHASE_AUTHENTICATING | PHASE_SERVERS)
            ))
            || snapshot.has(FLAG_MAP_DISCONNECTED);
        let dialog = (snapshot.dialog_token != 0
            && (snapshot.dialog_kind == DIALOG_DISCONNECTED
                || snapshot.dialog_kind == DIALOG_CONNECTION_FAILURE && failed))
            .then_some(Dialog {
                token: snapshot.dialog_token,
                kind: snapshot.dialog_kind,
            });
        if let Some(dialog) = dialog {
            if self.dialog_acknowledged == Some(dialog) {
                return true;
            }
            if self.dialog_seen != Some(dialog) {
                client.log(format_args!(
                    "autoLogin: connection dialog kind={}; automatic close={} delay={}s",
                    dialog.kind,
                    self.recovery.settings.dialogs,
                    self.recovery.settings.close_delay.as_secs()
                ));
                self.dialog_seen = Some(dialog);
            }
            if self.recovery.dialog_due(dialog.token as usize, now)
                && client.acknowledge_dialog(dialog.token, failed)
            {
                self.recovery.failed(now);
                self.reset_dispatch();
                self.dialog_acknowledged = Some(dialog);
                client.log(format_args!(
                    "autoLogin: dialog acknowledged; next login attempt in {}s",
                    self.recovery.settings.retry_delay.as_secs()
                ));
            }
            return true;
        }
        if phase.is_some_and(|phase| phase != PHASE_CHARACTERS) && snapshot.has(FLAG_MODAL) {
            return true;
        }
        self.dialog_seen = None;
        self.dialog_acknowledged = None;
        if self.recovery.has_dialog() {
            self.recovery.failed(now);
            self.reset_dispatch();
        }
        if phase == Some(PHASE_AUTHENTICATING) && self.recovery.timed_out(now) {
            if !snapshot.has(FLAG_SSO_IDLE) {
                if !self.sso_wait_logged {
                    client.log(format_args!("autoLogin: login timeout; waiting for native SSO completion before recovery"));
                    self.sso_wait_logged = true;
                }
                return true;
            }
            if snapshot.mode_token != 0 && client.return_to_login(snapshot.mode_token) {
                self.recovery.failed(now);
                self.reset_dispatch();
                client.log(format_args!("autoLogin: login timeout; native return to login requested; next attempt in {}s", self.recovery.settings.retry_delay.as_secs()));
            }
            return true;
        }
        false
    }
}
