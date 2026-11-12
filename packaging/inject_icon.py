#!/usr/bin/env python3
"""Inject Windows icon resources (RT_GROUP_ICON + RT_ICON) into PE executables.

Uses the Windows UpdateResource API (kernel32.BeginUpdateResource/UpdateResource/
EndUpdateResource) so no external resource compiler (windres/rc.exe) is needed.

Reads a multi-size .ico (assets/icon.ico) and injects:
  * RT_GROUP_ICON (type 14, ID 1) — the group directory with one entry per image
  * RT_ICON     (type 3,  IDs 1..N) — the raw image data for each size

Targets:
  * dist/Dado-Player.exe        (main binary)
  * packaging/out/Dado-Player.exe (standalone single-file launcher)
"""
import ctypes
from ctypes import wintypes
import struct
import sys
from pathlib import Path

kernel32 = ctypes.windll.kernel32

# --- Windows API signatures ---
kernel32.BeginUpdateResourceW.restype = wintypes.HANDLE
kernel32.BeginUpdateResourceW.argtypes = [wintypes.LPCWSTR, wintypes.BOOL]

kernel32.UpdateResourceW.restype = wintypes.BOOL
kernel32.UpdateResourceW.argtypes = [
    wintypes.HANDLE,   # hUpdate
    ctypes.c_void_p,   # lpType  (integer ID via IS_INTRESOURCE)
    ctypes.c_void_p,   # lpName  (integer ID via IS_INTRESOURCE)
    wintypes.WORD,     # wLanguage
    wintypes.LPVOID,   # lpData
    wintypes.DWORD,    # cbData
]

kernel32.EndUpdateResourceW.restype = wintypes.BOOL
kernel32.EndUpdateResourceW.argtypes = [wintypes.HANDLE, wintypes.BOOL]

# Resource type IDs
RT_ICON = 3
RT_GROUP_ICON = 14

# Language ID: LANG_NEUTRAL / SUBLANG_NEUTRAL
LANGID = 0


def parse_ico(ico_path: Path):
    """Parse a .ico file. Returns list of dicts with width, height, planes,
    bit_count, and raw image data (DIB header + pixels)."""
    data = ico_path.read_bytes()
    reserved = struct.unpack_from("<H", data, 0)[0]
    itype = struct.unpack_from("<H", data, 2)[0]
    count = struct.unpack_from("<H", data, 4)[0]
    if itype != 1:
        raise ValueError(f"Not an ICO file (type={itype})")

    images = []
    for i in range(count):
        off = 6 + i * 16
        width = data[off]
        height = data[off + 1]
        # bColorCount = data[off + 2]  # unused
        # bReserved   = data[off + 3]  # unused
        planes = struct.unpack_from("<H", data, off + 4)[0]
        bit_count = struct.unpack_from("<H", data, off + 6)[0]
        bytes_in_res = struct.unpack_from("<I", data, off + 8)[0]
        image_offset = struct.unpack_from("<I", data, off + 12)[0]

        image_data = data[image_offset:image_offset + bytes_in_res]
        # ICO stores 0 for 256px; normalize to 256 for display but 0 in GRPICONDIR
        images.append({
            "width": width if width != 0 else 256,
            "height": height if height != 0 else 256,
            "ico_width": width,   # 0 means 256 in ICO/GRPICONDIR
            "ico_height": height, # 0 means 256 in ICO/GRPICONDIR
            "planes": planes,
            "bit_count": bit_count,
            "data": image_data,
        })
    return images


def build_grpicon_data(images, base_id=1):
    """Build RT_GROUP_ICON resource payload: GRPICONDIR + GRPICONDIRENTRY[]."""
    buf = struct.pack("<HHH", 0, 1, len(images))  # reserved, type=1, count
    for i, img in enumerate(images):
        w = img["ico_width"]   # 0 for 256px
        h = img["ico_height"]  # 0 for 256px
        buf += struct.pack("<BBBBHHIH",
            w,                # bWidth  (0 = 256)
            h,                # bHeight (0 = 256)
            0,                # bColorCount
            0,                # bReserved
            img["planes"],    # wPlanes
            img["bit_count"], # wBitCount
            len(img["data"]), # dwBytesInRes
            base_id + i,      # nID -> RT_ICON resource ID
        )
    return buf


def inject_icon(exe_path: Path, ico_path: Path):
    images = parse_ico(ico_path)

    grpicon_data = build_grpicon_data(images)
    icon_data_list = [img["data"] for img in images]

    hUpdate = kernel32.BeginUpdateResourceW(str(exe_path), False)
    if not hUpdate or hUpdate == ctypes.c_void_p(0).value:
        err = ctypes.GetLastError()
        print(f"  FAIL: BeginUpdateResource failed for {exe_path} (err={err})")
        return False

    try:
        # RT_GROUP_ICON (type 14, ID 1)
        ok = kernel32.UpdateResourceW(
            hUpdate,
            ctypes.c_void_p(RT_GROUP_ICON),
            ctypes.c_void_p(1),
            LANGID,
            grpicon_data,
            len(grpicon_data),
        )
        if not ok:
            err = ctypes.GetLastError()
            print(f"  FAIL: UpdateResource RT_GROUP_ICON failed (err={err})")
            return False
        print(f"  OK: RT_GROUP_ICON (ID 1) — {len(grpicon_data)} bytes")

        # RT_ICON (type 3, IDs 1..N)
        for i, icon_data in enumerate(icon_data_list):
            res_id = i + 1
            ok = kernel32.UpdateResourceW(
                hUpdate,
                ctypes.c_void_p(RT_ICON),
                ctypes.c_void_p(res_id),
                LANGID,
                icon_data,
                len(icon_data),
            )
            if not ok:
                err = ctypes.GetLastError()
                print(f"  FAIL: UpdateResource RT_ICON ID {res_id} failed (err={err})")
                return False
            print(f"  OK: RT_ICON (ID {res_id}) — {len(icon_data)} bytes")

        ok = kernel32.EndUpdateResourceW(hUpdate, False)
        if not ok:
            err = ctypes.GetLastError()
            print(f"  FAIL: EndUpdateResource failed (err={err})")
            return False

        print(f"  -> Icon resources injected into {exe_path.name}")
        return True
    except Exception as e:
        try:
            kernel32.EndUpdateResourceW(hUpdate, True)
        except Exception:
            pass
        print(f"  FAIL: Exception injecting icon: {e}")
        return False


def main():
    ico_path = Path("assets/icon.ico")
    if not ico_path.exists():
        print(f"ERROR: {ico_path} not found — run packaging/make_icon.py first.")
        sys.exit(1)

    targets = [
        Path("dist/Dado-Player.exe"),
        Path("packaging/launcher/target/release/dado-launcher.exe"),
        Path("packaging/out/Dado-Player.exe"),
    ]

    all_ok = True
    for target in targets:
        if not target.exists():
            print(f"SKIP: {target} not found")
            all_ok = False
            continue
        print(f"Processing {target} ...")
        if not inject_icon(target, ico_path):
            all_ok = False

    if not all_ok:
        sys.exit(1)
    print("\nAll icon injections complete.")


if __name__ == "__main__":
    main()
