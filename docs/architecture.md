# Architecture

Rust owns application policy and state: lifecycle decisions, readiness, packet
buffering, timers, transport, persistence, and diagnostics. The upstream ZeroTier
engine will own the wire protocol and cryptography through its C API.

A small Java layer handles Android framework entry points, permissions,
notifications and callbacks, forwarding events and file descriptors across JNI.
The initial NativeBridge ABI handshake only proves packaging; no VPN service or
network permissions are requested by the scaffold.

`crates/core/` is the Rust boundary. `android/` contains the framework/UI shell.
`modules/` contains import-tree-discovered flake-parts modules. `flake.nix` is a
small hand-maintained input/composition entry point, not a generated file.

Host and Android native outputs use standard nixpkgs Rust build helpers. Android
uses nixpkgs' prebuilt NDK cross stdenv and the pinned Rust toolchain's Android
standard library. APK assembly uses the Gradle setup and dependency-cache hooks.
The Android `buildApp` helper in nixpkgs is Ant-based and does not cover this
modern Gradle project; the APK derivation only supplies project-specific staging,
installation and packaging checks around the normal Gradle hooks.

Source filesets separate Rust from Android sources. Java/UI edits should reuse
the native output; Rust, compiler, NDK or relevant lock changes invalidate it.
Dependency metadata is checked in as ordinary JSON. Evaluation must not build
outputs or read build results: avoid import from derivation.

Future implementation must distinguish engine ONLINE, network configuration,
established tunnel, DNS readiness and individual peer reachability. Preserve
identity safely and test buffering and network transitions on the intended
phones. Do not copy the official application's decompiled code into this project.

App data is excluded from Android cloud backup and device transfers. Older
Android versions use allowBackup=false and legacy backup rules; Android 12+ has extraction
rules. Future identity storage must use Android's no-backup directory and have
its own reviewed export/recovery design.

The bootstrap uses nixpkgs' default Gradle 8 and a compatible AGP 8 release.
Android lint keeps AGP upgrade notifications visible as information: toolchain
upgrades are reviewed together with the Nix pin rather than enforced as source
correctness errors. Other lint warnings remain fatal. See the official
[lint API](https://developer.android.com/reference/tools/gradle-api/8.13/com/android/build/api/dsl/Lint)
and [Android backup rules](https://developer.android.com/identity/data/autobackup).
