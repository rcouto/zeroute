{ inputs, lib, ... }:
{
  perSystem =
    {
      config,
      pkgs,
      zeroute,
      ...
    }:
    let
      # Follow the NDK actually selected by the compiler, not an independent pin.
      ndkDependencies = zeroute.androidPkgs.stdenv.cc.cc.propagatedBuildInputs;
      ndk =
        assert lib.assertMsg (
          builtins.length ndkDependencies == 1
        ) "Review the pinned Android compiler's NDK dependency interface";
        builtins.head ndkDependencies;
      ndkCoverage = lib.importJSON ../packaging/notices/ndk-coverage.json;
      ndkLicenses = lib.mapAttrs (
        component: hash:
        pkgs.fetchurl {
          url = "https://android.googlesource.com/toolchain/llvm-project/+/${ndkCoverage.llvm_revision}/${component}/LICENSE.TXT?format=TEXT";
          name = "${component}-license.base64";
          inherit hash;
        }
      ) ndkCoverage.components;
      exceptions = map (
        exception:
        exception
        // {
          files = map (
            file:
            file
            // {
              source = pkgs.fetchurl {
                url = "https://raw.githubusercontent.com/${exception.repository}/${exception.revision}/${file.path}";
                inherit (file) hash;
              };
            }
          ) exception.files;
        }
      ) (lib.importJSON ../packaging/notices/exceptions.json);
    in
    {
      packages.notices = pkgs.stdenvNoCC.mkDerivation {
        pname = "zeroute-notice-sources";
        inherit (zeroute) version;
        src = lib.fileset.toSource {
          root = ../.;
          fileset = lib.fileset.unions [
            ../Cargo.toml
            ../Cargo.lock
            ../crates
            ../LICENSE
            ../packaging/notices/about.toml
            ../packaging/notices/android-coverage.json
            ../packaging/notices/assemble.py
            ../packaging/notices/native-coverage.json
            ../packaging/notices/ndk-coverage.json
            ../packaging/notices/exceptions.json
            ../packaging/notices/prepare.py
            ../packaging/notices/test_notices.py
            ../android/app/build.gradle.kts
            ../android/build.gradle.kts
            ../android/settings.gradle.kts
            ../android/gradle/libs.versions.toml
          ];
        };
        nativeBuildInputs = [
          zeroute.rustToolchain
          pkgs.cargo-about
          pkgs.python3
          pkgs.rustPlatform.cargoSetupHook
        ];
        cargoDeps = config.packages.core.cargoDeps;
        noticeExceptions = pkgs.writeText "notice-exceptions.json" (builtins.toJSON exceptions);
        nativeSource = config.packages.zerotier-core-android.src;
        nativeRevision = inputs.zerotier-core.rev;
        nativeBuildFlags = builtins.toJSON config.packages.zerotier-core-android.makeFlags;
        rustDistribution = zeroute.rustToolchain;
        ndkLicenseInputs = pkgs.writeText "ndk-license-inputs.json" (builtins.toJSON ndkLicenses);
        ndkRoot = "${ndk}/libexec/android-sdk/ndk-bundle";
        androidCxx = "${zeroute.androidPkgs.stdenv.cc}/bin/${zeroute.androidPkgs.stdenv.cc.targetPrefix}c++";
        dontConfigure = true;
        prePatch = ''
          python packaging/notices/prepare.py "$cargoDepsCopy" "$noticeExceptions"
        '';
        buildPhase = ''
          runHook preBuild
          export HOME="$TMPDIR/home"
          mkdir -p "$HOME" "$out"
          cargo metadata --frozen --format-version 1 --filter-platform aarch64-linux-android > metadata.json
          cargo-about generate --frozen --fail --workspace --config packaging/notices/about.toml --format json > about.json
          python packaging/notices/assemble.py "$out"
          runHook postBuild
        '';
        doCheck = true;
        checkPhase = ''
          runHook preCheck
          python -m unittest discover -s packaging/notices -p 'test_*.py'
          runHook postCheck
        '';
        installPhase = ''
          runHook preInstall
          test -s "$out/index.html"
          test -s "$out/apk/index.html"
          test -s "$out/manifest.json"
          runHook postInstall
        '';
        meta = {
          description = "Generated notice-source review artifact from Zeroute's pinned dependencies";
          platforms = [ "x86_64-linux" ];
        };
      };
    };
}
