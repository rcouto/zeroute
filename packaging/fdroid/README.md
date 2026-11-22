# F-Droid flake adapter

Candidate preparation only. The metadata remains disabled and has no release
commit; it has not been submitted or built by F-Droid. The existing Nix `apk`
package owns all Rust/C++/Gradle compilation, dependencies and packaging checks.
The shell adapter exists outside Nix only because F-Droid may need to bootstrap
Nix before it can invoke that flake. It is not a second Android build pipeline.

## Files and use

- `org.rcouto.zeroute.yml`: candidate fdroiddata metadata. Replace the disabled
  placeholder with an approved public release commit/tag only after validation,
  versioning, complete notices, signing and inclusion review.
- `build.sh`: bounded invocation of `.#apk`, with explicit planning/build modes.
  The helper neither signs nor installs an APK, changes versions, writes a
  lockfile, creates releases, or contacts private services.

With Nix already installed, inspect the executor's current closure first:

```sh
./packaging/fdroid/build.sh --dry-run
```

Only after agreeing to that executor's downloads/build cost:

```sh
./packaging/fdroid/build.sh --build
```

The result is `result/zeroute-arm64-unsigned.apk`. Nix retains the existing real
backend, Android lint, packaging and native-symbol checks. No F-Droid `gradle` or
`ndk` field independently selects another compiler/toolchain; the flake and its
lock remain authoritative. The metadata's `License: MPL-2.0` is a candidate
binary-license representation for the combined package; application-owned source
remains MIT. Complete the dependency/notice review before enabling it.

## Bootstrap boundary

On a prepared disposable F-Droid builder with no Nix in PATH, the metadata invokes
`build.sh --bootstrap --build`. This is intentionally not the default. Its `sudo`
setup is for the standard disposable `vagrant` build user only; never run that
setup on an ordinary workstation or assume it is a host deployment procedure.
It creates `/nix` only when absent, without changing ownership of an existing
store. The adapter requires an unprivileged user, a writable prepared `/nix`, and
no pre-existing Nix profile before bootstrapping.

The official bootstrap installer URL and SHA-256 are pinned in the helper. It
checks that hash before executing the installer, installs without a daemon,
channel or shell-profile modification, then loads its own process's Nix profile.
Downloaded bootstrap material uses the build user's cache directory; an operator
can set `ZEROUTE_FDROID_BOOTSTRAP_DIR` to a task-specific private artifact directory.
The temporary installer directory is cleaned on exit; the actual Nix installation
and store are not removed. An existing Nix binary is used without installation.
Its actual version is printed for evidence.

Bootstrap is refused in dry-run mode if Nix is missing: a Nix dry-run cannot price
the prerequisite installer. The inspected x86_64 Linux bootstrap archive declares
27,377,140 bytes (about 26.1 MiB); this excludes unpacked store dependencies and
the Android build closure. Recheck its resource requirements before executing.
The bootstrap itself and disposable-builder namespace/user behavior remain
untested; syntax checks and a downloaded installer hash are narrower evidence.

## Cache and source evidence

The wrapper uses only `https://cache.nixos.org`, declines flake configuration and
does not add a private cache or credentials. It keeps sandboxing enabled, fixes
maximum jobs/cores/substitution concurrency at two, and refuses lock updates.
No substitutions are globally disabled: rebuilding all bootstrap tools would be
an unmeasured and potentially large cost.

For a source-build trial, record which application/native outputs actually
compile and which are reused or substituted. A warm cache hit is not fresh
compiler evidence. Source/binary scanning and source archive inclusion remain
F-Droid validation requirements; the wrapper does not ignore scanners or claim a
successful F-Droid build just because ordinary Nix builds work. Nix-on-NixOS
execution also does not prove a fresh F-Droid builder can bootstrap and run it.

No SDK, emulator, clean build or new CI matrix is installed or enabled merely by
adding these files. Packaging files are outside the existing Rust/APK source
filesets, so this adapter does not change their derivations. Existing PR checks
still run normally; there is no added CI job.

See [the confirmed SimpleX precedent and recipe investigation](../../docs/f-droid-flake-recipe.md)
and [release/signing readiness](../../docs/f-droid-readiness.md). A differently
signed distribution release requires the separately accepted reinstall/setup
migration; the helper does not uninstall test apps or handle identity keys.

## Preparation evidence

On 2026-11-22, `--dry-run` completed on the preparation workstation using the
wrapper's official-cache-only policy: 48 derivation actions and 518 substitute
paths, with 2.0 GiB cache downloads and 3.9 GiB unpacked substitutes. Actions
include fetches/wrappers as well as compilation; these are not 48 source package
compiles. Build scratch space, generated outputs and possible fixed-output
fetches are additional. These numbers are executor/cache specific, not a fresh
F-Droid VM or selected remote-builder measurement.

The bootstrap archive size above was inspected by HEAD only. Its small installer
script was downloaded to private preparation storage and hash-pinned; the
installer was not executed. Shell syntax, ShellCheck and YAML parsing passed;
full fdroiddata lint, source/binary scanners, disposable bootstrap and app build
remain unproven. No app compilation or phone operation was run during preparation.

The selected remote builder's matching official-cache dry-run subsequently
completed with no additional actions reported for the existing APK. This is warm
package availability, not fresh F-Droid bootstrap or source-compilation evidence.
Do not replace the preparation workstation's larger estimate with a zero-cost
claim for a disposable builder.

## Disposable bootstrap trial

A subsequent trial used Debian bookworm slim with an empty `/nix`, no Nix profile
and no mounted host store/cache. Nix 2.35.6 bootstrap passed with sandboxing enabled
and fallback forbidden. The actual cold plan was 63 derivation actions and 739
substitutes: 3.1 GiB downloaded and 6.0 GiB unpacked, plus source/SDK fetches and
build space. Container namespace/mount preparation was required; this was not the
standard F-Droid builder image or a complete F-Droid scanner trial.

The build failed on a changed Google Play SDK advisory snapshot recorded in the
Gradle inventory. This exposed a reproducibility defect that warm caches hid.
The inventory now excludes that feed and two mutable Maven version indexes;
normal offline lint remains enabled. See [inventory refresh guidance](../../docs/ci.md#keep-lint-advisory-feeds-out-of-dependency-locking).
The corrected APK trial result remains pending; bootstrap success alone does not
establish APK or F-Droid acceptance.
