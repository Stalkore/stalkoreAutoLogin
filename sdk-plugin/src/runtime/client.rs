//! Translate the public SDK result contract into policy operations.
use crate::diagnostics::log;
use stalkore_autologin::ClientOperations;
use stalkore_sdk_api::client::Client;
use std::{ffi::CStr, fmt};

pub(super) struct ClientServices(pub(super) Client);
impl ClientOperations for ClientServices {
    fn snapshot(&self) -> Result<stalkore_sdk_api::client::ClientSnapshotV1, &'static str> {
        self.0.snapshot().map_err(|error| error.message())
    }
    fn open_account_selection(&self, mode: u32) -> bool {
        self.0.open_account_selection(mode).unwrap_or(false)
    }
    fn select_account(&self, mode: u32) -> bool {
        self.0.select_account(mode).unwrap_or(false)
    }
    fn request_login(&self, mode: u32, user: &CStr, password: &CStr) -> bool {
        self.0.request_login(mode, user, password).unwrap_or(false)
    }
    fn select_server(&self, mode: u32, server: u32) -> bool {
        self.0.select_server(mode, server).unwrap_or(false)
    }
    fn select_character(&self, mode: u32, slot: u8) -> Result<bool, &'static str> {
        self.0
            .select_character(mode, slot)
            .map_err(|error| error.message())
    }
    fn pin_ready(&self, mode: u32, count: u32) -> bool {
        self.0.pin_ready(mode, count).unwrap_or(false)
    }
    fn pin_key(&self, mode: u32, key: u32, count: u32) -> bool {
        self.0.pin_key(mode, key, count).unwrap_or(false)
    }
    fn acknowledge_dialog(&self, token: u32, failed: bool) -> bool {
        self.0.acknowledge_dialog(token, failed).unwrap_or(false)
    }
    fn return_to_login(&self, mode: u32) -> bool {
        self.0.return_to_login(mode).unwrap_or(false)
    }
    fn log(&self, message: fmt::Arguments<'_>) {
        log(message);
    }
}
