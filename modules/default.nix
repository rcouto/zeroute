{ inputs, ... }:
{
  imports = [ inputs.git-hooks.flakeModule ];
  systems = [ "x86_64-linux" ];
}
