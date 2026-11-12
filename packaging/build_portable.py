#!/usr/bin/env python3
"""Build single-file portable Dado Player.

Steps:
  1. Generate assets/icon.ico from assets/icon.png (multi-size).
  2. Build launcher with embedded logo.
  3. (Optional) Build launcher's icon resource (.res) with rc.exe.
  4. Compress the 11 dist/ files (app + 10 DLLs), append package + footer.

The launcher and main app are expected to already contain their Windows icon
resources via packaging/inject_icon.py.

Output: packaging/out/Dado-Player.exe
"""
import hashlib
import ctypes
import importlib.util
import os
import struct
import subprocess
import sys
from pathlib import Path
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIST = os.path.join(ROOT, "dist")
LAUNCHER_MANIFEST = os.path.join(ROOT, "packaging", "launcher", "Cargo.toml")
OUT_DIR = os.path.join(ROOT, "packaging", "out")
os.makedirs(OUT_DIR, exist_ok=True)

# MSVC / Windows SDK tools.
RC = r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\rc.exe"

FILES = [
    "Dado-Player.exe",
    "libmpv-2.dll",
    "vulkan-1.dll",
    "VCRUNTIME140.dll",
    "VCRUNTIME140_1.dll",
    "msvcp140.dll",
    "msvcp140_1.dll",
    "msvcp140_2.dll",
    "msvcp_win.dll",
    "concrt140.dll",
    "d3dcompiler_47.dll",
]


def export_logo():
    print("[0/4] exporting logo (assets/icon.png -> launcher/logo.rgba + assets/icon.ico) ...")
    spec = importlib.util.spec_from_file_location(
        "export_logo", os.path.join(ROOT, "packaging", "export_logo.py")
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    rc = mod.main()
    if rc != 0:
        sys.exit("Logo-Export fehlgeschlagen")

    spec = importlib.util.spec_from_file_location(
        "make_icon", os.path.join(ROOT, "packaging", "make_icon.py")
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    rc = mod.main()
    if rc != 0:
        sys.exit("ICO-Export fehlgeschlagen")


def build_launcher():
    env = dict(os.environ)
    env["RUSTFLAGS"] = "-C target-feature=+crt-static"
    print("[1/4] building launcher (crt-static) ...")
    r = subprocess.run(
        ["cargo", "build", "--release", "--offline", "--manifest-path", LAUNCHER_MANIFEST],
        env=env,
    )
    if r.returncode != 0:
        sys.exit("Launcher-Build fehlgeschlagst")
    p = os.path.join(ROOT, "packaging", "launcher", "target", "release", "dado-launcher.exe")
    if not os.path.exists(p):
        sys.exit(f"Launcher nicht erzeugt: {p}")
    return p


def build_icon_resource(launcher_exe: str) -> str:
    print("[2/4] building launcher icon resource (rc.exe) ...")
    res = os.path.join(os.path.dirname(launcher_exe), "dado-icon.res")
    rc = subprocess.run(
        [RC, "/nologo", "/fo", res, os.path.join(ROOT, "packaging", "resource.rc")],
    )
    if rc.returncode != 0:
        sys.exit("rc.exe fehlgeschlagen")
    if not os.path.exists(res):
        sys.exit(f"Resource nicht erzeugt: {res}")
    return res


def apply_icon_resource(exe_path: str, res_path: str) -> str:
    """Apply a compiled .res to the launcher EXE using mt.exe.

    mt.exe's -inputresource accepts any resource type (including RT_ICON /
    RT_GROUP_ICON) from a compiled .res and merges it into the output PE.
    """
    MT = r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\mt.exe"
    if not os.path.exists(MT):
        print(f"  NOTE: mt.exe not found, launcher icon not embedded")
        return exe_path
    out_path = exe_path + ".withicon"
    r = subprocess.run(
        [MT, "-nologo", "-inputresource", res_path, "-outputresource", out_path],
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"  NOTE: mt.exe failed (rc={r.returncode}): {r.stderr.strip()[:120]}")
        print("  Falling back to launcher without embedded icon resource.")
        return exe_path
    if os.path.exists(out_path):
        os.replace(out_path, exe_path)
        print(f"  -> icon resource embedded via mt.exe")
        return exe_path
    return exe_path


def build_package():
    print("[3/4] compressing runtime ...")
    entries = []
    for name in FILES:
        path = os.path.join(DIST, name)
        if not os.path.exists(path):
            sys.exit(f"FEHLT: {path}")
        data = open(path, "rb").read()
        entries.append((name, data, zlib.compress(data, 9)))

    blob = bytearray()
    blob += b"DADOBLOB"
    blob += struct.pack("<I", len(entries))
    for name, data, comp in entries:
        nb = name.encode("utf-8")
        blob += struct.pack("<H", len(nb))
        blob += nb
        blob += struct.pack("<Q", len(data))
        blob += struct.pack("<I", zlib.crc32(data) & 0xFFFFFFFF)
        blob += struct.pack("<Q", len(comp))
        blob += comp
    blob = bytes(blob)

    content_id = hashlib.sha256(blob).hexdigest()[:12].encode("ascii").ljust(24, b" ")
    footer = bytearray()
    footer += b"DADOPK01"
    footer += struct.pack("<Q", 0)
    footer += struct.pack("<Q", len(blob))
    footer += struct.pack("<I", 1)
    footer += content_id[:24]
    footer += struct.pack("<I", 0)
    footer += struct.pack("<I", 0)
    footer += struct.pack("<I", 0)
    return blob, footer, entries, content_id.decode().strip()


def main():
    export_logo()
    launcher = build_launcher()
    # Inject the Windows icon resource into the launcher BEFORE packaging so the
    # standalone inherits it.  Injecting into the already-built standalone (which
    # is launcher + appended blob + footer) would truncate the overlay via
    # EndUpdateResourceW and destroy the embedded package.  See packaging/
    # inject_icon.py for the UpdateResource-based injection.
    spec = importlib.util.spec_from_file_location(
        "inject_icon", os.path.join(ROOT, "packaging", "inject_icon.py")
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    if not mod.inject_icon(
        Path(launcher), Path(os.path.join(ROOT, "assets", "icon.ico"))
    ):
        sys.exit("Launcher icon injection failed; refusing to package an unverified artifact")
    # `dist/Dado-Player.exe` is overwritten from target/release after every
    # cargo build, which strips its icon resource. Re-inject before packaging
    # so the extracted runtime keeps its own icon (window/taskbar), not only
    # the launcher (Explorer). UpdateResource replaces ID 1, so this is safe
    # to repeat on every build.
    dist_exe = Path(os.path.join(DIST, "Dado-Player.exe"))
    if dist_exe.exists() and not mod.inject_icon(
        dist_exe, Path(os.path.join(ROOT, "assets", "icon.ico"))
    ):
        sys.exit("Application icon injection failed; refusing to package an unverified artifact")
    blob, footer, entries, content_id = build_package()

    launcher_data = open(launcher, "rb").read()
    struct.pack_into("<Q", footer, 8, len(launcher_data))
    footer = bytes(footer)
    crc = zlib.crc32(footer[:60]) & 0xFFFFFFFF
    footer = footer[:60] + struct.pack("<I", crc)

    out = launcher_data + blob + footer
    out_path = os.path.join(OUT_DIR, "Dado-Player.exe")
    with open(out_path, "wb") as f:
        f.write(out)

    raw = sum(len(d) for _, d, _ in entries)
    print()
    print(f"  launcher          : {len(launcher_data):>12,} B")
    print(f"  package (raw)     : {raw:>12,} B")
    print(f"  package (deflate) : {len(blob):>12,} B")
    print(f"  final exe         : {len(out):>12,} B")
    print(f"  content id        : {content_id}")
    print()
    print(f"-> {out_path}")


if __name__ == "__main__":
    main()
