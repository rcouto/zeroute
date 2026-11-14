{ inputs, ... }:
{
  perSystem =
    { system, lib, ... }:
    let
      versions = (lib.importTOML ../android/gradle/libs.versions.toml).versions;
      packageConfig = {
        allowUnfree = true;
        android_sdk.accept_license = true;
      };
      pkgs = import inputs.nixpkgs {
        inherit system;
        config = packageConfig;
        overlays = [ inputs.rust-overlay.overlays.default ];
      };
    in
    {
      _module.args = {
        inherit pkgs;
        zeroute = {
          inherit versions;
          version = (lib.importTOML ../Cargo.toml).workspace.package.version;
          rustToolchain = pkgs.rust-bin.stable.latest.minimal.override {
            extensions = [
              "clippy"
              "rustfmt"
              "rust-src"
            ];
            targets = [ "aarch64-linux-android" ];
          };
          androidPkgs = import inputs.nixpkgs {
            localSystem = system;
            crossSystem = lib.systems.examples.aarch64-android-prebuilt // {
              androidSdkVersion = versions.minSdk;
              androidNdkVersion = versions.ndkMajor;
            };
            config = packageConfig;
          };
          sdk = pkgs.androidenv.composeAndroidPackages {
            platformVersions = [ versions.compileSdk ];
            buildToolsVersions = [ versions.buildTools ];
            includeCmake = false;
            includeNDK = false;
            includeEmulator = false;
            includeSystemImages = false;
            includeSources = false;
          };
        };
      };
    };
}
