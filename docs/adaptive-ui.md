# Adaptive connection UI

Status: implemented on the adaptive UI review branch. This describes source
behavior; acceptance on physical devices and across Android versions is pending.
The earlier [state-machine review](state-machine.md) remains the design inventory.

## Presentation and compatibility

A single native Android layout is used for every device: scrollable content,
system-scaled text, a centered readable column on wide windows, system/IME insets,
and at least 48dp actions. It does not select behavior by manufacturer/model.
Light/dark resources follow Android. Optional high contrast is stored locally as
an Android presentation preference; changing it recreates only the Activity.
Details, saved state and editable text retain normal Android restoration behavior.
This is an adaptive single-column implementation; the mock's optional two-column
wide-window arrangement is not implemented. Android API 26 remains the minimum;
existing ARM64 packaging remains the only built ABI. No universal device
compatibility or untested accessibility acceptance is claimed.

The primary action connects, cancels a pending start, disconnects, or retries
according to the Rust session. Setup is shown when no saved network exists.
Authorization denial exposes the public node ID and copy action directly.
Network settings, root connectivity and license details are collapsible. No
animated spinner or continuously changing packet counters occupy the main screen.

## Ownership and lifecycle

`session.rs` owns the command token, consent/start/run/stop/failure transitions,
underlay facts and presentation policy. Tokens scope Android permission results
and service intents; stale results and duplicate starts are rejected. Settings
saves share a lock with new session requests and are rejected while a session is
pending or active. Native engine attempt/configuration guards remain in place.

A single private consent Activity is used by both app and tile. Notification
permission denial does not cancel VPN startup. Revocation stops the session and
retains a failure explanation; the app does not automatically take back control
from another VPN. Initial native startup failures retain their original cause. Android foreground
startup rejection is caught and reported without taking down the UI process.

Disconnecting remains visible while the Java descriptor closes and the native
worker stops/joins. Only successful cleanup acknowledges Idle; failure leaves a
visible incomplete-shutdown state. Retry cannot start a new session before that
acknowledgement. A bound service can initialize a subsequent session without
reusing the old tunnel/monitor. Process restart and always-on remain unsupported.
The implementation does not add an identity-reset or recovery mechanism.

JNI API version 5 adds a bounded snapshot containing a numeric lifecycle/status
header, public node ID and public failure explanation. The snapshot acquires the
session and engine observations in one read operation. Settings, private identity
material, packet contents and endpoint addresses are excluded. All visible Android
surfaces consume this snapshot and use the same title mapping. Main-thread status
notifications read the latest snapshot rather than replaying captured stale ones.

## DNS and network evidence

Managed DNS is optional. An attached IP-only network can be Connected with
“DNS: Not configured (optional)”. Configured DNS requires a current matching
resolver response; missing evidence is a warning distinct from the tunnel.
Internal probe evidence does not prove Android application DNS or every peer.
Default-route forwarding remains disabled.

One reference-counted ConnectivityManager observer serves the visible Activity
and running VPN. It is released when neither needs it. It observes non-VPN
networks, capabilities and link changes without Wi-Fi/location permissions.
The screen labels **available connections**, because protected wildcard sockets
are not explicitly bound to one Android Network. It does not claim which of
several available transports is currently carrying each peer's traffic.

Network/link revisions wake the existing worker and invalidate the old resolver
transaction and readiness evidence. Reconnecting clears only after the worker
has processed the current revision and, when configured, obtained fresh DNS
proof. IP-only operation does not invent an equivalent path-health probe.
Sockets are not forcibly recreated or rebound by this UI change; actual roaming
recovery remains a device acceptance requirement.

Captive-portal labels require Android's positive CAPTIVE_PORTAL capability. Lack
of VALIDATED is only an unconfirmed-Internet notice, not a portal diagnosis or
an automatic private-transport failure. Available cellular connectivity can
coexist with captive Wi-Fi. The action opens Android Wi-Fi settings; it does not
use a hidden captive-portal launch API or automate sign-in. Metered/roaming
restriction policy and explicit underlay selection remain deferred.

## Tile, notification and resource use

The tile subscribes while listening and rereads current state on the next bind;
its active state means a pending/running/stopping session, including degraded
operation. A tap sends the displayed explicit Connect or Disconnect intent; an
intervening backend failure cannot turn a Disconnect tap into a new connection. It does not add a polling timer. Locked-device use still requires
unlocking; adding a tile and VPN consent remain user-controlled Android prompts.
The ongoing notification uses the same title and a token-scoped Stop action.
Updates use NotificationManager without re-entering foreground eligibility after
revocation. The existing 500ms service monitor publishes changes; Activity's old
1-second polling loop is removed. Battery consumption remains unmeasured.

## Validation boundary

Host regression tests cover optional DNS, stale/cancelled consent, duplicate
requests, shutdown acknowledgement, failure retention, and old DNS responses
after network invalidation. The JVM/JNI smoke test additionally exercises these
command/settings races across the real native boundary. Android Rust code is
checked with the integrated-core configuration enabled. APK compilation/lint and
packaging evidence must accompany the final branch; host checks cannot replace it.

Before phone acceptance, exercise Connect/Cancel/Disconnect from Activity, tile
and notification, consent cancellation and rotation, repeated toggles, saved and
unsaved settings, high contrast/night mode, 200% text, small/large windows, and
Wi-Fi/mobile/wired transitions. Include controller-denied admission, IP-only DNS,
configured DNS failure, competing VPN/revocation and background lifetime. No new
ABI, emulator, signing/release pipeline, dependency or CI job is introduced.
