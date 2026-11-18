//! Operations supplied by the SDK binding or a deterministic policy fixture.
use stalkore_sdk_api::client::ClientSnapshotV1;
use std::{ffi::CStr, fmt};

pub trait ClientOperations {
    fn snapshot(&self) -> Result<ClientSnapshotV1, &'static str>;
    fn open_account_selection(&self, mode: u32) -> bool;
    fn select_account(&self, mode: u32) -> bool;
    fn request_login(&self, mode: u32, user: &CStr, password: &CStr) -> bool;
    fn select_server(&self, mode: u32, server: u32) -> bool;
    fn select_character(&self, mode: u32, slot: u8) -> Result<bool, &'static str>;
    fn pin_ready(&self, mode: u32, count: u32) -> bool;
    fn pin_key(&self, mode: u32, key: u32, count: u32) -> bool;
    fn acknowledge_dialog(&self, dialog: u32, failed: bool) -> bool;
    fn return_to_login(&self, mode: u32) -> bool;
    fn log(&self, message: fmt::Arguments<'_>);
}
