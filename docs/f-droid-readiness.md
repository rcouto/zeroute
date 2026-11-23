# F-Droid readiness and proposed submission plan

Assessment date: 2026-11-22. Status: preparation proposal, not a submission or a
release. The initial assessment was source-only at `b1e6009`; the subsequent
Nix/Android compatibility trial passed at the `2ed929b` baseline after removing
mutable lint feeds. See the packaging README for measured evidence and
[the dependency/license review](third-party-licenses.md) for current notice gaps.
Signing, release and submission remain unapproved.

## Recommendation

Pursue inclusion using the existing Nix flake first. Nix supplies the locked
inputs and orchestrates compilation, including Gradle as the Android packaging
tool. Its declarative setup is not a defect, and there is no established
requirement here to maintain a second Gradle/NDK/Rust pipeline.

F-Droid supports custom build commands and an explicit APK output, so invoking
`nix build .#apk` is a candidate integration. SimpleX Chat provides a confirmed
Nix/flake build precedent, and F-Droid policy
explicitly permits some Nix-cache binaries. Zeroute’s isolated Debian bootstrap and real-backend APK/lint trial passed;
full F-Droid metadata/source/binary acceptance remains separate. See the [flake-based recipe investigation](f-droid-flake-recipe.md).
Do not upload the development-signed APK or claim that a Nix cache hit proves
F-Droid source-build or reproducibility requirements.

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
| Listing | README and public contribution route exist | Candidate description/summary exist; finalize icon, sanitized screenshots and release notes |
| Submission | No F-Droid-accepted recipe or submission exists | Verify recipe, prepare metadata, then request separate submission approval |

F-Droid's inclusion policy normally asks for three months of active development,
with exceptions including proven users. Zeroute is younger. Describe the existing
use honestly and ask maintainers whether an exception applies; two owner devices
are not a guarantee of acceptance. The separate client implementation and its
connection/readiness UX should be explained rather than presented as rebranding.

## Build compatibility: integrate the existing flake

`modules/android.nix` stages the Rust/ZeroTier shared library and upstream MPL
resource before invoking Gradle. This is normal Nix build orchestration. A plain
Gradle invocation is incomplete, but a plain Gradle invocation is not a stated
requirement for F-Droid inclusion. The existing `apk` package remains the intended
single build entry point. It already depends on the real native backend; do not
substitute a backend-free Cargo build as distribution evidence.

Keep the same source owners and current core-only flags, Android API 26 minimum,
16 KiB native alignment, shared Android libc, runtime archives and symbol checks.
The lock resolves the Rust overlay and nixpkgs cross toolchain, while the version
catalog selects Android/JDK requirements. A recipe should consume these owners
rather than introduce separate drifting toolchain versions.

SimpleX demonstrates that F-Droid can bootstrap Nix and build flake outputs.
The remaining proof is a working Zeroute recipe satisfying the applicable
source/binary scanning and dependency policies. Native
sources fetched into the Nix store must still be reviewable; application/library
binary substitution cannot silently stand in for source compilation. Exact
Nix/tool versions and policies need to be recorded for a successful trial.

A thin bootstrap/invocation adapter is prepared under `packaging/fdroid/`; no
alternative native build pipeline or accepted F-Droid recipe is implemented.
If maintainers identify a concrete blocker
to the flake approach, discuss that evidence and alternatives before choosing a
second build implementation.

## Licensing review boundary

All 29 registry archives now match their `Cargo.lock` checksums. Android
resolution has 11 runtime candidates and 15 build/procedural-macro crates; the
three Windows-only entries are excluded. Their exact declarations and the JNI
license-file provenance are recorded in [the license inventory](third-party-licenses.md).

The actual core target includes MIT Prometheus headers and public-domain crypto
notices in addition to MPL-covered code. Rust standard-library and static NDK
runtime notices are separate from the application Cargo graph. The current APK
only stages the MPL resource: complete, accessible notices and covered-source
links remain a distribution gate. Upstream archives also contain unused prebuilt
server libraries; a meaningful scanner check must cover the same clean sources
that the compiler uses, rather than hide those files with blanket ignores.

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
export/import UI is implemented. The owner accepts uninstalling the test apps
when the distribution release is ready, followed by entering network settings and authorizing new node identities.
This acceptance is not authorization to uninstall now; neither device changes
nor key operations are performed by this assessment.

## Proposed sequence and cost agreement

1. Adapt the confirmed SimpleX Nix/flake packaging precedent to Zeroute and
   agree to one ARM64 source-build trial on the selected remote builder.
   Inspect tool/source downloads and substitution policy before starting. Produce a
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

The isolated Nix bootstrap/APK trial passed after the mutable-feed fix; its
measured phases and interrupted/resumed cold-download history are recorded in
[CI documentation](ci.md). It is not an uninterrupted cold-build benchmark.
The full flake scanner check subsequently passed on 2026-11-23; its scope and
results are recorded in the CI documentation. Official inclusion remains separate.
Do not treat a cached Nix APK as proof that the new recipe is cheap or compatible.
Keep compilation bounded and use the already selected remote builder; no local
system/package build, emulator download or additional ABI is proposed.

Nix CI can keep reusing its locked native/toolchain outputs. A flake recipe should
reuse the same locked build definitions. F-Droid acceptance may require rebuilding
application/native library outputs instead of substituting them; source, upstream
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
proposals or inferences. The isolated recipe-adapter build is not an official
F-Droid build-server run or acceptance. Scanner evidence is recorded separately
in [CI documentation](ci.md).
