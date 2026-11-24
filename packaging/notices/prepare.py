"""Supply only checksum-pinned missing notice texts to a scratch Cargo vendor tree."""

import argparse
import base64
import hashlib
import json
from pathlib import Path
import tomllib


def apply_exception(vendor, exception):
    crate = vendor / f"{exception['name']}-{exception['version']}"
    manifest = tomllib.loads((crate / "Cargo.toml").read_text())["package"]
    if (manifest["name"], manifest["version"]) != (
        exception["name"], exception["version"]
    ):
        raise ValueError("notice exception no longer matches crate identity")
    vcs = json.loads((crate / ".cargo_vcs_info.json").read_text())
    if vcs["git"]["sha1"] != exception["revision"]:
        raise ValueError(f"notice exception revision changed: {manifest['name']}")
    for file in exception["files"]:
        data = Path(file["source"]).read_bytes()
        actual = "sha256-" + base64.b64encode(hashlib.sha256(data).digest()).decode()
        if actual != file["hash"]:
            raise ValueError("notice exception hash mismatch")
        if Path(file["path"]).name != file["path"]:
            raise ValueError("notice exception must name a root license file")
        destination = crate / file["path"]
        if destination.exists():
            raise ValueError("upstream now ships this notice; remove the exception")
        destination.write_bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("vendor", type=Path)
    parser.add_argument("exceptions", type=Path)
    args = parser.parse_args()
    for exception in json.loads(args.exceptions.read_text()):
        apply_exception(args.vendor, exception)
    # Prevent cargo-about's generic fallback from silently replacing missing
    # component-specific copyright text. A new unusual layout needs review.
    for crate in args.vendor.iterdir():
        if not (crate / "Cargo.toml").is_file():
            continue
        if not any(
            path.is_file()
            and path.name.lower().startswith(("license", "licence", "copying", "notice"))
            for path in crate.iterdir()
        ):
            raise ValueError(f"no original notice files found for {crate.name}")


if __name__ == "__main__":
    main()
