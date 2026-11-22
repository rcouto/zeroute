# Flake-based F-Droid recipe investigation

Status: concrete candidate adapter, not a submitted or F-Droid-tested recipe.
Preparation adds no Nix bootstrap, APK compilation, VM, signing operation or
submission. The existing flake remains the single build implementation.

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

## Concrete candidate adapter

The [candidate metadata](../packaging/fdroid/org.rcouto.zeroute.yml) and
[thin shell adapter](../packaging/fdroid/build.sh) now implement the SimpleX shape
around the existing flake. See their [operator/readiness notes](../packaging/fdroid/README.md)
for the explicit modes and bootstrap boundary. The metadata is still disabled,
uses a release-commit placeholder and is not submitted.

With Nix present, `build.sh --dry-run` inspects the current executor's needs
without compiling. The separately approved `--build` mode invokes `.#apk` with
bounded resources, sandboxing, official-cache-only substitution and no lockfile
changes. An opt-in bootstrap for a prepared disposable builder verifies the
pinned official installer before execution; it is not run during preparation.

The candidate metadata represents the combined binary as MPL-2.0, while owned
source remains MIT; complete the dependency/notices review before enabling it.
Version code 1 describes the current build, not a decided release scheme. No
release tag, signing automation or auto-update mode is enabled.

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

Inspect the candidate wrapper's dry-run on the actual trial executor, review its
bootstrap and download needs, and obtain agreement for the native compilation
trial. Keep it ARM64-only on the selected remote builder and verify the actual
F-Droid interfaces/scanners. The bootstrap/download plan is not covered by a warm
Nix output or by the dry-run of an already-installed Nix.

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
Zeroute-specific F-Droid compatibility remains untested. Preparation checks shell
syntax/lint and YAML parsing; these are not full fdroiddata lint, scanner or build
proof, and make no claim of inclusion eligibility.
