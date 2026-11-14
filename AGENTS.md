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
