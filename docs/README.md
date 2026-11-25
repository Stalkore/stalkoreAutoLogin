# Documentation

[Project overview](../README.md)

## Follow one login decision

Read [`configuration`](../src/configuration.rs) →
[`LoginPolicy::tick`](../src/engine.rs) →
[`dispatch_action`](../src/engine/dispatch.rs) →
[`SDK callback`](../sdk-plugin/src/runtime/events.rs). The policy decides
*when* to act; the SDK and its private adapter decide *how* to perform a
guarded client operation. Follow a recovery path in
[`recovery`](../src/engine/recovery.rs) too. The [architecture](architecture.md)
explains the boundary; [validation](validation.md) separates earlier map
observations from current client-update acceptance.

## Install and use

1. [Quick start](getting-started.md): build the DLL and place it in the SDK folder.
2. [Configuration](configuration.md): fill the INI and understand environment overrides.
3. [Recovery and PIN](recovery.md): learn when automation retries or requires manual input.
4. [Validation](validation.md): check the supported client and recorded evidence.

## Develop and maintain

1. [Architecture](architecture.md): policy, configuration, SDK binding and legacy integration.
2. [Tests](testing.md): checks to run for each affected path.
3. [Migration](migration.md): canonical names, retained exports and configuration upgrades.
4. [Contributing](contributing.md): scope, source ownership and review expectations.
5. [Client update maintenance](client-updates.md): diagnose compatibility rejection and revalidate the native flow after an update.
