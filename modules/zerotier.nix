{ inputs, lib, ... }:
{
  perSystem =
    { zeroute, ... }:
    {
      packages.zerotier-core-android = zeroute.androidPkgs.stdenv.mkDerivation {
        pname = "zerotier-core-android";
        version = "unstable-${inputs.zerotier-core.shortRev}";
        # Scanner and compiler share this boundary; core uses no server archives.
        src = lib.cleanSourceWith {
          src = inputs.zerotier-core;
          filter =
            path: type:
            !(
              type == "directory"
              && builtins.elem (baseNameOf path) [
                "nonfree"
                "rule-compiler"
              ]
            )
            && !(type == "regular" && lib.hasSuffix ".a" path);
        };
        strictDeps = true;
        dontConfigure = true;
        enableParallelBuilding = true;
        makeFlags = [
          "-f"
          "make-linux.mk"
          "ZT_EXTOSDEP=1"
          "ZT_SSO_SUPPORTED=0"
          "ZT_CONTROLLER=0"
          "ZT_NONFREE=0"
          "core"
        ];
        installPhase = ''
          runHook preInstall
          mkdir -p "$out/lib" "$out/include" "$out/share/licenses/zerotier-core"
          cp libzerotiercore.a "$out/lib/"
          cp include/ZeroTierOne.h version.h "$out/include/"
          cp LICENSE-MPL.txt "$out/share/licenses/zerotier-core/"
          runHook postInstall
        '';
        meta = {
          description = "Upstream ZeroTier core C API for Android, without the desktop service or controller";
          license = lib.licenses.mpl20;
          platforms = [ "aarch64-linux" ];
        };
      };
    };
}
