{ self, ... }:
{
  perSystem =
    {
      config,
      pkgs,
      zeroute,
      ...
    }:
    let
      sdkRoot = "${zeroute.sdk.androidsdk}/libexec/android-sdk";
    in
    {
      formatter = pkgs.nixfmt;
      pre-commit.settings = {
        src = self;
        settings.rust.check.cargoDeps = config.packages.core.cargoDeps;
        hooks = {
          nixfmt.enable = true;
          deadnix.enable = true;
          google-java-format.enable = true;
          ktlint = {
            enable = true;
            package = pkgs.ktlint;
            entry = "${pkgs.ktlint}/bin/ktlint --format";
            files = "\\.kts?$";
          };
          rustfmt = {
            enable = true;
            packageOverrides = {
              cargo = zeroute.rustToolchain;
              rustfmt = zeroute.rustToolchain;
            };
          };
          clippy = {
            enable = true;
            packageOverrides = {
              cargo = zeroute.rustToolchain;
              clippy = zeroute.rustToolchain;
            };
            settings.denyWarnings = true;
          };
          check-json.enable = true;
          check-toml.enable = true;
          check-xml.enable = true;
          check-merge-conflicts.enable = true;
          detect-private-keys.enable = true;
          end-of-file-fixer.enable = true;
          trim-trailing-whitespace.enable = true;
        };
      };
      devShells.default = pkgs.mkShell {
        packages = config.pre-commit.settings.enabledPackages ++ [
          zeroute.rustToolchain
          pkgs.rust-analyzer
          pkgs.gradle
          pkgs.jdk21_headless
          pkgs.android-tools
          pkgs.unzip
        ];
        ANDROID_HOME = sdkRoot;
        ANDROID_SDK_ROOT = sdkRoot;
        JAVA_HOME = pkgs.jdk21_headless.home;
        ZEROUTE_VERSION = zeroute.version;
        shellHook = config.pre-commit.shellHook;
      };
    };
}
