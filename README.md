# Zeroute

A personal Rust-first Android client for ZeroTier. Open source, experimental,
and maintained for the author's own devices, with no support or compatibility
commitment.

The development app saves a network ID and uses the upstream ZeroTier core
through a Rust-owned IPv4/IPv6 packet adapter. Android provides a foreground
VPN service, system consent, protected transport sockets and the TUN interface.
The public node address is shown for controller admission. Default routes are
ignored: this is initially a private-network VPN rather than an Internet exit.
Managed DNS is optional; when configured, resolver health requires an actual reply. Direct ARM64 phone checks cover IPv6 ping, Android DNS, private HTTPS and
Disconnect. Network handovers, long-idle behavior and process-death recovery
remain unverified; build checks alone do not establish VPN reliability.

## Source and contributions

[GitHub](https://github.com/rcouto/zeroute) is the public source and contribution
entry point. Development, maintainer merges and Buildbot CI run in a private
Gitea repository; contributors do not need access to it. GitHub currently receives
manual source updates, so this copy can lag behind development. No GitHub CI,
automatic PR import or continuous synchronization is configured.

**Contributions are welcome through GitHub pull requests.** Fork this repository,
make a focused change and open a PR against `main`. Maintainers bring selected
changes into private review, run the existing checks and report a sanitized result
on the GitHub PR. Accepted changes merge in Gitea and are then published here;
maintainers close the GitHub PR with links to the accepted public commits. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the workflow and local checks. This is a
personal project with no promised review or support turnaround.

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
until separately signed. The build creates no release signing key and does
not install applications or activate a VPN on any device.

See [adaptive UI](docs/adaptive-ui.md), [architecture](docs/architecture.md), [connection states and UX](docs/state-machine.md),
[CI](docs/ci.md) and [source-first publication](docs/source-sharing.md).
Application-owned code is MIT-licensed. The bundled upstream ZeroTier core retains its MPL-2.0 license, available in
the app and at the locked upstream source. Other third-party components retain
their respective licenses.
