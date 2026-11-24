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
      scanTools =
        builtins.head
          (pkgs.androidenv.composeAndroidPackages {
            toolsVersion = null;
            platformVersions = [ ];
            buildToolsVersions = [ zeroute.versions.buildTools ];
          }).build-tools;
    in
    {
      checks.fdroid =
        pkgs.runCommand "zeroute-fdroid-review"
          {
            nativeBuildInputs = [
              pkgs.fdroidserver
              (pkgs.python3.withPackages (python: [ python.pyyaml ]))
            ];
            noticeBundle = config.packages.notices;
            projectSource = self;
            coreSource = config.packages.zerotier-core-android.src;
            apk = "${config.packages.apk}/zeroute-arm64-unsigned.apk";
            # Scanning needs dexdump from the existing build-tools output, not
            # platforms, sdkmanager or the legacy SDK tools/extra JDK closure.
            sdk = "${scanTools}/libexec/android-sdk";
          }
          ''
            set -euo pipefail
            test -s "$noticeBundle/index.html"
            export HOME="$TMPDIR/home"
            export XDG_CACHE_HOME="$HOME/.cache"
            mkdir -p "$HOME" "$out" metadata config build/org.rcouto.zeroute
            cp "$noticeBundle/manifest.json" "$out/notices.json"
            cat > config.yml <<EOF
            sdk_path: $sdk
            scanner_signature_sources: [suss]
            refresh_scanner: false
            EOF
            # Local review repository supports the candidate's standard category.
            # Category/AntiFeature decisions remain subject to F-Droid review.
            cat > config/categories.yml <<EOF
            Connectivity:
              name: Connectivity
            EOF
            # Lint the real disabled candidate before making scan-only metadata.
            cp ${self}/packaging/fdroid/org.rcouto.zeroute.yml metadata/
            chmod u+w metadata/org.rcouto.zeroute.yml
            fdroid lint --force-yamllint org.rcouto.zeroute 2>&1 | tee "$out/metadata.log"
            # No Builds means official CLI scans current source without VCS/network.
            python - <<'PYTHON'
            from pathlib import Path
            import yaml
            path = Path("metadata/org.rcouto.zeroute.yml")
            app = yaml.safe_load(path.read_text())
            app.pop("Builds", None)
            path.write_text(yaml.safe_dump(app, sort_keys=False))
            PYTHON
            cp -r "$projectSource"/. build/org.rcouto.zeroute/
            mkdir build/org.rcouto.zeroute/zerotier-core
            cp -r "$coreSource"/. build/org.rcouto.zeroute/zerotier-core/
            chmod -R u+w build
            if ! fdroid scanner --exit-code --json org.rcouto.zeroute > "$out/source.json"; then
              cat "$out/source.json"
              exit 1
            fi
            fdroid scanner --exit-code "$apk" 2>&1 | tee "$out/apk.log"
            printf '%s\n' \
              'Passed: candidate metadata, current project/native source and unsigned ARM64 APK.' \
              'Rules: fdroidserver packaged SUSS snapshot; no live refresh.' \
              'Not release approval: placeholder build disabled; notices/signing/review remain.' \
              > "$out/scope.txt"
          '';
    };
}
