# Validation

[Documentation](README.md) · [Configuration](configuration.md) · [Tests](testing.md)

## Current executable update — 2026-11-24

The installed plugin reached SDK startup but recorded `unsupported client
executable`; the SDK recorded the plugin's nonzero startup status for the same
process. The private adapter's compiled executable identity differs from the
updated installed binary. No credential, INI or retry failure preceded the
rejection.

This is a confirmed compatibility failure at the adapter boundary. The earlier
map-entry record below remains valid only for its tested executable. Follow the
[client update guide](client-updates.md), then add a new record only after the
rebuilt stack reaches map mode and completes the required transition checks.

The private adapter's source catalog has since been migrated and passes x86
source checks. The SDK manifest and lockfile now pin adapter revision
`715f6d7`; the new runtime has not been installed or validated through a live
login. The plugin's policy and public API remain unchanged.

On 2026-11-25, the portable policy passed 26 tests; the x86 plugin passed its
unit test and two integration tests. All-target Clippy and the release
`autoLogin.dll` build also passed. These checks did not install the plugin or
exercise the updated client.

## File-only login reached map mode — 2026-11-18

The consumer rebuilt/copied the plugin and opened a new launcher using
`autoLogin.ini`, with all RO_* overrides removed. Read-only log inspection
identified process **23184** and the following sequence. Times use UTC−03:00.

| Time | Observation |
| --- | --- |
| 19:00:08.771 | Plugin startup requested |
| 19:00:26.361 | Owning-thread policy ready; dry_run=false |
| 19:00:28.389 | Login attempt 1/3; native SSO requested |
| 19:00:29.878 | Server selection dispatched |
| 19:00:30.950 | Existing-PIN confirmation requested |
| 19:00:31.964 | Character selection dispatched |
| **19:00:33.053** | **Map mode reached** |

The SDK startup log independently accepted the plugin callback for the same
process. No error was recorded in that run. The installed plugin matched the
consumer-built release DLL, with SHA-256:

```text
becf2cff1968831f019b031326ec2e184218ff7cc3797b0ea9db1a85057d03ee
```

The source selection statement comes from the consumer's launch setup. The
plugin deliberately does not log credential values or dump the environment.
This run validates launcher-to-map with file inputs, including server, existing
PIN and occupied-character selection, for the tested runtime/client profile.

## Environment-based startup — 2026-11-18

A previous process reached map mode at 18:07:41.259 with environment inputs,
and again at 18:09:01.124 after returning to character selection. The reason for
that return was not established; it does not validate a disconnect/retry cycle.

## Publication reorganization

The public root now owns portable policy, with separate phase orchestration,
dispatch, recovery and PIN implementation modules. Native compatibility code
moved to the private adapter without changing its retained carrier interface.
Client identities, addresses and profile evidence remain private.

The reorganized core passed 26 tests. Actual-DLL fixtures cover nine provider
scenarios, configuration priority, invalid overrides, safe errors, aliases,
missing SDK and logging. Formatting, Clippy and rustdoc passed. The plugin also
built from a detached copy containing only this public source and public
dependencies. Real SDK and Forwarder debug-loader fixtures passed in isolated
processes. These checks are separate from live evidence.

The live DLL hash above predates this source reorganization. A new build has its
own identity; passing deterministic tests does not create another live-client
observation.

## Limits

| Path | Evidence |
| --- | --- |
| File-only native login, PIN, character and map | Consumer launch plus matching-process logs |
| Environment-based login and map | Earlier matching-process logs |
| Source priority, retries, timeouts and PIN lifetime | Deterministic policy and isolated DLL checks |
| Full live disconnect/error recovery cycle | Still pending |
| PIN creation/change/expiration, incorrect PIN | Manual; no automatic acceptance claimed |
| Other client profiles | Require their own SDK/adapter and live acceptance |

Map observation confirms the reported mode at that time. It does not establish
connection duration or every gameplay action. Source checks do not start a
launcher, replace installed DLLs or collect account data.
