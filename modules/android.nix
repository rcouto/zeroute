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
      sdkRoot = "${zeroute.sdk.androidsdk}/libexec/android-sdk";
    in
    {
      packages.apk = pkgs.stdenv.mkDerivation (finalAttrs: {
        pname = "zeroute-android";
        inherit (zeroute) version;
        src = lib.fileset.toSource {
          root = ../android;
          fileset = lib.fileset.unions [
            ../android/settings.gradle.kts
            ../android/build.gradle.kts
            ../android/gradle.properties
            ../android/gradle
            ../android/app/build.gradle.kts
            ../android/app/src/main/AndroidManifest.xml
            ../android/app/src/main/java
            ../android/app/src/main/res
          ];
        };
        nativeBuildInputs = [
          pkgs.gradle
          pkgs.jdk21_headless
          pkgs.unzip
          pkgs.binutils
        ];
        ANDROID_HOME = sdkRoot;
        ANDROID_SDK_ROOT = sdkRoot;
        JAVA_HOME = pkgs.jdk21_headless.home;
        mitmCache = pkgs.gradle.fetchDeps {
          pkg = finalAttrs.finalPackage;
          data = ./gradle-deps.json;
        };
        gradleFlags = [
          "-PzerouteVersion=${zeroute.version}"
          "-Dorg.gradle.project.android.aapt2FromMavenOverride=${sdkRoot}/build-tools/${zeroute.versions.buildTools}/aapt2"
        ];
        postPatch = ''
          mkdir -p app/src/main/jniLibs/arm64-v8a
          cp ${config.packages.core-android}/lib/libzeroute_core.so app/src/main/jniLibs/arm64-v8a/
          mkdir -p app/src/main/res/raw
          cp ${config.packages.zerotier-core-android}/share/licenses/zerotier-core/LICENSE-MPL.txt app/src/main/res/raw/zerotier_core_license.txt
        '';
        gradleBuildTask = "assembleRelease";
        doCheck = true;
        gradleCheckTask = "lintRelease";
        gradleUpdateTask = "assembleRelease lintRelease";
        enableParallelBuilding = false;
        enableParallelChecking = false;
        enableParallelUpdating = false;
        installPhase = ''
          runHook preInstall
          mkdir -p "$out/nix-support"
          cp app/build/outputs/apk/release/app-release-unsigned.apk "$out/zeroute-arm64-unsigned.apk"
          printf 'file binary-dist %s\n' "$out/zeroute-arm64-unsigned.apk" > "$out/nix-support/hydra-build-products"
          runHook postInstall
        '';
        doInstallCheck = true;
        installCheckPhase = ''
          runHook preInstallCheck
          unzip -Z1 "$out/zeroute-arm64-unsigned.apk" | grep -Fx 'lib/arm64-v8a/libzeroute_core.so'
          unzip -p "$out/zeroute-arm64-unsigned.apk" 'lib/arm64-v8a/libzeroute_core.so' > libzeroute_core.so
          readelf -h libzeroute_core.so | grep -E 'Machine: +AArch64'
          # Android libc must stay dynamically linked; never ship its static copy.
          readelf -d libzeroute_core.so | grep -F 'Shared library: [libc.so]'
          readelf --wide --dyn-syms libzeroute_core.so | grep -F 'Java_org_rcouto_zeroute_NativeBridge_apiVersion'
          runHook postInstallCheck
        '';
        meta = {
          description = "Unsigned ARM64 Zeroute Android development scaffold";
          license = lib.licenses.mit;
          platforms = [ "x86_64-linux" ];
        };
      });
      packages.default = config.packages.apk;
      checks.android-apk = config.packages.apk;
      apps.update-gradle-deps = {
        type = "app";
        program = toString config.packages.apk.mitmCache.updateScript;
        meta.description = "Refresh the hash-pinned Gradle dependency inventory";
      };
    };
}
