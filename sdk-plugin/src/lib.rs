//! Login policy entry point for SDK-owned client services.

#[cfg(all(windows, target_arch = "x86"))]
mod diagnostics;
#[cfg(all(windows, target_arch = "x86"))]
mod runtime;

/// Subscribe to the pinned SDK dispatcher. Zero acknowledges submission. Use
/// `stalkore_autologin_status_v1` and autoLogin.log to observe readiness.
#[unsafe(no_mangle)]
pub extern "C" fn stalkore_sdk_on_startup_v1() -> i32 {
    #[cfg(all(windows, target_arch = "x86"))]
    return std::panic::catch_unwind(runtime::start).unwrap_or(1);
    #[cfg(not(all(windows, target_arch = "x86")))]
    1
}

/// 0 idle, 1 queued, 2 starting, 3 dispatcher ready, 4 failed, 5 stopped.
#[unsafe(no_mangle)]
pub extern "C" fn stalkore_autologin_status_v1() -> u32 {
    #[cfg(all(windows, target_arch = "x86"))]
    return runtime::status();
    #[cfg(not(all(windows, target_arch = "x86")))]
    4
}

/// Compatibility startup alias for previously deployed SDKs.
#[unsafe(no_mangle)]
pub extern "C" fn kore_sdk_on_startup_v1() -> i32 {
    stalkore_sdk_on_startup_v1()
}

/// Compatibility status alias for existing diagnostics.
#[unsafe(no_mangle)]
pub extern "C" fn kore_login_status_v1() -> u32 {
    stalkore_autologin_status_v1()
}
