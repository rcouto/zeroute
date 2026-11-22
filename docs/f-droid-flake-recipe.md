# Flake-based F-Droid recipe investigation

Status: candidate design, not a submitted or tested F-Droid recipe. No Nix install,
new build, VM, scanner run, key operation or submission was performed. The existing
flake remains the single intended build implementation.

## What the documented interface permits

F-Droid's quick-start guide permits custom toolchains through setup and build
commands. Its metadata reference supports `sudo` for privileged builder setup,
`build` for custom compilation, and `output` for locating the unsigned APK. It
does not require us to replace Nix with Gradle: Nix already invokes Gradle after
building/staging the native backend and notices.

## Confirmed upstream precedent: SimpleX Chat

F-Droid's canonical `chat.simplex.app.yml` contains current source-build entries
that prepare `/nix` and invoke SimpleX's Android build script. The inspected
7.1.4 ARM64 entry pins source commit
`62ea5dc4afb73a0256ba0088f0ae85be750afc99`.
That exact script installs hash-verified Nix 2.22.0 in single-user mode, enables
flakes with sandboxing, builds native libraries through `nix build`, then runs
Gradle for the Android package. This is a genuine Nix/flake build precedent,
not merely an application that uses Nix at runtime.

F-Droid's inclusion policy explicitly lists prebuilt FLOSS binaries from Nix
caches among sources that can be acceptable, while preferring Debian-packaged
options when available. Therefore a blanket claim that Nix/cache use is
unsupported is incorrect. A successful example does not approve every cache,
package or recipe; Zeroute still needs its own working packaging evidence.

The earlier indexed searches were incomplete: they missed Nix invoked indirectly
by an upstream script. Inspect canonical metadata and follow its actual scripts.
Nix-on-Droid's Gradle packaging is an unrelated comparison, not evidence against
the SimpleX precedent. An official 2024 F-Droid news post also describes SimpleX's
Nix builds and their substantial Haskell toolchain cost; that timing is not an
estimate for Zeroute.

Reuse the proven shape (approved bootstrap, writable store, flakes, source build,
explicit unsigned APK output), while keeping all Zeroute build steps inside its
existing flake. Do not copy SimpleX's lint relaxations, retry commands, multiple
ABIs, timeout or VM-specific ownership assumptions as requirements for Zeroute.

## Candidate metadata shape

The following is deliberately disabled and illustrative. `commit` and version
must be replaced with approved release values. No actual metadata file is enabled
in this repository or in fdroiddata.

```yaml
License: MIT
SourceCode: https://github.com/rcouto/zeroute
RepoType: git
Repo: https://github.com/rcouto/zeroute.git
Builds:
  - versionName: 0.1.0
    versionCode: 1
    disable: Draft only; Nix bootstrap, source scanning and substitution policy unverified
    commit: NOT_A_RELEASE
    output: result/zeroute-arm64-unsigned.apk
    build:
      - >-
        nix --extra-experimental-features 'nix-command flakes'
        build --no-update-lock-file --no-write-lock-file
        --max-jobs 2 --cores 2 --option max-substitution-jobs 2 .#apk
AutoUpdateMode: None
UpdateCheckMode: None
```

The MIT field describes application-owned code, not a blanket license for every
bundled component. Confirm the correct metadata/license representation with the
completed dependency inventory. Version code 1 here describes the current build,
not a decided release scheme. Automatic release detection is deliberately off
in this draft; future tags/version parsing require review.

A reviewed `sudo`/setup step must supply a suitable Nix version (for example, an
approved Debian package) and usable store/build-user configuration. No installer
URL or `curl | sh` is assumed. The command above alone does not establish the
accepted substitution policy, source inventory, bootstrap or network access.
Do not run it as an official F-Droid proof until those prerequisites are agreed.

## Specific questions to resolve before a trial

1. **Builder bootstrap:** will packagers accept Nix and its bootstrap provenance,
   minimum version, store permissions, daemon/build-user and sandbox setup in
   their standard builder? Do not disable sandboxing or install a privileged
   daemon silently. An ordinary NixOS build does not answer this question.
2. **Source visibility:** the inspected fdroidserver build flow scans the source
   directory and creates its source archive before executing custom `build`.
   Nix can fetch the pinned core/Cargo sources into its store rather than that
   directory. Determine how those exact sources will be made available for
   F-Droid inspection/archive review, without compiling during `prebuild` or
   hiding sources behind scanner exclusions. Do not add a submodule or duplicate
   upstream revision merely to satisfy a speculative recipe.
3. **Substitution policy:** resolve which toolchain binaries/caches F-Droid permits
   and which application/native libraries must be rebuilt. A downloaded final
   APK or cached Rust/ZeroTier library is not proof of their source compilation.
   Conversely, turning off all substitution can trigger enormous bootstrap/tool
   builds; do not choose that as an unmeasured default. Preserve trusted hashes
   and source pins; do not add a private cache or access credentials.
4. **Network/resources:** determine allowed fetching phases and hosts, store disk
   requirements and cleanup, and the exact cold download/build plan. Keep the
   selected trial ARM64-only and bounded; introduce no CI matrix change.
5. **Output/verification:** confirm the unsigned APK has the real core and bundled
   notices, correct version/code and ARM64 ABI, then run the applicable F-Droid
   source/binary/metadata checks. A successful normal Nix build alone is narrower
   evidence. Byte-for-byte reproducibility for upstream signatures is another
   separately costed test, not assumed here.

## Proposed next step

Adapt the confirmed SimpleX packaging shape into a concrete Zeroute recipe that
invokes the existing `apk` flake output; no separate native build pipeline is
needed on the present evidence. First inspect the bootstrap/download and native
rebuild plan and obtain the trial's resource agreement. Keep it ARM64-only on the
selected remote builder and verify the actual F-Droid interfaces/scanners.

A generic permission question about whether F-Droid permits Nix is no longer a
prerequisite. Ask packagers only about concrete unresolved Zeroute-specific
policy or builder constraints if the recipe investigation finds one. No upstream
message has been sent. If a specific incompatibility appears, retain the evidence
and discuss alternatives before changing build architecture.

## Sources and limits

- [F-Droid custom toolchain guidance](https://f-droid.org/docs/Submitting_to_F-Droid_Quick_Start_Guide/)
- [Build metadata: setup, custom build and APK output](https://f-droid.org/en/docs/Build_Metadata_Reference/)
- [Inclusion and permitted binary provenance](https://f-droid.org/docs/Inclusion_Policy/)
- [fdroidserver build/scanner ordering](https://gitlab.com/fdroid/fdroidserver/-/blob/master/fdroidserver/build.py)
- [Canonical SimpleX F-Droid recipe](https://gitlab.com/fdroid/fdroiddata/-/blob/master/metadata/chat.simplex.app.yml)
- [Exact SimpleX build script inspected](https://github.com/simplex-chat/simplex-chat/blob/62ea5dc4afb73a0256ba0088f0ae85be750afc99/scripts/android/build-android.sh)
- [F-Droid news confirming SimpleX Nix builds](https://f-droid.org/2024/02/08/twif.html)

These sources were checked on 2026-11-22. Nix/flake use has a confirmed precedent;
Zeroute-specific technical compatibility remains untested. The YAML is neither linted
as fdroiddata metadata nor executed, and makes no claim of inclusion eligibility.
