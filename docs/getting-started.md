# Quick start

[Documentation](README.md) · [Configuration](configuration.md) · [Validation](validation.md)

## Build

Requirements: Windows, a current Rust MSVC toolchain, the x86 target and an
existing compatible stalkoreSdk runtime. The SDK plugin needs the public API
Git dependency; it does not need private SDK/adapter source.

```powershell
git clone https://github.com/Stalkore/stalkoreAutoLogin.git
rustup target add i686-pc-windows-msvc
Set-Location '.\stalkoreAutoLogin\sdk-plugin'
cargo build --locked --release --target i686-pc-windows-msvc --jobs 1 --lib
if ($LASTEXITCODE -ne 0) { throw 'Plugin build failed.' }
```

Build from `sdk-plugin` so its Cargo configuration supplies rust-lld and static
CRT settings. The DLL is `target/i686-pc-windows-msvc/release/autoLogin.dll`.

## Install and keep local settings

Close the game and launcher before replacing a loaded DLL. The layout is:

### Combined Forwarder layout

Build against the updated local SDK API checkout so `SdkApi::connect()` can
discover the API export in the generated carrier. This is a one-time plugin
rebuild when migrating from the separate `sdk.dll` layout. From `sdk-plugin`:

```powershell
cargo --config 'patch."https://github.com/Stalkore/stalkoreSdkApi.git".stalkore-sdk-api.path="D:/GitHub/Stalkore/stalkoreSdkApi"' build --release --target i686-pc-windows-msvc --jobs 1 --lib
if ($LASTEXITCODE -ne 0) { throw 'Plugin build failed.' }
```

```text
client/
├── D3DX9_43.dll             generated Forwarder with embedded SDK
├── plugins.json             {"plugins":["Plugins/autoLogin.dll"]}
└── Plugins/
    ├── autoLogin.dll
    ├── autoLogin.ini
    └── autoLogin.log
```

The INI and log follow the plugin DLL, so their names and contents remain
unchanged. Keep `sdk.dll` out of this installation. Once the updated SDK API
revision is published and this plugin pins it, ordinary `--locked` builds can
replace the local patch command. The runtime still calls the fixed
`stalkore_sdk_on_startup_v1` export.

### Separate SDK compatibility layout

The following commands describe the older two-DLL installation:

```text
client/
├── D3DX9_43.dll              Forwarder
├── sdk.dll                  private SDK runtime
├── sdk.json                 explicit plugin paths
└── SDK/
    ├── autoLogin.dll         this plugin
    ├── autoLogin.ini         local login and automation settings
    └── autoLogin.log         append-only diagnostics
```

From the build directory:

```powershell
$clientDirectory = Read-Host 'Client directory'
if (-not (Test-Path -LiteralPath $clientDirectory -PathType Container)) {
    throw 'Client directory does not exist.'
}
$pluginDirectory = Join-Path $clientDirectory 'SDK'
New-Item -ItemType Directory -Path $pluginDirectory -Force | Out-Null
Copy-Item '.\target\i686-pc-windows-msvc\release\autoLogin.dll' $pluginDirectory -Force
$configurationPath = Join-Path $pluginDirectory 'autoLogin.ini'
if (-not (Test-Path -LiteralPath $configurationPath)) {
    Copy-Item '.\autoLogin.example.ini' $configurationPath
}
```

The runtime configuration retains the same path-only schema:

```json
{"plugins":["SDK/autoLogin.dll"]}
```

Use the [SDK example](https://github.com/Stalkore/stalkoreSdk/blob/main/fixtures/plugins.json). The host calls
`stalkore_sdk_on_startup_v1`; the old callback name remains an alias. No arbitrary
entrypoint, extra executable or separate client-adapter DLL is installed.

## Fill the local file and open the launcher

Edit `SDK/autoLogin.ini`. Fill username/password in `[login]`, choose the
zero-based server and occupied character indices, and optionally fill the
existing four-digit PIN. See [all configuration rules](configuration.md).

File-based startup does not require RO_* variables. If the current shell still
contains earlier overrides, remove them deliberately or use a fresh shell.
Start the launcher normally, or from PowerShell:

```powershell
$launcherFilename = Read-Host 'Launcher executable filename'
Start-Process -FilePath (Join-Path $clientDirectory $launcherFilename) -WorkingDirectory $clientDirectory
```

Use the installation's actual launcher filename. A launcher that starts the
game after edits must create a new game process for the new snapshot.

## Optional environment overrides

Set only the fields you want to override, before opening the launcher:

```powershell
$env:RO_CHAR = '1'
```

An environment-only launch remains supported. Secure prompts keep values out of
command history:

```powershell
$env:RO_USERNAME = [System.Net.NetworkCredential]::new('', (Read-Host 'Account' -AsSecureString)).Password
$env:RO_PASSWORD = [System.Net.NetworkCredential]::new('', (Read-Host 'Password' -AsSecureString)).Password
$env:RO_SERVER = Read-Host 'Zero-based server index'
$env:RO_CHAR = Read-Host 'Zero-based occupied character slot'
$env:RO_PIN = [System.Net.NetworkCredential]::new('', (Read-Host 'Existing PIN; Enter if unused' -AsSecureString)).Password
```

Defined values take priority over the file. [Configuration](configuration.md)
explains invalid and empty overrides.

## Observe the current process

`sdk.log` records startup callback acceptance. `SDK/autoLogin.log` records
dispatcher readiness, selected phases, recovery and map observation:

```text
autoLogin: ready
autoLogin: login attempt 1/3
autoLogin: native SSO login requested
autoLogin: dispatched SelectCharacter
autoLogin: map mode reached
```

Account, password and PIN values are not recorded. Logs append across processes;
compare timestamps/PIDs instead of treating a historical map line as current
success. Callback acceptance and dispatcher readiness alone do not mean map entry.
