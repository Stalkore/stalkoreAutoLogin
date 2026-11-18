# Recovery and PIN

[Documentation](README.md) · [Configuration](configuration.md) · [Validation](validation.md)

## X, Y and T

X is the wait before acknowledging a supported disconnect/connection notice.
T starts after a failed attempt or acknowledgment and delays the next login.
For X=5 and T=5, the notice closes after five seconds and another five seconds
pass before a new submission. The UI thread must be able to process messages.

Y includes the first submission. With Y=3, a failed third request exhausts the
cycle. Repeated ticks and persistent notices do not spend extra attempts.
Reaching map mode resets the cycle budget; there is no process-wide cumulative
cap. Configuration validation failures do not submit login or spend attempts.

## Notices and stalled connections

Recognition uses native window identity and connection state, independently
of localized text. Ordinary map notices stay open. Generic map errors qualify
only when the native connection has closed; login errors require an active
connection attempt. Other confirmation windows remain manual.

The timeout follows actual stage progress. An outstanding asynchronous native
SSO request must finish before reset/resubmission; the plugin cannot cancel it
by clearing a flag. Error acknowledgment remains allowed so the native completion
handler can finish. An acknowledgment can retire windows/modes, so policy returns
and waits for a new snapshot.

## Existing PIN only

A selected PIN must contain exactly four ASCII digits. Leading zeroes survive
file/environment selection and logical keypad dispatch. Randomized positions
are handled by the native adapter.

There is one automatic PIN attempt until map entry, including partial failure.
An ordinary login failure or reopened prompt does not renew it. Observed map
entry allows another automatic PIN for a later reconnection. Missing/empty PIN
leaves manual input available. Creation, change, expiration, reset, partially
filled and unsupported PIN states remain manual.

## Status exports

`stalkore_autologin_status_v1` returns the plugin lifecycle state.
`kore_login_status_v1` is its compatibility alias.

| Value | Meaning |
| --- | --- |
| 0 | Idle |
| 1 | Subscription queued |
| 2 | SDK dispatcher starting |
| 3 | Policy ready on the owning UI thread |
| 4 | Initialization/startup/dispatcher failure |
| 5 | Dispatcher stopped |

Status 3 does not prove authentication; map mode is recorded separately.
See [validation](validation.md) for what was observed in a real client.
