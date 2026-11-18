//! SDK callbacks own policy initialization and UI-thread progression.
use super::{CONTEXT, FAILED, READY, STATUS, STOPPED};
use crate::diagnostics::log;
use stalkore_autologin::LoginPolicy;
use stalkore_sdk_api::{scheduler::*, status, Status};
use std::{sync::atomic::Ordering, time::Instant};

pub(super) unsafe extern "C" fn on_event(
    _: *mut std::ffi::c_void,
    event: *const TickEventV1,
) -> Status {
    let result = std::panic::catch_unwind(|| {
        if event.is_null() {
            return status::INVALID_ARGUMENT;
        }
        let event = unsafe { &*event };
        if !event.header.supports::<TickEventV1>() {
            return status::UNSUPPORTED_VERSION;
        }
        let Some(context) = CONTEXT.get() else {
            return status::NOT_READY;
        };
        match event.kind {
            EVENT_READY => {
                let policy = match LoginPolicy::from_configuration(
                    &context.configuration,
                    context.account_capacity,
                ) {
                    Ok(policy) => policy,
                    Err(error) => {
                        log(format_args!("autoLogin: initialization failed: {error}"));
                        return status::INVALID_ARGUMENT;
                    }
                };
                let Ok(mut state) = context.policy.lock() else {
                    return status::INTERNAL_ERROR;
                };
                *state = Some(policy);
                STATUS.store(READY, Ordering::Release);
                log(format_args!(
                    "autoLogin: ready; UI thread={} dry_run={}",
                    event.thread_id,
                    context.configuration.dry_run()
                ));
            }
            EVENT_TICK => {
                // Native actions can pump messages. A nested event must not
                // deadlock or advance the same policy twice.
                if let Ok(mut state) = context.policy.try_lock() {
                    if let Some(policy) = state.as_mut() {
                        policy.tick(&context.client, Instant::now());
                    }
                }
            }
            EVENT_STOPPED => {
                STATUS.store(STOPPED, Ordering::Release);
                log("autoLogin: client UI thread stopped");
            }
            EVENT_FAILED => {
                STATUS.store(FAILED, Ordering::Release);
                log(format_args!(
                    "autoLogin: dispatcher failed: {}",
                    stalkore_sdk_api::Error(event.status)
                ));
            }
            _ => return status::INVALID_ARGUMENT,
        }
        status::OK
    });
    match result {
        Ok(status::OK) => status::OK,
        Ok(error) => {
            STATUS.store(FAILED, Ordering::Release);
            error
        }
        Err(_) => {
            STATUS.store(FAILED, Ordering::Release);
            log("autoLogin: policy callback panicked");
            status::INTERNAL_ERROR
        }
    }
}
