//! First-result retention and process-lifetime policy context.
mod client;
mod events;
mod startup;
use crate::diagnostics::log;
use client::ClientServices;
use stalkore_autologin::{LoginPolicy, PluginConfiguration};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Mutex, OnceLock,
};

const QUEUED: u32 = 1;
const STARTING: u32 = 2;
const READY: u32 = 3;
const FAILED: u32 = 4;
const STOPPED: u32 = 5;
static STATUS: AtomicU32 = AtomicU32::new(0);
static START: OnceLock<i32> = OnceLock::new();
static CONTEXT: OnceLock<Context> = OnceLock::new();
struct Context {
    client: ClientServices,
    policy: Mutex<Option<LoginPolicy>>,
    configuration: PluginConfiguration,
    account_capacity: u32,
}
pub fn status() -> u32 {
    STATUS.load(Ordering::Acquire)
}
pub fn start() -> i32 {
    *START.get_or_init(|| match startup::submit() {
        Ok(()) => 0,
        Err(error) => {
            STATUS.store(FAILED, Ordering::Release);
            log(format_args!("autoLogin: startup failed: {error}"));
            1
        }
    })
}
