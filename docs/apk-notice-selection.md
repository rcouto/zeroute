# Notice selection for the first ARM64 APK

The selection below applies to the inputs locked at
[`c69dacb`](https://github.com/rcouto/zeroute/tree/c69dacbf70284de1910a37c1b7f394834d741366).
The review used the existing unsigned APK (SHA-256
`6d730f4f303b2d627abd0d9f7969689878dcf4594015b7d77b3292a929343607`),
the generated notice manifest, the actual NDK archives, and pinned upstream
sources. The APK contains one DEX and one ARM64 shared library. Its
`DT_NEEDED` entries are Android's device-provided `libc.so`, `libdl.so`
and `libm.so`. The current Gradle declarations specify no packaged
Maven or Kotlin runtime dependency.

## Material to distribute

| Component | Selected notice and source |
| --- | --- |
| Zeroute Java, Rust, JNI adapter and artwork | Root MIT `LICENSE`. Link the exact released Zeroute source commit. |
| Cargo runtime dependencies | Select the 11 Android runtime entries in the generated manifest. Preserve their selected `cargo-about` texts and component-specific attribution, including the pinned JNI exception. Link each exact crate archive using its version and Cargo.lock checksum. |
| Cargo build and procedural macros | Retain attribution for the 15 Android build/macro entries in a separate section, conservatively covering code they may generate. Keep the accompanying component mapping when one license text serves both roles. The three Windows-only crates stay outside this selection. |
| ZeroTier core | Include `LICENSE-MPL.txt` and the upstream license boundary in `LICENSE.txt`. Supply the [exact covered source](https://github.com/zerotier/ZeroTierOne/tree/899352e38405968516bb12a770f0ac02f6058fa8) and state that it is available under MPL 2.0. The core uses filtered upstream sources and the separate Zeroute adapter. Preserve those build definitions in the release source. |
| ZeroTier's embedded code | Include the MIT notice for Prometheus C++ lite, the embedded LZ4 BSD-2-Clause block from [`node/Packet.cpp`](https://github.com/zerotier/ZeroTierOne/blob/899352e38405968516bb12a770f0ac02f6058fa8/node/Packet.cpp#L85), and the public-domain attributions from `ECC.cpp`, `SHA512.cpp`, `Poly1305.cpp`, `Salsa20.cpp` and `Salsa20.hpp`. The ARM64 core target lists `Packet.o`, `Salsa20.o` and `Metrics.o`. The cached APK also contains an LZ4 compression symbol. Prometheus' optional HTTP client sources stay in the source-review artifact. They are outside the inspected core header path. |
| Rust standard library and Rust compiler builtins | Keep the distribution's complete `COPYRIGHT-library.html` as a standard-library notice, preserving its exceptions and third-party attributions. It covers more than the Android-linked subset. Include the distribution's MIT and Apache-2.0 texts for offline reading. Rust 1.100.0 identifies [source commit `0ef1bc386059596dada8fb65b50425aab2ea637a`](https://github.com/rust-lang/rust/tree/0ef1bc386059596dada8fb65b50425aab2ea637a). |
| Android NDK C++ runtime | Include component-specific `LICENSE.TXT` from [libc++](https://android.googlesource.com/toolchain/llvm-project/+/d8003a456d14a3deb8054cdaa529ffbf02d9b262/libcxx/LICENSE.TXT), [libc++abi](https://android.googlesource.com/toolchain/llvm-project/+/d8003a456d14a3deb8054cdaa529ffbf02d9b262/libcxxabi/LICENSE.TXT), and [libunwind](https://android.googlesource.com/toolchain/llvm-project/+/d8003a456d14a3deb8054cdaa529ffbf02d9b262/libunwind/LICENSE.TXT). Preserve their Apache-2.0 with LLVM Exceptions texts and legacy terms. These are the three explicit archives in `crates/core/build.rs`. Their cached NDK r27 archive hashes match the generator manifest. |
| Android NDK compiler support | Also include the [compiler-rt component notice](https://android.googlesource.com/toolchain/llvm-project/+/d8003a456d14a3deb8054cdaa529ffbf02d9b262/compiler-rt/LICENSE.TXT) conservatively. The cached APK defines AArch64 outlined atomics and `__udivti3`. Those names also appear in the selected NDK's `libclang_rt.builtins-aarch64-android.a`. Rust compiler builtins can provide overlapping symbols, and the final ELF does not identify the contributing archive for every symbol. |

The NDK package is `27.0.12077973`. The four inspected archives report
the r522817 Clang producer at `d8003a456d14a3deb8054cdaa529ffbf02d9b262`.
The ELF also has an earlier Clang producer stamp
`d9f89f4d16663d5012e5c09495f3b30ece3d2362`. The four component license files
were fetched and compared at both revisions. Each pair was byte-identical.
Use the pinned NDK package and these upstream revisions for provenance.
Keep the broader `NOTICE` and `NOTICE.toolchain` in the review artifact.

## Coverage changes needed for packaging

PR #11's manifest covers the Cargo graph and records the three explicit NDK
archive identities. Its current native output omits the LZ4 block and Salsa20
attributions. The Rust standard-library HTML needs its companion license texts
for offline use. Add the four targeted NDK license texts and record the
compiler-builtins archive in the packaging inventory.

Extract the LZ4 block and Salsa20 preambles from the compiler's pinned ZeroTier
source. Read the Rust notices from the locked toolchain. Fetch the targeted
LLVM texts by the verified source revision and content hashes. Keep upstream
notice copies out of Git. Apply the packaging checks to the actual APK built
for release and repeat the source and link-boundary audit if its inputs change.
See [Mozilla's MPL distribution guidance](https://www.mozilla.org/en-US/MPL/2.0/FAQ/).
