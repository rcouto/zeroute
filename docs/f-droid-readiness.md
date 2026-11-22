# F-Droid readiness and proposed submission plan

Assessment date: 2026-11-22. Status: preparation proposal, not a submission or a
release. No signing key, package ID, version, dependency, CI job or build target
is changed by this assessment. The source baseline is main `b1e6009`.

## Recommendation

Pursue inclusion in the main F-Droid repository after a source-build compatibility
trial. Retain Nix as the developer/CI build; prove a small F-Droid-compatible
Gradle/NDK/Rust source recipe before committing to a distribution pipeline.
Do not upload the current development-signed APK or claim the Nix APK is already
reproducible in F-Droid's environment.

## Current evidence and gaps

| Area | Observed | Remaining requirement |
| --- | --- | --- |
| Public source | GitHub main is published; owned code has MIT license | Publish each proposed release commit/tag and its complete source |
| Application identity | `org.rcouto.zeroute`, Android API 26+, ARM64 only | Keep identity stable unless a separate variant is deliberately chosen; describe architecture limits |
| Functionality | Owner reports successful use on two phones, app/tile actions and mobile recovery | State experimental limits; no reliability or battery claim from these checks |
| Release versions | Workspace `0.1.0`; Gradle hardcodes `versionCode = 1` | Define one authoritative, monotonic version code and a readable release version/tag scheme |
| Runtime Java libraries | App Gradle file declares no runtime Maven dependencies | Confirm final source/binary dependency inventory and scanner results |
| Native source | Pinned ZeroTier core plus project-owned Rust/JNI adapter | Build the real core from source in the distribution environment, not an imported prebuilt `.so` |
| Licensing | MIT application; separately MPL core; upstream nonfree/controller components disabled | Complete embedded third-party/runtime notices and corresponding-source information |
| Signing | Existing phones use a development certificate | Decide F-Droid signing or developer signing/reproducibility, and a deliberate test-install migration |
| Listing | README and public contribution route exist | Add descriptions, suitable icon, sanitized screenshots and release notes; no listing metadata currently exists |
| Submission | No F-Droid recipe or submission exists | Verify recipe, prepare metadata, then request separate submission approval |

F-Droid's inclusion policy normally asks for three months of active development,
with exceptions including proven users. Zeroute is younger. Describe the existing
use honestly and ask maintainers whether an exception applies; two owner devices
are not a guarantee of acceptance. The separate client implementation and its
connection/readiness UX should be explained rather than presented as rebranding.

## Build compatibility: concrete blockers

`modules/android.nix` stages `libzeroute_core.so` and the upstream MPL text before
Gradle runs. Those files are not tracked source. `MainActivity` references the
staged raw license resource, so plain Gradle is not currently a self-contained
source build. Building Rust without `ZEROUTE_CORE_DIR` also deliberately excludes
the real backend; successfully compiling that fallback would not prove a working
F-Droid VPN.

A candidate source recipe must obtain the same pinned ZeroTier source, build its
core-only static library, compile the Rust/JNI adapter for Android ARM64, stage
that shared library and notices, then run Gradle from `android/` with the release
version supplied. Preserve the current flags and boundaries:

- `ZT_EXTOSDEP=1`, `ZT_SSO_SUPPORTED=0`, `ZT_CONTROLLER=0`, `ZT_NONFREE=0`;
- Android API 26 minimum, 16 KiB native alignment and undefined-symbol rejection;
- shared Android libc, with only the required C++ runtime archives linked;
- no dependency on a private forge, cache, credential, phone identity or Nix store
  path when F-Droid fetches source and rebuilds the app.

The build currently pins AGP 8.13.2, compile/target SDK 36, build-tools 36.0.0,
Java 21 and NDK major 27 through the version catalog plus locked Nix inputs.
Rust selects `stable.latest` from the locked overlay, so it is repeatable for the
lock but not a literal version F-Droid can use without resolving that lock.
Record exact resolved Rust, NDK, Gradle and toolchain revisions for a release;
do not introduce a second drifting version owner. Prove the upstream NDK compiler
invocations and runtime-file queries outside Nix wrappers instead of assuming
cross-toolchain equivalence. Choose a reviewed source mechanism such as pinned
F-Droid source-library metadata; do not add a Git submodule or scanner exclusions
as an unreviewed shortcut.

Prefer shared small native build steps if the compatibility trial reveals a
necessary helper. Keep Nix on standard nixpkgs language helpers; do not replace
its functioning packaging pipeline merely to match a speculative F-Droid recipe.
No helper or recipe is implemented by this document.

## Licensing review boundary

The lock contains 29 registry package entries. The 26 cached manifests inspected
here declare MIT, Apache-2.0 alternatives, Unlicense alternatives and a
Unicode-3.0 conjunction. Manifests for `winapi-util`, `windows-link` and
`windows-sys` were not in the source cache; resolve their exact declarations and
target inclusion before calling the inventory complete. A manifest declaration
is evidence for review, not a replacement for collecting required notices.

The pinned ZeroTier revision is
`899352e38405968516bb12a770f0ac02f6058fa8`. Its upstream license distinguishes MPL
code, external components and nonfree components. The recipe requests only core
and excludes nonfree/controller objects. The current APK includes the MPL text
but not a demonstrated complete inventory of Rust, upstream embedded code,
Android/C++ runtime and redistributed assets. Complete that inventory and provide
the exact corresponding-source location before distributing executables.

Nix's `allowUnfree`/SDK-license acceptance is a build configuration choice, not by
itself proof of a proprietary application dependency. F-Droid has documented
exceptions for official Android SDK binaries; review each actual dependency and
tool source against its policy. Also describe root/discovery and controller
services, privacy behavior and self-hosting limits to reviewers; do not assume
that an Anti-Feature is either required or absent without checking the final app.

## Signing choices requiring an owner decision

| Choice | Benefit | Cost and migration |
| --- | --- | --- |
| F-Droid signs its source build | Lowest initial release-operations burden; no developer signing automation needed | Different certificate from current test installs; channels signed by different keys cannot be assumed interchangeable |
| Developer release key with reproducible-build verification | Potentially one signature across developer and F-Droid distribution | Secure key custody/backup plus demonstrably matching independent native/Gradle APK builds; F-Droid agreement required |
| Own F-Droid-compatible repository | Alternative if main-repository inclusion is unsuitable | Hosting, index signing, updates and key recovery become project responsibilities; not recommended as the initial scope |

Recommend F-Droid-managed signing for the first inclusion unless a common signing
identity across channels is a requirement. Do not promote the existing test key
to a public release key by accident. Reproducible upstream APK verification is a
separate higher-cost capability, not a prerequisite to proposing ordinary
F-Droid-signed source builds.

Neither signing choice alone preserves the current installations across a
certificate change. Android backup is disabled and no secure identity
export/import UI is implemented. Do not uninstall existing apps, clear data or
promise identity preservation. Plan an explicit migration (potentially new node
authorization) separately, or deliberately design an isolated application ID if
parallel installations are required. This assessment performs no key operation.

## Proposed sequence and cost agreement

1. Agree to one ARM64 source-build compatibility trial on the selected remote
   builder. Inspect tool/source download requirements before starting. Produce a
   real-backend APK, Android lint/packaging evidence and a measured build record.
   No new CI job or architecture is proposed for this trial.
2. After compatibility evidence, choose signing/channel identity and the existing
   test-device migration policy. Define authoritative version/code values and
   release tags; add complete notices and clean listing assets/metadata.
3. Prepare a concrete fdroiddata recipe and run the appropriate metadata/source
   scanning and build checks. Validate a phone installation only under separate
   device/signing authorization. Recheck current F-Droid policy and tool support.
4. With build and licensing evidence, seek approval for a first release tag and
   submission. Submit a reviewed metadata merge request or packaging request;
   maintainers decide inclusion. No promise of acceptance or review timing.

Assessment work is source/documentation inspection only. The trial's cold time
and downloads are unknown; expect native C++/Rust compilation and toolchain
fetches, potentially minutes to tens of minutes (an estimate, not a measurement).
The existing Nix APK timings are not measurements of this different environment.
Do not treat a cached Nix APK as proof that the new recipe is cheap or compatible.
Keep compilation bounded and use the already selected remote builder; no local
system/package build, emulator download or additional ABI is proposed.

Nix CI can keep reusing its locked native/toolchain outputs. A separate source
recipe will not automatically reuse those compiled outputs; source, upstream
revision, Rust, NDK and compiler changes invalidate the relevant native work.
Metadata-only edits leave native inputs unchanged. Reproducibility testing would
add an independent clean rebuild and APK comparison; agree on that cost before
adding it. The lower-cost alternative is to stop at this static assessment and
keep publishing source until distribution work is prioritized.

## Official references

- [Inclusion policy](https://f-droid.org/docs/Inclusion_Policy/)
- [Submission quick start](https://f-droid.org/en/docs/Submitting_to_F-Droid_Quick_Start_Guide/)
- [Developer FAQ: signing, versioning and reproducibility](https://f-droid.org/en/docs/FAQ_-_App_Developers/)
- [Build metadata and native source/build hooks](https://f-droid.org/en/docs/Build_Metadata_Reference/)
- [Reproducible builds and native toolchain caveats](https://f-droid.org/en/docs/Reproducible_Builds/)
- [Anti-Features](https://f-droid.org/en/docs/Anti-Features/)

The source/build findings above come from the named repository files. Candidate
recipes, signing recommendations, duration estimates and policy suitability are
proposals or inferences; no F-Droid build, scanner run or acceptance is claimed.
