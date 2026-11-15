# Architecture

The [adaptive connection UI](adaptive-ui.md) describes the implemented lifecycle,
shared status snapshot and Android presentation. The [state-machine review](state-machine.md)
retains the baseline analysis and wider edge-case inventory.
[Source-first publication](source-sharing.md) separates source sharing from
future APK distribution. These reviews do not change runtime behavior.

Rust owns settings, readiness, engine lifetime, UDP transport, persistence and
diagnostics. A small Java Activity handles Android UI and forwards calls across
JNI using the maintained jni crate. The upstream ZeroTier core owns discovery,
the wire protocol and cryptography through its C API. No decompiled app code is
used.

## Android VPN boundary

The Activity saves a validated 64-bit network ID in its no-backup directory.
Connecting requests Android VPN consent, then starts a foreground VpnService
with a notification and disconnect action. The service protects both UDP
transport sockets before the Rust worker starts. Its lifetime is independent
of the Activity; screen locking no longer intentionally stops the connection.
Always-on and automatic process-restart recovery remain disabled initially.

The core callback copies addresses, routes, DNS and MAC/MTU into an original
bounded text ABI. Rust validates these inputs, derives connected routes and
ignores default routes. Only routable managed DNS servers are installed.
Unmatched traffic and unconfigured address families retain their physical-network
path. The Java service only translates validated configuration into Builder calls.

Configuration revisions guard TUN attachment. Java retains its descriptor and
Rust duplicates it; either owner can close safely. Configuration changes discard
the old Rust tunnel and reset downstream readiness before rebuilding. Core calls
never hold the packet-plane lock, because they may synchronously invoke callbacks.

Rust forwards raw TUN IP packets through ZeroTier Ethernet frames. IPv4 uses ARP;
IPv6 uses Neighbor Discovery with solicited-node multicast subscriptions and
ICMPv6 checksums. Neighbor cache, pending resolution packets, outgoing frames and
incoming packets are bounded. Unresolved packets expire rather than accumulating.
The worker polls sockets, TUN and an eventfd; attachment and stop requests wake it.

DNS readiness requires a matching UDP reply from the selected managed resolver
through the same packet adapter. The probe validates endpoints, transaction,
question, response flags and checksum, retries when unavailable and expires stale
readiness. It is distinct from application-level peer reachability. Configuration
of a DNS server alone never marks DNS ready.

Direct phone acceptance must establish DNS/HTTPS/SSH traffic and background
lifetime. Network handovers, long idle/battery behavior and multiple-resolver
failover remain unverified. No release or production reliability is claimed.

## Ownership and persistence

A single Rust worker owns the core and its callbacks. A boxed callback context
outlives core destruction. Nonblocking UDP/TUN descriptors and an eventfd support bounded
polling and stop/join. JNI contains panics and reports generic failures without
logging identity material. Core callbacks contain panics and stop the engine.

Core state stays under the Android no-backup directory. Identity lookups use
canonical filenames because initial reads use object ID zero. Writes use private
files and atomic replacement; unreadable identity fails closed rather than
silently generating a replacement. Cloud backup and device transfer are disabled.
No identity export or recovery workflow exists yet.

Each connection attempt has a generation. Stale callbacks are ignored; loss of
an engine/configuration prerequisite clears downstream readiness. Local engine
execution and root connectivity are separate facts: a root outage alone does not
invalidate an established peer path. Peer reachability is not inferred from
network configuration or DNS readiness.

## Builds and licensing

`crates/core/` owns Rust and the small original C++ C-API adapter. `android/`
owns framework/UI code. Import-tree discovers flake-parts modules in `modules/`.
The hand-maintained flake entry point pins upstream sources through flake.lock.
There is no import from derivation.

Rust outputs use nixpkgs buildRustPackage and its Android cross stdenv. The
ZeroTier package uses the upstream core make target with controller, SSO and
nonfree extensions disabled. It installs the core archive, API headers and
MPL-2.0 license. The Java app exposes that bundled license. Application-owned
code remains MIT; dependency licenses remain separate.

Gradle uses standard nixpkgs dependency-cache hooks and a checked-in dependency
inventory. The legacy Ant Android helper does not cover this Gradle project.
Separate source filesets let UI edits reuse native outputs. Core, Rust, NDK or
relevant lock changes invalidate native artifacts. The existing ARM64 APK job
assembles an unsigned artifact; signing and device installation happen separately.

Host tests cover settings, readiness and a real Java-to-Rust JNI round trip.
They cannot establish Android transport or VPN acceptance. Direct ARM64 phone
testing is the selected runtime path; there is no emulator target.

Native linking includes the static C++ standard library, C++ ABI and unwind
archives. Only those archives are staged into a private build directory: adding
their NDK directory to the global library search would also select static libc.
Android libc stays shared. Undefined symbols fail the native link, and APK
inspection requires the shared libc dependency. These checks complement, rather
than replace, loading the library on a phone.

The foreground service follows the Android
[VPN lifecycle](https://developer.android.com/develop/connectivity/vpn) and uses
the [systemExempted foreground type](https://developer.android.com/develop/background-work/services/fgs/service-types#system-exempted)
for a user-prepared VPN. System consent is never automated.

AGP 8.13's ForegroundServicePermission lint check recognizes alarm permissions
for systemExempted but omits the documented prepared-VPN eligibility. A service-
scoped manifest suppression covers that false positive; no alarm permission is
requested. The service checks VpnService.prepare() before startForeground, and
Android itself enforces eligibility at runtime. Other lint checks remain fatal.

Disconnect, revocation and failures release Java/native tunnel descriptors and
stop the engine explicitly before stopSelf. Cleanup is idempotent and serialized
on the service worker; onDestroy is only a fallback because Android can retain
a bound VPN service after stopSelf. The UI exposes numeric tunnel/frame/drop
counters for development diagnosis, never packet bodies or endpoint addresses.
A successful internal resolver probe does not establish Android application
traffic: that probe bypasses the TUN descriptor, so browser/SSH acceptance remains
an independent gate.

### Android IPv6 DNS compatibility

Phone diagnostics found direct AAAA queries and unfiltered/IPv6-only libc
lookups working, while AI_ADDRCONFIG returned no addresses. Its UDP route check
to 2000:: failed with ENETUNREACH; Android Private DNS was off. The
[AOSP resolver](https://android.googlesource.com/platform/packages/modules/DnsResolver/+/refs/heads/main/getaddrinfo.cpp)
uses that route check to decide whether to request IPv6 answers.

When IPv6 is assigned, the Android configuration adds only 2000::/128 if no
managed route already covers it. This enables the resolver's unsent connect
probe. It does not add an exit gateway or forwarding route: actual packets to
that address remain unrouted by the adapter. Default routes remain ignored.
Tests cover this isolation and preservation of controller-owned routes. Revisit
this compatibility rule if Android changes its probe. The phone resolver regression now passes: AI_ADDRCONFIG returns IPv6 and
the route probe succeeds. The operator confirmed private HTTPS loads in Firefox. The internal SOA
probe alone cannot establish application DNS; Android resolver and browser
checks remain separate acceptance evidence.

### Adaptive Android control

One adaptive native screen provides a primary connection action and optional high
contrast. Rust owns command generations and lifecycle/presentation semantics; JNI
API 5 provides one snapshot to the Activity, tile and foreground notification.
Explicit teardown acknowledgement gates Disconnected and subsequent starts.
DNS is optional; configured resolver health and individual peer/application
reachability remain separate. Non-VPN Android network callbacks drive underlay
notices and fresh resolver evidence on network changes. The tile refreshes while
listening without another polling timer. See [adaptive UI](adaptive-ui.md) for
behavior, evidence boundaries and pending device acceptance.
