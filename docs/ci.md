# CI and publication

Gitea is the private working repository. A minimal flake and license establish
main; the application and development scaffold are reviewed on feature/bootstrap.

Buildbot discovery uses the `buildbot-rcouto` repository topic. The `buildbot`
account has admin collaborator access, matching the existing repository setup
so it can manage webhooks. Discovery/webhook setup and actual remote CI cannot
be proved until code is pushed and Buildbot scans the repository.

The agreed initial matrix is one x86_64-linux builder running:

- pre-commit formatting/lint and source hygiene;
- native Rust build/test hooks;
- ARM64 Rust library cross-compilation, unsigned APK assembly and Android lint;
- an APK archive check confirming that the ARM64 native library is packaged.

There are no emulator, signing, deployment or publishing jobs. Bootstrap Rust
checks currently prove compilation/lint; they do not constitute VPN behavior or
on-device JNI acceptance tests. Device installation requires separate approval.

Remote CI timings are unmeasured. SDK/NDK/JDK/Rust downloads may dominate a cold
run. Nix caches toolchains, pinned Gradle artifacts, and separate native/APK
outputs. UI edits should reuse the native output; toolchain/core changes
invalidate native artifacts. Dependency inventory changes require an explicit
refresh; ordinary sandboxed builds must not resolve unpinned dependencies.

Public sharing will use a separately configured Gitea-to-GitHub push mirror.
No GitHub repository, mirror, token or release publication has been created.
Before enabling mirroring, review the complete history for secrets and personal
information. Keep the stable release signing key in private backed-up storage;
signing happens outside the Nix store. Protect release credentials from PR jobs.
No signing credentials are required for the unsigned CI artifact.

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

No APK was signed or installed. VPN functionality and on-device JNI execution
remain untested. Remote CI results are reported on the bootstrap pull request.

The development shell was smoke-tested with Rust/Cargo 1.100.0, JDK 21 and
Gradle 8.14.6. It installed normal pre-commit and pre-push hooks. Final local
validation uses `nix flake check path:.` with import-from-derivation disabled;
no Git commit or push is part of that validation.
