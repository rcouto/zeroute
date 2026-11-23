# Audited notice sources

This directory preserves notice material for preparing Zeroute's distribution
bundle. It does not change dependency selection, ship notices inside the APK,
or establish release readiness. Versions remain owned by `Cargo.lock`,
`flake.lock` and the Android version catalog. The owned application stays MIT;
see the root [LICENSE](../../LICENSE).

## Contents and provenance

`manifest.json` records the audited lockfile hashes, component versions, archive
checksums, source URLs and both original/stored notice hashes. The snapshot
contains 26 non-Windows crate components: 11 Android runtime candidates and 15
build/procedural-macro components. Build components are included conservatively;
this is not a claim that each contributes machine code to the APK. The three
Windows-only lockfile entries are intentionally absent.

The original crate archives matched `Cargo.lock` SHA-256 checksums before
extraction. Their notice files are preserved here, including Unicode notice
text. Three JNI archives omit license files: their notices come from the exact
upstream revisions in `.cargo_vcs_info.json`, with individual public provenance
links in the manifest. Both sides of dual-license expressions are retained;
this snapshot does not silently choose licensing terms for downstream users.

The ZeroTier material preserves its license boundary statement, MPL text,
Prometheus/header-library notices and public-domain crypto preambles. Extracted
preambles retain their exact upstream source links. Whitespace normalization is
limited to UTF-8 text, LF line endings, removal of trailing whitespace and one
final newline; license wording and copyright notices are retained. The manifest
records original and normalized hashes separately.

## Runtime boundary and maintenance

The Rust standard-library and Android NDK entries are receipt hashes for the
inspected distribution notices, not copied APK notice payloads. The NDK's
`NOTICE.toolchain` includes compiler/tool components beyond the linked C++
runtimes. Copying it wholesale would not be an accurate APK component list.
The applicable Rust, libc++, libc++abi and unwind notices still need selection
and inclusion with a user-accessible notices/source view.

When dependency owners change, re-audit the affected archive/toolchain notice
files and refresh this snapshot. Never treat a version copied into this audit
record as permission to change the actual dependency pin. Preserve notices
required by source, generated code and linked runtime components; a successful
scanner is not a substitute for that review. The canonical review boundary is
[third-party licensing](../../docs/third-party-licenses.md).

No application, signing or release settings are changed by this source-only
collection. It stays outside the APK's Android source fileset and native Cargo
source fileset, so its edits reuse those existing build outputs.
