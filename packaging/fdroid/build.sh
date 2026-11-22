#!/bin/sh
# F-Droid adapter only; compilation and dependencies remain owned by the flake.
set -eu

mode=
bootstrap=false
for argument in "$@"; do
  case "$argument" in
    --dry-run | --build)
      if [ -n "$mode" ]; then
        printf '%s\n' 'Choose exactly one of --dry-run or --build.' >&2
        exit 2
      fi
      mode=$argument
      ;;
    --bootstrap) bootstrap=true ;;
    *)
      printf '%s\n' 'Usage: build.sh [--bootstrap] (--dry-run | --build)' >&2
      exit 2
      ;;
  esac
done
if [ -z "$mode" ]; then
  printf '%s\n' 'Choose --dry-run to inspect costs, or --build for an approved build.' >&2
  exit 2
fi

repository=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$repository"

if ! command -v nix >/dev/null 2>&1; then
  if [ "$bootstrap" != true ]; then
    printf '%s\n' 'Nix is missing. Bootstrap is opt-in for a prepared disposable builder.' >&2
    exit 1
  fi
  if [ "$mode" = --dry-run ]; then
    printf '%s\n' 'Bootstrap downloads are not covered by nix --dry-run; prepare Nix separately first.' >&2
    exit 1
  fi
  if [ "$(id -u)" -eq 0 ]; then
    printf '%s\n' 'Bootstrap must run as the unprivileged disposable build user.' >&2
    exit 1
  fi
  if [ ! -d /nix ] || [ ! -w /nix ]; then
    printf '%s\n' 'Bootstrap requires a pre-created /nix writable by the disposable build user.' >&2
    exit 1
  fi
  if [ -e "$HOME/.nix-profile" ] || [ -L "$HOME/.nix-profile" ]; then
    printf '%s\n' 'An existing Nix profile was found; restore its PATH instead of reinstalling.' >&2
    exit 1
  fi

  # Pinned official bootstrap, independent of the app toolchains in flake.lock.
  nix_version=2.35.6
  installer_hash=6f7bf206efb54cd4941fb274930dbc74fdfc16fcb06757ef261c8d7d69cfd60e
  bootstrap_cache=${ZEROUTE_FDROID_BOOTSTRAP_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/zeroute-fdroid}
  umask 077
  mkdir -p "$bootstrap_cache"
  bootstrap_tmp=$(mktemp -d "$bootstrap_cache/bootstrap.XXXXXXXX")
  trap 'rm -rf "$bootstrap_tmp"' EXIT
  trap 'exit 1' HUP INT TERM
  installer=$bootstrap_tmp/install
  curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
    "https://releases.nixos.org/nix/nix-$nix_version/install" --output "$installer"
  printf '%s  %s\n' "$installer_hash" "$installer" | sha256sum --check --status
  TMPDIR=$bootstrap_tmp sh "$installer" --no-daemon --no-channel-add --no-modify-profile
  # shellcheck source=/dev/null
  . "$HOME/.nix-profile/etc/profile.d/nix.sh"
fi

command -v nix >/dev/null 2>&1
nix --version
set -- nix --extra-experimental-features 'nix-command flakes' build \
  --no-update-lock-file --no-write-lock-file \
  --max-jobs 2 --cores 2 --option max-substitution-jobs 2 \
  --option sandbox true --option accept-flake-config false \
  --option substituters https://cache.nixos.org \
  --out-link result .#apk
if [ "$mode" = --dry-run ]; then
  exec "$@" --dry-run
fi
"$@"
test -s result/zeroute-arm64-unsigned.apk
printf '%s\n' 'Unsigned APK: result/zeroute-arm64-unsigned.apk'
