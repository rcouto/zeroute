{ withSystem, ... }:
{
  flake.herculesCI = _: {
    onSchedule.nfu = {
      when = {
        hour = 14;
        minute = 30;
      };
      outputs.effects.update-lockfile = withSystem "x86_64-linux" (
        { pkgs, ... }:
        pkgs.stdenvNoCC.mkDerivation {
          name = "zeroute-nfu";
          isEffect = true;
          secretsMap = builtins.toJSON { };
          phases = [ "effectPhase" ];
          effectPhase = ''
            echo "Zeroute requested a lockfile refresh"
          '';
        }
      );
    };
  };
}
