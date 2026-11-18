# Migration

[Documentation](README.md) · [Quick start](getting-started.md)

## Public organization and package layout

The canonical repository is
[Stalkore/stalkoreAutoLogin](https://github.com/Stalkore/stalkoreAutoLogin).
It is published with a clean initial history. Update existing clones:

```powershell
git remote set-url origin https://github.com/Stalkore/stalkoreAutoLogin.git
```

A clone based on the former private history has a different ancestry. Use a
fresh clone to work against the public initial commit; preserve any local work
before changing checkouts. Local build caches are not a release input.

The root `stalkore-autologin` crate contains portable configuration, login,
PIN and recovery policy. The former separate policy directory has been folded
into the root to remove an unnecessary package layer. The production
`stalkore-autologin-plugin` crate depends on it and the public SDK contract.

Client-specific native compatibility code belongs to a private companion library.
Legacy carriers consume that library under their existing Rust alias. The public
plugin has no private path dependency, native feature or embedded profile.

## Installed contracts

The DLL remains `autoLogin.dll`, the local file remains `autoLogin.ini` and
the versioned sample remains `autoLogin.example.ini`. Existing configuration
keys, source priority, timers and PIN rules remain compatible.

Canonical exports are `stalkore_sdk_on_startup_v1` and
`stalkore_autologin_status_v1`. Older startup/status spellings remain aliases
sharing the same initialization state. RO_* names remain configuration aliases;
a defined variable takes precedence over its corresponding file field.

Diagnostic lines use the neutral `autoLogin:` prefix. Log consumers should
match the current process and event text rather than an old client-specific prefix.
Existing live validation records identify their original binary.

## Configuration upgrade

Copy the sample only when the local file is absent. To adopt file startup,
append its [login] section, fill values locally and remove previous environment
overrides only when file values are intended. See [configuration](configuration.md).

The SDK still declares DLL paths in `sdk.json`. No public ABI version,
native profile or deployed-path change is required. Build/copy the plugin using
the [quick start](getting-started.md); existing SDK/Forwarder runtimes remain compatible.
