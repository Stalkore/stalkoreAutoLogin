//! Pin the carrier, resolve the public SDK and register one startup snapshot.
use super::{client::ClientServices, events, Context, CONTEXT, QUEUED, STARTING, STATUS};
use crate::diagnostics::{self, log};
use stalkore_autologin::PluginConfiguration;
use stalkore_sdk_api::{client::CAP_LOGIN, status, SdkApi};
use std::{
    ffi::OsString,
    io,
    os::windows::ffi::OsStringExt,
    path::PathBuf,
    sync::{atomic::Ordering, Mutex},
};
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_PIN,
};

pub(super) fn submit() -> io::Result<()> {
    let mut module = std::ptr::null_mut();
    // Startup failure may release the host's load reference. Pin before retaining
    // any callback, including asynchronous failure notifications.
    if unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN,
            super::start as *const () as *const u16,
            &mut module,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut path = vec![0u16; 32_768];
    let len = unsafe { GetModuleFileNameW(module, path.as_mut_ptr(), path.len() as u32) } as usize;
    if len == 0 || len >= path.len() {
        return Err(io::Error::last_os_error());
    }
    let path = PathBuf::from(OsString::from_wide(&path[..len]));
    diagnostics::set_log_path(path.with_extension("log"));
    log("autoLogin: startup requested");
    let api = SdkApi::connect().map_err(io::Error::other)?;
    let client = api.client().map_err(io::Error::other)?;
    let info = client.probe().map_err(io::Error::other)?;
    if info.capabilities & CAP_LOGIN == 0 {
        return Err(io::Error::other(stalkore_sdk_api::Error(
            status::UNSUPPORTED_CLIENT,
        )));
    }
    let config = match std::fs::read_to_string(path.with_extension("ini")) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let configuration = PluginConfiguration::parse(&config).map_err(io::Error::other)?;
    let interval = configuration.tick_milliseconds();
    CONTEXT
        .set(Context {
            client: ClientServices(client),
            policy: Mutex::new(None),
            configuration,
            account_capacity: info.account_name_capacity,
        })
        .map_err(|_| io::Error::other("autoLogin context already initialized"))?;
    STATUS.store(QUEUED, Ordering::Release);
    log("autoLogin: waiting for the SDK client dispatcher");
    STATUS.store(STARTING, Ordering::Release);
    unsafe {
        api.scheduler().map_err(io::Error::other)?.subscribe(
            events::on_event,
            std::ptr::null_mut(),
            interval,
        )
    }
    .map_err(io::Error::other)?;
    Ok(())
}
