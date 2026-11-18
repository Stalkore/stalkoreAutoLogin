//! Synthetic SDK provider; no client, profiles, hook or authentication exists here.
use stalkore_sdk_api::{client::*, scheduler::*, *};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Mutex,
};
static CALLBACK: Mutex<Option<TickCallbackV1>> = Mutex::new(None);
static PHASE: AtomicU32 = AtomicU32::new(PHASE_CREDENTIALS);
static REVISION: AtomicU32 = AtomicU32::new(1);
static CALLS: AtomicU32 = AtomicU32::new(0);
unsafe extern "C" fn probe(out: *mut ClientInfoV1) -> Status {
    unsafe {
        out.write(ClientInfoV1 {
            capabilities: CAP_LOGIN | CAP_CHARACTER | CAP_PIN,
            account_name_capacity: 50,
            ..Default::default()
        });
    }
    status::OK
}
unsafe extern "C" fn snapshot(out: *mut ClientSnapshotV1) -> Status {
    let phase = PHASE.load(Ordering::SeqCst);
    unsafe {
        out.write(ClientSnapshotV1 {
            mode: if phase == PHASE_MAP {
                MODE_MAP
            } else {
                MODE_LOGIN
            },
            mode_token: 1,
            phase,
            progress_revision: REVISION.load(Ordering::SeqCst),
            flags: FLAG_LOGIN_READY | FLAG_LOGIN_STAGE_VALID | FLAG_SSO_IDLE | FLAG_SERVER_COUNT,
            server_count: 2,
            ..Default::default()
        });
    }
    status::OK
}
unsafe extern "C" fn request(_: u32, user: BytesV1, password: BytesV1) -> Status {
    if user.len == 0 || password.len == 0 || user.data.is_null() || password.data.is_null() {
        return status::INVALID_ARGUMENT;
    }
    // Synthetic expected values prove the production DLL used the selected
    // source. No bytes are printed or stored in diagnostics.
    if unsafe { std::slice::from_raw_parts(user.data, user.len as usize) } != b"fixture-user"
        || unsafe { std::slice::from_raw_parts(password.data, password.len as usize) }
            != b"fixture-password"
    {
        return status::INVALID_ARGUMENT;
    }
    CALLS.fetch_or(1, Ordering::SeqCst);
    status::OK
}
unsafe extern "C" fn server(_: u32, slot: u32) -> Status {
    if slot != 1 {
        return status::INVALID_ARGUMENT;
    }
    CALLS.fetch_or(2, Ordering::SeqCst);
    status::OK
}
unsafe extern "C" fn character(_: u32, slot: u32) -> Status {
    if slot != 2 {
        return status::INVALID_ARGUMENT;
    }
    CALLS.fetch_or(4, Ordering::SeqCst);
    status::OK
}
static CLIENT: ClientApiV1 = ClientApiV1 {
    header: ApiHeader::of::<ClientApiV1>(),
    probe: Some(probe),
    snapshot: Some(snapshot),
    open_account_selection: None,
    select_account: None,
    request_login: Some(request),
    select_server: Some(server),
    select_character: Some(character),
    pin_ready: None,
    pin_key: None,
    acknowledge_dialog: None,
    return_to_login: None,
};
unsafe extern "C" fn subscribe(
    callback: Option<TickCallbackV1>,
    context: *mut std::ffi::c_void,
    _: u32,
    id: *mut u32,
) -> Status {
    if !context.is_null() {
        return status::INVALID_ARGUMENT;
    }
    *CALLBACK.lock().unwrap() = callback;
    unsafe {
        id.write(1);
    }
    emit(EVENT_READY);
    status::OK
}
unsafe extern "C" fn unsubscribe(_: u32) -> Status {
    *CALLBACK.lock().unwrap() = None;
    status::OK
}
static SCHEDULER: SchedulerApiV1 = SchedulerApiV1 {
    header: ApiHeader::of::<SchedulerApiV1>(),
    subscribe: Some(subscribe),
    unsubscribe: Some(unsubscribe),
};
#[unsafe(no_mangle)]
pub unsafe extern "C" fn StalkoreSdkQueryInterfaceV1(
    service: u32,
    version: u32,
    out: *mut *const ApiHeader,
) -> Status {
    if version != ABI_VERSION {
        return status::UNSUPPORTED_VERSION;
    }
    unsafe {
        out.write(match service {
            CLIENT_SERVICE => &CLIENT.header,
            SCHEDULER_SERVICE => &SCHEDULER.header,
            _ => return status::UNKNOWN_SERVICE,
        });
    }
    status::OK
}
fn emit(kind: u32) {
    let callback = *CALLBACK.lock().unwrap();
    if let Some(callback) = callback {
        unsafe {
            callback(
                std::ptr::null_mut(),
                &TickEventV1 {
                    header: ApiHeader::of::<TickEventV1>(),
                    kind,
                    status: status::OK,
                    subscription: 1,
                    thread_id: 1,
                },
            );
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn fixture_tick_v1(phase: u32) -> u32 {
    if PHASE.swap(phase, Ordering::SeqCst) != phase {
        REVISION.fetch_add(1, Ordering::SeqCst);
    }
    emit(EVENT_TICK);
    CALLS.load(Ordering::SeqCst)
}
