# Configuration

[Documentation](README.md) · [Quick start](getting-started.md) · [Recovery and PIN](recovery.md)

## File location and startup snapshot

The plugin reads `autoLogin.ini` beside the loaded `autoLogin.dll`, normally
`SDK/autoLogin.ini`. Its working directory does not affect that lookup.
Copy [autoLogin.example.ini](../sdk-plugin/autoLogin.example.ini) to create a
local file. Existing files are preserved during the documented install.

The file is read once during startup. Environment overrides are selected when
the SDK announces dispatcher readiness. Restart the game after changing either.
A missing file keeps automation defaults and allows an environment-only launch.

## Login fields and environment priority

```ini
[login]
username =
password =
server = 0
character = 0
pin =
```

Fill username/password locally. Server and character are numeric, zero-based
indices; a numeric character range does not imply the client has that many slots.

| INI key in [login] | Environment override | Validation |
| --- | --- | --- |
| `username` | `RO_USERNAME` | Required; fits the account-name capacity reported by the SDK, including NUL |
| `password` | `RO_PASSWORD` | Required; fewer than 64 UTF-8 bytes, without NUL |
| `server` | `RO_SERVER` | Required; 0–99 and present in the client's returned server list |
| `character` | `RO_CHAR` | Required; 0–255 and an occupied, non-deleting slot |
| `pin` | `RO_PIN` | Optional; four ASCII digits, with leading zeroes retained |

Priority is evaluated independently for every field:

1. A defined environment variable supplies that field.
2. Otherwise its value comes from `[login]`.
3. Missing or invalid required data rejects initialization before login.

An empty required environment value overrides the file and fails validation.
An invalid environment index/PIN also fails; it never falls back to valid file
data. An empty selected PIN disables automatic PIN entry and leaves manual input.

For example, `RO_CHAR=1` overrides only the character slot. The account,
password, server and PIN can still come from the file. To use only the file
from a shell that already has overrides, clear them explicitly:

```powershell
Remove-Item Env:RO_USERNAME, Env:RO_PASSWORD, Env:RO_SERVER, Env:RO_CHAR, Env:RO_PIN -ErrorAction SilentlyContinue
```

Environment values are inherited by a new launcher/game process. An already
running launcher does not inherit subsequent changes made in another shell.

## Values, punctuation and comments

Section/key names are case-sensitive. Use the lower-case names shown above.
Login values are literal text after the first `=`; `#`, `;`, `:`, backslashes
and additional `=` characters are preserved. Put comments on separate lines.
Matching outer single/double quotes are removed; use them to preserve leading
or trailing spaces. Quoted contents are literal, with no escape expansion or
environment interpolation. Unquoted surrounding whitespace is trimmed.

The file uses UTF-8; a UTF-8 BOM is accepted. Keep one login key per line.
The parser's syntax errors and validation diagnostics do not echo the contents.

The local INI contains plaintext login data. Keep it outside source control;
the ignore rules exclude every `autoLogin.ini`. Only the empty example belongs
in the repository. Do not attach a filled file to issues or logs.

## Automation options

Keep these settings before `[login]`, as in the example. Their existing names,
defaults and numeric limits are retained.

| Key | Default | Behavior |
| --- | --- | --- |
| `dry_run` | `false` | `true` suppresses credentials, PIN/character entry and recovery acknowledgment/reset |
| `tick_milliseconds` | `50` | Clamped to 10–1000; invalid text uses the default |
| `autologin_dialog_enabled` | `true` | `true`/`false` or `1`/`0`; allow supported notice acknowledgment |
| `autologin_dialog_close_seconds` | `5` | X: 0–3600 seconds before acknowledging a supported notice |
| `autologin_max_attempts` | `3` | Y: 1–100 submissions per recovery cycle, including the first |
| `autologin_retry_delay_seconds` | `5` | T: 1–3600 seconds after failure/acknowledgment before another submission |
| `autologin_timeout_seconds` | `45` | 1–3600 seconds without connection-stage progress before native recovery |

Existing automation `#` inline comments remain supported. Recovery values outside
their supported range reject startup. These settings have no new environment
aliases; the RO_* overrides apply to the five login fields only.

See [recovery and PIN](recovery.md) for state transitions and manual cases.
