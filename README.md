# Zeroute

A personal Rust-first Android client for ZeroTier. Open source, experimental,
and maintained for the author's own devices, with no support or compatibility
commitment.

This repository currently contains a build/environment scaffold. The APK checks
that Java can load the Rust library and displays a development notice. It does
not implement a VPN or bundle the ZeroTier engine yet.

## Development

```console
nix develop
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
nix build .#apk
nix flake check
```

The flake uses flake-parts and import-tree. Its lock pins nixpkgs, Rust tooling,
and hook inputs. The Android version catalog owns SDK/API requirements. The
project enables Android SDK/NDK license acceptance with the owner's approval.
The NDK is selected by nixpkgs' Android cross toolchain. No emulator is included.

`nix develop` installs normal pre-commit hooks. Their configuration lives in
`modules/development.nix`; the generated YAML is ignored. `nix fmt` formats Nix.
Use `nix run .#update-gradle-deps` after dependency changes and review the updated
`modules/gradle-deps.json`. Ordinary builds use the hash-pinned dependency cache.
The Gradle version comes from nixpkgs; use Gradle inside the Nix development shell.

Keep signing material outside the worktree.

The default build produces an **unsigned ARM64 APK**. It cannot be installed
until separately signed. This scaffold creates no release signing key and does
not install applications or activate a VPN on any device.

See [architecture](docs/architecture.md) and [CI and publication](docs/ci.md).
Application-owned code is MIT-licensed. Future upstream ZeroTier components
retain their own licenses; do not assume this license covers third-party code.
