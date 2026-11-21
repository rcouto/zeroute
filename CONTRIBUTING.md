# Contributing to Zeroute

Use [GitHub pull requests](https://github.com/rcouto/zeroute/pulls) to propose
changes. You do not need an account on the private development forge. This is a
personal experimental project: reviews, acceptance and support have no promised
turnaround. GitHub issues are currently disabled. For a large change, explain the
proposal in a draft PR before investing in the implementation.

## Submit a change

1. Fork the GitHub repository and create a branch from its `main`.
2. Keep the change focused. Explain the problem, resulting behavior and any
   user-visible limitations; update relevant documentation in the same PR.
3. Run the checks appropriate to your change and report their results. Say which
   checks you could not run. For device tests, include the Android version and
   device model, without serial numbers or network identities.
4. Open a GitHub PR against `main`. Keep discussion and updates on that PR;
   maintainers may ask you to adjust the branch if the public copy has fallen
   behind private development.

For code changes, the Nix development shell provides the pinned tools and normal
Git hooks:

```sh
nix develop
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
```

Changes affecting Android/native integration also need `nix build .#apk`, which
produces an unsigned ARM64 APK and runs Android lint/packaging checks. A cold
SDK/NDK/toolchain fetch can be substantial; compilation alone does not establish
VPN behavior on a phone. Do not add ABIs, emulators, dependencies or CI jobs
without discussing their build and maintenance costs. For documentation-only
changes, inspect the diff and run `git diff --check`; keep normal Git hooks on.

## Review, CI and acceptance

GitHub is the public contribution entry point. Gitea is the private development
source, and Buildbot supplies its CI. No GitHub CI, automatic contributor-branch
import or bidirectional synchronization is configured.

A maintainer reviews a proposed change before bringing it into a private Gitea
review branch. Importing and testing are maintainer actions; opening or updating
a GitHub PR does not automatically run the private build system. Maintainers must
keep credentials and private operational data out of untrusted code and logs.

Maintainers post a sanitized summary of the reviewed revision and check results
on the GitHub PR. Private Buildbot links are not required for contributors to
participate. Missing GitHub check badges mean that no checks are reported there;
they are not evidence of success or failure.

Accepted changes merge through protected Gitea `main`, then are published to
GitHub. A maintainer closes the original GitHub PR with links to the accepted
public commits and notes any integration changes. This can show as a closed PR
rather than a GitHub merge, especially after cherry-picking or adaptation. Until
publication completes, acceptance in private development is not yet visible in
the public source. Other proposals receive a brief explanation when closed.

Public source updates are currently manual, so GitHub can lag behind development.
This workflow does not publish an APK, sign a release or update installed apps.

## Privacy, licensing and scope

Never include credentials, identity keys, real network IDs, node identities,
private endpoints, packet captures, signing material or unredacted logs in a PR.
Use synthetic examples and minimal sanitized diagnostics. Public Git history and
PR conversations can be permanent; do not post a secret while asking for help.

Keep application behavior in Rust and the Android framework adapter small; see
[architecture](docs/architecture.md) and [connection states](docs/state-machine.md).
Repository-owned contributions use the MIT license. Upstream components retain
their own licenses, including the MPL-2.0 ZeroTier core. Do not copy decompiled
application code or remove upstream notices.
