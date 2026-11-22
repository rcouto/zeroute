# Zeroute agent instructions

Keep application behavior in Rust and the Android framework adapter small.
Follow the architecture in docs/architecture.md. Compose Nix through flake-parts
and import-tree; use nixpkgs Rust/Gradle/Android helpers before custom machinery.
Avoid import from derivation. Pin inputs and dependency inventories.

This is a personal experimental application, not a supported VPN product.
Do not claim networking or device acceptance from scaffold build checks.
Do not copy decompiled application code. Retain third-party license boundaries.

Preserve unrelated work. Use feature branches and normal Git hooks; never bypass
hooks. Commits, pushes, mirrors, releases, signing-key operations, app installation
and device/network changes require explicit authorization. Never put credentials,
identity keys, keystores, private device logs or real network inventories into
Git, Nix derivations or build logs. Use synthetic examples.

Run focused checks appropriate to changes and review formatting fixes. The
initial CI matrix and its costs are documented in docs/ci.md. Discuss added ABIs,
emulators, toolchain rebuilds or release jobs before expanding that matrix.

## Publish GitHub after an approved Gitea merge

While native mirroring is unavailable, an authorized merge into this repository's
Gitea `main` includes authorization and a requirement to publish the resulting
Gitea `main` to `https://github.com/rcouto/zeroute.git`. Complete publication in
the same task; a successful private merge alone is not the handoff. This does
not authorize the merge itself, publishing feature branches/tags, or enabling a
mirror. Keep Gitea authoritative and normal Git hooks enabled.

After confirming the approved merge and required CI passed, fetch `origin` and
push its `main` directly. Use supported GitHub authentication; never print or
put credentials into URLs, arguments or source. With the authenticated GitHub
CLI available, the command-scoped HTTPS helper avoids changing Git settings:

```sh
git fetch origin
git -c credential.helper= -c 'credential.helper=!gh auth git-credential' \
  push https://github.com/rcouto/zeroute.git \
  refs/remotes/origin/main:refs/heads/main
```

If `gh` is unavailable, provide it with `nix-shell -p gh` before running the
command. Follow the global SSH-agent SOP for the origin fetch. Do not use
`--force`, `--mirror`, disable hooks, or publish a working feature branch.
Verify GitHub `refs/heads/main` equals the fetched Gitea main SHA, using public
`git ls-remote` or the GitHub refs API. Report merge and publication separately;
if authentication fails, refs diverge, or publication is rejected, retain the
private merge, report publication as blocked, and resolve without rewriting main.

Gitea repository settings/mirrors are managed through Gitea's UI or supported
`tea` API, not OpenTofu. Revisit this fallback when upstream Gitea NAT64 issue
https://github.com/go-gitea/gitea/issues/40142 is resolved and native mirroring is
separately approved and verified.
