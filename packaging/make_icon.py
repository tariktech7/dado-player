#!/usr/bin/env python3
"""Generate a multi-size Windows .ico from assets/icon.png.

Sizes: 16x16, 32x32, 48x48, 64x64, 128x128, 256x256.
Each image is stored as a PNG blob inside the .ico (modern Windows format).
Output: assets/icon.ico
"""
import io
import struct
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "assets" / "icon.png"
OUT = ROOT / "assets" / "icon.ico"

SIZES = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]


def main() -> int:
    img = Image.open(SRC).convert("RGBA")

    # Build PNG blob for each size.
    png_blobs = []
    for size in SIZES:
        resized = img.resize(size, Image.LANCZOS)
        buf = io.BytesIO()
        resized.save(buf, format="PNG")
        png_blobs.append(buf.getvalue())

    # ICO header: reserved=0, type=1, count=N
    out = io.BytesIO()
    out.write(struct.pack("<HHH", 0, 1, len(png_blobs)))

    # Directory entries, then image data after all entries.
    entry_size = 16
    data_offset = 6 + entry_size * len(png_blobs)
    for i, blob in enumerate(png_blobs):
        w, h = SIZES[i]
        # 0 width/height means 256 in ICO directory
        ico_w = 0 if w == 256 else w
        ico_h = 0 if h == 256 else h
        out.write(struct.pack("<BBBBHHII",
            ico_w,           # bWidth
            ico_h,           # bHeight
            0,               # bColorCount
            0,               # bReserved
            0,               # wPlanes
            32,              # wBitCount
            len(blob),       # dwBytesInRes
            data_offset,     # dwImageOffset
        ))
        data_offset += len(blob)

    # Image data (PNG blobs)
    for blob in png_blobs:
        out.write(blob)

    OUT.write_bytes(out.getvalue())

    # Verify by re-opening the ICO and checking the directory count.
    raw = OUT.read_bytes()
    count = struct.unpack_from("<H", raw, 4)[0]
    print(f"wrote {OUT} ({count} images)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
