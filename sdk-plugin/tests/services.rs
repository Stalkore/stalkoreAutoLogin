#![cfg(all(windows, target_arch = "x86"))]
mod support;
use stalkore_sdk_api::client::*;
use std::{
    os::windows::{ffi::OsStrExt, process::CommandExt},
    process::Command,
    time::Duration,
};
use windows_sys::Win32::System::{
    LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
    Threading::CREATE_NO_WINDOW,
};
const FILE: &str =
    "[login]\nusername=fixture-user\npassword=fixture-password\nserver=1\ncharacter=2\npin=\n";
const ENVIRONMENT: &[(&str, &str)] = &[
    ("RO_USERNAME", "fixture-user"),
    ("RO_PASSWORD", "fixture-password"),
    ("RO_SERVER", "1"),
    ("RO_CHAR", "2"),
];

#[test]
fn plugin_uses_only_public_services_for_login_server_character_and_map() {
    const CHILD: &str = "SDK_AUTOLOGIN_SERVICE_FIXTURE";
    let Ok(directory) = std::env::var(CHILD) else {
        for case in [
            "valid",
            "missing",
            "invalid",
            "file",
            "mixed",
            "invalid_override",
            "invalid_pin_override",
            "file_missing",
            "syntax",
        ] {
            let directory = std::env::temp_dir()
                .join(format!("autologin-services-{}-{case}", std::process::id()));
            std::fs::create_dir_all(&directory).unwrap();
            let executable = std::env::current_exe().unwrap();
            for (name, package) in [
                ("sdk.dll", "stalkore-sdk-api-provider-fixture"),
                ("autoLogin.dll", "stalkore-autologin-plugin"),
            ] {
                std::fs::copy(support::dll(name, package), directory.join(name)).unwrap();
            }
            if matches!(
                case,
                "file"
                    | "mixed"
                    | "invalid_override"
                    | "invalid_pin_override"
                    | "file_missing"
                    | "syntax"
            ) {
                let config = match case {
                    "syntax" => "[fixture-sensitive-syntax".to_owned(),
                    "mixed" => FILE
                        .replace("fixture-password", "file-fixture-password")
                        .replace("server=1", "server=0")
                        .replace("character=2", "character=0"),
                    "file_missing" => FILE.replace("character=2\n", ""),
                    "invalid_pin_override" => FILE.replace("pin=\n", "pin=0123\n"),
                    _ => FILE.to_owned(),
                };
                std::fs::write(directory.join("autoLogin.ini"), config).unwrap();
            }
            let mut command = Command::new(executable);
            command
                .args([
                    "--exact",
                    "plugin_uses_only_public_services_for_login_server_character_and_map",
                    "--nocapture",
                ])
                .env(CHILD, &directory)
                .env("STALKORE_AUTOLOGIN_ENV_CASE", case)
                .creation_flags(CREATE_NO_WINDOW);
            for name in [
                "RO_USERNAME",
                "RO_PASSWORD",
                "RO_SERVER",
                "RO_CHAR",
                "RO_PIN",
            ] {
                command.env_remove(name);
            }
            if matches!(case, "valid" | "missing" | "invalid") {
                command.envs(ENVIRONMENT.iter().copied());
            }
            match case {
                "missing" => {
                    command.env_remove("RO_CHAR");
                }
                "invalid" | "invalid_override" => {
                    command.env("RO_CHAR", "256");
                }
                "invalid_pin_override" => {
                    command.env("RO_PIN", "invalid-fixture");
                }
                "mixed" => {
                    command
                        .env("RO_PASSWORD", "fixture-password")
                        .env("RO_SERVER", "1")
                        .env("RO_CHAR", "2");
                }
                _ => {}
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{case}: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let log = std::fs::read_to_string(directory.join("autoLogin.log")).unwrap();
            if matches!(case, "valid" | "file" | "mixed") {
                assert!(log.contains("native SSO login requested"));
                assert!(log.contains("map mode reached"));
            } else {
                let expected = match case {
                    "missing" | "file_missing" => "RO_CHAR is missing",
                    "invalid_pin_override" => "RO_PIN",
                    "syntax" => "autoLogin.ini has invalid INI syntax",
                    _ => "RO_CHAR must be",
                };
                assert!(log.contains(expected));
                assert!(!log.contains("native SSO login requested"));
            }
            for secret in [
                "fixture-user",
                "fixture-password",
                "file-fixture-password",
                "invalid-fixture",
                "fixture-sensitive-syntax",
            ] {
                assert!(!log.contains(secret));
            }
            std::fs::remove_dir_all(directory).unwrap();
        }
        return;
    };
    unsafe {
        let load = |name: &str| {
            let path = std::path::Path::new(&directory).join(name);
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
            let module = LoadLibraryW(wide.as_ptr());
            assert!(!module.is_null());
            module
        };
        let sdk = load("sdk.dll");
        let plugin = load("autoLogin.dll");
        let start: unsafe extern "C" fn() -> i32 = std::mem::transmute(
            GetProcAddress(plugin, c"stalkore_sdk_on_startup_v1".as_ptr().cast()).unwrap(),
        );
        let status: unsafe extern "C" fn() -> u32 = std::mem::transmute(
            GetProcAddress(plugin, c"stalkore_autologin_status_v1".as_ptr().cast()).unwrap(),
        );
        let tick: unsafe extern "C" fn(u32) -> u32 =
            std::mem::transmute(GetProcAddress(sdk, c"fixture_tick_v1".as_ptr().cast()).unwrap());
        let case = std::env::var("STALKORE_AUTOLOGIN_ENV_CASE").unwrap();
        let startup_result = if case == "syntax" { 1 } else { 0 };
        assert_eq!(start(), startup_result);
        let legacy_start: unsafe extern "C" fn() -> i32 = std::mem::transmute(
            GetProcAddress(plugin, c"kore_sdk_on_startup_v1".as_ptr().cast()).unwrap(),
        );
        let legacy_status: unsafe extern "C" fn() -> u32 = std::mem::transmute(
            GetProcAddress(plugin, c"kore_login_status_v1".as_ptr().cast()).unwrap(),
        );
        assert_eq!(legacy_start(), startup_result);
        assert_eq!(legacy_status(), status());
        if !matches!(
            std::env::var("STALKORE_AUTOLOGIN_ENV_CASE")
                .unwrap()
                .as_str(),
            "valid" | "file" | "mixed"
        ) {
            assert_eq!(status(), 4);
            assert_eq!(tick(PHASE_CREDENTIALS), 0);
            return;
        }
        assert_eq!(status(), 3);
        assert_eq!(start(), 0);
        assert_eq!(tick(PHASE_CREDENTIALS), 1);
        std::thread::sleep(Duration::from_millis(1050));
        assert_eq!(tick(PHASE_SERVERS), 3);
        std::thread::sleep(Duration::from_millis(1050));
        assert_eq!(tick(PHASE_CHARACTERS), 7);
        std::thread::sleep(Duration::from_millis(1050));
        assert_eq!(tick(PHASE_MAP), 7);
        for name in ["bridge.dll", "sdkNative.dll", "stalkoreClientAdapter.dll"] {
            let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
            assert!(GetModuleHandleW(wide.as_ptr()).is_null());
        }
    }
}
