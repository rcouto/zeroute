# CI and publication

Maintainer review and Buildbot CI run in a private Gitea repository. GitHub is
the public source and contribution entry point; it has no configured CI or
automatic PR import. Contributors need no private access. Maintainers report
sanitized validation results on the public PR, merge accepted changes privately,
and publish the resulting commits to GitHub. A GitHub PR with no checks is not
CI approval. See [contribution guidance](../CONTRIBUTING.md).

## Build scope and caching

The agreed matrix uses one x86_64-linux builder for source hygiene/pre-commit,
host Rust build/tests and the ARM64 unsigned APK with Android lint and packaging
checks. Core integration adds upstream C++ compilation to the existing ARM64 job,
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
