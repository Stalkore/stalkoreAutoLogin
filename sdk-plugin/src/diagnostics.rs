use std::{
    cell::Cell,
    fmt::Display,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use windows_sys::Win32::Foundation::{GetLastError, SetLastError, SYSTEMTIME};
use windows_sys::Win32::System::{SystemInformation::GetLocalTime, Threading::GetCurrentProcessId};

thread_local! {
    static IN_LOG: Cell<bool> = const { Cell::new(false) };
}
static WRITER: Mutex<()> = Mutex::new(());
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Select the carrier's append log before starting native workers.
pub fn set_log_path(path: PathBuf) {
    let _ = LOG_PATH.set(path);
}

struct LogCall(u32);

impl Drop for LogCall {
    fn drop(&mut self) {
        IN_LOG.with(|busy| busy.set(false));
        unsafe { SetLastError(self.0) };
    }
}

/// Appends one record, preserving the caller's Win32 error even on I/O failure.
pub fn log(line: impl Display) {
    append(
        LOG_PATH
            .get()
            .map_or(Path::new("autoLogin.log"), PathBuf::as_path),
        line,
    );
}

fn append(path: &Path, line: impl Display) {
    let error = unsafe { GetLastError() };
    // Formatting and opening the destination can themselves trigger diagnostics.
    // Reject recursion per thread, while other threads retain their records.
    if IN_LOG.with(|busy| busy.replace(true)) {
        unsafe { SetLastError(error) };
        return;
    }
    let _call = LogCall(error);
    let mut now: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut now) };
    let record = format!(
        "[{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03} pid {}] {line}\n",
        now.wYear,
        now.wMonth,
        now.wDay,
        now.wHour,
        now.wMinute,
        now.wSecond,
        now.wMilliseconds,
        unsafe { GetCurrentProcessId() }
    );
    let _writer = WRITER.lock().unwrap_or_else(|error| error.into_inner());
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(record.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fmt,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn logging_preserves_errors_and_records_across_recursion_and_threads() {
        let root = std::env::temp_dir().join(format!(
            "stalkore-autologin-log-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("diagnostic.log");
        std::fs::write(&path, "existing evidence\n").unwrap();
        struct Recursive<'a>(&'a Path);
        impl Display for Recursive<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                append(self.0, "recursive record must be skipped");
                unsafe { SetLastError(456) };
                formatter.write_str("outer record")
            }
        }
        unsafe { SetLastError(123) };
        append(&path, Recursive(&path));
        assert_eq!(unsafe { GetLastError() }, 123);
        append(&root.join("missing").join("log"), "unwritable");
        assert_eq!(unsafe { GetLastError() }, 123);
        std::thread::scope(|scope| {
            for thread in 0..8 {
                let path = &path;
                scope.spawn(move || {
                    for record in 0..16 {
                        append(path, format_args!("worker={thread} record={record}"));
                    }
                });
            }
        });
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("existing evidence\n"));
        assert_eq!(text.lines().count(), 130);
        assert_eq!(text.matches("outer record").count(), 1);
        assert!(!text.contains("recursive record"));
        for thread in 0..8 {
            for record in 0..16 {
                assert_eq!(
                    text.matches(&format!("] worker={thread} record={record}\n"))
                        .count(),
                    1
                );
            }
        }
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
