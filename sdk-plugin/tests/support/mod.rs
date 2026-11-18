//! Resolve test DLLs in Cargo's flat and per-build-unit artifact layouts.
use std::path::{Path, PathBuf};

pub fn dll(name: &str, package: &str) -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    locate(&executable, name, package)
}

fn locate(executable: &Path, name: &str, package: &str) -> PathBuf {
    let adjacent = executable.parent().unwrap().join(name);
    if adjacent.is_file() {
        return adjacent;
    }
    // Newer Cargo versions keep test executables and DLLs in separate build
    // units. Search only this profile's exact package, never installed DLL paths.
    let build = executable
        .ancestors()
        .find(|directory| directory.file_name().is_some_and(|name| name == "build"))
        .expect("test executable is outside a recognized Cargo build layout");
    let candidates: Vec<_> = std::fs::read_dir(build.join(package))
        .expect("fixture package artifacts are missing")
        .map(|entry| entry.unwrap().path().join("out").join(name))
        .filter(|path| path.is_file())
        .collect();
    assert_eq!(
        candidates.len(),
        1,
        "expected one {name} artifact for {package}; found {}",
        candidates.len()
    );
    candidates.into_iter().next().unwrap()
}
