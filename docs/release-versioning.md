# Release versions and tags

Zeroute's release name comes from `workspace.package.version` in
[`Cargo.toml`](../Cargo.toml). Use stable `MAJOR.MINOR.PATCH` versions for
public releases and tag the approved commit as `vMAJOR.MINOR.PATCH`.
For example, release version `0.1.0` uses tag `v0.1.0`.

Android uses `versionCode` to order updates to `org.rcouto.zeroute`.
Gradle and the APK packaging check read `androidVersionCode` from the
[Android version catalog](../android/gradle/libs.versions.toml). The initial
candidate uses code `1`. For each subsequent public Android build,
advance the code to an unused integer greater than the highest code previously
distributed for that application ID. Several test builds of an unpublished
candidate can share a code. Once a code is published, reserve it permanently
for that build.

## Release preflight

1. Identify the candidate commit and confirm its required checks passed.
2. Inspect the candidate APK manifest. Check `org.rcouto.zeroute`, the workspace
   version as `versionName`, and the intended Android `versionCode`. The
   existing APK check performs this comparison against the source declarations
   during the build.
3. Compare the candidate code against published Android versions in all
   Zeroute distribution channels. For subsequent releases, stop if the
   candidate code fails to exceed the greatest published code. Check that
   the release tag is unused and matches the workspace version.
4. For an F-Droid submission, copy the verified APK version pair and exact
   release reference into its build record. That record describes the release
   F-Droid will build.
5. After release approval, create the tag on the verified Gitea commit, push
   the same tag to GitHub and verify that both tags resolve to that commit.
   Publish release notes and any separately approved artifacts under that tag.

See [source sharing](source-sharing.md) for the publication workflow.
