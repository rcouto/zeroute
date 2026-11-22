# Flake-based F-Droid recipe investigation

Status: candidate design, not a submitted or tested F-Droid recipe. No Nix install,
new build, VM, scanner run, key operation or submission was performed. The existing
flake remains the single intended build implementation.

## What the documented interface permits

F-Droid's quick-start guide permits custom toolchains through setup and build
commands. Its metadata reference supports `sudo` for privileged builder setup,
`build` for custom compilation, and `output` for locating the unsigned APK. It
does not require us to replace Nix with Gradle: Nix already invokes Gradle after
building/staging the native backend and notices.

This establishes a possible integration interface, not approval of Nix's
bootstrap, dependencies, caches or source-review model. Searches of the indexed
F-Droid metadata mirror found no `nix-bin`, `nix-build` or `nix build` recipe
precedent. That search is not exhaustive proof that none exists. The inspected
Nix-on-Droid application metadata uses Gradle; the app's name/runtime use of Nix
is not evidence that F-Droid builds it using a flake.

## Candidate metadata shape

The following is deliberately disabled and illustrative. `commit` and version
must be replaced with approved release values. No actual metadata file is enabled
in this repository or in fdroiddata.

```yaml
License: MIT
SourceCode: https://github.com/rcouto/zeroute
RepoType: git
Repo: https://github.com/rcouto/zeroute.git
Builds:
  - versionName: 0.1.0
    versionCode: 1
    disable: Draft only; Nix bootstrap, source scanning and substitution policy unverified
    commit: NOT_A_RELEASE
    output: result/zeroute-arm64-unsigned.apk
    build:
      - >-
        nix --extra-experimental-features 'nix-command flakes'
        build --no-update-lock-file --no-write-lock-file
        --max-jobs 2 --cores 2 --option max-substitution-jobs 2 .#apk
AutoUpdateMode: None
UpdateCheckMode: None
```

The MIT field describes application-owned code, not a blanket license for every
bundled component. Confirm the correct metadata/license representation with the
completed dependency inventory. Version code 1 here describes the current build,
not a decided release scheme. Automatic release detection is deliberately off
in this draft; future tags/version parsing require review.

A reviewed `sudo`/setup step must supply a suitable Nix version (for example, an
approved Debian package) and usable store/build-user configuration. No installer
URL or `curl | sh` is assumed. The command above alone does not establish the
accepted substitution policy, source inventory, bootstrap or network access.
Do not run it as an official F-Droid proof until those prerequisites are agreed.

## Specific questions to resolve before a trial

1. **Builder bootstrap:** will packagers accept Nix and its bootstrap provenance,
   minimum version, store permissions, daemon/build-user and sandbox setup in
   their standard builder? Do not disable sandboxing or install a privileged
   daemon silently. An ordinary NixOS build does not answer this question.
2. **Source visibility:** the inspected fdroidserver build flow scans the source
   directory and creates its source archive before executing custom `build`.
   Nix can fetch the pinned core/Cargo sources into its store rather than that
   directory. Determine how those exact sources will be made available for
   F-Droid inspection/archive review, without compiling during `prebuild` or
   hiding sources behind scanner exclusions. Do not add a submodule or duplicate
   upstream revision merely to satisfy a speculative recipe.
3. **Substitution policy:** resolve which toolchain binaries/caches F-Droid permits
   and which application/native libraries must be rebuilt. A downloaded final
   APK or cached Rust/ZeroTier library is not proof of their source compilation.
   Conversely, turning off all substitution can trigger enormous bootstrap/tool
   builds; do not choose that as an unmeasured default. Preserve trusted hashes
   and source pins; do not add a private cache or access credentials.
4. **Network/resources:** determine allowed fetching phases and hosts, store disk
   requirements and cleanup, and the exact cold download/build plan. Keep the
   selected trial ARM64-only and bounded; introduce no CI matrix change.
5. **Output/verification:** confirm the unsigned APK has the real core and bundled
   notices, correct version/code and ARM64 ABI, then run the applicable F-Droid
   source/binary/metadata checks. A successful normal Nix build alone is narrower
   evidence. Byte-for-byte reproducibility for upstream signatures is another
   separately costed test, not assumed here.

## Proposed next step

Prepare a focused packager question describing this candidate and these source
and cache boundaries, before investing in another build implementation. Posting
that question or submitting metadata requires explicit authorization; no message
has been sent upstream. If Nix is acceptable, inspect the concrete closure and
agree to one build-server-compatible trial on the selected remote builder.

If maintainers reject the Nix approach, retain their concrete reason and discuss
choices: address the specific incompatibility, explicitly approve another recipe,
or consider an own F-Droid-compatible binary repository with its additional
hosting/signing burden. Do not silently switch to a duplicate build pipeline.

## Prepared packager question (not sent)

> We maintain [Zeroute](https://github.com/rcouto/zeroute), an experimental
> MIT-licensed Android ZeroTier client with a separately MPL-licensed native core.
> Its public, locked Nix flake builds the core and Rust JNI library from source,
> then invokes Gradle to produce an unsigned ARM64 APK with bundled notices.
> Would a custom fdroiddata build invoking `nix build .#apk` be acceptable in
> principle, using an approved Nix bootstrap in the build VM?
>
> Which Nix toolchain substitutions would be permitted, and which outputs must be
> rebuilt? How should pinned application/library sources fetched into the Nix
> store be exposed to F-Droid's pre-build source scanner and source archive?
> We want to retain one build definition, avoid using prebuilt application/native
> outputs as source-build evidence, and understand these constraints before a
> submission or expensive compatibility trial.

## Sources and limits

- [F-Droid custom toolchain guidance](https://f-droid.org/docs/Submitting_to_F-Droid_Quick_Start_Guide/)
- [Build metadata: setup, custom build and APK output](https://f-droid.org/en/docs/Build_Metadata_Reference/)
- [Inclusion and permitted binary provenance](https://f-droid.org/docs/Inclusion_Policy/)
- [fdroidserver build/scanner ordering](https://gitlab.com/fdroid/fdroidserver/-/blob/master/fdroidserver/build.py)
- [Nix-on-Droid app metadata inspected as a limited comparison](https://gitlab.com/fdroid/fdroiddata/-/blob/master/metadata/com.termux.nix.yml)

These sources were checked on 2026-11-22. Interface support is documented; Nix
acceptance and technical compatibility are still open. The YAML is neither linted
as fdroiddata metadata nor executed, and makes no claim of inclusion eligibility.
