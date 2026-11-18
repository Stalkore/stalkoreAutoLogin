# Tests

[Documentation](README.md) · [Architecture](architecture.md) · [Validation](validation.md)

## Choose the affected path

| Change | Checks |
| --- | --- |
| Configuration, phases, recovery or PIN | Root portable tests and plugin DLL fixtures |
| Startup, binding, logging or lifetime | Plugin x86 tests, Clippy and SDK/Forwarder loader check |
| Public contract | Contract checks plus affected consumers |
| Private client implementation | Separate adapter acceptance; a public policy test cannot establish native correctness |

Run Rust compilers sequentially with `--jobs 1`. Owned native children run
without visible windows. Ordinary tests do not start installed clients, read
login files or submit real credentials. Fixture children remove inherited RO_*
values before supplying synthetic data.

## Portable policy

From the repository root:

```powershell
cargo fmt --all -- --check
cargo test --locked --jobs 1
cargo clippy --locked --all-targets --jobs 1 -- -D warnings
```

The suite covers per-field priority, invalid/empty overrides, literal
punctuation/quotes/backslashes, UTF-8 BOMs, buffer bounds, phase progression,
native SSO guards, one-attempt PIN lifetime, dialog delays, timeout progress,
retry budgets and renewal after map observation.

## Production DLL

```powershell
Set-Location '.\sdk-plugin'
cargo fmt --all -- --check
cargo test --locked --target i686-pc-windows-msvc --jobs 1
cargo clippy --locked --all-targets --target i686-pc-windows-msvc --jobs 1 -- -D warnings
```

Nine synthetic-provider scenarios load the actual plugin DLL with environment-only,
file-only, mixed and invalid configuration. The provider checks exact synthetic
credential bytes, server and character. Failed initialization sends no login.
Other checks cover missing SDK, startup/status alias retention and append logging.

The harnesses support adjacent and per-build-unit Cargo artifacts. They search
only the matching package in the current profile and reject missing or ambiguous
DLLs. They never substitute an installed client plugin.

## Documentation and integration

Run root and x86 plugin rustdoc with `RUSTDOCFLAGS=-D warnings`. Check relative
links, PowerShell snippets and the SVG after documentation changes. Review staged
files and publication history for credentials, binaries and local data.

Private SDK and Forwarder maintainers separately run loader fixtures with the
real debug SDK/plugin DLLs. Those checks establish loading and rejection order,
not a live map connection. The source must also build from a checkout containing
only this public repository and its pinned public dependencies.

GitHub checks run portable policy on Linux and production DLL tests on Windows
x86. Client-specific legacy carrier tests remain in the private adapter repository.
