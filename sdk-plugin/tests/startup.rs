#![cfg(all(windows, target_arch = "x86"))]
mod support;

use std::os::windows::{ffi::OsStrExt, process::CommandExt};
use std::process::Command;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

#[test]
fn missing_sdk_is_reported_once_without_loading_bridge() {
    const CHILD: &str = "SDK_AUTOLOGIN_STARTUP_FIXTURE";
    let Ok(directory) = std::env::var(CHILD) else {
        let directory =
            std::env::temp_dir().join(format!("stalkore-autologin-plugin-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let deps = std::env::current_exe().unwrap();
        std::fs::copy(
            support::dll("autoLogin.dll", "stalkore-autologin-plugin"),
            directory.join("autoLogin.dll"),
        )
        .unwrap();
        let output = Command::new(deps)
            .args([
                "--exact",
                "missing_sdk_is_reported_once_without_loading_bridge",
                "--nocapture",
            ])
            .env(CHILD, &directory)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let log = std::fs::read_to_string(directory.join("autoLogin.log")).unwrap();
        assert_eq!(log.matches("autoLogin: startup requested").count(), 1);
        assert!(log.contains("SDK API is unavailable"));
        assert!(!log.contains("autoLogin: ready"));
        std::fs::remove_dir_all(directory).unwrap();
        return;
    };
    let path = std::path::Path::new(&directory).join("autoLogin.dll");
    let path: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    unsafe {
        let module = LoadLibraryW(path.as_ptr());
        assert!(!module.is_null());
        let start: unsafe extern "C" fn() -> i32 = std::mem::transmute(
            GetProcAddress(module, c"kore_sdk_on_startup_v1".as_ptr().cast()).unwrap(),
        );
        let status: unsafe extern "C" fn() -> u32 = std::mem::transmute(
            GetProcAddress(module, c"kore_login_status_v1".as_ptr().cast()).unwrap(),
        );
        assert_eq!(status(), 0);
        assert_eq!(start(), 1);
        assert_eq!(status(), 4);
        assert_eq!(start(), 1);
        assert_eq!(status(), 4);
        assert!(
            GetModuleHandleW("bridge.dll\0".encode_utf16().collect::<Vec<_>>().as_ptr()).is_null()
        );
    }
}
