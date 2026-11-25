# Client update maintenance

[Home](../README.md) · [Getting started](getting-started.md) · [Validation](validation.md)

AutoLogin owns portable policy, configuration and SDK calls. It does not own
client addresses, native layouts or executable fingerprints. A client update
should therefore be handled at the SDK adapter boundary before changing this
plugin.

## Recognize a compatibility failure

The typical update failure is:

```text
autoLogin: startup failed: unsupported client executable
```

The SDK log records the plugin's nonzero startup status for the same process.
This means SDK startup reached AutoLogin, the public API connected, and the
adapter rejected the executable identity or one of its structural guards. It
does not mean that `autoLogin.ini`, credentials or retry policy are wrong.

Do not retry around this error and do not copy addresses into this public
repository. Re-establish support in the private client adapter, rebuild the SDK
against the reviewed adapter revision, then retest this unchanged plugin first.

## Maintenance sequence

1. Preserve the previous working executable and record old/new SHA-256 values.
2. Follow the private adapter's client-update procedure to review identity,
   layouts, methods, calling conventions and native lifetimes.
3. Update the SDK's pinned adapter revision and rebuild `sdk.dll`.
4. Rebuild `autoLogin.dll` only if its SDK API pin or policy source changed.
5. Keep the existing local `autoLogin.ini`; do not copy the example over it.
6. Validate startup, account flow, server selection, PIN, character entry, map
   entry, a portal transition and one supported recovery path in that order.

## Rebuild and source checks

```powershell
Set-Location 'D:\GitHub\Stalkore\stalkoreAutoLogin'
cargo fmt --all -- --check
cargo test --locked --jobs 1
cargo clippy --locked --all-targets --jobs 1 -- -D warnings

Set-Location '.\sdk-plugin'
cargo fmt --all -- --check
cargo test --locked --target i686-pc-windows-msvc --jobs 1
cargo clippy --locked --all-targets --target i686-pc-windows-msvc --jobs 1 -- -D warnings
cargo build --locked --release --target i686-pc-windows-msvc --jobs 1 --lib
```

The output is `sdk-plugin/target/i686-pc-windows-msvc/release/autoLogin.dll`.
Source checks use synthetic SDK providers; only the staged live sequence proves
compatibility with an updated client.

## API pin rule

The plugin pins a stalkoreSdkApi Git revision in `sdk-plugin/Cargo.toml`.
Do not update it merely because the client executable changed. Update the pin
only when the SDK contract changes, then refresh `Cargo.lock`, run the complete
plugin tests and verify the old callback aliases remain available.

## Validation record

For each supported update, record only:

- executable SHA-256 or an internal build identifier;
- adapter, SDK API, SDK and AutoLogin commit revisions;
- configuration source used (`autoLogin.ini` or environment overrides);
- the last completed live stage and timestamp;
- any untested recovery or map-transition path.

Never record account names, passwords, PINs, tokens or raw authentication
traffic. A startup callback accepted by the SDK is not equivalent to map entry.
