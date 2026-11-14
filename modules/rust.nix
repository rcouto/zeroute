{ lib, ... }:
{
  perSystem =
    {
      config,
      pkgs,
      zeroute,
      ...
    }:
    let
      source = lib.fileset.toSource {
        root = ../.;
        fileset = lib.fileset.unions [
          ../Cargo.toml
          ../Cargo.lock
          ../crates
        ];
      };
      packageArguments = {
        pname = "zeroute-core";
        inherit (zeroute) version;
        src = source;
        cargoLock.lockFile = ../Cargo.lock;
        auditable = false;
        meta = {
          description = "Zeroute native application boundary";
          license = lib.licenses.mit;
        };
      };
      hostPlatform = pkgs.makeRustPlatform {
        cargo = zeroute.rustToolchain;
        rustc = zeroute.rustToolchain;
      };
      androidPlatform = zeroute.androidPkgs.makeRustPlatform {
        cargo = zeroute.rustToolchain;
        rustc = zeroute.rustToolchain;
      };
    in
    {
      packages.core = hostPlatform.buildRustPackage (
        packageArguments
        // {
          doCheck = true;
        }
      );
      packages.core-android = androidPlatform.buildRustPackage (
        packageArguments
        // {
          doCheck = false;
          ZEROUTE_CORE_DIR = config.packages.zerotier-core-android;
          preBuild = ''
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CXX"
          '';
          # Android native libraries must also load on devices using 16 KiB pages.
          RUSTFLAGS = [
            "-C"
            "link-arg=-Wl,-z,max-page-size=16384"
            "-C"
            "link-arg=-Wl,--no-undefined"
            "-C"
            "target-feature=-crt-static"
          ];
        }
      );
      checks.rust = config.packages.core;
    };
}
