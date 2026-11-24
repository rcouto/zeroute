# CI and publication

Maintainer review and Buildbot CI run in a private Gitea repository. GitHub is
the public source and contribution entry point; it has no configured CI or
automatic PR import. Contributors need no private access. Maintainers report
sanitized validation results on the public PR, merge accepted changes privately,
and publish the resulting commits to GitHub. A GitHub PR with no checks is not
CI approval. See [contribution guidance](../CONTRIBUTING.md).

## Scheduled flake updates

Buildbot schedules a daily flake update at 14:30 in the controller time zone.
The shared publisher opens a private Gitea PR when the lockfile changes. The
existing CI matrix applies to that PR. GitHub publication remains a separate step.

## Build scope and caching

The agreed matrix uses one x86_64-linux builder for source hygiene/pre-commit,
host Rust build/tests and the ARM64 unsigned APK with Android lint and packaging
checks, plus one F-Droid metadata/source/APK scanner check. The scanner consumes
that same APK rather than building another one. Core integration adds upstream C++ compilation to the existing ARM64 job,
as approved by the owner. There are no extra ABIs, emulator jobs, signing jobs,
deployment jobs or publishing jobs.

Nix caches the pinned SDK/NDK/JDK/Rust toolchains, Gradle dependency inventory,
upstream core archive, Rust native output and APK separately. UI-only edits reuse
the native output. Core, Rust, NDK and relevant lock changes invalidate it. Cold
core-integration and full CI timings remain unmeasured; ordinary builds do not
resolve unpinned dependencies. Refresh the dependency inventory explicitly when
changing Gradle dependencies.

Public source is available at [rcouto/zeroute](https://github.com/rcouto/zeroute).
Updates currently require an explicit maintainer publication; continuous
synchronization remains inactive. Public APK distribution is deferred. Signing
material stays outside Git and the Nix store, and unsigned CI artifacts require
no signing credentials. Never expose credentials to unreviewed contributor code.
See [source-first publication](source-sharing.md) for the current boundary.

## Implementation validation

Nineteen host Rust tests cover settings persistence, stale readiness events,
configuration validation, private-route policy, bounded packet queues, ARP,
an IPv6 neighbor/data exchange, and DNS matching/readiness expiry. Host Clippy,
Android Rust typechecking and the versioned Java/JNI smoke test pass. The full
ARM64 native library, APK assembly, Android lint and packaging checks passed on
the selected remote builder. Linking rejects unresolved symbols; archive checks
require AArch64,
the JNI entry point and shared Android libc. The scoped prepared-VPN lint
exception is explained in [architecture](architecture.md).

An early phone launch exposed missing C++ ABI symbols and accidental static-libc
selection. The corrected build stages only the required C++ runtime archives.
Phone feedback then exposed Disconnect waiting for service destruction; explicit
teardown before stopSelf fixed that path and the operator confirmed it works.

DNS diagnosis distinguished the working tunnel/resolver from Android lookups:
direct AAAA and unfiltered/forced-IPv6 libc lookups succeeded, while AI_ADDRCONFIG
returned no addresses and its global IPv6 route probe failed. Android Private DNS
was off. After the Android-only probe host route was added, the route check and
AI_ADDRCONFIG succeeded, and the operator confirmed private HTTPS loads in
Firefox. Default-route forwarding remains disabled.

Basic IPv6 ping, Android DNS, private HTTPS and Disconnect have phone evidence.
Network handovers, cold-start timing, long-idle/battery behavior and process-death
recovery remain unverified. Test APKs were signed with a development key outside
Git/Nix; no release key was created. Build checks alone do not establish VPN
reliability.

## Remote build measurements

With toolchains and the core archive cached, one bring-up build reported 2m 28s
for APK assembly and 34s for lint. The final DNS compatibility build reported
1m 10s for assembly and 29s for lint. These are targeted remote builds, not fresh CI or clean-machine benchmarks.

## Bootstrap measurements

Local x86_64-linux measurements on 2026-11-13:

- Initial ARM64 Rust build: 237 seconds including cold NDK/toolchain preparation.
  Nix reported about 1.0 GiB downloaded and 2.9 GiB unpacked for that closure.
- With toolchains cached, dependency recording plus APK assembly/lint completed
  in 79 seconds. The checked-in inventory pins the resolved artifact hashes.
- First sandboxed APK build using that inventory: 145 seconds, including roughly
  100 MiB of cached dependency substitutions plus uncached lint artifacts.
  APK assembly took 44 seconds; the separate Android lint phase took 9 seconds.
- The APK packaging check confirmed AArch64 machine type and the exported JNI
  handshake symbol in the packaged native library.

These are workstation measurements, not remote CI benchmarks or clean-machine
end-to-end timings. Earlier cold setup suffered intermittent cache DNS timeouts
and slow large transfers; interrupted diagnostic attempts are not included in
the successful-run timings above. Local-only builder and HTTP/1 overrides were
used for bootstrap diagnostics; no persistent Nix settings were changed.

The sandboxed pre-commit check receives the same lock-file-derived Cargo vendor
closure as the host Rust package through git-hooks.nix
`settings.rust.check.cargoDeps`. Offline Clippy must not depend on a developer
Cargo cache. This reuses existing dependencies and adds no job or ABI.

## Adaptive UI validation

The adaptive UI uses the existing host and ARM64 jobs and dependencies. Twenty-five
host tests include new lifecycle/optional-DNS regression cases, with stale DNS
response coverage extended in the packet test. JNI API 5 smoke checks exercise
consent cancellation, duplicate start rejection, settings serialization and
teardown acknowledgement. The source changes require the existing Rust native
output and APK to rebuild; the upstream core/toolchain definitions are unchanged.
Final APK/lint evidence and CI status are reported on the implementation PR.
Phone evidence above applies to the prior app; adaptive-layout, lifecycle and
network-change acceptance on physical devices remains pending. See
[adaptive UI](adaptive-ui.md).

## Keep lint advisory feeds out of dependency locking

The dependency inventory must contain versioned build artifacts, not live lint
advisory/version feeds. A disposable Debian build with an empty Nix store found
that Google's `play-sdk/index/snapshot.gz` had changed since recording, causing
a fixed-output hash mismatch hidden by warm-store builds. The Maven
`master-index.xml` and `com/android/application/group-index.xml` endpoints have
the same mutable-URL risk; they are not application dependencies.

The standard pinned nixpkgs Gradle hook runs normal builds with `--offline`.
AGP passes that mode to lint, which avoids remote advisory connections and uses
its bundled SDK index. Android lint remains enabled with warnings as errors.
Live advisory freshness is not claimed by this reproducible build.

After `nix run .#update-gradle-deps`, review the inventory and remove these
recording-only feeds if lint requested them during the online refresh:

- `https://dl.google.com/play-sdk/index/snapshot.gz`
- `https://maven.google.com/master-index.xml`
- `https://maven.google.com/com/android/application/group-index.xml`

Do not merely refresh their hashes: the next upstream update would break a cold
build again. Retain all versioned Maven artifacts needed by assembly and lint,
and validate the resulting inventory with the ordinary sandboxed APK build.

The corrected inventory passed in the isolated Debian trial: ZeroTier core
compiled in 51 seconds, Rust in 33 seconds, APK assembly in 1m 28s and lint in
24 seconds. Packaging checks verified AArch64, shared Android libc and the JNI
entry point. The corrected resumed segment took 25m 17s including remaining
cold downloads. The earlier segments failed or were interrupted before the fix;
this is not an uninterrupted successful cold-build benchmark. No host store or
private binary cache was shared with the trial.

## F-Droid review check

`nix build .#checks.x86_64-linux.fdroid` runs the locked nixpkgs `fdroidserver`:
metadata lint, source scanning and unsigned ARM64 APK scanning. It is one added
Buildbot check. Reports are retained as `metadata.log`, `source.json`, `apk.log`
and `scope.txt` in the check output. A scanner error fails the check.

The actual disabled candidate is linted first. A scratch copy without `Builds`
then selects the official scanner's current-source mode, avoiding VCS cloning or
recipe execution. It scans the tracked project plus the exact cleaned ZeroTier
source used by the compiler. The native source filter removes the unused nonfree
controller, unused JavaScript rule compiler (which has no npm lockfile), and
bundled server `.a` files, without blanket scan ignores.
The local review repository declares the candidate's Connectivity category;
F-Droid's final category and Anti-Feature decisions remain separate.

An empty disposable home/cache and `refresh_scanner: false` select the packaged
SUSS rule snapshot. There is no live rule refresh in this sandboxed check.
Snapshot freshness, downloaded dependency/toolchain source, licensing obligations,
release references and F-Droid inclusion are not proven by a passing scan.
Recheck current scanner rules before submission. See
[the license inventory](third-party-licenses.md) for APK notice gaps.

The approved scanner closure plan was 70 MiB downloaded / 236.5 MiB unpacked
locally, all substitutions; remote cache savings were unknown. Initial native
source cleanup invalidates C++/Rust/APK outputs once: prior measured warm phases
totaled 3m16s, not a new end-to-end timing. Scanner time remains unmeasured.
Metadata-only edits reuse native/APK outputs; source, APK, tool/rule and lock
changes invalidate the relevant check. Garbage collection can remove caches.

On 2026-11-23, the full check passed on the selected remote builder for source
commit `fd04852`: official metadata lint, project/native source scan and unsigned
APK scan completed successfully. Clean source-scan JSON was `{}`; the earlier
unlocked rule-compiler manifest correctly failed the same check before cleanup.
The final native rebuild reported 46s for C++, 24.51s for Rust, 2m16s for APK
assembly and 27s for lint. These are phase timings with cached toolchains, not a
cold end-to-end benchmark. The complete scanner duration was not measured.
All pre-commit hooks passed. No phone installation, signing or submission occurred.

The first Buildbot run with this check passed in 13m06s: flake evaluation took
12m35s and the build-dispatch phase 29s. Native/APK outputs were already present
from direct validation; this was not a cold toolchain-download benchmark. The
next source-only PR run took 2m27s, and the merged-main run took 11s. Successful
scanner-builder runs took 8s and 11s including job overhead. These observations
do not establish a general evaluator-cache guarantee; avoid treating scanner
execution time as the total PR-CI latency. No IETS workaround or service change
was required for these runs.

## Generated notice-source review

`packages.notices` reuses the existing Cargo vendor closure, native source and
selected Rust/NDK distributions. Standard locked-nixpkgs cargo-about gathers
Android-target crate notices offline; a small assembler preserves native/runtime
notice material and records coverage. The existing `fdroid` check depends on it
and retains `notices.json`; there is no new job or APK/native-input change.

The approved cargo-about dry-run was 5.8 MiB downloaded / 14.4 MiB unpacked,
with six pinned missing-license fetches totaling about 36 KiB. Rust/NDK notice
files were rechecked as present on the selected builder before implementation.
Generator timing is not yet measured; seconds are expected. Prior scanner jobs
took 8–11s, while evaluator latency is separate and can take minutes. These are
cache-dependent observations, not a zero-download guarantee after garbage
collection. A Python-only collector was considered as a lower-download
alternative; the standard crate-license tool avoids maintaining that logic here.

The artifact rebuilds for affected source/runtime pins, generator/configuration
or exception changes. Native compilation remains cached when only notice inputs
change. The small provenance/coverage tests execute within this derivation, so
they add no separate CI job. Complete in-APK notice packaging remains separate.

The notice artifact subsequently built successfully on the selected builder,
including six attribution/provenance/coverage regression tests. It reports 26
Android-target crates (11 runtime, 15 build/macro), preserves 11 native/runtime
notice documents and identifies the three C++ archives selected by the compiler.
No APK/native application inputs changed.

Scanner SDK inputs now select only pinned build tools through supported nixpkgs
composition (`toolsVersion = null`, no platforms). The scanner needs dexdump,
not sdkmanager, platform images or legacy SDK tools. This reduces restoration
work without overriding/patching upstream packages or changing APK compilation.
After an interrupted full-check restoration, the measured remaining scanner
plan was 127.5 MiB downloaded / 182.8 MiB unpacked, plus a host compatibility
library build and SDK binary packaging. This is a remaining-cache-state plan,
not a total cold closure or the notice generator's own cost. After the owner
approved restoration, the full revised F-Droid check passed on 2026-11-24:
metadata, project/native source and unchanged unsigned ARM64 APK scanning, plus
the generated notice artifact and all six regression tests. The minimal SDK
input was sufficient; no application/native rebuild was needed. Final pre-commit
hooks passed. APK notice integration and release approval remain separate.
