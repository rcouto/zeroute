# Generated notice-source artifact

`nix build .#notices` generates the review index and manifest from pinned build
inputs. Its `apk/` directory contains the selected notice pages. The Android
build copies those pages into assets and compares each packaged page with the
generated file. Use **Open source notices** in the app to read them offline.
Tapped HTTPS source links open in the device browser.
For a phone smoke test, disable Wi-Fi in airplane mode, open both **Embedded
LZ4** and **ZeroTier MPL 2.0**, and confirm each page shows the license text.
Use **Contents** to return to the index between pages.

Inspect the selected executor's dry-run first. Garbage collection can force
restoration of the Rust, NDK and Android build inputs.

## Source owners and coverage

- **Rust crates:** standard nixpkgs `cargo-about`, the existing Cargo vendor
  closure and `Cargo.lock`. Android-target filtering excludes Windows-only
  dependencies. Build/procedural-macro attributions remain included, with runtime
  graph roles recorded separately. The accepted-license order selects MIT when
  a component offers MIT OR Apache-2.0; compound requirements such as Unicode
  remain represented. This does not relabel upstream source.
- **Missing crate notices:** `exceptions.json` contains only exact upstream
  revisions and hash-pinned fetches for three JNI archives that omit license
  files. Nix fetches those small texts separately. Version/revision checks run
  before supplementing the notice generator's writable vendor copy. The actual
  compiler vendor input is unchanged. Stale exceptions and missing original
  notice files fail instead of silently using generic license text.
- **Native source:** the compiler's cleaned ZeroTier input provides the MPL
  boundary/text, embedded Prometheus/header-library notices and public-domain
  crypto preambles. `native-coverage.json` is an audit receipt: a source pin or
  build-flag change requires reviewing embedded dependency coverage. It does not
  select the actual source/build flags. No upstream text copies remain in Git.
- **Language/native runtimes:** the locked Rust distribution supplies
  `COPYRIGHT-library.html`; the NDK selected by the existing compiler supplies
  `NOTICE`, `NOTICE.toolchain` and archive identities for libc++, libc++abi,
  libunwind and compiler-rt builtins. The APK includes targeted LLVM license
  texts pinned to the audited NDK revision. The broader NDK documents remain
  available in the review output.
- **Android/JVM:** the current adapter is Java using device framework APIs; it
  declares no runtime Maven/Kotlin libraries. `android-coverage.json` checks the
  relevant build declarations so changes require a new dependency-coverage
  review. Kotlin Gradle scripts and Nix build tools are not automatically APK
  dependencies. Nixpkgs `meta.license` identifiers alone do not supply notices.

## Build and validation

Nixpkgs' Cargo setup hook provides the existing vendor closure. `cargo metadata`
and `cargo-about` run frozen/offline, without compiling application/native code
or making live license queries. A small assembler preserves native/runtime
notice sources, strips local paths from the crate report, checks generated crate
coverage and renders an escaped HTML index. The artifact is generated data;
license texts are not hand-maintained repository files.

The existing F-Droid flake check depends on this package and retains its manifest
as `notices.json`. It selects only the pinned scanner build tools, excluding the
unused full/legacy SDK closure. No extra CI job or ABI is
introduced. Packaging a changed notice bundle invalidates the APK and scanner
outputs.
Tests cover attribution preservation, stale source revisions, changed notice
texts and omitted dependency coverage. A successful artifact does not establish
F-Droid acceptance or prove that the viewer works on a physical phone.

Changes to crate/native pins, runtime distributions, generator configuration or
exception pins invalidate the relevant artifact. Android declaration changes
require reviewing and updating the coverage receipt. See
[CI costs](../../docs/ci.md) and [licensing review](../../docs/third-party-licenses.md).
