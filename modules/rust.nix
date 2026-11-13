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
          # Android native libraries must also load on devices using 16 KiB pages.
          RUSTFLAGS = [
            "-C"
            "link-arg=-Wl,-z,max-page-size=16384"
          ];
        }
      );
      checks.rust = config.packages.core;
    };
}
