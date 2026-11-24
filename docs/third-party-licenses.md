# Third-party licensing review

Review baseline: public source `2ed929b`, with the versions in `Cargo.lock`,
`flake.lock` and the Android version catalog. This is a source/dependency audit,
not a complete notice bundle and not release approval. Recheck it when those
owners change; this document does not select dependency versions.

## Owned source and native core

Zeroute-owned Rust/Java/Nix code and vector launcher/tile artwork use the root
MIT license. Android Gradle declares no runtime Java/Maven libraries.

The ZeroTier input is revision
[`899352e38405968516bb12a770f0ac02f6058fa8`](https://github.com/zerotier/ZeroTierOne/tree/899352e38405968516bb12a770f0ac02f6058fa8).
Its [license boundary](https://github.com/zerotier/ZeroTierOne/blob/899352e38405968516bb12a770f0ac02f6058fa8/LICENSE.txt)
excludes `ext/` and `nonfree/` from the general MPL statement. The actual
`CORE_OBJS` includes `node/Metrics.cpp`, which includes the MIT-licensed
Prometheus C++ lite headers. Retain that project's copyright/permission text;
MPL text alone is insufficient. ECC, SHA512 and Poly1305 sources also contain
public-domain declarations, which should retain their provenance.

`ZT_NONFREE=0`, `ZT_CONTROLLER=0`, `ZT_SSO_SUPPORTED=0` and the `core` target
exclude the nonfree controller and desktop service. They do not make every
file in the upstream archive part of the APK. In particular, unused Redis,
hiredis and PostgreSQL client directories contain prebuilt static archives.
Those are source-scanning hazards even though the core target does not link
them; scan the same clean source boundary used by the build, not an ignored
view that differs from the compiler input. The unused JavaScript rule compiler
also has an unlocked npm manifest. The native input filter excludes these unused
sources/artifacts; no JavaScript dependencies are installed for the core target.

MPL remains file-level: do not relabel owned MIT files. The candidate F-Droid
`License: MPL-2.0` describes its provisional binary-level classification, not
an inventory of every component. F-Droid accepts one identifier in this field;
retain per-component obligations here and in the eventual APK notices.
[Mozilla's MPL FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) explains combining
covered files into a larger work, and the
[F-Droid field reference](https://f-droid.org/en/docs/Build_Metadata_Reference/#license)
explains the metadata limitation.

## Locked Rust dependencies

All 29 registry archives were checked against the SHA-256 checksums in
`Cargo.lock`. The three previously missing Windows archives were retrieved from
the official crate archive host and verified the same way. Their declarations
are FLOSS and they are absent from Android dependency resolution.

The graph was evaluated with the pinned Rust toolchain and Cargo's locked,
offline `aarch64-linux-android` platform filter. Runtime means reachable through
normal dependencies, excluding procedural-macro targets and build edges. This
is a graph classification, not proof that every dependency emits linked machine
code or that macro-generated code carries no obligations. Build-only entries
remain relevant when redistributing their source/tool binaries.

### Android runtime graph (11)

| Crate | Version | Manifest license expression |
| --- | --- | --- |
| [bytes](https://crates.io/crates/bytes/1.12.1) | `1.12.1` | `MIT` |
| [cfg-if](https://crates.io/crates/cfg-if/1.0.5) | `1.0.5` | `MIT OR Apache-2.0` |
| [combine](https://crates.io/crates/combine/4.6.9) | `4.6.9` | `MIT` |
| [jni](https://crates.io/crates/jni/0.22.4) | `0.22.4` | `MIT OR Apache-2.0` |
| [jni-sys](https://crates.io/crates/jni-sys/0.4.1) | `0.4.1` | `MIT OR Apache-2.0` |
| [libc](https://crates.io/crates/libc/0.2.191) | `0.2.191` | `MIT OR Apache-2.0` |
| [log](https://crates.io/crates/log/0.4.35) | `0.4.35` | `MIT OR Apache-2.0` |
| [memchr](https://crates.io/crates/memchr/2.8.3) | `2.8.3` | `Unlicense OR MIT` |
| [simd_cesu8](https://crates.io/crates/simd_cesu8/1.2.0) | `1.2.0` | `Apache-2.0 OR MIT` |
| [simdutf8](https://crates.io/crates/simdutf8/0.1.5) | `0.1.5` | `MIT OR Apache-2.0` |
| [thiserror](https://crates.io/crates/thiserror/2.0.21) | `2.0.21` | `MIT OR Apache-2.0` |

### Build scripts and procedural macros (15)

| Crate | Version | Manifest license expression |
| --- | --- | --- |
| [cc](https://crates.io/crates/cc/1.11.2) | `1.11.2` | `MIT OR Apache-2.0` |
| [find-msvc-tools](https://crates.io/crates/find-msvc-tools/0.1.15) | `0.1.15` | `MIT OR Apache-2.0` |
| [jni-macros](https://crates.io/crates/jni-macros/0.22.4) | `0.22.4` | `MIT OR Apache-2.0` |
| [jni-sys-macros](https://crates.io/crates/jni-sys-macros/0.4.1) | `0.4.1` | `MIT OR Apache-2.0` |
| [proc-macro2](https://crates.io/crates/proc-macro2/1.0.107) | `1.0.107` | `MIT OR Apache-2.0` |
| [quote](https://crates.io/crates/quote/1.0.47) | `1.0.47` | `MIT OR Apache-2.0` |
| [rustc_version](https://crates.io/crates/rustc_version/0.4.1) | `0.4.1` | `MIT OR Apache-2.0` |
| [same-file](https://crates.io/crates/same-file/1.0.6) | `1.0.6` | `Unlicense/MIT` |
| [semver](https://crates.io/crates/semver/1.0.28) | `1.0.28` | `MIT OR Apache-2.0` |
| [shlex](https://crates.io/crates/shlex/2.0.1) | `2.0.1` | `MIT OR Apache-2.0` |
| [syn](https://crates.io/crates/syn/2.0.119) | `2.0.119` | `MIT OR Apache-2.0` |
| [syn](https://crates.io/crates/syn/3.0.10) | `3.0.10` | `MIT OR Apache-2.0` |
| [thiserror-impl](https://crates.io/crates/thiserror-impl/2.0.21) | `2.0.21` | `MIT OR Apache-2.0` |
| [unicode-ident](https://crates.io/crates/unicode-ident/1.0.27) | `1.0.27` | `(MIT OR Apache-2.0) AND Unicode-3.0` |
| [walkdir](https://crates.io/crates/walkdir/2.5.0) | `2.5.0` | `Unlicense/MIT` |

### Windows-only lockfile entries (3)

| Crate | Version | Manifest license expression |
| --- | --- | --- |
| [winapi-util](https://crates.io/crates/winapi-util/0.1.11) | `0.1.11` | `Unlicense OR MIT` |
| [windows-link](https://crates.io/crates/windows-link/0.2.1) | `0.2.1` | `MIT OR Apache-2.0` |
| [windows-sys](https://crates.io/crates/windows-sys/0.61.2) | `0.61.2` | `MIT OR Apache-2.0` |

`Unlicense/MIT` is the legacy manifest spelling in `same-file`/`walkdir`;
normalize it to an OR expression only in an exported SPDX inventory.

The published archives for `jni`, `jni-macros` and `jni-sys-macros` declare
MIT OR Apache-2.0 but omit root license files. The files were recovered from the
exact revisions in each archive's `.cargo_vcs_info.json`:

- `jni`: [MIT](https://github.com/jni-rs/jni-rs/blob/5ae9458a4ec44c5318f37ddc7569c1d4ae8a69e7/LICENSE-MIT),
  [Apache](https://github.com/jni-rs/jni-rs/blob/5ae9458a4ec44c5318f37ddc7569c1d4ae8a69e7/LICENSE-APACHE).
- `jni-macros`: [MIT](https://github.com/jni-rs/jni-rs/blob/33045a124105c939d1e2cbdcb5a39e5d868ffa03/LICENSE-MIT),
  [Apache](https://github.com/jni-rs/jni-rs/blob/33045a124105c939d1e2cbdcb5a39e5d868ffa03/LICENSE-APACHE).
- `jni-sys-macros`: [MIT](https://github.com/jni-rs/jni-sys/blob/64d77b7a5f119d7b55b4e2c169a4668067ff59e6/LICENSE-MIT),
  [Apache](https://github.com/jni-rs/jni-sys/blob/64d77b7a5f119d7b55b4e2c169a4668067ff59e6/LICENSE-APACHE).

## Statically linked language/runtime components

The native adapter explicitly links `c++_static`, `c++abi` and `unwind` from NDK
`27.0.12077973` (Clang 18.0.1, based on r522817). The cached NDK contains
`NOTICE` and `NOTICE.toolchain`; the latter includes LLVM exception text and
libc++/libc++abi licensing. Collect the applicable library notices, not just
SDK acceptance terms or a compiler package's top-level label.

Rust's standard-library distribution includes `COPYRIGHT-library.html` and its
referenced license texts. It identifies Apache-2.0 OR MIT as the general terms
and records exceptions such as Unicode data. It is a different inventory from
Cargo's application lockfile; include the relevant runtime notices too. The
separate full compiler `COPYRIGHT.html` is not a precise APK component list.
Android's shared `libc.so` is required dynamically rather than copied into the
APK as a static archive.

## Generated notice sources

The [notice-source generator](../packaging/notices/README.md) replaces committed
copies with a standalone Nix artifact. Standard cargo-about consumes the existing
Cargo vendor closure; Git holds hash-pinned references for missing JNI
texts and targeted LLVM notices. The native compiler's source input and selected Rust/NDK
distributions supply their own notice material. Crate coverage, original JNI
attribution and runtime archive identities are checked during generation.

The APK consumes the selected `apk/` pages. The broader NDK distribution
documents stay in the review output. Android build-declaration receipts require
a new coverage review if runtime Maven or Kotlin dependencies are introduced.

## Distribution gates

The app exposes the selected material through **Open source notices**. The APK
build compares included assets with generator output. The release still needs
a phone check and a link to the exact published Zeroute source from the release
record. The app already links the exact covered ZeroTier source revision.
See the [ARM64 APK notice selection](apk-notice-selection.md).

Scanner results do not prove license compliance. Metadata remains disabled until
complete APK notices, version/tag ownership, signing and submission review are
resolved. Toolchain binaries, APK code, source archives and artwork have separate
redistribution boundaries. Official Android SDK/Nix-cache allowances in
[F-Droid policy](https://f-droid.org/docs/Inclusion_Policy/) do not excuse missing
runtime notices or authorize proprietary application dependencies.

Peer discovery uses the pinned upstream default root world, persisted as
`planet` by the Rust state adapter. The app exposes no custom planet/moon setup.
A self-hosted controller selected through a network ID does not by itself replace
those root defaults. Disclose this limitation to packagers for a
[TetheredNet/NonFreeNet review](https://f-droid.org/en/docs/Anti-Features/);
do not infer that either flag is required or absent merely from a FLOSS core
license. This review found no application-owned advertising/analytics SDK;
upstream internal metrics counters are not proof of remote analytics export.
