"""Assemble source notices without committing generated dependency texts."""

import base64
import hashlib
import html
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_android_coverage(root):
    coverage = json.loads((root / "packaging/notices/android-coverage.json").read_text())
    for name, expected in coverage["files"].items():
        if digest(root / name) != expected:
            raise ValueError("Android dependency owners changed; review runtime notice coverage")
    return coverage["scope"]


def runtime_ids(metadata):
    packages = {p["id"]: p for p in metadata["packages"]}
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    found, pending = set(), list(metadata["workspace_members"])
    while pending:
        identity = pending.pop()
        if identity in found:
            continue
        found.add(identity)
        for dep in nodes[identity]["deps"]:
            package = packages[dep["pkg"]]
            if any("proc-macro" in target["kind"] for target in package["targets"]):
                continue
            if any(kind["kind"] is None for kind in dep["dep_kinds"]):
                pending.append(dep["pkg"])
    return found


def clean_licenses(about, packages):
    expected = {(p["name"], p["version"]) for p in packages if p["source"]}
    local = {(p["name"], p["version"]) for p in packages if not p["source"]}
    observed, licenses = set(), []
    for license in about["licenses"]:
        if not license["text"].strip():
            raise ValueError("empty generated license text")
        users = []
        for usage in license["used_by"]:
            crate = usage["crate"]
            observed.add((crate["name"], crate["version"]))
            users.append({"name": crate["name"], "version": crate["version"]})
        licenses.append({"id": license["id"], "name": license["name"],
                         "text": license["text"], "used_by": users})
    if expected - observed or observed - expected - local:
        raise ValueError(f"cargo-about coverage mismatch: missing={expected - observed}, unexpected={observed - expected - local}")
    return licenses


def apk_notices(output, native, rust_docs, ndk, base, components, licenses):
    bundle = output / "apk"
    bundle.mkdir()
    sources = [
        ("Zeroute MIT license", Path("LICENSE"), "zeroute.txt",
         "https://github.com/rcouto/zeroute/releases"),
        ("ZeroTier MPL 2.0", native / "LICENSE-MPL.txt", "zerotier-mpl.txt",
         base.replace("/blob/", "/tree/")),
        ("ZeroTier license boundary", native / "LICENSE.txt", "zerotier-boundary.txt", base + "LICENSE.txt"),
        ("Prometheus C++ lite", native / "ext/prometheus-cpp-lite-1.0/LICENSE", "prometheus.txt",
         base + "ext/prometheus-cpp-lite-1.0/LICENSE"),
        ("Rust standard library", rust_docs / "COPYRIGHT-library.html", "rust.html",
         "https://github.com/rust-lang/rust/tree/" + subprocess.check_output(
             ["rustc", "--version", "--verbose"], text=True).split("commit-hash: ", 1)[1].splitlines()[0]),
        ("Rust MIT terms", rust_docs / "licenses/MIT.txt", "rust-mit.txt", None),
        ("Rust Apache 2.0 terms", rust_docs / "licenses/Apache-2.0.txt", "rust-apache.txt", None),
    ]
    entries = []

    def add(title, filename, text, origin=None):
        if not text.strip():
            raise ValueError(f"empty APK notice: {filename}")
        if filename.endswith(".txt"):
            filename = filename.removesuffix(".txt") + ".html"
            text = html_page(title, f"<h1>{html.escape(title)}</h1><pre>{html.escape(text)}</pre>")
        (bundle / filename).write_text(text, encoding="utf-8")
        link = f'<li><a href="{filename}">{html.escape(title)}</a>'
        if origin:
            link += f' · <a href="{html.escape(origin)}">Source</a>'
        entries.append((filename, link + "</li>"))

    for title, path, filename, origin in sources:
        add(title, filename, path.read_text(encoding="utf-8"), origin)
    for file in ("ECC.cpp", "SHA512.cpp", "Poly1305.cpp", "Salsa20.cpp", "Salsa20.hpp"):
        source = f"node/{file}"
        text = (native / source).read_text().split("#include", 1)[0]
        # The header can place its include guard between the attribution and includes.
        text = text.split("#ifndef", 1)[0].strip()
        if "public domain" not in text.lower():
            raise ValueError(f"public-domain attribution changed: {source}")
        add(file + " attribution", file + ".txt", text, base + source)
    packet = (native / "node/Packet.cpp").read_text()
    embedded = packet.split("/* lz4.h", 1)[1].split("*/", 1)[1].lstrip()
    lz4 = embedded.split("*/", 1)[0] + "*/"
    if not all(marker in lz4 for marker in ("Yann Collet", "BSD 2-Clause License", "THIS SOFTWARE IS PROVIDED")):
        raise ValueError("embedded LZ4 notice boundary changed")
    add("Embedded LZ4", "lz4.txt", lz4, base + "node/Packet.cpp#L85")

    coverage = json.loads(Path("packaging/notices/ndk-coverage.json").read_text())
    version_line = f"Pkg.Revision = {coverage['ndk_version']}"
    if version_line not in (ndk / "source.properties").read_text().splitlines():
        raise ValueError("NDK revision changed; review APK runtime license sources")
    inputs = json.loads(Path(os.environ["ndkLicenseInputs"]).read_text())
    llvm = "https://android.googlesource.com/toolchain/llvm-project/+/" + coverage["llvm_revision"]
    for component, path in sorted(inputs.items()):
        text = base64.b64decode(Path(path).read_bytes(), validate=True).decode("utf-8")
        add(component, component + ".txt", text, f"{llvm}/{component}/LICENSE.TXT")

    crate_rows = {}
    for component in components:
        name, version = component["name"], component["version"]
        url = f"https://static.crates.io/crates/{name}/{name}-{version}.crate"
        label = html.escape(f"{name} {version}")
        crate_rows[(name, version)] = (
            component["role"], f'<a href="{html.escape(url)}">{label}</a>'
        )
    cargo = ["<h1>Rust crate notices</h1>"]
    for license in licenses:
        users = [
            crate_rows[(user["name"], user["version"])]
            for user in license["used_by"] if (user["name"], user["version"]) in crate_rows
        ]
        if not users:
            continue
        cargo.append(f'<h2>{html.escape(license["name"])}</h2>')
        for role, label in (("android-runtime", "Android runtime"),
                            ("build-or-procedural-macro", "Build and procedural macros")):
            links = ", ".join(link for user_role, link in users if role == user_role)
            if links:
                cargo.append(f"<p>{label}: {links}</p>")
        cargo.append(f'<pre>{html.escape(license["text"])}</pre>')
    add("Rust crate notices", "cargo.html", html_page("Rust crate notices", "".join(cargo)))

    intro = ("<h1>Open source notices</h1><p>ZeroTier covered source is available "
             "under the Mozilla Public License 2.0.</p><ul>")
    index = html_page("Open source notices", intro + "".join(link for _, link in entries) + "</ul>")
    (bundle / "index.html").write_text(index, encoding="utf-8")
    return {filename: digest(bundle / filename) for filename in sorted(
        ["index.html", *(filename for filename, _ in entries)])}


def html_page(title, body):
    return ('<!doctype html><html lang="en"><head><meta charset="utf-8">'
            '<meta name="viewport" content="width=device-width, initial-scale=1">'
            f'<title>{html.escape(title)}</title>'
            '<style>body{font:1rem sans-serif;margin:1rem}li{margin:.65em 0}'
            'pre{white-space:pre-wrap;overflow-wrap:anywhere}</style></head>'
            f'<body>{body}</body></html>\n')


def main():
    output = Path(sys.argv[1])
    root = Path.cwd()
    android_scope = check_android_coverage(root)
    native_coverage = json.loads((root / "packaging/notices/native-coverage.json").read_text())
    if (native_coverage["revision"] != os.environ["nativeRevision"]
            or native_coverage["make_flags"] != json.loads(os.environ["nativeBuildFlags"])):
        raise ValueError("native source/build flags changed; review embedded dependency notices")
    metadata = json.loads(Path("metadata.json").read_text())
    resolved = {node["id"] for node in metadata["resolve"]["nodes"]}
    packages = [p for p in metadata["packages"] if p["id"] in resolved]
    runtime = runtime_ids(metadata)
    licenses = clean_licenses(json.loads(Path("about.json").read_text()), packages)
    texts = "\n".join(license["text"] for license in licenses)
    exceptions = json.loads(Path(os.environ["noticeExceptions"]).read_text())
    for exception in exceptions:
        mit = next(file for file in exception["files"] if file["path"] == "LICENSE-MIT")
        if Path(mit["source"]).read_text().strip() not in texts:
            raise ValueError("pinned JNI copyright attribution missing from generated report")
    (output / "cargo.json").write_text(json.dumps({"licenses": licenses}, indent=2) + "\n")
    locked = tomllib.loads((root / "Cargo.lock").read_text())
    checksums = {(p["name"], p["version"]): p["checksum"]
                 for p in locked["package"] if "checksum" in p}
    components = [{"name": p["name"], "version": p["version"],
                   "archive_sha256": checksums[(p["name"], p["version"])],
                   "license_expression": p["license"],
                   "role": "android-runtime" if p["id"] in runtime else "build-or-procedural-macro"}
                  for p in packages if p["source"]]
    notices = []

    def preserve(source, relative, component, scope, origin):
        destination = output / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        notices.append({"file": relative, "component": component, "scope": scope,
                        "origin": origin, "sha256": digest(destination)})

    preserve(root / "LICENSE", "native/zeroute-LICENSE", "Zeroute", "owned source", "LICENSE")
    native = Path(os.environ["nativeSource"])
    revision = os.environ["nativeRevision"]
    base = f"https://github.com/zerotier/ZeroTierOne/blob/{revision}/"
    for name in ("LICENSE.txt", "LICENSE-MPL.txt", "ext/prometheus-cpp-lite-1.0/LICENSE",
                 "ext/prometheus-cpp-lite-1.0/3rdparty/http-client-lite/LICENSE"):
        preserve(native / name, "native/zerotier/" + name, "ZeroTier/embedded third-party code",
                 "source notice; not every upstream file is linked", base + name)
    for name in ("ECC", "SHA512", "Poly1305"):
        source = f"node/{name}.cpp"
        preamble = (native / source).read_text().split("#include", 1)[0].rstrip() + "\n"
        if "public domain" not in preamble.lower():
            raise ValueError(f"upstream crypto notice boundary changed: {name}")
        relative = f"native/zerotier/{name}-preamble.txt"
        path = output / relative
        path.write_text(preamble)
        notices.append({"file": relative, "component": name, "scope": "source preamble",
                        "origin": base + source, "sha256": digest(path)})
    rust_docs = Path(os.environ["rustDistribution"]) / "share/doc/rust"
    preserve(rust_docs / "COPYRIGHT-library.html", "runtime/rust-COPYRIGHT-library.html", "Rust standard library",
             "distribution notice including build/target-specific components", "pinned Rust distribution")
    ndk = Path(os.environ["ndkRoot"])
    for name in ("NOTICE", "NOTICE.toolchain"):
        preserve(ndk / name, "runtime/ndk-" + name, "Android NDK",
                 "distribution notice broader than the linked runtime archives", "compiler-selected NDK distribution")
    archives = []
    for name in ("c++_static", "c++abi", "unwind", "clang_rt.builtins-aarch64-android"):
        path = Path(subprocess.check_output([os.environ["androidCxx"],
                    f"--print-file-name=lib{name}.a"], text=True).strip())
        if not path.is_absolute() or not path.is_file():
            raise ValueError(f"compiler-selected runtime archive missing: {name}")
        archives.append({"name": f"lib{name}.a", "sha256": digest(path)})
    manifest = {"schema": 1, "scope": "Notice-source review and selected APK notice assets",
                "android_scope": android_scope, "cargo_components": components,
                "native_revision": revision, "notices": notices, "ndk_runtime_archives": archives,
                "rust_version": subprocess.check_output(["rustc", "--version"], text=True).strip(),
                "ndk_source_properties": (ndk / "source.properties").read_text(),
                "cargo_lock_sha256": digest(root / "Cargo.lock"),
                "apk_notice_sha256": apk_notices(output, native, rust_docs, ndk, base, components, licenses)}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    sections = ["<!doctype html><html><head><meta charset='utf-8'><title>Zeroute notice sources</title></head><body>",
                "<h1>Zeroute notice-source review</h1>",
                "<p>Generated from pinned sources. <a href='apk/index.html'>Selected APK notices</a>. "
                "Rust crates include Android build/macro attributions. Runtime distribution notices "
                "are broader than the linked APK components and require review before packaging.</p>",
                "<h2>Native and runtime notice sources</h2><ul>"]
    for notice in notices:
        sections.append(f'<li><a href="{html.escape(notice["file"])}">{html.escape(notice["component"])}</a>: {html.escape(notice["scope"])}</li>')
    sections.append("</ul><h2>Rust crate notices</h2>")
    for license in licenses:
        names = ", ".join(f'{p["name"]} {p["version"]}' for p in license["used_by"])
        sections.append(f'<h3>{html.escape(license["name"])}</h3><p>{html.escape(names)}</p><pre>{html.escape(license["text"])}</pre>')
    sections.append("</body></html>\n")
    (output / "index.html").write_text("\n".join(sections))


if __name__ == "__main__":
    main()
